<script setup lang="ts">
// 搜索结果独立页面：读取首页传入的关键词/搜索源参数，执行搜索并展示聚合结果
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '../lib/tauri'
import { useSitesStore } from '../stores/sites'
import type { MagnetItem, SiteOutcome } from '../types'
import MagnetDetailDialog from '../components/MagnetDetailDialog.vue'
import StatisticsDialog from '../components/StatisticsDialog.vue'

const route = useRoute()
const router = useRouter()
const sitesStore = useSitesStore()

const keyword = ref(String(route.query.k ?? ''))
const engineId = ref<string | null>(String(route.query.e ?? '') || null) // null = 全部
const page = ref(1)
const searching = ref(false)
const outcomes = ref<SiteOutcome[]>([])
const searchedKeyword = ref('')
const errorMsg = ref('')

// 排序/过滤
const filterText = ref('')
const sortBy = ref('default')

// 统计
const showStats = ref(false)
const totalElapsed = computed(() => outcomes.value.reduce((a, b) => a + b.elapsed_ms, 0))

const successCount = computed(() => outcomes.value.filter((o) => o.success).length)
const totalItems = computed(() => outcomes.value.reduce((a, o) => a + o.items.length, 0))

const filteredOutcomes = computed(() => {
  let list = outcomes.value.map((o) => ({ ...o, items: o.items }))
  const ft = filterText.value.trim().toLowerCase()
  if (ft) {
    list = list
      .map((o) => ({ ...o, items: o.items.filter((i) => i.title.toLowerCase().includes(ft)) }))
      .filter((o) => o.items.length > 0 || !o.success)
  }
  if (sortBy.value !== 'default') {
    list = list.map((o) => ({
      ...o,
      items: [...o.items].sort((a, b) => {
        if (sortBy.value === 'size-desc') return parseSize(b.size) - parseSize(a.size)
        if (sortBy.value === 'size-asc') return parseSize(a.size) - parseSize(b.size)
        if (sortBy.value === 'date-desc') return parseDate(b.date) - parseDate(a.date)
        return 0
      }),
    }))
  }
  return list
})

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

async function doSearch(kw = keyword.value, p = 1) {
  const k = kw.trim()
  if (!k) return
  keyword.value = k
  page.value = p
  searchedKeyword.value = k
  searching.value = true
  errorMsg.value = ''
  try {
    const ids = engineId.value ? [engineId.value] : undefined
    outcomes.value = await invoke<SiteOutcome[]>('search_sites', {
      keyword: k,
      siteIds: ids,
      page: p,
    })
    if (outcomes.value.length === 0) errorMsg.value = '没有启用的搜索源，请先到「站点管理」订阅或启用'
  } catch (e) {
    errorMsg.value = String(e)
    outcomes.value = []
  } finally {
    searching.value = false
  }
}

/** 更新地址栏参数（不产生新的历史记录，返回键仍回首页） */
function submit() {
  router.replace({ path: '/search', query: { k: keyword.value.trim(), e: engineId.value ?? '' } })
}

watch(
  () => route.query,
  () => {
    const k = String(route.query.k ?? '')
    if (k && k !== keyword.value) keyword.value = k
    const e = String(route.query.e ?? '')
    engineId.value = e || null
    if (k) void doSearch(k, 1)
  },
  { immediate: true },
)

// 详情弹窗
const detailOpen = ref(false)
const detailItem = ref<MagnetItem | null>(null)
const detailSite = ref('')

function openDetail(item: MagnetItem, siteName: string) {
  detailItem.value = item
  detailSite.value = siteName
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
  <div class="d-flex flex-column" style="height: 100dvh">
    <v-app-bar color="surface" border="b" height="56">
      <template #prepend>
        <v-btn icon="mdi-arrow-left" variant="text" title="返回" @click="router.push('/')" />
      </template>
      <v-app-bar-title>
        <span class="text-subtitle-1 font-weight-bold">搜索结果</span>
      </v-app-bar-title>
    </v-app-bar>

    <v-main class="flex-grow-1 overflow-y-auto">
      <div class="px-3 px-sm-6 py-3 mx-auto" style="max-width: 1040px">
        <!-- 搜索区（可修改关键词重新搜索） -->
        <v-sheet rounded="xl" class="pa-3" elevation="1" color="surface" border="sm">
          <div class="d-flex flex-column flex-sm-row ga-2">
            <v-text-field
              v-model="keyword"
              label="关键词"
              variant="solo"
              rounded="lg"
              hide-details
              clearable
              density="comfortable"
              @keyup.enter="submit"
            />
            <div class="d-flex ga-2">
              <v-select
                v-model="engineId"
                :items="[
                  { title: '全部启用的搜索源', value: null },
                  ...sitesStore.enabledSites.map((s) => ({ title: s.name, value: s.id })),
                ]"
                item-title="title"
                item-value="value"
                label="搜索源"
                hide-details
                style="min-width: 180px"
              />
              <v-btn color="primary" variant="flat" size="large" :loading="searching" :disabled="!keyword.trim()" @click="submit">
                <v-icon icon="mdi-magnify" class="mr-1" /> 搜索
              </v-btn>
            </div>
          </div>
        </v-sheet>

        <!-- 结果区 -->
        <template v-if="searchedKeyword">
          <div class="d-flex align-center ga-3 mt-3 flex-wrap">
            <div class="text-subtitle-1">
              “<strong>{{ searchedKeyword }}</strong>” 的搜索结果
              <span class="text-medium-emphasis text-caption">
                · {{ successCount }}/{{ outcomes.length }} 站成功 · {{ totalItems }} 条结果 · 总耗时 {{ totalElapsed }}ms
              </span>
            </div>
            <v-spacer />
            <v-btn variant="tonal" size="small" color="primary" prepend-icon="mdi-chart-bar" @click="showStats = true">
              搜索统计
            </v-btn>
          </div>

          <!-- 过滤/排序 -->
          <div class="d-flex flex-column flex-sm-row ga-2 mt-1">
            <v-text-field
              v-model="filterText"
              label="在当前结果中过滤标题"
              density="compact"
              hide-details
              clearable
              style="max-width: 320px"
            />
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
              label="排序"
              density="compact"
              hide-details
              style="max-width: 180px"
            />
          </div>

          <v-alert v-if="errorMsg" type="warning" class="mt-2">{{ errorMsg }}</v-alert>

          <div v-if="searching" class="mt-4">
            <v-progress-linear indeterminate color="primary" />
            <div class="text-center text-medium-emphasis mt-2">正在并发请求各搜索源…</div>
          </div>

          <div v-else-if="filteredOutcomes.length === 0" class="mt-6">
            <v-empty-state
              icon="mdi-magnet"
              title="没有可显示的结果"
              :text="errorMsg || '请尝试更换关键词或检查搜索源状态'"
            />
          </div>

          <!-- 分站点结果 -->
          <div v-else class="mt-2 d-flex flex-column ga-3">
            <v-card v-for="o in filteredOutcomes" :key="o.site_id" rounded="lg">
              <v-card-item>
                <template #prepend>
                  <v-avatar color="primary" variant="tonal" size="34">
                    <v-icon icon="mdi-antenna" size="20" />
                  </v-avatar>
                </template>
                <v-card-title class="text-subtitle-1 font-weight-bold">
                  {{ o.site_name }}
                  <v-chip v-if="o.is_default" size="x-small" color="primary" variant="flat" class="ml-2">默认</v-chip>
                </v-card-title>
                <template #append>
                  <span v-if="o.success" class="text-caption text-medium-emphasis">
                    {{ o.items.length }} 条 · {{ o.elapsed_ms }}ms
                  </span>
                  <v-chip v-else color="error" size="small" variant="tonal">失败</v-chip>
                </template>
              </v-card-item>
              <v-divider />
              <v-alert v-if="!o.success && o.error" type="error" variant="tonal" density="compact" class="ma-3">
                {{ o.error }}
              </v-alert>
              <v-list v-else lines="two">
                <v-list-item
                  v-for="(item, idx) in o.items"
                  :key="idx"
                  class="result-item"
                  @click="openDetail(item, o.site_name)"
                >
                  <template #prepend>
                    <v-icon icon="mdi-magnet" color="primary" class="mt-1" />
                  </template>
                  <v-list-item-title class="text-body-2">{{ item.title }}</v-list-item-title>
                  <v-list-item-subtitle>
                    <span v-if="item.size" class="mr-3">
                      <v-icon icon="mdi-database-outline" size="14" /> {{ item.size }}
                    </span>
                    <span v-if="item.date" class="mr-3">
                      <v-icon icon="mdi-calendar-outline" size="14" /> {{ item.date }}
                    </span>
                    <span class="text-medium-emphasis">{{ o.site_name }}</span>
                  </v-list-item-subtitle>
                  <template #append>
                    <v-btn
                      icon="mdi-content-copy"
                      size="small"
                      variant="text"
                      title="复制磁力链接"
                      @click.stop="openDetail(item, o.site_name)"
                    />
                  </template>
                </v-list-item>
              </v-list>
            </v-card>

            <!-- 分页 -->
            <div class="d-flex justify-center">
              <v-pagination v-model="page" :length="10" :total-visible="5" @update:model-value="doSearch(searchedKeyword, page)" />
            </div>
          </div>
        </template>

        <!-- 未搜索时的引导 -->
        <v-empty-state
          v-else
          icon="mdi-magnify"
          title="输入关键词开始搜索"
          text="关键词将并发请求所有已启用的搜索源并聚合结果"
          class="mt-6"
        />

        <MagnetDetailDialog
          v-model="detailOpen"
          :item="detailItem"
          :site-name="detailSite"
          :keyword="searchedKeyword"
        />
        <StatisticsDialog v-model="showStats" :outcomes="outcomes" :keyword="searchedKeyword" />
      </div>
    </v-main>
  </div>
</template>

<style scoped>
.result-item {
  cursor: pointer;
  border-bottom: 1px solid rgba(127, 127, 127, 0.12);
}
.result-item:last-child {
  border-bottom: none;
}
</style>
