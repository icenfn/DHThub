<script setup lang="ts">
// 应用外壳：仅承载全局逻辑（主题 / 更新提示），页面框架由各路由页面自行提供
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useTheme } from 'vuetify'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
import {
  checkUpdate,
  openReleasePage,
  updateHasNew,
  updateMsg,
  updateToast,
} from './lib/update'

const theme = useTheme()
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

async function init() {
  await settings.ready()
  if (typeof window.matchMedia === 'function') {
    mq.value = window.matchMedia('(prefers-color-scheme: dark)')
    mq.value.addEventListener('change', onSystemThemeChange)
  }
  applyTheme()
  // 自动检测更新：启动即静默检查（snackbar 提示）
  if (isTauri && settings.get('autoCheckUpdate')) {
    void checkUpdate()
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
</script>

<template>
  <v-app>
    <router-view />

    <!-- 更新检测提示（snackbar，无弹窗） -->
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
