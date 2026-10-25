<script setup lang="ts">
// 历史记录：浏览记录（本地存储）
// - 点击任意一条进入资源详情页
// - 页头左侧为「清空」，列表行不再提供复制按钮
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '../lib/tauri'
import type { HistoryEntry } from '../types'

const KIND = 'browse'
const router = useRouter()

const list = ref<HistoryEntry[]>([])
const loading = ref(false)

// 分批渲染：旧内核一次性挂载数百张卡片会直接白屏
const BATCH = 40
const shown = ref(BATCH)

const sorted = computed(() => [...list.value].sort((a, b) => b.time - a.time))
const visible = computed(() => sorted.value.slice(0, shown.value))
const hasMore = computed(() => shown.value < sorted.value.length)

function loadMore() {
  shown.value += BATCH
}

function fmtTime(ms: number): string {
  const d = new Date(ms)
  const diff = Date.now() - ms
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function shortText(m: string): string {
  if (!m) return ''
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

async function clearAll() {
  await invoke('clear_history', { kind: KIND })
  await load()
}

/** 点击历史项：有磁力进详情页，仅有来源地址则直达来源页 */
function openEntry(entry: HistoryEntry) {
  const m = entry.magnet || ''
  if (!m) return
  if (m.toLowerCase().startsWith('magnet:')) {
    void router.push({
      path: '/res',
      query: {
        k: entry.keyword || '',
        t: entry.keyword || '历史记录',
        m,
        src: '历史记录',
      },
    })
  } else {
    void router.push({ path: '/res', query: { k: entry.keyword || '', t: entry.keyword || '历史记录', u: m, src: '历史记录' } })
  }
}

onMounted(load)
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner">
      <!-- 页头：操作按钮靠左 -->
      <div v-if="list.length" class="head">
        <v-btn
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
          tabindex="0"
          @click="openEntry(entry)"
          @keyup.enter="openEntry(entry)"
        >
          <div class="row__lead">
            <v-icon icon="mdi-eye-outline" size="17" color="primary" />
          </div>
          <div class="row__body">
            <div class="row__title">{{ entry.keyword || '—' }}</div>
            <div class="row__magnet mono">{{ shortText(entry.magnet) }}</div>
          </div>
          <div class="row__side">
            <span class="row__time">{{ fmtTime(entry.time) }}</span>
            <v-icon icon="mdi-chevron-right" size="18" class="row__chev" />
          </div>
        </v-card>

        <v-btn
          v-if="hasMore"
          variant="tonal"
          rounded="pill"
          size="small"
          block
          class="morebtn"
          @click="loadMore"
        >
          加载更多（剩余 {{ sorted.length - shown }} 条）
        </v-btn>
      </div>
    </div>
  </div>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 10px;
  margin-bottom: 12px;
}

.head__btn {
  flex-shrink: 0;
  text-transform: none;
}

.empty {
  padding: 44px 16px;
}

.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 12px;
  cursor: pointer;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
}

.row:active {
  background: rgb(var(--v-theme-surface-container));
}

.row__lead {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
}

.row__body {
  flex: 1 1 auto;
  min-width: 0;
}

.row__title {
  font-size: 0.86rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row__magnet {
  margin-top: 3px;
  font-size: 0.7rem;
  line-height: 1.5;
  color: rgb(var(--v-theme-on-surface-variant));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row__side {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 2px;
}

.row__time {
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
}

.row__chev {
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.7;
}

.morebtn {
  margin-top: 4px;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}
</style>
