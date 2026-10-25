<script setup lang="ts">
// 首页（搜索 tab）：内容在可视区域水平 + 垂直居中
// 搜索框上方只保留应用图标；搜索按钮使用 search 图标。
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { refDebounced } from '@vueuse/core'
import { settings } from '../stores/settings'
import { mirrorUrl, HOTWORDS_URL } from '../lib/mirrors'
import { httpGetText } from '../lib/http'

const router = useRouter()
const keyword = ref('')
const hotWords = ref<string[]>([])
const searchHistory = ref<string[]>([])
const hotLoading = ref(false)

// 输入防抖：避免每次按键都触发按钮可用性重算与潜在的重复渲染
const debouncedKeyword = refDebounced(keyword, 120)
const canSearch = computed(() => debouncedKeyword.value.trim().length > 0)

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
  // 每次点搜索都带上新的请求序号 t：即使关键词与上次相同，搜索结果页也会强制重新搜索
  router.push({ path: '/search', query: { k, t: String(Date.now()) } })
}

function selectHotWord(w: string) {
  doSearch(w)
}

async function clearSearchHistory() {
  await settings.clearSearchHistory()
  refreshLocalWords()
}

/** 从仓库抓取热词总表（经选中镜像 + 超时），成功后本地缓存；失败用缓存兜底 */
async function loadHotWords() {
  hotWords.value = [...settings.get('hotWords')]
  hotLoading.value = true
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
  } finally {
    hotLoading.value = false
  }
}

onMounted(async () => {
  refreshLocalWords()
  void loadHotWords()
})
</script>

<template>
  <div class="home">
    <div class="home__inner">
      <!-- 搜索框上方只保留应用图标 -->
      <div class="hero">
        <img class="hero__icon" src="/app-icon.png" alt="DHThub" draggable="false" />
      </div>

      <!-- 搜索输入 -->
      <v-text-field
        v-model="keyword"
        class="home__field"
        variant="solo"
        density="comfortable"
        rounded="pill"
        flat
        hide-details
        single-line
        clearable
        placeholder="输入关键词：电影 / 剧集 / 游戏"
        enterkeyhint="search"
        autocomplete="off"
        @keyup.enter="doSearch()"
      >
        <template #append-inner>
          <v-btn
            icon="mdi-magnify"
            color="primary"
            variant="flat"
            size="small"
            rounded="pill"
            class="home__go"
            :disabled="!canSearch"
            title="搜索"
            @click="doSearch()"
          />
        </template>
      </v-text-field>

      <!-- 热门推荐 -->
      <section v-if="hotWords.length" class="block">
        <div class="block__head">
          <span class="section-label">
            <v-icon icon="mdi-fire" size="14" color="warning" class="mr-1" />热门推荐
          </span>
          <v-btn
            variant="text"
            size="x-small"
            color="primary"
            prepend-icon="mdi-refresh"
            class="home__link"
            @click="shuffleHotWords"
          >
            换一换
          </v-btn>
        </div>
        <div class="chips">
          <v-chip
            v-for="(w, i) in hotWords.slice(0, 12)"
            :key="`h${i}`"
            variant="tonal"
            color="primary"
            size="small"
            link
            @click="selectHotWord(w)"
          >
            {{ w }}
          </v-chip>
        </div>
      </section>

      <!-- 搜索历史 -->
      <section v-if="searchHistory.length" class="block">
        <div class="block__head">
          <span class="section-label">
            <v-icon icon="mdi-history" size="14" class="mr-1" />搜索历史
          </span>
          <v-btn
            variant="text"
            size="x-small"
            color="error"
            prepend-icon="mdi-delete-outline"
            class="home__link"
            @click="clearSearchHistory"
          >
            清空
          </v-btn>
        </div>
        <div class="chips">
          <v-chip
            v-for="(w, i) in searchHistory.slice(0, 12)"
            :key="`s${i}`"
            variant="outlined"
            size="small"
            link
            @click="selectHotWord(w)"
          >
            {{ w }}
          </v-chip>
        </div>
      </section>

      <!-- 空态引导 -->
      <section v-if="!hotWords.length && !searchHistory.length" class="block">
        <v-alert
          icon="mdi-lightbulb-on-outline"
          color="primary"
          variant="tonal"
          density="compact"
          rounded="lg"
          text="先到「搜索源」订阅站点，启用后即可并发搜索"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
/* 内容整体在可视区域内水平 + 垂直居中：
   用 margin:auto 而非 justify-content:center，内容高于容器时不会被裁掉顶部 */
.home {
  min-height: 100%;
  display: flex;
  width: 100%;
}

.home__inner {
  width: 100%;
  max-width: 640px;
  margin: auto;
  padding: 20px 18px;
  display: flex;
  flex-direction: column;
}

/* ---------- 应用图标 ---------- */
.hero {
  display: flex;
  justify-content: center;
  margin-bottom: 24px;
}

.hero__icon {
  width: 84px;
  height: 84px;
  border-radius: 22px;
  object-fit: contain;
  user-select: none;
  -webkit-user-drag: none;
}

/* ---------- 搜索框 ---------- */
.home__field {
  margin-bottom: 22px;
}

.home__field :deep(.v-field) {
  background: rgb(var(--v-theme-surface-container));
  /* 尾部内嵌搜索按钮留出空隙 */
  padding-inline-end: 4px;
}

.home__field :deep(.v-field--focused) {
  background: rgb(var(--v-theme-surface));
}

/* ---------- 区块 ---------- */
.block + .block {
  margin-top: 18px;
}

.block__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 9px;
}

.home__link {
  text-transform: none;
  letter-spacing: 0;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}

/* 搜索按钮：与输入框同高、同圆角，内嵌在字段尾部 */
.home__go {
  width: 36px !important;
  height: 36px !important;
  min-width: 36px;
  margin-inline-end: 4px;
}
</style>
