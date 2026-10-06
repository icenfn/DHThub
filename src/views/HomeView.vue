<script setup lang="ts">
// 首页（搜索 tab）：居中搜索框 + 热门推荐/搜索历史；结果跳转独立搜索结果页
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { settings } from '../stores/settings'
import { mirrorUrl, HOTWORDS_URL } from '../lib/mirrors'
import { httpGetText } from '../lib/http'

const router = useRouter()
const keyword = ref('')
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
  router.push({ path: '/search', query: { k } })
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
})
</script>

<template>
  <div class="home-view">
    <div class="home-inner">
      <div class="flex-grow-1" />

      <!-- 品牌搜索区 -->
      <div class="text-center mb-5">
        <v-avatar color="primary-container" rounded="xl" size="60" class="mb-3">
          <v-icon icon="mdi-magnet" color="on-primary-container" size="32" />
        </v-avatar>
        <h1 class="text-headline-small font-weight-bold mb-1">多源磁力搜索</h1>
        <p class="text-body-medium text-medium-emphasis">
          并发请求所有已启用的搜索源，聚合去重展示结果
        </p>
      </div>

      <!-- 搜索卡片 -->
      <v-sheet class="search-card" color="surface-container-low" rounded="xl" border>
        <div class="d-flex flex-column flex-sm-row ga-2">
          <v-text-field
            v-model="keyword"
            placeholder="输入关键词，例如：电影名 / 剧集 / 游戏"
            variant="solo"
            flat
            rounded="pill"
            hide-details
            clearable
            density="comfortable"
            prepend-inner-icon="mdi-magnify"
            bg-color="surface-container-highest"
            class="search-field"
            @keyup.enter="doSearch()"
          />
          <v-btn
            color="primary"
            variant="flat"
            size="large"
            rounded="pill"
            class="px-6 flex-shrink-0"
            :disabled="!keyword.trim()"
            @click="doSearch()"
          >
            搜索
          </v-btn>
        </div>
      </v-sheet>

      <!-- 热门推荐 -->
      <section v-if="hotWords.length" class="chips-section">
        <div class="chips-head">
          <span class="text-label-large text-medium-emphasis">
            <v-icon icon="mdi-fire" size="16" color="warning" class="mr-1" />热门推荐
          </span>
          <v-btn variant="text" size="small" color="primary" rounded="pill" @click="shuffleHotWords">
            <v-icon icon="mdi-refresh" size="16" class="mr-1" />换一换
          </v-btn>
        </div>
        <div class="chips-wrap">
          <v-chip
            v-for="(w, i) in hotWords.slice(0, 12)"
            :key="i"
            size="small"
            color="primary"
            variant="tonal"
            @click="selectHotWord(w)"
          >
            {{ w }}
          </v-chip>
        </div>
      </section>

      <!-- 搜索历史 -->
      <section v-if="searchHistory.length" class="chips-section">
        <div class="chips-head">
          <span class="text-label-large text-medium-emphasis">
            <v-icon icon="mdi-history" size="16" class="mr-1" />搜索历史
          </span>
          <v-btn
            variant="text"
            size="small"
            color="error"
            rounded="pill"
            @click="clearSearchHistory"
          >
            <v-icon icon="mdi-delete-outline" size="16" class="mr-1" />清空
          </v-btn>
        </div>
        <div class="chips-wrap">
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
      </section>

      <div class="flex-grow-1" />
    </div>
  </div>
</template>

<style scoped>
.home-view {
  min-height: 100%;
  display: flex;
  justify-content: center;
}

.home-inner {
  width: 100%;
  max-width: 680px;
  min-height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 20px 32px;
}

.search-card {
  width: 100%;
  max-width: 600px;
  padding: 12px;
}

.search-field :deep(input) {
  font-size: 0.95rem;
}

.chips-section {
  width: 100%;
  max-width: 600px;
  margin-top: 20px;
  display: flex;
  flex-direction: column;
}

.chips-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.chips-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
