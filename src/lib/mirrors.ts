// GitHub 镜像工具：URL 转换、测速（测速为纯前端实现，参考 moretools 方案：
// no-cors fetch + AbortController 超时，DNS/连接/响应头任一步超时都会立即返回）

import type { GithubMirror, MirrorSpeedResult } from '../types'

/** 内置 GitHub 镜像（不可删除；镜像列表仅展示链接） */
export const BUILTIN_MIRRORS: GithubMirror[] = [
  { id: 'direct', name: 'GitHub 官方（直连）', base: '', builtin: true },
  { id: 'ghproxy', name: 'gh-proxy.com', base: 'https://gh-proxy.com/', builtin: true },
  { id: 'axisnow', name: 'axisnow.gh-proxy.org', base: 'https://axisnow.gh-proxy.org/', builtin: true },
  { id: 'cdn', name: 'cdn.gh-proxy.org', base: 'https://cdn.gh-proxy.org/', builtin: true },
  { id: 'ghdpik', name: 'gh.dpik.top', base: 'https://gh.dpik.top/', builtin: true },
]

/** 默认订阅源（探针/订阅测试目标） */
export const MIRROR_PROBE_URL =
  'https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json'

/** 热门推荐热词总表（仓库托管，经镜像抓取 + 本地缓存） */
export const HOTWORDS_URL =
  'https://raw.githubusercontent.com/icenfn/DHThub/main/sites/hotwords.json'

/** 单镜像测速超时（毫秒）：覆盖 DNS/建连/响应头全过程 */
export const SPEED_TEST_TIMEOUT_MS = 6000

/** 将原始 GitHub URL 套用镜像前缀；镜像为空或未选择时返回原 URL */
export function mirrorUrl(mirror: GithubMirror | undefined | null, url: string): string {
  const base = (mirror?.base ?? '').trim()
  if (!base) return url
  return `${base.replace(/\/+$/, '')}/${url.replace(/^\/+/, '')}`
}

/** 由镜像前缀地址推导内部名称（取主机名），用户无需填写镜像名称 */
export function deriveMirrorName(base: string): string {
  try {
    const host = new URL(base).host
    return host || base
  } catch {
    return base
  }
}

/**
 * 前端测速单次请求：no-cors 模式（镜像站普遍不返回 CORS 头，但可达性/时延可测），
 * fetch 在响应头到达时即 resolve，配合 AbortController 墙钟超时，任何情况都不挂起
 */
async function measureInFrontend(url: string, timeoutMs: number): Promise<number> {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeoutMs)
  const start = performance.now()
  try {
    await fetch(url, {
      method: 'GET',
      signal: ctrl.signal,
      cache: 'no-store',
      mode: 'no-cors',
      redirect: 'follow',
    })
    return Math.round(performance.now() - start)
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error(`测速超时（>${timeoutMs}ms）`)
    }
    throw e
  } finally {
    clearTimeout(timer)
  }
}

/** 单个镜像测速：返回耗时（毫秒）或错误（带超时兜底，永不挂起） */
export async function speedTestMirror(mirror: GithubMirror): Promise<MirrorSpeedResult> {
  const url = mirrorUrl(mirror, MIRROR_PROBE_URL)
  try {
    const latency = await measureInFrontend(url, SPEED_TEST_TIMEOUT_MS)
    return { id: mirror.id, name: mirror.name, base: mirror.base, latency, error: null }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    return { id: mirror.id, name: mirror.name, base: mirror.base, latency: null, error: msg }
  }
}

/** 校验用户输入的自定义镜像前缀，返回规范化后的 base */
export function normalizeMirrorBase(input: string): string {
  const v = input.trim()
  if (!/^https?:\/\/[^\s]+$/i.test(v)) {
    throw new Error('镜像地址必须以 http(s):// 开头')
  }
  return v.endsWith('/') ? v : `${v}/`
}
