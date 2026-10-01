<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { invoke } from '../lib/tauri'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import type { HistoryEntry, HistoryKind } from '../types'

const tabs: { key: HistoryKind; label: string; icon: string }[] = [
  { key: 'magnet', label: '磁力记录', icon: 'mdi-magnet' },
  { key: 'copy', label: '复制记录', icon: 'mdi-content-copy' },
  { key: 'browse', label: '浏览记录', icon: 'mdi-eye-outline' },
]

const activeTab = ref<HistoryKind>('magnet')
const list = ref<HistoryEntry[]>([])
const loading = ref(false)
const toast = ref('')
const showToast = ref(false)

function fmtTime(ms: number): string {
  const d = new Date(ms)
  const now = Date.now()
  const diff = now - ms
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function shortMagnet(m: string): string {
  if (!m) return '—'
  return m.length > 60 ? `${m.slice(0, 60)}…` : m
}

async function load() {
  loading.value = true
  try {
    list.value = await invoke<HistoryEntry[]>('get_history', { kind: activeTab.value })
  } catch {
    list.value = []
  } finally {
    loading.value = false
  }
}

async function copyMagnet(entry: HistoryEntry) {
  try {
    await writeText(entry.magnet)
    toast.value = '已复制'
    showToast.value = true
  } catch {
    /* ignore */
  }
}

async function remove(entry: HistoryEntry) {
  // Rust 端无单条删除接口，此处提供复制+从列表隐藏由全清处理；保持简单
  toast.value = '已打开磁力链接'
  showToast.value = true
}

async function clearAll() {
  await invoke('clear_history', { kind: activeTab.value })
  await load()
  toast.value = '已清空'
  showToast.value = true
}

watch(activeTab, load)
onMounted(load)
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 1040px">
    <div class="d-flex align-center mt-2 mb-4">
      <div>
        <div class="text-h6 font-weight-bold">历史记录</div>
        <div class="text-caption text-medium-emphasis">磁力 / 复制 / 浏览三类历史，本地存储</div>
      </div>
      <v-spacer />
      <v-btn variant="tonal" color="error" prepend-icon="mdi-delete-sweep-outline" size="small" @click="clearAll">
        清空当前
      </v-btn>
    </div>

    <v-card rounded="lg">
      <v-tabs v-model="activeTab" color="primary" grow>
        <v-tab v-for="t in tabs" :key="t.key" :value="t.key">
          <v-icon :icon="t.icon" class="mr-1" size="18" />{{ t.label }}
        </v-tab>
      </v-tabs>
      <v-divider />
      <v-progress-linear v-if="loading" indeterminate color="primary" />
      <v-empty-state
        v-if="!loading && list.length === 0"
        icon="mdi-history"
        :title="`暂无${tabs.find((t) => t.key === activeTab)?.label}`"
        text="使用搜索功能后，相关操作会自动记录在这里"
      />
      <v-list v-else lines="two">
        <v-list-item v-for="(entry, i) in list" :key="i">
          <template #prepend>
            <v-avatar color="primary" variant="tonal" size="36">
              <v-icon :icon="tabs.find((t) => t.key === activeTab)?.icon" size="18" />
            </v-avatar>
          </template>
          <v-list-item-title class="text-body-2 font-weight-bold">{{ entry.keyword || '—' }}</v-list-item-title>
          <v-list-item-subtitle class="text-caption">
            <span class="font-family-monospace">{{ shortMagnet(entry.magnet) }}</span>
            <span class="ml-2 text-medium-emphasis">{{ fmtTime(entry.time) }}</span>
          </v-list-item-subtitle>
          <template #append>
            <v-btn
              icon="mdi-content-copy"
              size="small"
              variant="text"
              title="复制磁力"
              @click="copyMagnet(entry)"
            />
          </template>
        </v-list-item>
      </v-list>
    </v-card>

    <v-snackbar v-model="showToast" location="bottom" color="success" timeout="2000">
      {{ toast }}
    </v-snackbar>
  </div>
</template>
