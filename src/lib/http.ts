// 网络请求统一入口：Tauri 下走 plugin-http（WebView 原生栈，DNS/建连不挂起），
// 浏览器预览降级为普通 fetch；全部带显式超时（AbortController），永不无限转圈。

import { isTauri } from './tauri'

export const DEFAULT_HTTP_TIMEOUT_MS = 15000
export const DEFAULT_CONNECT_TIMEOUT_MS = 8000

export interface HttpGetOptions {
  timeoutMs?: number
  connectTimeoutMs?: number
  headers?: Record<string, string>
}

/** GET 请求并返回文本；超时/网络错误一律 reject（调用方提示用户） */
export async function httpGetText(url: string, opts: HttpGetOptions = {}): Promise<string> {
  const timeoutMs = opts.timeoutMs ?? DEFAULT_HTTP_TIMEOUT_MS
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeoutMs)
  try {
    if (isTauri) {
      const { fetch } = await import('@tauri-apps/plugin-http')
      const res = await fetch(url, {
        method: 'GET',
        headers: opts.headers,
        signal: ctrl.signal,
        connectTimeout: opts.connectTimeoutMs ?? DEFAULT_CONNECT_TIMEOUT_MS,
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      return await res.text()
    }
    const res = await fetch(url, {
      method: 'GET',
      headers: opts.headers,
      signal: ctrl.signal,
    })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    return await res.text()
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error(`请求超时（${timeoutMs}ms），请检查网络或镜像`)
    }
    throw e
  } finally {
    clearTimeout(timer)
  }
}

/** GET 请求并返回 Response（可读 response.url 解析重定向结果） */
export async function httpGetRaw(
  url: string,
  opts: HttpGetOptions = {},
): Promise<Response> {
  const timeoutMs = opts.timeoutMs ?? DEFAULT_HTTP_TIMEOUT_MS
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeoutMs)
  try {
    if (isTauri) {
      const { fetch } = await import('@tauri-apps/plugin-http')
      const res = await fetch(url, {
        method: 'GET',
        headers: opts.headers,
        signal: ctrl.signal,
        connectTimeout: opts.connectTimeoutMs ?? DEFAULT_CONNECT_TIMEOUT_MS,
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      return res as unknown as Response
    }
    const res = await fetch(url, {
      method: 'GET',
      headers: opts.headers,
      signal: ctrl.signal,
    })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    return res
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error(`请求超时（${timeoutMs}ms），请检查网络或镜像`)
    }
    throw e
  } finally {
    clearTimeout(timer)
  }
}
