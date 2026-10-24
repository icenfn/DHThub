// 安全区 / 输入法（IME）内边距桥接
//
// Android 端（MainActivity.kt）开启 edge-to-edge 后，把系统栏、刘海、
// 手势条以及软键盘高度换算成 CSS px，通过
//   window.__dhthubApplyInsets(top, bottom, left, right, keyboard)
// 注入为 CSS 变量：
//   --safe-area-inset-top / -bottom / -left / -right
//   --keyboard-height
// 样式层统一写 var(--safe-area-inset-top, env(safe-area-inset-top, 0px))，
// 于是「原生给值优先，浏览器 env() 兜底」只应用一次内边距，不会叠加两次。
//
// 同时兼容 visualViewport：若 WebView 未上报键盘高度，用可视视口差估算兜底，
// 保证输入框不会被输入法遮住。

import { ref } from 'vue'

export const insetTop = ref(0)
export const insetBottom = ref(0)
export const insetLeft = ref(0)
export const insetRight = ref(0)
export const keyboardHeight = ref(0)
/** 键盘是否弹出（用于隐藏底部导航等需要避让的元素） */
export const keyboardOpen = ref(false)

let applied = false

function setVar(name: string, px: number): void {
  document.documentElement.style.setProperty(name, `${Math.max(0, Math.round(px))}px`)
}

/** 将一次 insets 应用为 CSS 变量（原生调用入口） */
export function applyInsets(
  top: number,
  bottom: number,
  left: number,
  right: number,
  keyboard = 0,
): void {
  const kb = Math.max(0, keyboard || 0)
  insetTop.value = Math.max(0, top || 0)
  insetLeft.value = Math.max(0, left || 0)
  insetRight.value = Math.max(0, right || 0)
  // 系统栏与键盘重叠时只取一份，避免底部出现双份留白
  insetBottom.value = Math.max(0, (bottom || 0) - kb)
  keyboardHeight.value = kb
  keyboardOpen.value = kb > 0

  setVar('--safe-area-inset-top', insetTop.value)
  setVar('--safe-area-inset-bottom', insetBottom.value)
  setVar('--safe-area-inset-left', insetLeft.value)
  setVar('--safe-area-inset-right', insetRight.value)
  setVar('--keyboard-height', kb)

  const root = document.documentElement
  if (root.classList) root.classList.toggle('kb-open', kb > 0)
}

declare global {
  interface Window {
    __dhthubApplyInsets?: (
      top: number,
      bottom: number,
      left: number,
      right: number,
      keyboard?: number,
    ) => void
  }
}

/** 安装原生桥接 + visualViewport 兜底（幂等） */
export function installInsetsBridge(): void {
  if (applied || typeof window === 'undefined') return
  applied = true

  let nativeSeen = false

  // 原生上报优先；一旦原生给过非零键盘高度，就不再让兜底覆盖
  window.__dhthubApplyInsets = (t, b, l, r, kb = 0) => {
    if (kb > 0) nativeSeen = true
    applyInsets(t, b, l, r, kb)
  }

  // visualViewport 兜底：原生未上报键盘高度时，用高度差估算
  const vv = window.visualViewport
  if (!vv) return

  let baseHeight = vv.height
  const sync = (): void => {
    if (nativeSeen) return
    const delta = Math.max(0, baseHeight - vv.height)
    // 小于 100px 的差值多为浏览器地址栏伸缩，不作为键盘
    const kb = delta > 100 ? delta : 0
    applyInsets(insetTop.value, insetBottom.value, insetLeft.value, insetRight.value, kb)
  }

  vv.addEventListener('resize', sync)
  window.addEventListener('orientationchange', () => {
    baseHeight = window.visualViewport?.height ?? baseHeight
    setTimeout(sync, 260)
  })
  sync()
}
