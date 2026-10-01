// 站点状态：订阅源、自定义站点、开关、默认引擎，全部由 Rust 端持久化

import { defineStore } from 'pinia'
import { invoke, isTauri } from '../lib/tauri'
import type { SiteConfig } from '../types'

export const useSitesStore = defineStore('sites', {
  state: () => ({
    sites: [] as SiteConfig[],
    loading: false,
    error: '',
    subscribeUrl: '',
    subscribedAt: '',
    inBrowser: !isTauri,
  }),

  getters: {
    enabledSites: (s) => s.sites.filter((x) => x.enabled),
    customSites: (s) => s.sites.filter((x) => x.is_custom),
    subscribedSites: (s) => s.sites.filter((x) => !x.is_custom),
    defaultSiteId: (s): string | null => s.sites.find((x) => x.is_default)?.id ?? null,
  },

  actions: {
    async load() {
      this.loading = true
      this.error = ''
      try {
        this.sites = await invoke<SiteConfig[]>('get_sites')
      } catch (e) {
        this.error = String(e)
      } finally {
        this.loading = false
      }
    },

    async subscribe(url: string) {
      const sites = await invoke<SiteConfig[]>('subscribe_sites', { url })
      this.sites = sites
      this.subscribeUrl = url
      return sites
    },

    async addCustom(site: SiteConfig) {
      this.sites = await invoke<SiteConfig[]>('add_custom_site', { site })
    },

    async updateCustom(site: SiteConfig) {
      this.sites = await invoke<SiteConfig[]>('update_custom_site', { site })
    },

    async remove(id: string) {
      this.sites = await invoke<SiteConfig[]>('delete_site', { id })
    },

    async setEnabled(id: string, enabled: boolean) {
      this.sites = await invoke<SiteConfig[]>('set_site_enabled', { id, enabled })
    },

    async setDefault(id: string | null) {
      this.sites = await invoke<SiteConfig[]>('set_default_site', { id })
    },

    async exportJson(): Promise<string> {
      return invoke<string>('export_sites')
    },

    async importJson(json: string) {
      this.sites = await invoke<SiteConfig[]>('import_sites', { json })
    },

    async reset() {
      this.sites = await invoke<SiteConfig[]>('reset_sites')
    },
  },
})
