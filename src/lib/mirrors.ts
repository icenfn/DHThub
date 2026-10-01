// GitHub 镜像工具：URL 转换、测速
// 镜像采用「前缀代理」模式：base + 原始 GitHub URL，直连时 base 为空字符串

import { invoke, isTauri } from './tauri'
import type { GithubMirror, MirrorSpeedResult } from '../types'

/** 内置 GitHub 镜像（3 个，不可删除） */
export const BUILTIN_MIRRORS: GithubMirror[] = [
  { id: 'direct', name: 'GitHub 官方（直连）', base: '', builtin: true },
  { id: 'ghfast', name: 'ghfast.top', base: 'https://ghfast.top/', builtin: true },
  { id: 'ghproxy', name: 'gh-proxy.com', base: 'https://gh-proxy.com/', builtin: true },
]

/** 默认订阅源（探针/订阅测试目标） */
export const MIRROR_PROBE_URL =
  'https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json'

/** 热门推荐热词总表（仓库托管，经镜像抓取 + 本地缓存） */
export const HOTWORDS_URL =
  'https://raw.githubusercontent.com/icenfn/DHThub/main/hotwords.json'

/** 将原始 GitHub URL 套用镜像前缀；镜像为空或未选择时返回原 URL */
export function mirrorUrl(mirror: GithubMirror | undefined | null, url: string): string {
  const base = (mirror?.base ?? '').trim()
  if (!base) return url
  return `${base.replace(/\/+$/, '')}/${url.replace(/^\/+/, '')}`
}

/** 浏览器预览模式下的兜底测速（raw.githubusercontent 支持 CORS） */
async function measureInBrowser(url: string): Promise<number> {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), 10_000)
  const start = performance.now()
  try {
    const resp = await fetch(url, { signal: ctrl.signal, cache: 'no-store' })
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
    return Math.round(performance.now() - start)
  } finally {
    clearTimeout(timer)
  }
}

/** 单个镜像测速：返回耗时（毫秒）或错误 */
export async function speedTestMirror(mirror: GithubMirror): Promise<MirrorSpeedResult> {
  const url = mirrorUrl(mirror, MIRROR_PROBE_URL)
  try {
    const latency = isTauri
      ? await invoke<number>('test_mirror_speed', { url })
      : await measureInBrowser(url)
    return { id: mirror.id, name: mirror.name, base: mirror.base, latency, error: null }
  } catch (e) {
    return { id: mirror.id, name: mirror.name, base: mirror.base, latency: null, error: String(e) }
  }
}

/** 并发测速全部镜像 */
export async function speedTestAll(mirrors: GithubMirror[]): Promise<MirrorSpeedResult[]> {
  return Promise.all(mirrors.map(speedTestMirror))
}

/** 从测速结果中选取最快可用镜像 */
export function pickFastest(results: MirrorSpeedResult[]): MirrorSpeedResult | null {
  const ok = results
    .filter((r) => r.latency != null && r.latency > 0)
    .sort((a, b) => (a.latency ?? Infinity) - (b.latency ?? Infinity))
  return ok[0] ?? null
}

/** 校验用户输入的自定义镜像前缀，返回规范化后的 base */
export function normalizeMirrorBase(input: string): string {
  const v = input.trim()
  if (!/^https?:\/\/[^\s]+$/i.test(v)) {
    throw new Error('镜像地址必须以 http(s):// 开头')
  }
  return v.endsWith('/') ? v : `${v}/`
}
