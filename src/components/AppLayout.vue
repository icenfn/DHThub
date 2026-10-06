<script setup lang="ts">
// 首页框架：顶栏（标题随 tab 变化 + 设置入口）+ 主体为 @zebra-ui/swiper 滑动窗口内嵌 3 页
// （搜索 / 站点 / 历史），左右滑动切换；桌面端顶部分段式 tab，移动端底部导航栏
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ZSwiper, ZSwiperItem } from '@zebra-ui/swiper'
import type Swiper from '@zebra-ui/swiper/types/swiper-class'
import type { SwiperInterface } from '@zebra-ui/swiper/types/swiper-class'
import '@zebra-ui/swiper/index.scss'
import HomeView from '../views/HomeView.vue'
import SitesView from '../views/SitesView.vue'
import HistoryView from '../views/HistoryView.vue'

const router = useRouter()
const tabs = [
  { title: '搜索', icon: 'mdi-text-search' },
  { title: '搜索源', icon: 'mdi-antenna' },
  { title: '历史记录', icon: 'mdi-history' },
]
const titles = tabs.map((t) => t.title)
const icons = tabs.map((t) => t.icon)
const tab = ref(0)
const swiperRef = ref<SwiperInterface | null>(null)

const appTitle = computed(() => titles[tab.value])

// 事件回调参数按库声明的 Swiper 类型接收，实例成员通过 SwiperInterface 类型访问
function onSwiper(swiper: Swiper) {
  swiperRef.value = swiper as unknown as SwiperInterface
}

function onSlideChange(swiper: Swiper) {
  const s = swiper as unknown as SwiperInterface
  tab.value = s.activeIndex
  document.title = `${titles[tab.value]} · DHThub`
}

function goTab(i: number) {
  swiperRef.value?.slideTo(i, 300)
}
</script>

<template>
  <div class="app-shell">
    <!-- 顶栏：药丸式标题 + 设置入口 -->
    <header class="app-topbar">
      <div class="app-topbar__brand">
        <v-avatar color="primary-container" rounded="lg" size="34">
          <v-icon icon="mdi-magnet" color="on-primary-container" size="20" />
        </v-avatar>
        <div class="app-topbar__title">
          <span class="text-title-medium font-weight-bold">{{ appTitle }}</span>
          <span class="text-label-small text-medium-emphasis">DHThub · 多源磁力聚合</span>
        </div>
      </div>
      <v-btn
        icon="mdi-cog-outline"
        variant="text"
        title="设置"
        size="small"
        @click="router.push('/settings')"
      />
    </header>

    <!-- 桌面端：分段式药丸 tab（MD3 风格） -->
    <div class="d-none d-sm-flex justify-center py-2">
      <div class="segmented-tabs">
        <button
          v-for="(t, i) in tabs"
          :key="i"
          class="segmented-tabs__item"
          :class="{ 'segmented-tabs__item--active': tab === i }"
          type="button"
          @click="goTab(i)"
        >
          <v-icon :icon="t.icon" size="18" />
          <span>{{ t.title }}</span>
        </button>
      </div>
    </div>

    <div class="app-body">
      <z-swiper style="height: 100%" @swiper="onSwiper" @slide-change="onSlideChange">
        <z-swiper-item style="height: 100%">
          <div class="swiper-page-container">
            <HomeView />
          </div>
        </z-swiper-item>
        <z-swiper-item style="height: 100%">
          <div class="swiper-page-container">
            <SitesView />
          </div>
        </z-swiper-item>
        <z-swiper-item style="height: 100%">
          <div class="swiper-page-container">
            <HistoryView />
          </div>
        </z-swiper-item>
      </z-swiper>
    </div>

    <!-- 移动端底部导航栏 -->
    <v-bottom-navigation
      v-model="tab"
      grow
      mandatory
      height="68"
      color="primary"
      class="d-sm-none app-bottomnav"
      @update:model-value="goTab(Number($event))"
    >
      <v-btn v-for="(t, i) in tabs" :key="i">
        <v-icon :icon="t.icon" size="22" />
        <span class="text-label-medium">{{ t.title }}</span>
      </v-btn>
    </v-bottom-navigation>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  background: rgb(var(--v-theme-surface));
}

.app-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 18px;
  background: rgb(var(--v-theme-surface-container-low));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  flex-shrink: 0;
}

.app-topbar__brand {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.app-topbar__title {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
  min-width: 0;
}

/* 分段式药丸 tab：MD3 secondary container 交互 */
.segmented-tabs {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  border-radius: 999px;
  background: rgb(var(--v-theme-surface-container-high));
}

.segmented-tabs__item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 18px;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.86rem;
  font-weight: 600;
  cursor: pointer;
  transition:
    background 0.18s ease,
    color 0.18s ease;
}

.segmented-tabs__item:hover {
  background: rgba(var(--v-theme-on-surface), 0.06);
}

.segmented-tabs__item--active {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
}

.segmented-tabs__item--active:hover {
  background: rgb(var(--v-theme-primary));
}

.app-body {
  flex-grow: 1;
  min-height: 0;
}

.app-bottomnav {
  background: rgb(var(--v-theme-surface-container)) !important;
  padding-bottom: env(safe-area-inset-bottom);
  flex-shrink: 0;
  position: static;
  width: 100%;
}

.swiper-page-container {
  overflow-y: auto;
  overflow-x: hidden;
  height: 100%;
}

/* 自定义元素默认 display:inline，height:100% 会被忽略导致内容撑高、挤压底部栏 */
z-swiper,
z-swiper-item {
  display: block;
  height: 100%;
}
</style>
