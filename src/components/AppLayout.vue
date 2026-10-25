<script setup lang="ts">
// 首页框架：顶栏（居中动态标题）+ 主体为 @zebra-ui/swiper 滑动窗口内嵌 5 页
// （搜索 / 搜索源 / 历史 / 下载 / 设置），可左右滑动切换。
// 底部导航只放 4 个 tab（搜索源 / 历史 / 下载 / 设置），
// 「搜索」页由底部导航正中央悬浮的圆形按钮进入。
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ZSwiper, ZSwiperItem } from '@zebra-ui/swiper'
import type Swiper from '@zebra-ui/swiper/types/swiper-class'
import type { SwiperInterface } from '@zebra-ui/swiper/types/swiper-class'
import '@zebra-ui/swiper/index.scss'
import HomeView from '../views/HomeView.vue'
import SitesView from '../views/SitesView.vue'
import HistoryView from '../views/HistoryView.vue'
import DownloadsView from '../views/DownloadsView.vue'
import SettingsView from '../views/SettingsView.vue'
import { keyboardHeight, keyboardOpen } from '../lib/insets'

const route = useRoute()

// 页面顺序即 swiper 下标，与底部导航/桌面分段的 index 一一对应
const pages = [
  { title: '搜索', icon: 'mdi-text-search' },
  { title: '搜索源', icon: 'mdi-antenna' },
  { title: '历史', icon: 'mdi-history' },
  { title: '下载', icon: 'mdi-download-outline' },
  { title: '设置', icon: 'mdi-cog-outline' },
]
const titles = pages.map((p) => p.title)

/** 底部导航左组 */
const navLeft = [1, 2]
/** 底部导航右组 */
const navRight = [3, 4]

const tab = ref(0)
const swiperRef = ref<SwiperInterface | null>(null)

const appTitle = computed(() => titles[tab.value] ?? titles[0])

// 键盘弹出时给主体让出键盘高度，输入区不会被输入法遮住
const shellStyle = computed(() => ({
  paddingBottom: keyboardOpen.value ? `${keyboardHeight.value}px` : '0px',
}))

function onSwiper(swiper: Swiper) {
  swiperRef.value = swiper as unknown as SwiperInterface
}

function onSlideChange(swiper: Swiper) {
  const s = swiper as unknown as SwiperInterface
  tab.value = s.activeIndex
  document.title = `${titles[tab.value]} · DHThub`
}

function goTab(i: number) {
  if (i === tab.value) return
  swiperRef.value?.slideTo(i, 280)
}

// 从搜索结果 / 资源详情返回首页时，保持离开前所在的 tab
watch(
  () => route.path,
  (p) => {
    if (p === '/' && swiperRef.value && swiperRef.value.activeIndex !== tab.value) {
      swiperRef.value.slideTo(tab.value, 0)
    }
  },
)

function openSearchPage() {
  goTab(0)
}

function openSitesPage() {
  goTab(1)
}
</script>

<template>
  <div class="shell" :style="shellStyle">
    <!-- 顶栏：仅保留居中标题 -->
    <v-toolbar class="topbar" color="surface" flat height="auto">
      <span class="topbar__title">{{ appTitle }}</span>
    </v-toolbar>

    <!-- 桌面端分段式 tab（含搜索页） -->
    <v-btn-toggle
      class="segmented"
      :model-value="tab"
      mandatory
      divided
      rounded="pill"
      density="comfortable"
      color="primary"
      variant="flat"
      @update:model-value="(v) => typeof v === 'number' && goTab(v)"
    >
      <v-btn
        v-for="(t, i) in pages"
        :key="i"
        :value="i"
        :prepend-icon="t.icon"
        class="segmented__item"
      >
        {{ t.title }}
      </v-btn>
    </v-btn-toggle>

    <!-- 滑动窗口主体 -->
    <div class="body">
      <z-swiper style="height: 100%" @swiper="onSwiper" @slide-change="onSlideChange">
        <z-swiper-item style="height: 100%">
          <div class="pane">
            <HomeView />
          </div>
        </z-swiper-item>
        <z-swiper-item style="height: 100%">
          <div class="pane">
            <SitesView />
          </div>
        </z-swiper-item>
        <z-swiper-item style="height: 100%">
          <div class="pane">
            <HistoryView />
          </div>
        </z-swiper-item>
        <z-swiper-item style="height: 100%">
          <div class="pane">
            <DownloadsView @go-sites="openSitesPage" />
          </div>
        </z-swiper-item>
        <z-swiper-item style="height: 100%">
          <div class="pane">
            <SettingsView />
          </div>
        </z-swiper-item>
      </z-swiper>
    </div>

    <!-- 移动端底部导航（键盘弹出时隐藏，避免遮挡输入区）
         4 个 tab 分列两侧，正中央悬浮圆形「搜索」按钮 -->
    <v-sheet v-show="!keyboardOpen" class="bottomnav" color="surface" flat>
      <div class="bottomnav__group">
        <v-btn
          v-for="i in navLeft"
          :key="i"
          class="bottomnav__item"
          :class="{ 'bottomnav__item--active': tab === i }"
          :color="tab === i ? 'primary' : undefined"
          variant="text"
          stacked
          :ripple="false"
          @click="goTab(i)"
        >
          <v-icon :icon="pages[i].icon" size="21" />
          <span class="bottomnav__label">{{ pages[i].title }}</span>
        </v-btn>
      </div>

      <div class="bottomnav__gap" />

      <div class="bottomnav__group">
        <v-btn
          v-for="i in navRight"
          :key="i"
          class="bottomnav__item"
          :class="{ 'bottomnav__item--active': tab === i }"
          :color="tab === i ? 'primary' : undefined"
          variant="text"
          stacked
          :ripple="false"
          @click="goTab(i)"
        >
          <v-icon :icon="pages[i].icon" size="21" />
          <span class="bottomnav__label">{{ pages[i].title }}</span>
        </v-btn>
      </div>

      <!-- 中央悬浮圆形搜索按钮 -->
      <v-btn
        class="bottomnav__fab"
        :class="{ 'bottomnav__fab--active': tab === 0 }"
        color="primary"
        variant="flat"
        icon
        rounded="circle"
        elevation="6"
        title="搜索"
        aria-label="搜索"
        @click="openSearchPage"
      >
        <v-icon icon="mdi-magnify" size="26" />
      </v-btn>
    </v-sheet>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  /* 旧内核无 dvh：由 lib/viewport.ts 注入的 --app-height 提供确定高度，
     否则 .shell 高度退化为 auto，底部 tab 栏被挤出视口 */
  height: 100%;
  height: var(--app-height, 100%);
  background: rgb(var(--v-theme-surface));
  overflow: hidden;
  transition: padding-bottom 0.18s ease;
}

/* ---------- 顶栏（标题居中） ---------- */
.topbar {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 10px 16px;
  padding-top: calc(10px + var(--safe-area-inset-top, env(safe-area-inset-top, 0px)));
  padding-left: calc(16px + var(--safe-area-inset-left, env(safe-area-inset-left, 0px)));
  padding-right: calc(16px + var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  flex-shrink: 0;
}

.topbar :deep(.v-toolbar__content) {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}

.topbar__title {
  font-size: 1.06rem;
  font-weight: 700;
  letter-spacing: -0.01em;
  text-align: center;
  color: rgb(var(--v-theme-on-surface));
}

/* ---------- 桌面分段 tab（移动端隐藏） ---------- */
.segmented {
  display: none;
}

@media (min-width: 600px) {
  .segmented {
    display: inline-flex;
    align-self: center;
    margin: 8px 0 2px;
    flex-shrink: 0;
  }
}

.segmented__item {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

/* ---------- 主体 ---------- */
.body {
  flex: 1 1 auto;
  min-height: 0;
  position: relative;
}

.pane {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  -webkit-overflow-scrolling: touch;
  /* 横向由 swiper 接管、纵向必须交还给页面：
     否则内嵌页在触摸设备上无法上下滚动 */
  touch-action: pan-y;
  overscroll-behavior-y: contain;
}

/* 自定义元素默认 inline，height:100% 会被忽略 */
z-swiper,
z-swiper-item {
  display: block;
  height: 100%;
  min-height: 0;
}

/* ---------- 移动底部导航 ---------- */
.bottomnav {
  position: relative;
  display: flex;
  align-items: stretch;
  flex-shrink: 0;
  background: rgb(var(--v-theme-surface));
  border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  padding-bottom: var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px));
  padding-left: var(--safe-area-inset-left, env(safe-area-inset-left, 0px));
  padding-right: var(--safe-area-inset-right, env(safe-area-inset-right, 0px));
  border-radius: 0;
}

@media (min-width: 600px) {
  .bottomnav {
    display: none;
  }
}

.bottomnav__group {
  display: flex;
  flex: 1 1 0;
  min-width: 0;
}

/* 中央为悬浮按钮留出的空隙 */
.bottomnav__gap {
  flex: 0 0 72px;
}

.bottomnav__item {
  flex: 1 1 0;
  min-width: 0;
  height: 58px;
  border-radius: 0;
  color: rgb(var(--v-theme-on-surface-variant));
}

.bottomnav__item--active {
  color: rgb(var(--v-theme-primary));
}

.bottomnav__label {
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.01em;
}

/* 悬浮圆形搜索按钮：压在内嵌页面与 tab 栏的交汇处 */
.bottomnav__fab {
  position: absolute;
  left: 50%;
  top: -24px;
  transform: translateX(-50%);
  width: 56px !important;
  height: 56px !important;
  min-width: 56px;
  border-radius: 50%;
  z-index: 3;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
}

.bottomnav__fab--active {
  outline: 2px solid rgba(var(--v-theme-primary), 0.25);
  outline-offset: 2px;
}
</style>
