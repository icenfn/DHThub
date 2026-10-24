// 视口高度桥接
//
// 旧内核（Android WebView 91 / 老 WebKitGTK）不支持 `dvh` 单位，
// 而 `height: 100%` 在缺少父级明确高度时会退化为 auto —— 结果是
// 整个应用外壳塌陷、底部 tab 栏被挤出视口、页面无法滚动。
//
// 这里把窗口真实高度写成固定像素的 CSS 变量 --app-height，
// 样式层统一 `height: var(--app-height, 100%)`：现代内核有 dvh 兜底，
// 旧内核由本模块提供的像素值兜底，两条路都拿到确定高度。
//
// 键盘弹出（visualViewport 收缩）不改变 --app-height：
// 键盘避让由 lib/insets.ts 的内边距负责，避免高度被重复扣减。

let applied = false
let lastHeight = 0

function sync(): void {
  if (typeof window === 'undefined') return
  const vv = window.visualViewport
  // 优先 innerHeight：Android WebView 下它等于 WebView 视口高度，
  // 且不像 visualViewport.height 那样随软键盘收缩。
  const h = Math.round(
    window.innerHeight ||
      (vv ? vv.height : 0) ||
      document.documentElement.clientHeight ||
      0,
  )
  if (h <= 0 || h === lastHeight) return
  lastHeight = h
  document.documentElement.style.setProperty('--app-height', `${h}px`)
}

/** 安装视口高度桥接（幂等，首帧前调用） */
export function installViewportBridge(): void {
  if (applied || typeof window === 'undefined') return
  applied = true

  sync()
  window.addEventListener('resize', sync)
  window.addEventListener('orientationchange', () => {
    // 旋转后尺寸会有一次异步变化，稍后补一次
    window.setTimeout(sync, 120)
    window.setTimeout(sync, 420)
  })
  // 部分旧内核依赖 resize 之外的信号（状态栏收起等）
  const vv = window.visualViewport
  if (vv) vv.addEventListener('resize', sync)
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) sync()
  })
}
