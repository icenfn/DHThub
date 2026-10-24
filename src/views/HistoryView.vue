<script setup lang="ts">
// 历史记录：浏览记录（本地存储），支持一键复制磁力 / 清空
import { computed, onMounted, ref } from 'vue'
import { useClipboard, useTimeAgo } from '@vueuse/core'
import { invoke } from '../lib/tauri'
import type { HistoryEntry } from '../types'

const KIND = 'browse'

const list = ref<HistoryEntry[]>([])
const loading = ref(false)
const toast = ref('')
const showToast = ref(false)

const { copy } = useClipboard({ legacy: true })

// 分批渲染：旧内核一次性挂载数百张卡片会直接白屏，
// 这里每批 40 条，滚动到底部再追加
const BATCH = 40
const shown = ref(BATCH)
const visible = computed(() => sorted.value.slice(0, shown.value))
const hasMore = computed(() => shown.value < sorted.value.length)

function loadMore() {
  shown.value += BATCH
}

// 历史记录按时间倒序展示（Rust 端已排序，这里再兜底一次，防止存储顺序差异）
const sorted = computed(() => [...list.value].sort((a, b) => b.time - a.time))

function fmtTime(ms: number): string {
  const d = new Date(ms)
  const diff = Date.now() - ms
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function shortMagnet(m: string): string {
  if (!m) return '—'
  return m.length > 56 ? `${m.slice(0, 56)}…` : m
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
  if (!entry.magnet) return
  try {
    const mod = await import('@tauri-apps/plugin-clipboard-manager')
    await mod.writeText(entry.magnet)
  } catch {
    try {
      await copy(entry.magnet)
    } catch {
      return
    }
  }
  toast.value = '已复制磁力链接'
  showToast.value = true
}

async function clearAll() {
  await invoke('clear_history', { kind: KIND })
  await load()
  toast.value = '已清空浏览记录'
  showToast.value = true
}

onMounted(load)
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner">
      <!-- 页头：标题/副标题已移除（顶栏已显示页名），只保留清空入口 -->
      <div class="head head--actions">
        <v-btn
          v-if="list.length"
          variant="outlined"
          size="small"
          rounded="pill"
          color="error"
          prepend-icon="mdi-delete-sweep-outline"
          class="head__btn"
          @click="clearAll"
        >
          清空
        </v-btn>
      </div>

      <v-progress-linear v-if="loading" indeterminate color="primary" rounded height="2" />

      <!-- 空态 -->
      <v-empty-state
        v-if="!loading && list.length === 0"
        icon="mdi-history"
        title="暂无浏览记录"
        text="点击搜索结果查看详情后，会自动记录在这里"
        class="empty"
      />

      <!-- 列表 -->
      <div v-else class="list">
        <v-card
          v-for="(entry, i) in visible"
          :key="i"
          class="row"
          variant="outlined"
          rounded="lg"
        >
          <div class="row__lead">
            <v-icon icon="mdi-eye-outline" size="17" color="primary" />
          </div>
          <div class="row__body">
            <div class="row__title">{{ entry.keyword || '—' }}</div>
            <div class="row__magnet mono">{{ shortMagnet(entry.magnet) }}</div>
          </div>
          <div class="row__side">
            <span class="row__time">{{ fmtTime(entry.time) }}</span>
            <v-btn
              icon="mdi-content-copy"
              variant="text"
              size="x-small"
              title="复制磁力"
              @click="copyMagnet(entry)"
            />
          </div>
        </v-card>
      </div>

      <v-snackbar
        v-model="showToast"
        location="bottom"
        color="inverse-surface"
        rounded="lg"
        :timeout="2000"
      >
        {{ toast }}
      </v-snackbar>
    </div>
  </div>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}

.head__text {
  flex: 1 1 auto;
  min-width: 0;
}

.head__title {
  font-size: 1.02rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.head__sub {
  margin-top: 2px;
  font-size: 0.72rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.head--actions {
  justify-content: flex-end;
  margin-bottom: 10px;
}

.head__btn {
  flex-shrink: 0;
  text-transform: none;
}

/* ---------- 空态 ---------- */
.empty {
  padding: 44px 16px;
}

/* ---------- 列表 ---------- */
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
  background: rgb(var(--v-theme-surface));
}

.row + .row {
  margin-top: 8px;
}

.row__lead {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: rgba(var(--v-theme-primary), 0.12);
}

.row__body {
  flex: 1 1 auto;
  min-width: 0;
}

.row__title {
  font-size: 0.84rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row__magnet {
  margin-top: 3px;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row__side {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
}

.row__time {
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
}

.morebtn {
  margin-top: 4px;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}
</style>
