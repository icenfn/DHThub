<script setup lang="ts">
// 搜索结果页
//
// 搜索模型：**一次搜索全部启用的搜索源**，结果全部留在内存；
// 顶部 chip 只是对已得结果做客户端过滤，切换平台不会再发一次请求。
//
// 交互约定：
//   - chip-group 单选：选中某个源只看该源结果，选中「聚合」看全部；
//     再次点击当前项取消选择，自动回到「聚合」。
//   - 排序：按钮 + 对话框选择。
//   - 结果行：点击进入独立「资源详情」页。本页由 App.vue 的 <keep-alive> 缓存，
//     因此从详情页返回时不会重新请求，列表 / 滚动 / 筛选 / 排序状态都保留。
import { computed, onActivated, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDebounceFn } from '@vueuse/core'
import { invoke, isTauri } from '../lib/tauri'
import { useSitesStore } from '../stores/sites'
import { settings } from '../stores/settings'
import type { MagnetItem, SiteOutcome } from '../types'

defineOptions({ name: 'SearchResultsView' })

const route = useRoute()
const router = useRouter()
const sitesStore = useSitesStore()

const keyword = ref(String(route.query.k ?? ''))
const searchedKeyword = ref('')

// ---------- 搜索源（纯客户端过滤，单选） ----------
const engineId = ref('all')
const engineOptions = computed(() => [
  { id: 'all', name: '聚合' },
  ...sitesStore.enabledSites.map((s) => ({ id: s.id ?? '', name: s.name })),
])

const isAll = computed(() => engineId.value === 'all')
const modeLabel = computed(() => (isAll.value ? '聚合合并' : '单源筛选'))

/** chip-group 单选变更：再次点击当前项则回到「聚合」 */
function onEngineChange(next: unknown) {
  const v = typeof next === 'string' ? next : ''
  if (!v || v === engineId.value) {
    if (v && v !== 'all' && engineId.value === v) return
  }
  engineId.value = v || 'all'
}

// ---------- 搜索状态 ----------
// 流式搜索：后端每完成一个站点就推送一次结果，前端立刻并入列表并渲染，
// 不再等全部站点搜索完成后才一次性显示。
const searching = ref(false)
const errorMsg = ref('')

type MergedItem = MagnetItem & { siteId: string; siteName: string; isDefault: boolean }

// 结果按「站点 × 页码」分桶；展示时轮询交错合并，
// 聚合视图下各源结果穿插出现，而不是某个源成块挤在前面。
const bucketOrder = ref<string[]>([])
const bucketMap = ref<Record<string, MergedItem[]>>({})
const outcomes = ref<SiteOutcome[]>([])
const sourcesExhausted = ref(false)
const sourcePage = ref(1)
const MAX_SOURCE_PAGES = 50
const PAGE_SIZE = 20

const successCount = computed(() => outcomes.value.filter((o) => o.success).length)
const failedSites = computed(() => outcomes.value.filter((o) => !o.success))

/** 从磁力链接里取 btih，用于跨站点去重（同一资源常被多个站点收录） */
function btihOf(m: string): string {
  const t = m.trim()
  if (!t.toLowerCase().startsWith('magnet:?')) return ''
  const m1 = t.match(/xt=urn:btih:([a-z0-9]+)/i)
  return m1 ? m1[1].toLowerCase() : ''
}

/** 去重键：btih > 来源地址 > 标题（同一资源只保留首次出现的那条） */
function dedupKey(it: MergedItem): string {
  const h = btihOf(it.magnet)
  if (h) return `btih:${h}`
  const u = (it.url || '').trim()
  if (u) return `url:${u}`
  return `t:${(it.title || '').trim().toLowerCase()}|${it.size || ''}`
}

/** 轮询交错合并全部桶（跨站点去重） */
const allItems = computed<MergedItem[]>(() => {
  const buckets = bucketOrder.value.map((k) => bucketMap.value[k] ?? [])
  const out: MergedItem[] = []
  const seen = new Set<string>()
  const maxLen = buckets.reduce((m, b) => Math.max(m, b.length), 0)
  for (let i = 0; i < maxLen; i += 1) {
    for (const b of buckets) {
      const it = b[i]
      if (!it) continue
      const key = dedupKey(it)
      if (seen.has(key)) continue
      seen.add(key)
      out.push(it)
    }
  }
  return out
})

/** 按当前 chip 选择过滤（不发请求） */
const filteredItems = computed<MergedItem[]>(() => {
  if (isAll.value) return allItems.value
  return allItems.value.filter((i) => i.siteId === engineId.value)
})

// ---------- 排序 ----------
const sortBy = ref('default')
const sortDialog = ref(false)
const sortOptions = [
  { title: '默认排序', value: 'default', icon: 'mdi-sort' },
  { title: '大小 从大到小', value: 'size-desc', icon: 'mdi-sort-descending' },
  { title: '大小 从小到大', value: 'size-asc', icon: 'mdi-sort-ascending' },
  { title: '日期 从新到旧', value: 'date-desc', icon: 'mdi-calendar-arrow-down' },
]
const sortLabel = computed(
  () => sortOptions.find((o) => o.value === sortBy.value)?.title ?? '默认排序',
)

function chooseSort(v: string) {
  sortBy.value = v
  sortDialog.value = false
  page.value = 1
}

function parseSize(s: string): number {
  const m = s.match(/([\d.]+)\s*([KMGTP]?)B?/i)
  if (!m) return 0
  const n = parseFloat(m[1])
  const unit = (m[2] || '').toUpperCase()
  const mult: Record<string, number> = {
    K: 1024,
    M: 1024 ** 2,
    G: 1024 ** 3,
    T: 1024 ** 4,
    P: 1024 ** 5,
  }
  return n * (mult[unit] ?? 1)
}

function parseDate(s: string): number {
  if (!s) return 0
  const t = Date.parse(s)
  return Number.isNaN(t) ? 0 : t
}

const sortedItems = computed<MergedItem[]>(() => {
  const base = filteredItems.value
  if (sortBy.value === 'default') return base
  const arr = [...base]
  if (sortBy.value === 'size-desc') arr.sort((a, b) => parseSize(b.size) - parseSize(a.size))
  else if (sortBy.value === 'size-asc') arr.sort((a, b) => parseSize(a.size) - parseSize(b.size))
  else arr.sort((a, b) => parseDate(b.date) - parseDate(a.date))
  return arr
})

// ---------- 分页 ----------
const page = ref(1)
// 已加载条数能填满的页数；还有更多数据时再多给 1 页作为「继续加载」入口
const totalPages = computed(() => {
  const loaded = Math.max(1, Math.ceil(sortedItems.value.length / PAGE_SIZE))
  return sourcesExhausted.value ? loaded : loaded + 1
})
const pagedItems = computed(() =>
  sortedItems.value.slice((page.value - 1) * PAGE_SIZE, page.value * PAGE_SIZE),
)

// 切换筛选后页码可能越界，回到第 1 页
watch(engineId, () => {
  page.value = 1
})

/** 把一个站点的结果放进对应桶，并更新统计 */
function applyOutcome(o: SiteOutcome, pageNo: number) {
  const key = `${o.site_id || o.site_name}#${pageNo}`
  if (!bucketOrder.value.includes(key)) {
    bucketOrder.value = [...bucketOrder.value, key]
  }
  bucketMap.value = {
    ...bucketMap.value,
    [key]: o.items.map((it) => ({
      ...it,
      siteId: o.site_id,
      siteName: o.site_name,
      isDefault: !!o.is_default,
    })),
  }
  const rest = outcomes.value.filter((x) => x.site_id !== o.site_id)
  outcomes.value = [...rest, o]
}

let streamSeq = 0

/** 流式搜索某一页：先挂事件，再触发后端；每个站点返回即并入列表 */
async function runStreamPage(k: string, pageNo: number): Promise<void> {
  const dns = settings.getSelectedDns()?.base ?? ''
  if (!isTauri) {
    // 浏览器预览没有事件通道，退化为一次性请求
    const result = await invoke<SiteOutcome[]>(
      'search_sites',
      { keyword: k, siteIds: undefined, page: pageNo, dns },
      60000,
    )
    result.forEach((o) => applyOutcome(o, pageNo))
    return
  }
  const { listen } = await import('@tauri-apps/api/event')
  const streamId = `${Date.now()}-${++streamSeq}-p${pageNo}`
  let resolveDone: (() => void) | null = null
  const done = new Promise<void>((r) => {
    resolveDone = r
  })
  // 必须先注册监听，再调用命令，否则首批站点事件会丢失
  const offSite = await listen<{ streamId: string; outcome: SiteOutcome }>(
    'search-site',
    (ev) => {
      if (ev.payload.streamId !== streamId) return
      applyOutcome(ev.payload.outcome, pageNo)
    },
  )
  const offDone = await listen<{ streamId: string }>('search-done', (ev) => {
    if (ev.payload.streamId === streamId) resolveDone?.()
  })
  try {
    await invoke<number>(
      'search_stream',
      { keyword: k, siteIds: undefined, page: pageNo, dns, streamId },
      30000,
    )
    // 兜底超时：极少数站点既不返回也不报错时，不让页面无限等
    await Promise.race([
      done,
      new Promise<void>((r) => {
        setTimeout(r, 70000)
      }),
    ])
  } finally {
    offSite()
    offDone()
  }
}

/** 全新搜索：清空后流式拉取第 1 页 */
async function doSearch(kw = keyword.value) {
  const k = kw.trim()
  if (!k) return
  keyword.value = k
  searchedKeyword.value = k
  searching.value = true
  errorMsg.value = ''
  outcomes.value = []
  bucketOrder.value = []
  bucketMap.value = {}
  sourcesExhausted.value = false
  sourcePage.value = 1
  page.value = 1
  savedScrollTop = 0
  try {
    await runStreamPage(k, 1)
    if (allItems.value.length === 0) {
      errorMsg.value = failedSites.value.length
        ? '搜索源均未返回结果，请检查网络或在「搜索源」中更换站点'
        : '没有返回结果，请尝试更换关键词'
    }
  } catch (e) {
    errorMsg.value = String(e)
  } finally {
    searching.value = false
  }
}

/** 翻页时按需补齐数据（同源按磁力 / 链接去重） */
async function ensureLoaded(targetPage: number): Promise<boolean> {
  if (sourcesExhausted.value) return false
  if (targetPage * PAGE_SIZE <= allItems.value.length) return false
  if (sourcePage.value >= MAX_SOURCE_PAGES) {
    sourcesExhausted.value = true
    return false
  }
  const before = allItems.value.length
  const next = sourcePage.value + 1
  try {
    await runStreamPage(searchedKeyword.value, next)
    sourcePage.value = next
    if (allItems.value.length === before) {
      sourcesExhausted.value = true
      return false
    }
    return true
  } catch {
    sourcesExhausted.value = true
    return false
  }
}

// 记录主页面的滚动位置：从详情页返回时用于复位（keep-alive 只保住组件状态，
// 内层滚动容器一旦被重建就会回到顶部）
const mainRef = ref<HTMLElement | null>(null)
let savedScrollTop = 0

function rememberScroll() {
  savedScrollTop = mainRef.value?.scrollTop ?? 0
}

const restoreScroll = useDebounceFn(() => {
  if (mainRef.value && savedScrollTop > 0) mainRef.value.scrollTop = savedScrollTop
}, 30)

const scrollToList = useDebounceFn(() => {
  if (mainRef.value) mainRef.value.scrollTop = 0
}, 80)

async function goPage(p: number) {
  const target = Math.min(Math.max(1, p), totalPages.value)
  if (target === page.value) return
  page.value = target
  await ensureLoaded(target)
  scrollToList()
}

/** 更新地址栏（replace，不新增历史栈，硬件返回键仍回首页）；不重复搜索同一关键词 */
function submit() {
  const k = keyword.value.trim()
  if (!k) return
  router.replace({ path: '/search', query: { k } })
  if (k !== searchedKeyword.value) void doSearch(k)
}

// 最近一次用于搜索的「请求序号」：首页每次点搜索都会带上新的 t，
// 因此只要从首页再次进入，无论关键词是否相同都重新搜索。
let lastNonce = ''

// 从详情页返回时本页已被 keep-alive 缓存，保持原状不重新请求；
// 但若带着新的 t 从首页重新发起搜索，则强制重新搜索（含同关键词）。
onActivated(() => {
  const k = String(route.query.k ?? '')
  const nonce = String(route.query.t ?? '')
  if (nonce && nonce !== lastNonce) {
    lastNonce = nonce
    savedScrollTop = 0
    if (k) {
      keyword.value = k
      void doSearch(k)
    }
    return
  }
  if (k && k !== searchedKeyword.value) {
    keyword.value = k
    void doSearch(k)
    return
  }
  // 从详情页返回：保持筛选与列表，滚动回原位置
  restoreScroll()
})

// 首次进入 / 直接带参数打开
void (async () => {
  await sitesStore.load().catch(() => undefined)
  lastNonce = String(route.query.t ?? '')
  const k = String(route.query.k ?? '')
  if (k) {
    keyword.value = k
    await doSearch(k)
  }
})()

// ---------- 进入资源详情 ----------
function openDetail(item: MergedItem) {
  // 记录浏览历史（历史页读取的正是 browse 类型）
  void invoke('add_history', {
    kind: 'browse',
    keyword: searchedKeyword.value || item.title,
    magnet: item.magnet || item.url || '',
  }).catch(() => undefined)
  router.push({
    path: '/res',
    query: {
      t: item.title,
      s: item.size,
      d: item.date,
      m: item.magnet || '',
      u: item.url || '',
      src: item.siteName,
      // 站点 ID：详情页据此按需回源解析真实磁力
      sid: item.siteId || '',
    },
  })
}

</script>

<template>
  <div class="sr">
    <!-- 吸顶搜索栏 -->
    <v-toolbar class="sr-bar" color="surface" flat height="auto">
      <v-btn icon="mdi-arrow-left" variant="text" size="small" title="返回" @click="router.push('/')" />
      <v-text-field
        v-model="keyword"
        class="sr-field"
        placeholder="输入关键词"
        variant="solo"
        density="compact"
        rounded="pill"
        flat
        hide-details
        single-line
        clearable
        enterkeyhint="search"
        @keyup.enter="submit"
      />
      <v-btn
        class="sr-go"
        color="primary"
        variant="flat"
        rounded="circle"
        icon
        :disabled="!keyword.trim() || searching"
        title="搜索"
        aria-label="搜索"
        @click="submit"
      >
        <v-progress-circular v-if="searching" indeterminate size="16" width="2" color="on-primary" />
        <v-icon v-else icon="mdi-magnify" size="20" />
      </v-btn>
    </v-toolbar>

    <!-- 搜索源 chip：单选，仅作客户端过滤 -->
    <div class="sr-engines">
      <v-chip-group
        :model-value="engineId"
        :column="false"
        class="sr-engines__track"
        selected-class="echip--on"
        @update:model-value="onEngineChange"
      >
        <v-chip
          v-for="opt in engineOptions"
          :key="opt.id"
          :value="opt.id"
          variant="outlined"
          size="small"
          class="echip"
          :prepend-icon="engineId === opt.id ? 'mdi-check' : undefined"
        >
          {{ opt.name }}
        </v-chip>
      </v-chip-group>
    </div>

    <!-- 结果区 -->
    <main ref="mainRef" class="sr-main" @scroll.passive="rememberScroll">
      <template v-if="searchedKeyword">
        <!-- 摘要 + 排序 -->
        <div class="summary">
          <div class="summary__line">
            <span class="summary__kw">“{{ searchedKeyword }}”</span>
            <span class="summary__sep">·</span>
            <span>{{ modeLabel }}</span>
            <span class="summary__sep">·</span>
            <span :class="{ 'summary__warn': failedSites.length }">
              {{ successCount }}/{{ outcomes.length }} 站成功
            </span>
            <span class="summary__sep">·</span>
            <span>{{ sortedItems.length }} 条</span>
          </div>

          <v-btn
            class="sortbtn"
            variant="tonal"
            size="small"
            rounded="pill"
            append-icon="mdi-chevron-down"
            prepend-icon="mdi-sort"
            @click="sortDialog = true"
          >
            {{ sortLabel }}
          </v-btn>
        </div>

        <v-alert
          v-if="errorMsg"
          type="warning"
          variant="tonal"
          density="compact"
          class="mb-3"
          :text="errorMsg"
        />

        <!-- 搜索进度：屏幕中央浮层，进度文字统一由页顶摘要展示 -->
        <div v-if="searching" class="loading-center">
          <v-progress-circular indeterminate size="34" width="3" color="primary" />
        </div>

        <v-empty-state
          v-else-if="sortedItems.length === 0"
          icon="mdi-magnet"
          title="没有可显示的结果"
          :text="errorMsg || '请尝试更换关键词，或在「搜索源」中启用更多站点'"
          class="empty"
        />

        <template v-else>
          <div class="list">
            <v-card
              v-for="(item, idx) in pagedItems"
              :key="`${item.siteId}-${idx}`"
              class="row"
              variant="outlined"
              rounded="lg"
              @click="openDetail(item)"
            >
              <div class="row__lead">
                <v-icon icon="mdi-magnet" size="16" color="primary" />
              </div>

              <div class="row__body">
                <div class="row__title">{{ item.title }}</div>
                <div class="row__meta">
                  <v-chip
                    v-if="item.size"
                    size="x-small"
                    variant="text"
                    prepend-icon="mdi-database-outline"
                  >
                    {{ item.size }}
                  </v-chip>
                  <v-chip
                    v-if="item.date"
                    size="x-small"
                    variant="text"
                    prepend-icon="mdi-calendar-outline"
                  >
                    {{ item.date }}
                  </v-chip>
                  <v-chip
                    size="x-small"
                    :color="item.isDefault ? 'primary' : undefined"
                    :variant="item.isDefault ? 'tonal' : 'text'"
                  >
                    {{ item.siteName }}
                  </v-chip>
                </div>
              </div>

            </v-card>
          </div>

          <!-- 分页：翻页时按需向后端补齐下一页数据 -->
          <v-pagination
            v-if="totalPages > 1"
            class="pager"
            :model-value="page"
            :length="totalPages"
            :total-visible="5"
            density="comfortable"
            rounded="circle"
            active-color="primary"
            @update:model-value="goPage"
          />
        </template>
      </template>

      <v-empty-state
        v-else
        icon="mdi-text-search"
        title="输入关键词开始搜索"
        text="关键词将并发请求全部已启用的搜索源，合并去重后统一展示"
        class="empty"
      />
    </main>

    <!-- 排序选择 -->
    <v-dialog v-model="sortDialog" max-width="360">
      <v-card rounded="xl" variant="flat" color="surface-container-low" border>
        <v-card-title class="text-title-small d-flex align-center ga-2">
          <v-icon icon="mdi-sort" size="17" color="primary" />
          排序方式
        </v-card-title>
        <v-list density="comfortable" nav>
          <v-list-item
            v-for="opt in sortOptions"
            :key="opt.value"
            :prepend-icon="opt.icon"
            :title="opt.title"
            :active="sortBy === opt.value"
            color="primary"
            @click="chooseSort(opt.value)"
          >
            <template #append>
              <v-icon v-if="sortBy === opt.value" icon="mdi-check" size="16" />
            </template>
          </v-list-item>
        </v-list>
      </v-card>
    </v-dialog>

  </div>
</template>

<style scoped>
.sr {
  display: flex;
  flex-direction: column;
  /* 旧内核不支持 dvh：用 --app-height 兜底，否则高度退化为 auto 后无法滚动 */
  height: 100%;
  height: var(--app-height, 100%);
  background: rgb(var(--v-theme-surface));
  overflow: hidden;
}

/* ---------- 顶栏 ---------- */
.sr-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 12px;
  padding-top: calc(9px + var(--safe-area-inset-top, env(safe-area-inset-top, 0px)));
  padding-left: calc(12px + var(--safe-area-inset-left, env(safe-area-inset-left, 0px)));
  padding-right: calc(12px + var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  flex-shrink: 0;
  background: rgb(var(--v-theme-surface));
}

.sr-bar :deep(.v-toolbar__content) {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0;
}

.sr-field {
  flex: 1 1 auto;
  min-width: 0;
}

.sr-field :deep(.v-field) {
  background: rgb(var(--v-theme-surface-container));
  border-radius: 999px;
}

.sr-field :deep(.v-field--focused) {
  background: rgb(var(--v-theme-surface));
}

.sr-go {
  flex: 0 0 auto;
  width: 40px !important;
  height: 40px !important;
  min-width: 40px;
}

/* ---------- 搜索源 chip ---------- */
.sr-engines {
  flex-shrink: 0;
  padding: 0 12px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  background: rgb(var(--v-theme-surface));
}

.sr-engines__track {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow-x: auto;
  flex-wrap: nowrap;
  padding: 9px 0;
  scrollbar-width: none;
}

.sr-engines__track::-webkit-scrollbar {
  display: none;
}

.echip {
  flex-shrink: 0;
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
  touch-action: manipulation;
}

/* 选中态：用主题色实底，文字取 on-primary，暗色主题下自动反色 */
.echip--on {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
  border-color: rgb(var(--v-theme-primary));
}

/* ---------- 主体 ---------- */
.sr-main {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  -webkit-overflow-scrolling: touch;
  touch-action: pan-y;
  overscroll-behavior-y: contain;
  padding: 12px 14px calc(20px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
}

.summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.summary__line {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 5px;
  font-size: 0.74rem;
  color: rgb(var(--v-theme-on-surface-variant));
  min-width: 0;
}

.summary__kw {
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
  max-width: 12em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.summary__sep {
  opacity: 0.55;
}

.summary__warn {
  color: rgb(var(--v-theme-warning));
  font-weight: 600;
}

.sortbtn {
  flex-shrink: 0;
  text-transform: none;
}

/* 搜索进度：浮在屏幕中央，不占列表布局 */
.loading-center {
  position: fixed;
  inset: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.empty {
  padding: 48px 16px;
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
  padding: 10px 12px;
  cursor: pointer;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
  transition: background-color 0.16s ease;
}

.row:hover {
  background: rgb(var(--v-theme-surface-container-low));
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
  line-height: 1.4;
  color: rgb(var(--v-theme-on-surface));
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.row__meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 2px;
  margin-top: 4px;
}

.pager {
  margin-top: 16px;
}
</style>
