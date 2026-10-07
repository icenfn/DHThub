<script setup lang="ts">
// 搜索结果独立页面：多源并发聚合，合并展示（全部 / 单选 / 多选），合并分页
// 搜索输入框 + 搜索按钮置于顶部应用栏内
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '../lib/tauri'
import { useSitesStore } from '../stores/sites'
import { settings } from '../stores/settings'
import type { MagnetItem, SiteOutcome } from '../types'
import MagnetDetailDialog from '../components/MagnetDetailDialog.vue'

const route = useRoute()
const router = useRouter()
const sitesStore = useSitesStore()

const keyword = ref(String(route.query.k ?? ''))

// 搜索源筛选：'all' = 全部（独占）；其余可单选/多选
const engineIds = ref<string[]>(['all'])
const engineOptions = computed(() => [
  { id: 'all', name: '全部' },
  ...sitesStore.enabledSites.map((s) => ({ id: s.id ?? '', name: s.name })),
])
const mode = computed<'all' | 'single' | 'multi'>(() => {
  if (engineIds.value.includes('all')) return 'all'
  return engineIds.value.length > 1 ? 'multi' : 'single'
})
const modeLabel = computed(() =>
  mode.value === 'all' ? '全部合并' : mode.value === 'single' ? '单源' : `多源合并 · ${engineIds.value.length} 站`,
)
// 选「全部」时独占：只保留最后选中的一项
watch(engineIds, (v) => {
  if (v.length === 0) {
    engineIds.value = ['all']
    return
  }
  if (v.includes('all')) {
    const last = v[v.length - 1]
    if (engineIds.value.length !== 1 || engineIds.value[0] !== last) engineIds.value = [last]
  }
})

const searching = ref(false)
const loadingMore = ref(false)
const errorMsg = ref('')
const searchedKeyword = ref('')

// ---- 合并结果模型 ----
type MergedItem = MagnetItem & { siteId: string; siteName: string; isDefault: boolean }
const outcomes = ref<SiteOutcome[]>([])
const allItems = ref<MergedItem[]>([])
const sourcesExhausted = ref(false)
const sourcePage = ref(1)
const MAX_SOURCE_PAGES = 50
const PAGE_SIZE = 20

const successCount = computed(() => outcomes.value.filter((o) => o.success).length)
const totalElapsed = computed(() => outcomes.value.reduce((a, b) => a + Math.max(b.elapsed_ms, 0), 0))

// 排序
const sortBy = ref('default')

const sortedItems = computed<MergedItem[]>(() => {
  if (sortBy.value === 'default') return allItems.value
  const arr = [...allItems.value]
  const cmp =
    sortBy.value === 'size-desc'
      ? (a: MagnetItem, b: MagnetItem) => parseSize(b.size) - parseSize(a.size)
      : sortBy.value === 'size-asc'
        ? (a: MagnetItem, b: MagnetItem) => parseSize(a.size) - parseSize(b.size)
        : (a: MagnetItem, b: MagnetItem) => parseDate(b.date) - parseDate(a.date)
  return arr.sort(cmp)
})

const page = ref(1)
// 分页长度：已加载条目不足一页且源未穷尽时，多给一页用于「加载更多」
const totalPages = computed(() => {
  const loaded = Math.max(1, Math.ceil(sortedItems.value.length / PAGE_SIZE))
  return sourcesExhausted.value ? loaded : loaded + 1
})
const pagedItems = computed(() =>
  sortedItems.value.slice((page.value - 1) * PAGE_SIZE, page.value * PAGE_SIZE),
)

function parseSize(s: string): number {
  const m = s.match(/([\d.]+)\s*([KMGTP]?)B?/i)
  if (!m) return 0
  const n = parseFloat(m[1])
  const unit = (m[2] || '').toUpperCase()
  const mult: Record<string, number> = { K: 1024, M: 1024 ** 2, G: 1024 ** 3, T: 1024 ** 4, P: 1024 ** 5 }
  return n * (mult[unit] ?? 1)
}

function parseDate(s: string): number {
  if (!s) return 0
  const t = Date.parse(s)
  return Number.isNaN(t) ? 0 : t
}

function selectedIds(): string[] | undefined {
  return engineIds.value.includes('all') ? undefined : engineIds.value
}

/** 拉取某一「源页码」的多源结果，返回本批新增的合并条目 */
async function fetchSourcePage(k: string, p: number): Promise<MergedItem[]> {
  const result = await invoke<SiteOutcome[]>('search_sites', {
    keyword: k,
    siteIds: selectedIds(),
    page: p,
    dns: settings.getSelectedDns()?.base ?? '',
  }, 60000)
  // 记录/更新源状态（用于成功数、失败提示）
  outcomes.value = result
  const merged: MergedItem[] = []
  for (const o of result) {
    for (const it of o.items) {
      merged.push({ ...it, siteId: o.site_id, siteName: o.site_name, isDefault: !!o.is_default })
    }
  }
  return merged
}

/** 全新搜索：重置并拉取第 1 个源页码 */
async function doSearch(kw = keyword.value) {
  const k = kw.trim()
  if (!k) return
  keyword.value = k
  searchedKeyword.value = k
  searching.value = true
  errorMsg.value = ''
  outcomes.value = []
  allItems.value = []
  sourcesExhausted.value = false
  sourcePage.value = 1
  page.value = 1
  try {
    const merged = await fetchSourcePage(k, 1)
    allItems.value = merged
    if (merged.length === 0) {
      errorMsg.value = '没有返回结果，请尝试更换关键词或检查搜索源状态'
    }
  } catch (e) {
    errorMsg.value = String(e)
    outcomes.value = []
    allItems.value = []
  } finally {
    searching.value = false
  }
}

/** 合并分页：合并缓冲不足时，自动拉取下一个源页码并追加 */
async function ensureLoaded(targetPage: number): Promise<boolean> {
  if (sourcesExhausted.value) return false
  if (targetPage * PAGE_SIZE <= allItems.value.length) return false
  if (sourcePage.value >= MAX_SOURCE_PAGES) {
    sourcesExhausted.value = true
    return false
  }
  loadingMore.value = true
  try {
    const next = sourcePage.value + 1
    const merged = await fetchSourcePage(searchedKeyword.value, next)
    sourcePage.value = next
    if (merged.length === 0) {
      sourcesExhausted.value = true
      return false
    }
    // 去重（同源同磁力/链接）
    const seen = new Set(allItems.value.map((i) => `${i.siteId}|${i.magnet || i.url}`))
    const fresh = merged.filter((i) => {
      const key = `${i.siteId}|${i.magnet || i.url}`
      if (seen.has(key)) return false
      seen.add(key)
      return true
    })
    allItems.value = [...allItems.value, ...fresh]
    if (fresh.length === 0) sourcesExhausted.value = true
    return fresh.length > 0
  } catch {
    sourcesExhausted.value = true
    return false
  } finally {
    loadingMore.value = false
  }
}

async function onPageChange(p: number) {
  page.value = p
  // 目标页超出缓冲则尝试加载更多（保持当前页）
  await ensureLoaded(p)
}

async function loadMore() {
  const ok = await ensureLoaded(page.value + 1)
  if (!ok && !errorMsg.value) {
    errorMsg.value = sourcesExhausted.value ? '' : '无法加载更多结果'
  }
}

/** 更新地址栏参数（不产生新的历史记录，返回键仍回首页） */
function submit() {
  router.replace({
    path: '/search',
    query: { k: keyword.value.trim(), e: engineIds.value.join(',') },
  })
}

// 切换搜索源：重新搜索（保持关键词）
watch(engineIds, () => {
  if (searchedKeyword.value) void doSearch(searchedKeyword.value)
})

watch(
  () => route.query,
  () => {
    const k = String(route.query.k ?? '')
    if (k && k !== keyword.value) keyword.value = k
    const e = String(route.query.e ?? '')
    const ids = e.split(',').filter(Boolean)
    const next = ids.length ? ids : ['all']
    if (engineIds.value.length !== next.length || engineIds.value.some((x, i) => x !== next[i])) {
      engineIds.value = next
    }
    if (k) void doSearch(k)
  },
  { immediate: true },
)

// 详情弹窗
const detailOpen = ref(false)
const detailItem = ref<MagnetItem | null>(null)
const detailSite = ref('')

function openDetail(item: MergedItem) {
  detailItem.value = item
  detailSite.value = item.siteName
  detailOpen.value = true
  void invoke('add_history', { kind: 'browse', keyword: searchedKeyword.value, magnet: item.magnet })
}

onMounted(async () => {
  try {
    await sitesStore.load()
  } catch {
    /* 浏览器预览下忽略 */
  }
})
</script>

<template>
  <div class="search-page">
    <!-- 应用栏：返回 + 搜索输入框 + 搜索按钮 -->
    <header class="app-bar">
      <v-btn icon="mdi-arrow-left" variant="text" size="small" title="返回" @click="router.push('/')" />
      <div class="app-bar__search">
        <v-text-field
          v-model="keyword"
          placeholder="输入关键词"
          variant="solo"
          flat
          rounded="pill"
          hide-details
          clearable
          density="compact"
          prepend-inner-icon="mdi-magnify"
          bg-color="surface-container-highest"
          class="app-bar__field"
          @keyup.enter="submit"
        />
        <v-btn
          color="primary"
          variant="flat"
          rounded="pill"
          class="px-5 flex-shrink-0"
          :loading="searching"
          :disabled="!keyword.trim()"
          @click="submit"
        >
          搜索
        </v-btn>
      </div>
    </header>

    <v-main class="search-main">
      <div class="search-inner">
        <!-- 搜索源筛选：单行横向滚动 -->
        <div class="engine-scroll">
          <v-chip-group v-model="engineIds" multiple mandatory color="primary" class="engine-chips">
            <v-chip v-for="opt in engineOptions" :key="opt.id" :value="opt.id" size="small" variant="tonal">
              {{ opt.name }}
            </v-chip>
          </v-chip-group>
        </div>

        <!-- 结果区 -->
        <template v-if="searchedKeyword">
          <div class="result-head">
            <div class="text-body-medium">
              “<strong>{{ searchedKeyword }}</strong>” 的结果
              <span class="text-medium-emphasis text-body-small">
                · {{ modeLabel }} · {{ successCount }}/{{ outcomes.length }} 站成功 · {{ allItems.length }} 条 · 总耗时 {{ totalElapsed }}ms
              </span>
            </div>
            <v-spacer />
            <v-select
              v-model="sortBy"
              :items="[
                { title: '默认排序', value: 'default' },
                { title: '大小 ↓', value: 'size-desc' },
                { title: '大小 ↑', value: 'size-asc' },
                { title: '日期 新→旧', value: 'date-desc' },
              ]"
              item-title="title"
              item-value="value"
              density="compact"
              variant="outlined"
              hide-details
              class="sort-select"
            />
          </div>

          <v-alert v-if="errorMsg" type="warning" variant="tonal" rounded="lg" class="mt-2">{{ errorMsg }}</v-alert>

          <div v-if="searching" class="mt-4">
            <v-progress-linear indeterminate color="primary" rounded />
            <div class="text-center text-medium-emphasis mt-2">正在并发请求各搜索源…</div>
          </div>

          <div v-else-if="allItems.length === 0" class="mt-6">
            <v-empty-state
              icon="mdi-magnet"
              title="没有可显示的结果"
              :text="errorMsg || '请尝试更换关键词或检查搜索源状态'"
            />
          </div>

          <!-- 合并结果列表 -->
          <template v-else>
            <v-card rounded="xl" variant="flat" color="surface-container-low" class="mt-3 merged-card">
              <div
                v-for="(item, idx) in pagedItems"
                :key="idx"
                class="result-row"
                @click="openDetail(item)"
              >
                <v-icon icon="mdi-magnet" color="primary" size="18" class="result-row__icon" />
                <div class="result-row__body">
                  <div class="text-body-medium text-truncate">{{ item.title }}</div>
                  <div class="result-row__meta">
                    <span v-if="item.size">
                      <v-icon icon="mdi-database-outline" size="13" /> {{ item.size }}
                    </span>
                    <span v-if="item.date">
                      <v-icon icon="mdi-calendar-outline" size="13" /> {{ item.date }}
                    </span>
                    <v-chip size="x-small" variant="tonal" :color="item.isDefault ? 'primary' : 'secondary'">
                      {{ item.siteName }}
                    </v-chip>
                  </div>
                </div>
                <v-btn
                  icon="mdi-content-copy"
                  size="small"
                  variant="text"
                  title="查看 / 复制磁力链接"
                  rounded="lg"
                  @click.stop="openDetail(item)"
                />
              </div>
            </v-card>

            <!-- 合并分页 -->
            <div class="pager">
              <v-pagination
                :model-value="page"
                :length="totalPages"
                :total-visible="5"
                density="comfortable"
                @update:model-value="onPageChange"
              />
            </div>
            <div class="d-flex justify-center mt-1">
              <v-progress-circular v-if="loadingMore" indeterminate size="20" width="2" color="primary" />
              <v-btn
                v-else-if="!sourcesExhausted"
                variant="text"
                color="primary"
                size="small"
                rounded="pill"
                @click="loadMore"
              >
                加载更多
              </v-btn>
              <span v-else class="text-body-small text-medium-emphasis">已加载全部结果</span>
            </div>
          </template>
        </template>

        <!-- 未搜索引导 -->
        <v-empty-state
          v-else
          icon="mdi-magnify"
          title="输入关键词开始搜索"
          text="关键词将并发请求所选搜索源并合并展示结果"
          class="mt-6"
        />

        <MagnetDetailDialog
          v-model="detailOpen"
          :item="detailItem"
          :site-name="detailSite"
          :keyword="searchedKeyword"
        />
      </div>
    </v-main>
  </div>
</template>

<style scoped>
.search-page {
  min-height: 100dvh;
  background: rgb(var(--v-theme-surface));
  display: flex;
  flex-direction: column;
}

.app-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: rgb(var(--v-theme-surface-container-low));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  position: sticky;
  top: 0;
  z-index: 10;
}

.app-bar__search {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.app-bar__field {
  flex: 1;
  min-width: 0;
}

.search-main {
  flex: 1;
}

.search-inner {
  max-width: 1040px;
  margin: 0 auto;
  padding: 12px 18px 40px;
}

/* 单行横向滚动：chip-group 不换行，容器水平滚动 */
.engine-scroll {
  overflow-x: auto;
  overflow-y: hidden;
  -webkit-overflow-scrolling: touch;
  scrollbar-width: none;
}
.engine-scroll::-webkit-scrollbar {
  display: none;
}
.engine-chips {
  flex-wrap: nowrap !important;
}
.engine-chips :deep(.v-chip) {
  flex-shrink: 0;
}

.result-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 14px;
}

.sort-select {
  max-width: 170px;
  flex-shrink: 0;
}

.merged-card {
  overflow: hidden;
}

.result-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  cursor: pointer;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition: background 0.15s ease;
}
.result-row:last-child {
  border-bottom: none;
}
.result-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.04);
}

.result-row__icon {
  flex-shrink: 0;
}

.result-row__body {
  flex: 1;
  min-width: 0;
}

.result-row__meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 3px;
  font-size: 0.74rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.pager {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}
</style>
