// Tauri 环境检测与 IPC 封装：浏览器中开发预览时自动降级

export const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** 单次 IPC 调用的默认超时（毫秒）：避免 Rust 端异常挂起时前端无限转圈 */
export const INVOKE_TIMEOUT_MS = 25000

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
  timeoutMs = INVOKE_TIMEOUT_MS,
): Promise<T> {
  if (!isTauri) {
    throw new Error('当前为浏览器预览模式，请通过 Tauri 应用运行')
  }
  const { invoke } = await import('@tauri-apps/api/core')
  if (!timeoutMs) return invoke<T>(cmd, args)
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(
        () => reject(new Error(`请求超时（${timeoutMs}ms）`)),
        timeoutMs,
      ),
    ),
  ])
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
