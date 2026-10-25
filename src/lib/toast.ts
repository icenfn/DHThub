// 全局轻提示：任意 TS/组件内直接调用 toast.show()/success()/error()，
// 由 AppSnackbar.vue 单点渲染，避免每个页面各写一份 v-snackbar。
import { reactive } from 'vue'

export type ToastColor = 'default' | 'success' | 'error' | 'warning' | 'info'

export interface ToastState {
  visible: boolean
  text: string
  color: ToastColor
  timeout: number
}

export const toastState = reactive<ToastState>({
  visible: false,
  text: '',
  color: 'default',
  timeout: 2400,
})

let hideTimer: number | undefined

/** 显示一条提示；同一时刻只保留最新一条（连续调用直接替换文案） */
export function showToast(text: string, color: ToastColor = 'default', timeout = 2400) {
  toastState.text = text
  toastState.color = color
  toastState.timeout = timeout
  toastState.visible = true
  if (hideTimer) window.clearTimeout(hideTimer)
  hideTimer = window.setTimeout(() => {
    toastState.visible = false
  }, timeout)
}

export const toast = {
  show: (text: string, timeout?: number) => showToast(text, 'default', timeout),
  success: (text: string, timeout?: number) => showToast(text, 'success', timeout),
  error: (text: string, timeout?: number) => showToast(text, 'error', timeout),
  warning: (text: string, timeout?: number) => showToast(text, 'warning', timeout),
  info: (text: string, timeout?: number) => showToast(text, 'info', timeout),
  hide: () => {
    toastState.visible = false
  },
}
