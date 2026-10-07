<script setup lang="ts">
// 历史记录：浏览记录（本地存储），支持一键复制磁力 / 清空
import { onMounted, ref } from 'vue'
import { invoke } from '../lib/tauri'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import type { HistoryEntry } from '../types'

const KIND = 'browse'

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
    list.value = await invoke<HistoryEntry[]>('get_history', { kind: KIND })
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

async function clearAll() {
  await invoke('clear_history', { kind: KIND })
  await load()
  toast.value = '已清空'
  showToast.value = true
}

onMounted(load)
</script>

<template>
  <div class="page-wrap">
    <div class="page-head">
      <div>
        <h2 class="text-title-large font-weight-bold">历史记录</h2>
        <p class="text-body-small text-medium-emphasis">浏览记录，本地存储</p>
      </div>
      <v-spacer />
      <v-btn
        variant="tonal"
        color="error"
        prepend-icon="mdi-delete-sweep-outline"
        size="small"
        rounded="pill"
        @click="clearAll"
      >
        清空浏览记录
      </v-btn>
    </div>

    <v-progress-linear v-if="loading" indeterminate color="primary" rounded />

    <v-sheet
      v-if="!loading && list.length === 0"
      rounded="xl"
      color="surface-container-low"
      border
      class="pa-8 text-center"
    >
      <v-icon icon="mdi-history" size="48" color="outline" class="mb-2" />
      <div class="text-title-medium font-weight-bold">暂无浏览记录</div>
      <div class="text-body-small text-medium-emphasis">点击搜索结果查看详情后，会自动记录在这里</div>
    </v-sheet>

    <v-card v-else rounded="xl" variant="flat" color="surface-container-low" class="list-card">
      <div
        v-for="(entry, i) in list"
        :key="i"
        class="history-row"
      >
        <v-avatar color="primary-container" rounded="lg" size="40">
          <v-icon icon="mdi-eye-outline" size="20" color="on-primary-container" />
        </v-avatar>
        <div class="history-row__body">
          <div class="text-body-medium font-weight-bold text-truncate">{{ entry.keyword || '—' }}</div>
          <div class="text-body-small text-medium-emphasis text-truncate">
            <span class="mono">{{ shortMagnet(entry.magnet) }}</span>
          </div>
          <div class="text-label-small text-disabled">{{ fmtTime(entry.time) }}</div>
        </div>
        <v-btn
          icon="mdi-content-copy"
          size="small"
          variant="text"
          title="复制磁力"
          rounded="lg"
          @click="copyMagnet(entry)"
        />
      </div>
    </v-card>

    <v-snackbar v-model="showToast" location="bottom" color="inverse-surface" rounded="lg" timeout="2000">
      {{ toast }}
    </v-snackbar>
  </div>
</template>

<style scoped>
.page-wrap {
  max-width: 1040px;
  margin: 0 auto;
  padding: 16px 20px 24px;
}

.page-head {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
  gap: 8px;
}

.list-card {
  overflow: hidden;
}

.history-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.history-row:last-child {
  border-bottom: none;
}

.history-row__body {
  flex: 1;
  min-width: 0;
}

.mono {
  font-family: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', Menlo, monospace;
  font-size: 0.72rem;
}
</style>
