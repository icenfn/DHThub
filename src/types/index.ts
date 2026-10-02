// 与 Rust 端 models.rs 对应的类型定义

export interface FieldSpec {
  // 字符串 = 文本选择器；对象 = 属性选择器
  sel?: string
  attr?: string
}

export interface RequestConfig {
  method: string
  search_url: string
  headers: Record<string, string>
  timeout_ms?: number
}

export interface ExpressionModel {
  group: string
  title?: FieldSpec | string
  date?: FieldSpec | string
  size?: FieldSpec | string
  url?: FieldSpec | string
  magnet?: FieldSpec | string
}

export interface SiteConfig {
  id?: string
  name: string
  info?: string
  state?: boolean
  time?: string
  update_time?: string
  request: RequestConfig
  expression_model: ExpressionModel
  is_custom?: boolean
  enabled?: boolean
  is_default?: boolean
}

export interface MagnetItem {
  title: string
  date: string
  size: string
  url: string
  magnet: string
}

export interface SiteOutcome {
  site_id: string
  site_name: string
  success: boolean
  elapsed_ms: number
  items: MagnetItem[]
  error: string | null
  is_default?: boolean
}

export interface HistoryEntry {
  keyword: string
  magnet: string
  time: number
}

/** 更新检测结果（releases/latest 重定向解析） */
export interface UpdateCheckResult {
  current_version: string
  latest_version: string
  has_update: boolean
}

export type HistoryKind = 'magnet' | 'copy' | 'browse'

/** GitHub 镜像配置：base 为前缀代理地址，空字符串 = 官方直连 */
export interface GithubMirror {
  id: string
  name: string
  base: string
  /** 是否内置镜像（内置不可删除） */
  builtin?: boolean
}

/** 镜像测速结果 */
export interface MirrorSpeedResult {
  id: string
  name: string
  base: string
  latency: number | null
  error: string | null
}

/** 设置导出文件结构 */
export interface SettingsExportFile {
  app: 'DHThub'
  schemaVersion: 1
  exportedAt: string
  settings: Record<string, unknown>
}
