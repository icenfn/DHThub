// Tauri 环境检测与 IPC 封装：浏览器中开发预览时自动降级

export const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri) {
    throw new Error('当前为浏览器预览模式，请通过 Tauri 应用运行')
  }
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(cmd, args)
}

export function platform(): string {
  if (!isTauri) return 'browser'
  // @ts-expect-error tauri 注入的运行时信息
  const p = window.__TAURI_INTERNALS__?.metadata?.platform ?? 'unknown'
  return p
}

export function isAndroid(): boolean {
  return platform() === 'android'
}
