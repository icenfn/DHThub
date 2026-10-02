<script setup lang="ts">
// 主页面框架：顶栏 + 桌面抽屉 + 移动端底栏（搜索 / 站点 / 历史 / 关于共用）
import { useRoute, useRouter } from 'vue-router'
import { useDisplay } from 'vuetify'
import { checkUpdate } from '../lib/update'

const display = useDisplay()
const router = useRouter()
const route = useRoute()

const navItems = [
  { to: '/', icon: 'mdi-magnify', label: '搜索' },
  { to: '/sites', icon: 'mdi-antenna', label: '站点' },
  { to: '/history', icon: 'mdi-history', label: '历史' },
]

function go(to: string) {
  router.push(to)
}
</script>

<template>
  <!-- MD3 顶部应用栏 -->
  <v-app-bar color="surface" border="b" height="56">
    <template #prepend>
      <div class="d-flex align-center ml-2">
        <v-icon icon="mdi-flash-outline" color="primary" size="28" />
      </div>
    </template>
    <v-app-bar-title>
      <span class="text-subtitle-1 font-weight-bold">DHThub</span>
      <span class="text-caption text-medium-emphasis ml-2 d-none d-sm-inline">磁力聚合搜索</span>
    </v-app-bar-title>
    <template #append>
      <v-btn icon="mdi-update" title="检查更新" variant="text" @click="checkUpdate" />
      <v-btn
        icon="mdi-cog-outline"
        title="设置"
        variant="text"
        :active="route.path === '/settings'"
        @click="go('/settings')"
      />
    </template>
  </v-app-bar>

  <!-- 桌面端导航抽屉 -->
  <v-navigation-drawer v-if="!display.mobile.value" width="216" :permanent="true" color="surface">
    <v-divider class="mx-4 mt-2" />
    <v-list nav density="comfortable" class="px-2 py-2">
      <v-list-item
        v-for="item in navItems"
        :key="item.to"
        :active="route.path === item.to"
        :prepend-icon="item.icon"
        :title="item.label"
        rounded="xl"
        @click="go(item.to)"
      />
    </v-list>
    <template #append>
      <div class="pa-4 text-caption text-medium-emphasis">v0.2.6 · GitHub 发布</div>
    </template>
  </v-navigation-drawer>

  <v-main class="pb-16 pb-sm-0">
    <router-view />
  </v-main>

  <!-- 移动端底部导航 -->
  <v-bottom-navigation
    v-if="display.mobile.value"
    :model-value="route.path"
    color="primary"
    grow
  >
    <v-btn v-for="item in navItems" :key="item.to" :value="item.to" @click="go(item.to)">
      <v-icon>{{ item.icon }}</v-icon>
      {{ item.label }}
    </v-btn>
  </v-bottom-navigation>
</template>
