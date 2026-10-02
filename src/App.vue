<script setup lang="ts">
// 应用外壳：仅承载全局逻辑（主题 / 镜像 / 更新提示 / 首启协议），页面框架由各路由页面自行提供
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
import { speedTestAll, pickFastest } from './lib/mirrors'
import {
  checkUpdate,
  openReleasePage,
  updateHasNew,
  updateMsg,
  updateToast,
} from './lib/update'

const theme = useTheme()
const router = useRouter()
const showAgreement = ref(false)
const mq = ref<MediaQueryList | null>(null)

async function applyTheme() {
  await settings.ready()
  const mode = settings.get('theme')
  if (mode === 'system') {
    theme.change(mq.value?.matches ? 'dark' : 'light')
  } else {
    theme.change(mode)
  }
}

function onSystemThemeChange(e: MediaQueryListEvent) {
  if (settings.get('theme') === 'system') {
    theme.change(e.matches ? 'dark' : 'light')
  }
}

/** 自动模式：后台测速全部镜像并选最快（失败不打扰） */
async function autoSelectMirror() {
  if (settings.get('githubMirrorMode') !== 'auto') return
  try {
    const results = await speedTestAll(settings.getMirrors())
    const best = pickFastest(results)
    if (best) await settings.selectMirror(best.id)
  } catch {
    /* 静默失败 */
  }
}

async function init() {
  await settings.ready()
  if (typeof window.matchMedia === 'function') {
    mq.value = window.matchMedia('(prefers-color-scheme: dark)')
    mq.value.addEventListener('change', onSystemThemeChange)
  }
  applyTheme()
  // 首启协议
  if (!settings.get('agreedVersion') && isTauri) {
    showAgreement.value = true
  } else if (isTauri && settings.get('autoCheckUpdate')) {
    // 已同意过协议且开启自动更新：启动即静默检测（snackbar 提示）
    void checkUpdate()
  }
  // 自动模式启动即测速选最快镜像（后台执行）
  void autoSelectMirror()
  // 浏览器预览提示
  if (!isTauri) {
    console.info('[DHThub] 浏览器预览模式：搜索/站点功能需在 Tauri 应用中运行')
  }
}

watch(
  () => settings.get('theme'),
  () => applyTheme(),
)

watch(
  () => settings.get('githubMirrorMode'),
  (mode) => {
    if (mode === 'auto') void autoSelectMirror()
  },
)

onMounted(init)
onBeforeUnmount(() => {
  mq.value?.removeEventListener('change', onSystemThemeChange)
})

async function acceptAgreement() {
  await settings.set('agreedVersion', '1')
  showAgreement.value = false
  if (settings.get('autoCheckUpdate')) void checkUpdate()
}
</script>

<template>
  <v-app>
    <router-view />

    <!-- 首启协议 -->
    <v-dialog v-model="showAgreement" persistent max-width="560" :scrim="true">
      <v-card>
        <v-card-title class="d-flex align-center">
          <v-icon icon="mdi-shield-check-outline" color="primary" class="mr-2" />
          使用说明与免责声明
        </v-card-title>
        <v-divider />
        <v-card-text style="max-height: 60vh; overflow-y: auto" class="text-body-2">
          <p>欢迎使用 <strong>DHThub</strong>。本应用是一款磁力链接聚合搜索工具，仅用于技术学习与合法信息检索。</p>
          <p class="mt-2">使用前请知悉：</p>
          <ol class="pl-5 mt-1" style="list-style: decimal">
            <li>搜索能力来自第三方公开网页，结果的可用性、时效性与合法性由来源站点决定，DHThub 不存储、不缓存任何资源内容。</li>
            <li>请遵守所在地法律法规，仅检索、传播与下载您拥有合法权利的内容。因使用本工具产生的一切法律风险由使用者自行承担。</li>
            <li>订阅源由用户自行配置，DHThub 对订阅源内容不承担审核与担保责任。</li>
            <li>本应用为开源项目，不包含任何广告与商业追踪。</li>
          </ol>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" color="error" @click="router.push('/disclaimer'); showAgreement = false">
            查看免责声明
          </v-btn>
          <v-btn color="primary" variant="flat" @click="acceptAgreement">同意并进入</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 更新检测提示（snackbar，替代更新弹窗） -->
    <v-snackbar
      v-model="updateToast"
      location="bottom"
      multi-line
      :timeout="updateHasNew ? 8000 : 2500"
      color="surface"
    >
      <div class="d-flex align-center ga-2">
        <v-icon :icon="updateHasNew ? 'mdi-update' : 'mdi-check-circle'" :color="updateHasNew ? 'primary' : 'success'" size="20" />
        <span class="text-body-2">{{ updateMsg }}</span>
      </div>
      <template v-if="updateHasNew" #actions>
        <v-btn color="primary" variant="text" size="small" @click="openReleasePage">前往下载</v-btn>
      </template>
    </v-snackbar>
  </v-app>
</template>
