<script setup lang="ts">
// 首页框架：顶栏（仅动态标题）+ 主体为 @zebra-ui/swiper 滑动窗口内嵌 4 页
// （搜索 / 搜索源 / 历史 / 设置），可左右滑动切换；
// 桌面端顶部分段式 tab（v-btn-toggle），移动端底部导航（v-btn 堆叠图标 + 文字）。
// 设置页不再是独立路由，作为 swiper 的第 4 页存在。
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ZSwiper, ZSwiperItem } from '@zebra-ui/swiper'
import type Swiper from '@zebra-ui/swiper/types/swiper-class'
import type { SwiperInterface } from '@zebra-ui/swiper/types/swiper-class'
import '@zebra-ui/swiper/index.scss'
import HomeView from '../views/HomeView.vue'
import SitesView from '../views/SitesView.vue'
import HistoryView from '../views/HistoryView.vue'
import SettingsView from '../views/SettingsView.vue'
import { keyboardHeight, keyboardOpen } from '../lib/insets'

const route = useRoute()

const tabs = [
  { title: '搜索', icon: 'mdi-text-search' },
  { title: '搜索源', icon: 'mdi-antenna' },
  { title: '历史', icon: 'mdi-history' },
  { title: '设置', icon: 'mdi-cog-outline' },
]
const titles = tabs.map((t) => t.title)
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
</script>

<template>
  <div class="shell" :style="shellStyle">
    <!-- 顶栏：仅保留随 tab 变化的标题 -->
    <v-toolbar class="topbar" color="surface" flat height="auto">
      <span class="topbar__title">{{ appTitle }}</span>
    </v-toolbar>

    <!-- 桌面端分段式 tab -->
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
        v-for="(t, i) in tabs"
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
            <SettingsView />
          </div>
        </z-swiper-item>
      </z-swiper>
    </div>

    <!-- 移动端底部导航（键盘弹出时隐藏，避免遮挡输入区） -->
    <v-sheet v-show="!keyboardOpen" class="bottomnav" color="surface" flat>
      <v-btn
        v-for="(t, i) in tabs"
        :key="i"
        class="bottomnav__item"
        :class="{ 'bottomnav__item--active': tab === i }"
        :color="tab === i ? 'primary' : undefined"
        variant="text"
        stacked
        :ripple="false"
        @click="goTab(i)"
      >
        <v-icon :icon="t.icon" size="21" />
        <span class="bottomnav__label">{{ t.title }}</span>
      </v-btn>
    </v-sheet>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  height: 100dvh;
  background: rgb(var(--v-theme-surface));
  overflow: hidden;
  transition: padding-bottom 0.18s ease;
}

/* ---------- 顶栏 ---------- */
.topbar {
  display: flex;
  align-items: center;
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
  padding: 0;
}

.topbar__title {
  font-size: 1.06rem;
  font-weight: 700;
  letter-spacing: -0.01em;
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
}

/* 自定义元素默认 inline，height:100% 会被忽略 */
z-swiper,
z-swiper-item {
  display: block;
  height: 100%;
}

/* ---------- 移动底部导航 ---------- */
.bottomnav {
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
</style>
