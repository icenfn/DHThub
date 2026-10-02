<script setup lang="ts">
// 应用外壳：仅承载全局逻辑（主题 / 更新提示），页面框架由各路由页面自行提供
import { onMounted, ref, watch } from 'vue'
import { useTheme } from 'vuetify'
import { useMediaQuery } from '@vueuse/core'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
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
