<script setup lang="ts">
// 首页（搜索 tab）：垂直流式内容 —— Logo + 描述文案 + 搜索输入框（输入框/按钮）+ 热门推荐标签组 + 搜索历史
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
  hotWords.value = [...settings.get('hotWords')]
  try {
    const mirror = settings.getSelectedMirror()
    const url = mirrorUrl(mirror, HOTWORDS_URL)
    const text = await httpGetText(url, { timeoutMs: 12000 })
    const list: unknown = JSON.parse(text)
    if (Array.isArray(list) && list.length > 0 && list.every((x) => typeof x === 'string')) {
      hotWords.value = (list as string[]).slice(0, 50)
      await settings.set('hotWords', hotWords.value)
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
      <!-- 1. Logo + 描述文案 -->
      <header class="home-hero">
        <v-avatar color="primary-container" rounded="xl" size="56">
          <v-icon icon="mdi-magnet" color="on-primary-container" size="30" />
        </v-avatar>
        <h1 class="text-headline-small font-weight-bold">多源磁力搜索</h1>
        <p class="text-body-medium text-medium-emphasis">
          并发请求所有已启用的搜索源，聚合去重展示结果
        </p>
      </header>

      <!-- 2. 搜索输入框 + 搜索按钮 -->
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

      <!-- 3. 热门推荐标签组 -->
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

      <!-- 4. 搜索历史 -->
      <section v-if="searchHistory.length" class="chips-section">
        <div class="chips-head">
          <span class="text-label-large text-medium-emphasis">
            <v-icon icon="mdi-history" size="16" class="mr-1" />搜索历史
          </span>
          <v-btn variant="text" size="small" color="error" rounded="pill" @click="clearSearchHistory">
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
    </div>
  </div>
</template>

<style scoped>
.home-view {
  min-height: 100%;
  display: flex;
  justify-content: center;
}

/* 垂直流式主体内容：自上而下自然排列 */
.home-inner {
  width: 100%;
  max-width: 680px;
  display: flex;
  flex-direction: column;
  padding: 24px 20px 32px;
}

.home-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 6px;
  margin-bottom: 20px;
}

.search-card {
  width: 100%;
  padding: 12px;
}

.search-field :deep(input) {
  font-size: 0.95rem;
}

.chips-section {
  width: 100%;
  margin-top: 22px;
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
