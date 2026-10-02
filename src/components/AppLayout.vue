<script setup lang="ts">
// 首页框架：应用栏（左上角仅标题，随 tab 变化；右上角仅设置按钮）
// 主体为 @zebra-ui/swiper 滑动窗口内嵌 3 页（搜索 / 站点 / 历史），左右滑动切换，tab 跟随
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
const titles = ['搜索', '站点管理', '历史记录']
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
  <div class="d-flex flex-column" style="height: 100dvh">
    <v-app-bar color="surface" border="b" height="52">
      <v-app-bar-title>
        <span class="text-subtitle-1 font-weight-bold">{{ appTitle }}</span>
      </v-app-bar-title>
      <template #append>
        <v-btn icon="mdi-cog-outline" title="设置" variant="text" @click="router.push('/settings')" />
      </template>
    </v-app-bar>

    <v-tabs v-model="tab" color="primary" grow density="compact" @update:model-value="goTab(Number($event))">
      <v-tab v-for="(t, i) in titles" :key="i">{{ t }}</v-tab>
    </v-tabs>

    <div class="flex-grow-1" style="min-height: 0">
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
  </div>
</template>

<style scoped>
.swiper-page-container {
  overflow-y: auto;
  overflow-x: hidden;
  height: 100%;
}
</style>
