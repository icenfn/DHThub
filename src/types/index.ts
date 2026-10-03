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
  /** 关键词编码：默认 URL 编码；'base64' 时先 base64 再填入 [keyword] */
  keyword_encode?: string
  /** POST 表单关键词字段名（默认 keyword），部分站点用 wd 等字段名 */
  keyword_field?: string
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

export type HistoryKind = 'magnet' | 'copy' | 'browse'

/** GitHub 镜像配置：base 为前缀代理地址，空字符串 = 官方直连 */
export interface GithubMirror {
  id: string
  name: string
  base: string
  /** 是否内置镜像（内置不可删除） */
  builtin?: boolean
}

/** DNS 服务器配置：base 为 IP 或 IP:端口 */
export interface DnsServer {
  id: string
  name: string
  base: string
  /** 是否内置（内置不可删除） */
  builtin?: boolean
}

/** DNS 测速结果 */
export interface DnsSpeedResult {
  id: string
  name: string
  base: string
  latency: number | null
  error: string | null
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
