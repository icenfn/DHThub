// 全局弹层（弹窗/菜单）栈：支持手机返回键 / PC ESC 一键关闭最上层弹层
// 各弹窗组件在打开时注册关闭回调，关闭时自动注销；Vuetify 自身仍负责 ESC 关闭，
// 本栈仅兜底（含 Android 硬件返回键派发的 android:back 事件）

type Closer = () => void

const stack: Closer[] = []

/** 注册一个弹层关闭回调，返回注销函数 */
export function pushOverlay(closer: Closer): () => void {
  stack.push(closer)
  let alive = true
  return () => {
    if (!alive) return
    alive = false
    const i = stack.indexOf(closer)
    if (i >= 0) stack.splice(i, 1)
  }
}

/** 关闭最上层弹层；有弹层被关闭返回 true，否则 false */
export function closeTopOverlay(): boolean {
  const c = stack.pop()
  if (c) {
    c()
    return true
  }
  return false
}
