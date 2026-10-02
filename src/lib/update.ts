// 更新检测（轻量版）：releases/latest 解析 + snackbar 提示，替代笨重的更新弹窗

import { ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke, isTauri } from './tauri'
import { settings } from '../stores/settings'
import type { UpdateCheckResult } from '../types'

/** 全局更新提示状态（App.vue 渲染 snackbar，任意页面可触发） */
export const updateToast = ref(false)
export const updateMsg = ref('')
export const updateHasNew = ref(false)
export const updateChecking = ref(false)

export const RELEASES_PAGE_URL = 'https://github.com/icenfn/DHThub/releases/latest'

/** 检查更新（供自动检测与手动按钮共用），结果通过 snackbar 展示 */
export async function checkUpdate(): Promise<void> {
  if (!isTauri) return
  if (updateChecking.value) return
  updateChecking.value = true
  try {
    const mirror = settings.getSelectedMirror()
    const info = await invoke<UpdateCheckResult>('check_update', { mirrorBase: mirror.base })
    if (info.has_update) {
      updateHasNew.value = true
      updateMsg.value = `发现新版本 v${info.latest_version}（当前 v${info.current_version}），点击查看下载`
    } else {
      updateHasNew.value = false
      updateMsg.value = `已是最新版本 v${info.current_version}`
    }
  } catch (e) {
    updateHasNew.value = false
    updateMsg.value = `检查更新失败：${e instanceof Error ? e.message : String(e)}`
  } finally {
    updateChecking.value = false
    updateToast.value = true
  }
}

/** 打开 GitHub Releases latest 页面 */
export async function openReleasePage(): Promise<void> {
  try {
    await openUrl(RELEASES_PAGE_URL)
  } catch {
    window.open(RELEASES_PAGE_URL, '_blank')
  }
}
