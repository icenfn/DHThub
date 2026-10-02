// 沉浸式状态栏（beta）：Android 下通过窗口全屏隐藏系统状态栏，实现沉浸效果
import { isAndroid } from './tauri'

export async function applyImmersive(enabled: boolean): Promise<void> {
  if (!isAndroid()) return
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().setFullscreen(enabled)
  } catch {
    /* 不支持时忽略 */
  }
}
