// 设置与轻量数据的持久化：Tauri 下用 plugin-store，浏览器预览用 localStorage 降级

import { isTauri } from '../lib/tauri'

export interface SettingsData {
  theme: 'system' | 'light' | 'dark'
  subscribeUrl: string
  hotWords: string[]
  agreedVersion: string | null
  searchHistory: string[]
}

export const DEFAULT_SUBSCRIBE_URL =
  'https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json'

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
}

class SettingsStore {
  private data: SettingsData = { ...DEFAULTS }
  private loaded = false
  private file?: Awaited<ReturnType<typeof import('@tauri-apps/plugin-store').load>>

  constructor() {
    if (isTauri) {
      void this.loadTauri()
    } else {
      const raw = localStorage.getItem('dhthub:settings')
      if (raw) {
        try {
          this.data = { ...DEFAULTS, ...JSON.parse(raw) }
        } catch {
          /* ignore */
        }
      }
      this.loaded = true
    }
  }

  private async loadTauri() {
    try {
      const { load } = await import('@tauri-apps/plugin-store')
      this.file = await load('settings.json', { autoSave: false })
      const raw = await this.file.get<SettingsData>('settings')
      this.data = { ...DEFAULTS, ...(raw ?? {}) }
    } catch {
      /* ignore */
    }
    this.loaded = true
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

  async set<K extends keyof SettingsData>(key: K, value: SettingsData[K]) {
    this.data[key] = value
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
}

export const settings = new SettingsStore()
