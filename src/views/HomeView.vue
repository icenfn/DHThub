<script setup lang="ts">
// 首页（搜索 tab）：输入关键词 + 搜索源选择 + 热门推荐/搜索历史；结果跳转独立搜索结果页
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { isTauri } from '../lib/tauri'
import { useSitesStore } from '../stores/sites'
import { settings } from '../stores/settings'
import { mirrorUrl, HOTWORDS_URL } from '../lib/mirrors'
import { httpGetText } from '../lib/http'

const router = useRouter()
const sitesStore = useSitesStore()
const keyword = ref('')
const engineId = ref<string | null>(null) // null = 全部
const hotWords = ref<string[]>([])
const searchHistory = ref<string[]>([])

function refreshLocalWords() {
  hotWords.value = [...settings.get('hotWords')]
  searchHistory.value = [...settings.get('searchHistory')]
}

function shuffleHotWords() {
  hotWords.value = [...hotWords.value].sort(() => Math.random() - 0.5)
}

/** 发起搜索：跳转独立搜索结果页 */
function doSearch(kw = keyword.value) {
  const k = kw.trim()
  if (!k) return
  keyword.value = k
  void settings.pushSearchHistory(k)
  refreshLocalWords()
  router.push({ path: '/search', query: { k, e: engineId.value ?? '' } })
}

function selectHotWord(w: string) {
  doSearch(w)
}

async function clearSearchHistory() {
  await settings.clearSearchHistory()
  refreshLocalWords()
}

/** 从 GitHub 仓库抓取热词总表（经选中镜像，plugin-http + 超时），成功后本地缓存；失败用缓存兜底 */
async function loadHotWords() {
  // 先显示本地缓存，避免空白
  hotWords.value = [...settings.get('hotWords')]
  try {
    const mirror = settings.getSelectedMirror()
    const url = mirrorUrl(mirror, HOTWORDS_URL)
    const text = await httpGetText(url, { timeoutMs: 12000 })
    const list: unknown = JSON.parse(text)
    if (Array.isArray(list) && list.length > 0 && list.every((x) => typeof x === 'string')) {
      hotWords.value = (list as string[]).slice(0, 50)
      await settings.set('hotWords', hotWords.value) // 本地缓存
    }
  } catch {
    /* 网络失败时沿用本地缓存 */
  }
}

onMounted(async () => {
  refreshLocalWords()
  void loadHotWords()
  if (isTauri) {
    try {
      await sitesStore.load()
    } catch {
      /* 忽略 */
    }
  }
})
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 1040px">
    <v-sheet rounded="xl" class="pa-4 pa-sm-6" elevation="1" color="surface" border="sm">
      <div class="text-h6 font-weight-bold mb-3">多源磁力搜索</div>
      <div class="d-flex flex-column flex-sm-row ga-2">
        <v-text-field
          v-model="keyword"
          label="输入关键词，例如：电影名 / 剧集 / 游戏"
          variant="solo"
          rounded="lg"
          hide-details
          clearable
          density="comfortable"
          @keyup.enter="doSearch()"
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
          <v-btn
            color="primary"
            variant="flat"
            size="large"
            :disabled="!keyword.trim()"
            @click="doSearch()"
          >
            <v-icon icon="mdi-magnify" class="mr-1" /> 搜索
          </v-btn>
        </div>
      </div>

      <!-- 热门推荐 / 搜索历史 -->
      <div v-if="hotWords.length" class="mt-4">
        <div class="d-flex align-center">
          <span class="text-subtitle-2 text-medium-emphasis mr-2">热门推荐</span>
          <v-btn variant="text" size="x-small" color="primary" @click="shuffleHotWords">
            <v-icon icon="mdi-refresh" size="16" class="mr-1" />换一换
          </v-btn>
        </div>
        <div class="d-flex flex-wrap ga-2 mt-1">
          <v-chip
            v-for="(w, i) in hotWords.slice(0, 12)"
            :key="i"
            size="small"
            variant="tonal"
            @click="selectHotWord(w)"
          >
            {{ w }}
          </v-chip>
        </div>
      </div>
      <div v-if="searchHistory.length" class="mt-3">
        <div class="d-flex align-center">
          <span class="text-subtitle-2 text-medium-emphasis mr-2">搜索历史</span>
          <v-btn variant="text" size="x-small" color="error" @click="clearSearchHistory">
            <v-icon icon="mdi-delete-outline" size="16" class="mr-1" />清空
          </v-btn>
        </div>
        <div class="d-flex flex-wrap ga-2 mt-1">
          <v-chip
            v-for="(w, i) in searchHistory.slice(0, 12)"
            :key="i"
            size="small"
            variant="outlined"
            @click="selectHotWord(w)"
          >
            {{ w }}
          </v-chip>
        </div>
      </div>
    </v-sheet>

    <v-empty-state
      icon="mdi-flash-outline"
      title="开始你的第一次搜索"
      text="输入关键词，DHThub 将并发请求所有已启用的搜索源并聚合结果"
      class="mt-8"
    />
  </div>
</template>
