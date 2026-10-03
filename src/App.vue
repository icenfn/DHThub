<script setup lang="ts">
// 应用外壳：仅承载全局逻辑（主题 / 更新提示 / 返回键），页面框架由各路由页面自行提供
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { useMediaQuery } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
import { closeTopOverlay, pushOverlay } from './lib/overlay'
import {
  checkUpdate,
  openReleasePage,
  updateHasNew,
  updateMsg,
  updateToast,
} from './lib/update'

// 有更新 -> 弹窗通知；无更新/失败 -> snackbar 轻提示
const updateDialog = ref(false)
const updateSnackbar = ref(false)

watch([updateToast, updateHasNew], () => {
  if (!updateToast.value) return
  if (updateHasNew.value) {
    updateDialog.value = true
  } else {
    updateSnackbar.value = true
  }
  updateToast.value = false
})

const theme = useTheme()

const router = useRouter()

// 弹层返回键支持：PC ESC 直接关闭最上层弹层（Vuetify 自身的 ESC 关闭仍生效）；
// Android 硬件返回键由原生 MainActivity 派发 android:back 事件，前端关闭弹层 / 返回历史 / 退出
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && closeTopOverlay()) {
    e.preventDefault()
  }
}
let onAndroidBack: (() => void) | null = null
function setupAndroidBack() {
  if (!isTauri) return
  const handler = async () => {
    // 有弹层先关弹层
    if (closeTopOverlay()) return
    // 有历史记录则返回上一页
    if (window.history.length > 1) {
      router.back()
      return
    }
    // 兜底退出应用
    try {
      await invoke('exit_app')
    } catch {
      /* 桌面端无此命令时忽略 */
    }
  }
  window.addEventListener('android:back', handler)
  onAndroidBack = () => window.removeEventListener('android:back', handler)
}
onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  setupAndroidBack()
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  onAndroidBack?.()
})

// 更新弹窗本身也注册到弹层栈（关闭时自动注销）
watch(updateDialog, (v, _o, onCleanup) => {
  if (v) {
    const off = pushOverlay(() => {
      updateDialog.value = false
    })
    onCleanup(off)
  }
})
const prefersDark = useMediaQuery('(prefers-color-scheme: dark)')

async function applyTheme() {
  await settings.ready()
  const mode = settings.get('theme')
  if (mode === 'system') {
    theme.change(prefersDark.value ? 'dark' : 'light')
  } else {
    theme.change(mode)
  }
}

watch(prefersDark, (v) => {
  if (settings.get('theme') === 'system') theme.change(v ? 'dark' : 'light')
})

watch(
  () => settings.get('theme'),
  () => applyTheme(),
)

onMounted(async () => {
  await settings.ready()
  applyTheme()
  // 自动检测更新：启动即静默检查（snackbar 提示）
  if (isTauri && settings.get('autoCheckUpdate')) {
    void checkUpdate()
  }
  // 浏览器预览提示
  if (!isTauri) {
    console.info('[DHThub] 浏览器预览模式：搜索/站点功能需在 Tauri 应用中运行')
  }
})
</script>

<template>
  <v-app>
    <router-view />

    <!-- 发现新版本：弹窗通知 -->
    <v-dialog v-model="updateDialog" max-width="420">
      <v-card rounded="lg">
        <v-card-title class="d-flex align-center ga-2">
          <v-icon icon="mdi-update" color="primary" />
          <span class="text-subtitle-1 font-weight-bold">发现新版本</span>
        </v-card-title>
        <v-divider />
        <v-card-text class="pt-4">{{ updateMsg }}</v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="updateDialog = false">稍后</v-btn>
          <v-btn color="primary" variant="flat" @click="openReleasePage">前往下载</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 无更新 / 检查失败：snackbar 轻提示 -->
    <v-snackbar v-model="updateSnackbar" location="bottom" multi-line :timeout="2500" color="surface">
      <div class="d-flex align-center ga-2">
        <v-icon icon="mdi-check-circle" color="success" size="20" />
        <span class="text-body-2">{{ updateMsg }}</span>
      </div>
    </v-snackbar>
  </v-app>
</template>

<style>
/* 长按 UI 文字不弹系统选择框；输入框内仍可选择/编辑 */
* {
  -webkit-user-select: none;
  user-select: none;
  -webkit-touch-callout: none;
}
input,
textarea,
[contenteditable='true'] {
  -webkit-user-select: text;
  user-select: text;
  -webkit-touch-callout: default;
}
</style>
