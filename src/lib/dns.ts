// 内置 DNS 服务器预设（搜索请求用）
import type { DnsServer } from '../types'

export const BUILTIN_DNS: DnsServer[] = [
  { id: 'alidns', name: 'AliDNS', base: '223.5.5.5', builtin: true },
  { id: 'dnspod', name: 'DNSPod', base: '119.29.29.29', builtin: true },
  { id: 'cloudflare', name: 'Cloudflare', base: '1.1.1.1', builtin: true },
  { id: '114dns', name: '114DNS', base: '114.114.114.114', builtin: true },
]

/** 由地址推导名称（无名称时的兜底） */
export function deriveDnsName(base: string): string {
  return base.replace(/^https?:\/\//, '').replace(/[:/]+$/, '')
}
