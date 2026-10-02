// 设置与轻量数据的持久化：Tauri 下用 plugin-store，浏览器预览用 localStorage 降级

import { reactive } from 'vue'
import { isTauri } from '../lib/tauri'
import { BUILTIN_MIRRORS, deriveMirrorName } from '../lib/mirrors'
import { setDebug } from '../lib/debug'
import type { GithubMirror, SettingsExportFile } from '../types'

export type ThemeMode = 'system' | 'light' | 'dark'
export type MirrorMode = 'manual' | 'auto'

export interface SettingsData {
  theme: ThemeMode
  subscribeUrl: string
  /** 热门推荐热词（仓库 hotwords.json 抓取后的本地缓存） */
  hotWords: string[]
  agreedVersion: string | null
  searchHistory: string[]
  /** GitHub 镜像列表（内置 + 自定义） */
  githubMirrors: GithubMirror[]
  /** 镜像选择方式：手动 / 自动测速选最快 */
  githubMirrorMode: MirrorMode
  /** 当前选中镜像 id */
  githubMirrorId: string
  /** 自动检测更新：启动时静默检查新版本，发现后弹窗提醒 */
  autoCheckUpdate: boolean
  /** 调试模式：开启后输出详细日志与测速错误信息 */
  debug: boolean
}

export const DEFAULT_SUBSCRIBE_URL =
  'https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json'

/** 热词初始缓存（仓库抓取失败时的兜底） */
export const DEFAULT_HOT_WORDS = [
  '复仇者联盟4',
  '流浪地球2',
  '奥本海默',
  '沙丘2',
  '周杰伦演唱会',
  '美剧 2026',
  '宫崎骏',
  '纪录片 4K',
  '老友记',
  '航海王',
  '哈利波特',
  '权力的游戏',
]

const DEFAULTS: SettingsData = {
  theme: 'system',
  subscribeUrl: DEFAULT_SUBSCRIBE_URL,
  hotWords: DEFAULT_HOT_WORDS,
  agreedVersion: null,
  searchHistory: [],
  githubMirrors: BUILTIN_MIRRORS.map((m) => ({ ...m })),
  githubMirrorMode: 'manual',
  githubMirrorId: 'direct',
  autoCheckUpdate: true,
  debug: false,
}

class SettingsStore {
  /** 响应式数据：主题/镜像等跨页面即时生效 */
  private data: SettingsData = reactive({
    ...DEFAULTS,
    githubMirrors: DEFAULTS.githubMirrors.map((m) => ({ ...m })),
  })
  private loaded = false
  private file?: Awaited<ReturnType<typeof import('@tauri-apps/plugin-store').load>>

  constructor() {
    if (isTauri) {
      void this.loadTauri()
    } else {
      const raw = localStorage.getItem('dhthub:settings')
      if (raw) {
        try {
          Object.assign(this.data, this.merge({ ...DEFAULTS, ...JSON.parse(raw) }))
        } catch {
          /* ignore */
        }
      }
      setDebug(this.data.debug)
      this.loaded = true
    }
  }

  private async loadTauri() {
    try {
      const { load } = await import('@tauri-apps/plugin-store')
      this.file = await load('settings.json', { autoSave: false })
      const raw = await this.file.get<SettingsData>('settings')
      Object.assign(this.data, this.merge({ ...DEFAULTS, ...(raw ?? {}) }))
    } catch {
      /* ignore */
    }
    setDebug(this.data.debug)
    this.loaded = true
  }

  /** 合并外部数据：保证结构合法、内置镜像存在且选中 id 有效 */
  private merge(incoming: Partial<SettingsData>): SettingsData {
    const merged: SettingsData = {
      ...DEFAULTS,
      ...incoming,
      githubMirrors: DEFAULTS.githubMirrors.map((m) => ({ ...m })),
      searchHistory: Array.isArray(incoming.searchHistory)
        ? incoming.searchHistory.filter((x): x is string => typeof x === 'string').slice(0, 20)
        : [],
      hotWords: Array.isArray(incoming.hotWords)
        ? incoming.hotWords.filter((x): x is string => typeof x === 'string').slice(0, 50)
        : [],
    }
    // 主题与订阅地址合法性
    if (merged.theme !== 'system' && merged.theme !== 'light' && merged.theme !== 'dark') {
      merged.theme = 'system'
    }
    if (typeof merged.subscribeUrl !== 'string') {
      merged.subscribeUrl = DEFAULT_SUBSCRIBE_URL
    }
    if (merged.githubMirrorMode !== 'manual' && merged.githubMirrorMode !== 'auto') {
      merged.githubMirrorMode = 'manual'
    }
    // 镜像：内置 + 合法自定义，去重
    const seen = new Set<string>()
    merged.githubMirrors.forEach((m) => {
      if (
        m &&
        typeof m.id === 'string' &&
        typeof m.name === 'string' &&
        typeof m.base === 'string' &&
        !seen.has(m.id)
      ) {
        seen.add(m.id)
      }
    })
    const custom = (Array.isArray(incoming.githubMirrors) ? incoming.githubMirrors : [])
      .filter(
        (m): m is GithubMirror =>
          !!m &&
          typeof m.id === 'string' &&
          typeof m.name === 'string' &&
          typeof m.base === 'string' &&
          !m.builtin &&
          !seen.has(m.id) &&
          /^https?:\/\//i.test(m.base),
      )
      .map((m) => ({ ...m }))
    merged.githubMirrors = [...merged.githubMirrors, ...custom]
    if (!merged.githubMirrors.some((m) => m.id === merged.githubMirrorId)) {
      merged.githubMirrorId = merged.githubMirrors[0]?.id ?? 'direct'
    }
    if (typeof merged.autoCheckUpdate !== 'boolean') merged.autoCheckUpdate = true
    if (typeof merged.debug !== 'boolean') merged.debug = false
    return merged
  }

  private async persist() {
    if (isTauri && this.file) {
      await this.file.set('settings', this.data)
      await this.file.save()
    } else if (!isTauri) {
      localStorage.setItem('dhthub:settings', JSON.stringify(this.data))
    }
  }

  async ready(): Promise<void> {
    if (isTauri && !this.loaded) {
      await new Promise((r) => setTimeout(r, 50))
      while (!this.loaded) await new Promise((r) => setTimeout(r, 50))
    }
  }

  get<K extends keyof SettingsData>(key: K): SettingsData[K] {
    return this.data[key]
  }

  /** 当前完整设置快照（导出用） */
  snapshot(): SettingsData {
    return JSON.parse(JSON.stringify(this.data))
  }

  async set<K extends keyof SettingsData>(key: K, value: SettingsData[K]) {
    this.data[key] = value
    if (key === 'debug') setDebug(value as boolean)
    await this.persist()
  }

  /** 记录搜索历史（最近 20 条，去重） */
  async pushSearchHistory(keyword: string) {
    const k = keyword.trim()
    if (!k) return
    const list = [k, ...this.data.searchHistory.filter((x) => x !== k)].slice(0, 20)
    this.data.searchHistory = list
    await this.persist()
  }

  async clearSearchHistory() {
    this.data.searchHistory = []
    await this.persist()
  }

  // ---------- GitHub 镜像 ----------

  getMirrors(): GithubMirror[] {
    return this.data.githubMirrors
  }

  /** 当前选中的镜像（默认直连） */
  getSelectedMirror(): GithubMirror {
    return (
      this.data.githubMirrors.find((m) => m.id === this.data.githubMirrorId) ??
      this.data.githubMirrors[0] ??
      BUILTIN_MIRRORS[0]
    )
  }

  async selectMirror(id: string) {
    if (!this.data.githubMirrors.some((m) => m.id === id)) return
    this.data.githubMirrorId = id
    await this.persist()
  }

  async setMirrorMode(mode: MirrorMode) {
    this.data.githubMirrorMode = mode
    await this.persist()
  }

  /** 添加自定义镜像（base 已校验；名称由地址自动推导，无需用户填写） */
  async addMirror(base: string): Promise<GithubMirror> {
    const v = base.trim()
    if (!/^https?:\/\/[^\s]+$/i.test(v)) {
      throw new Error('镜像地址必须以 http(s):// 开头')
    }
    const normalized = v.endsWith('/') ? v : `${v}/`
    if (this.data.githubMirrors.some((m) => m.base === normalized)) {
      throw new Error('该镜像地址已存在')
    }
    const mirror: GithubMirror = {
      id: `m${Date.now().toString(36)}`,
      name: deriveMirrorName(normalized),
      base: normalized,
    }
    this.data.githubMirrors = [...this.data.githubMirrors, mirror]
    await this.persist()
    return mirror
  }

  /** 删除自定义镜像（内置不可删）；若删除的是当前选中项则回退直连 */
  async removeMirror(id: string) {
    const target = this.data.githubMirrors.find((m) => m.id === id)
    if (!target || target.builtin) return
    this.data.githubMirrors = this.data.githubMirrors.filter((m) => m.id !== id)
    if (this.data.githubMirrorId === id) {
      this.data.githubMirrorId = 'direct'
    }
    await this.persist()
  }

  // ---------- 设置导出 / 导入 ----------

  /** 导出全部设置为 JSON 文本（含镜像配置） */
  async exportJson(): Promise<string> {
    const file: SettingsExportFile = {
      app: 'DHThub',
      schemaVersion: 1,
      exportedAt: new Date().toISOString(),
      settings: this.snapshot() as unknown as Record<string, unknown>,
    }
    return JSON.stringify(file, null, 2)
  }

  /** 导入设置 JSON：合并合法字段，返回导入的字段数 */
  async importJson(json: string): Promise<number> {
    let parsed: SettingsExportFile
    try {
      parsed = JSON.parse(json)
    } catch {
      throw new Error('文件不是合法的 JSON')
    }
    if (parsed?.app !== 'DHThub') {
      throw new Error('不是 DHThub 设置文件（缺少 app 标识）')
    }
    const incoming = (parsed.settings ?? {}) as Record<string, unknown>
    const merged = this.merge({ ...this.data, ...incoming } as Partial<SettingsData>)
    // 统计实际导入的字段数
    let imported = 0
    ;(['theme', 'subscribeUrl', 'hotWords', 'searchHistory', 'githubMirrorMode', 'githubMirrorId', 'autoCheckUpdate', 'debug'] as const).forEach(
      (k) => {
        if (incoming[k] !== undefined) imported++
      },
    )
    if (Array.isArray(incoming.githubMirrors)) imported++
    Object.assign(this.data, merged)
    await this.persist()
    return imported
  }
}

export const settings = new SettingsStore()
