<script setup lang="ts">
// 首页框架：应用栏（左上角仅标题，随 tab 变化；右上角仅设置按钮）
// 主体为滑动窗口内嵌 3 页（搜索 / 站点 / 历史），手机左右滑动切换，tab 跟随
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import HomeView from '../views/HomeView.vue'
import SitesView from '../views/SitesView.vue'
import HistoryView from '../views/HistoryView.vue'

const router = useRouter()
const TAB_COUNT = 3
const tab = ref(0)
const titles = ['搜索', '站点管理', '历史记录']
const winRef = ref<HTMLElement | null>(null)
/** 程序化滚动期间屏蔽 scroll 事件回写，避免抖动 */
let syncing = false
let syncTimer: ReturnType<typeof setTimeout> | undefined

const appTitle = computed(() => titles[tab.value])

function scrollToTab(i: number) {
  const el = winRef.value
  if (!el) return
  syncing = true
  if (syncTimer) clearTimeout(syncTimer)
  syncTimer = setTimeout(() => {
    syncing = false
  }, 400)
  el.scrollTo({ left: i * el.clientWidth, behavior: 'smooth' })
}

function onScroll() {
  const el = winRef.value
  if (!el || syncing) return
  const i = Math.round(el.scrollLeft / el.clientWidth)
  if (i !== tab.value && i >= 0 && i < TAB_COUNT) {
    tab.value = i
  }
}

watch(tab, (v) => {
  document.title = `${titles[v]} · DHThub`
})

onMounted(() => {
  document.title = `${titles[0]} · DHThub`
})
</script>

<template>
  <div class="d-flex flex-column" style="height: 100dvh">
    <v-app-bar color="surface" border="b" height="56">
      <v-app-bar-title>
        <span class="text-subtitle-1 font-weight-bold">{{ appTitle }}</span>
      </v-app-bar-title>
      <template #append>
        <v-btn icon="mdi-cog-outline" title="设置" variant="text" @click="router.push('/settings')" />
      </template>
    </v-app-bar>

    <v-tabs v-model="tab" color="primary" grow density="comfortable" @update:model-value="scrollToTab(Number($event))">
      <v-tab v-for="(t, i) in titles" :key="i">{{ t }}</v-tab>
    </v-tabs>

    <!-- 滑动窗口：3 页横向排布，scroll-snap 切换，各页内部独立纵向滚动 -->
    <div ref="winRef" class="swipe-window" @scroll.passive="onScroll">
      <div class="swipe-track">
        <section class="swipe-page"><HomeView /></section>
        <section class="swipe-page"><SitesView /></section>
        <section class="swipe-page"><HistoryView /></section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.swipe-window {
  flex: 1;
  min-height: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scroll-snap-type: x mandatory;
  scroll-behavior: smooth;
  scrollbar-width: none;
}
.swipe-window::-webkit-scrollbar {
  display: none;
}
.swipe-track {
  display: flex;
  width: 300%;
  height: 100%;
}
.swipe-page {
  width: 33.3333%;
  flex-shrink: 0;
  height: 100%;
  overflow-y: auto;
  scroll-snap-align: start;
}
</style>
