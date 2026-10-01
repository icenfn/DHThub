<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
import UpdateDialog from './components/UpdateDialog.vue'

const theme = useTheme()
const router = useRouter()
const route = useRoute()
const showAgreement = ref(false)
const mq = ref<MediaQueryList | null>(null)
const updateOpen = ref(false)

const navItems = [
  { to: '/', icon: 'mdi-magnify', label: '搜索' },
  { to: '/sites', icon: 'mdi-antenna', label: '站点' },
  { to: '/history', icon: 'mdi-history', label: '历史' },
  { to: '/settings', icon: 'mdi-cog-outline', label: '设置' },
]

const isAgreementPage = computed(() => route.name === 'agreement' || route.name === 'disclaimer')
const appTitle = computed(() => String(route.meta.title ?? 'DHThub'))

async function applyTheme() {
  await settings.ready()
  const mode = settings.get('theme')
  if (mode === 'system') {
    theme.global.name.value = mq.value?.matches ? 'dark' : 'light'
  } else {
    theme.global.name.value = mode
  }
}

function onSystemThemeChange(e: MediaQueryListEvent) {
  if (settings.get('theme') === 'system') {
    theme.global.name.value = e.matches ? 'dark' : 'light'
  }
}

async function init() {
  await settings.ready()
  if (typeof window.matchMedia === 'function') {
    mq.value = window.matchMedia('(prefers-color-scheme: dark)')
    mq.value.addEventListener('change', onSystemThemeChange)
  }
  applyTheme()
  // 首启协议（B3）
  if (!settings.get('agreedVersion') && isTauri) {
    showAgreement.value = true
  }
  // 浏览器预览提示
  if (!isTauri) {
    console.info('[DHThub] 浏览器预览模式：搜索/站点功能需在 Tauri 应用中运行')
  }
}

watch(
  () => settings.get('theme'),
  () => applyTheme(),
)

onMounted(init)
onBeforeUnmount(() => {
  mq.value?.removeEventListener('change', onSystemThemeChange)
})

async function acceptAgreement() {
  await settings.set('agreedVersion', '1')
  showAgreement.value = false
  silentCheckUpdate()
}

// 启动静默更新检测（A7）：发现新版本才弹窗
async function silentCheckUpdate() {
  if (!isTauri) return
  try {
    const { invoke } = await import('./lib/tauri')
    const info = await invoke<{ has_update: boolean }>('check_update')
    if (info.has_update) updateOpen.value = true
  } catch {
    /* 静默失败不打扰用户 */
  }
}

function go(to: string) {
  router.push(to)
}
</script>

<template>
  <v-app>
    <v-navigation-drawer
      v-if="!isAgreementPage"
      class="d-none d-sm-flex"
      width="210"
      :permanent="true"
      color="background"
    >
      <div class="d-flex align-center ga-3 px-4 py-4">
        <v-icon icon="mdi-flash-outline" color="primary" size="30" />
        <div>
          <div class="text-h6 font-weight-bold">DHThub</div>
          <div class="text-caption text-medium-emphasis">磁力聚合搜索</div>
        </div>
      </div>
      <v-divider />
      <v-list nav density="comfortable">
        <v-list-item
          v-for="item in navItems"
          :key="item.to"
          :active="route.path === item.to"
          :prepend-icon="item.icon"
          :title="item.label"
          @click="go(item.to)"
        />
      </v-list>
      <template #append>
        <div class="pa-4 text-caption text-medium-emphasis">
          v0.1.0 · GitHub 发布
        </div>
      </template>
    </v-navigation-drawer>

    <v-main class="pb-16 pb-sm-0">
      <router-view />
    </v-main>

    <v-bottom-navigation
      v-if="!isAgreementPage"
      class="d-flex d-sm-none"
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

    <!-- 首启协议（B3） -->
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
