// Android 系统状态栏 / 导航栏颜色跟随应用主题（桌面端 no-op）
import { isAndroid } from './tauri'

export async function applySystemBarTheme(dark: boolean): Promise<void> {
  if (!isAndroid()) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('set_system_bar_theme', { dark })
  } catch {
    /* 原生通道不可用时忽略 */
  }
}
