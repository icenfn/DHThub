<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDisplay, useTheme } from 'vuetify'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
import { speedTestAll, pickFastest } from './lib/mirrors'
import { debugLog } from './lib/debug'
import UpdateDialog from './components/UpdateDialog.vue'

const theme = useTheme()
const display = useDisplay()
const router = useRouter()
const route = useRoute()
const showAgreement = ref(false)
const mq = ref<MediaQueryList | null>(null)
const updateOpen = ref(false)

// 主导航：搜索 / 站点 / 历史（设置页为独立页面，经顶栏图标路由跳转）
const navItems = [
  { to: '/', icon: 'mdi-magnify', label: '搜索' },
  { to: '/sites', icon: 'mdi-antenna', label: '站点' },
  { to: '/history', icon: 'mdi-history', label: '历史' },
]

const isAgreementPage = computed(() => route.name === 'agreement' || route.name === 'disclaimer')

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
    debugLog('[镜像] 自动测速结果：', results)
    if (best) {
      debugLog(`[镜像] 已自动选择：${best.base || '直连'}（${best.latency}ms）`)
      await settings.selectMirror(best.id)
    }
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
    // 已同意过协议且开启自动更新：启动即静默检测
    void silentCheckUpdate()
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
  if (settings.get('autoCheckUpdate')) silentCheckUpdate()
}

// 启动静默更新检测：发现新版本才弹窗（使用当前选中镜像）
async function silentCheckUpdate() {
  if (!isTauri || !settings.get('autoCheckUpdate')) return
  try {
    const { invoke } = await import('./lib/tauri')
    const mirror = settings.getSelectedMirror()
    const info = await invoke<{ has_update: boolean }>('check_update', {
      mirrorBase: mirror.base,
    })
    debugLog('[更新] 检测结果：', info)
    if (info.has_update) updateOpen.value = true
  } catch (e) {
    debugLog('[更新] 检测失败：', e)
    /* 静默失败不打扰用户 */
  }
}

function go(to: string) {
  router.push(to)
}
</script>

<template>
  <v-app>
    <!-- MD3 顶部应用栏 -->
    <v-app-bar v-if="!isAgreementPage" color="surface" border="b" height="56">
      <template #prepend>
        <div class="d-flex align-center ml-2">
          <v-icon icon="mdi-flash-outline" color="primary" size="28" />
        </div>
      </template>
      <v-app-bar-title>
        <span class="text-subtitle-1 font-weight-bold">DHThub</span>
        <span class="text-caption text-medium-emphasis ml-2 d-none d-sm-inline">磁力聚合搜索</span>
      </v-app-bar-title>
      <template #append>
        <v-btn icon="mdi-update" title="检查更新" variant="text" @click="updateOpen = true" />
        <v-btn
          icon="mdi-cog-outline"
          title="设置"
          variant="text"
          :active="route.path === '/settings'"
          @click="go('/settings')"
        />
      </template>
    </v-app-bar>

    <!-- 桌面端导航抽屉 -->
    <v-navigation-drawer
      v-if="!isAgreementPage && !display.mobile.value"
      width="216"
      :permanent="true"
      color="surface"
    >
      <v-divider class="mx-4 mt-2" />
      <v-list nav density="comfortable" class="px-2 py-2">
        <v-list-item
          v-for="item in navItems"
          :key="item.to"
          :active="route.path === item.to"
          :prepend-icon="item.icon"
          :title="item.label"
          rounded="xl"
          @click="go(item.to)"
        />
      </v-list>
      <template #append>
        <div class="pa-4 text-caption text-medium-emphasis">v0.2.5 · GitHub 发布</div>
      </template>
    </v-navigation-drawer>

    <v-main class="pb-16 pb-sm-0">
      <router-view />
    </v-main>

    <!-- 移动端底部导航 -->
    <v-bottom-navigation
      v-if="!isAgreementPage && display.mobile.value"
      :model-value="route.path"
      color="primary"
      grow
    >
      <v-btn v-for="item in navItems" :key="item.to" :value="item.to" @click="go(item.to)">
        <v-icon>{{ item.icon }}</v-icon>
        {{ item.label }}
      </v-btn>
    </v-bottom-navigation>

    <UpdateDialog v-model="updateOpen" />

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
  </v-app>
</template>
