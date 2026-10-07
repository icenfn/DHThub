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
  <div
    class="px-3 px-sm-6 pt-1 pb-4 mx-auto d-flex flex-column align-center"
    style="min-height: 100%; max-width: 760px"
  >
    <!-- 上下弹性垫片：内容垂直居中（内容超高时自动恢复滚动，不裁切顶部） -->
    <div class="flex-grow-1" />

    <!-- 居中简洁的搜索区 -->
    <div class="text-h6 font-weight-bold mb-1">多源磁力搜索</div>
    <div class="text-caption text-medium-emphasis mb-4">并发请求所有已启用的搜索源，聚合去重展示结果</div>

    <v-sheet
      rounded="lg"
      elevation="1"
      color="surface"
      border="sm"
      class="pa-2"
      style="width: 100%; max-width: 560px"
    >
      <div class="d-flex ga-2">
        <v-text-field
          v-model="keyword"
          label="输入关键词，例如：电影名 / 剧集 / 游戏"
          variant="solo"
          flat
          rounded="lg"
          hide-details
          clearable
          density="compact"
          @keyup.enter="doSearch()"
        />
        <v-btn
          color="primary"
          variant="flat"
          size="large"
          class="px-5"
          :disabled="!keyword.trim()"
          @click="doSearch()"
        >
          <v-icon icon="mdi-magnify" class="mr-1" /> 搜索
        </v-btn>
      </div>
    </v-sheet>

    <!-- 热门推荐 -->
    <div v-if="hotWords.length" class="mt-4 d-flex flex-column align-center">
      <div class="d-flex align-center">
        <span class="text-caption text-medium-emphasis mr-2">热门推荐</span>
        <v-btn variant="text" size="x-small" color="primary" @click="shuffleHotWords">
          <v-icon icon="mdi-refresh" size="16" class="mr-1" />换一换
        </v-btn>
      </div>
      <div class="d-flex flex-wrap justify-center ga-2 mt-1">
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

    <!-- 搜索历史 -->
    <div v-if="searchHistory.length" class="mt-3 d-flex flex-column align-center">
      <div class="d-flex align-center">
        <span class="text-caption text-medium-emphasis mr-2">搜索历史</span>
        <v-btn variant="text" size="x-small" color="error" @click="clearSearchHistory">
          <v-icon icon="mdi-delete-outline" size="16" class="mr-1" />清空
        </v-btn>
      </div>
      <div class="d-flex flex-wrap justify-center ga-2 mt-1">
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

    <div class="flex-grow-1" />
  </div>
</template>
