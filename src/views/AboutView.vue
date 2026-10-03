<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke, isTauri } from '../lib/tauri'
import { RELEASES_PAGE_URL } from '../lib/update'

const version = ref('0.4.0')

onMounted(async () => {
  if (isTauri) {
    try {
      const { getVersion } = await import('@tauri-apps/api/app')
      version.value = await getVersion()
    } catch {
      /* ignore */
    }
  }
})

async function openUrlSafe(url: string) {
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}
</script>

<template>
  <div>
    <!-- 独立页框架：返回顶栏 + 内容 -->
    <v-app-bar color="surface" border="b" height="52">
      <template #prepend>
        <v-btn icon="mdi-arrow-left" variant="text" title="返回" @click="$router.push('/')" />
      </template>
      <v-app-bar-title>
        <span class="text-subtitle-1 font-weight-bold">关于</span>
      </v-app-bar-title>
    </v-app-bar>

    <v-main>
      <div class="px-3 px-sm-6 py-3 mx-auto" style="max-width: 760px">
        <v-card rounded="lg" class="mt-3 pa-3">
          <div class="d-flex flex-column align-center text-center pa-4">
            <v-avatar color="primary" size="64" rounded="lg">
              <v-icon icon="mdi-flash-outline" size="36" />
            </v-avatar>
            <div class="text-h6 font-weight-bold mt-2">DHThub</div>
            <div class="text-body-2 text-medium-emphasis">多源磁力链接聚合搜索</div>
            <v-chip size="small" variant="tonal" color="primary" class="mt-1">v{{ version }}</v-chip>
            <div class="text-caption text-medium-emphasis mt-2" style="max-width: 480px">
              Tauri 2.12 · Vue 3 · Vuetify 4（MD3）· Rust
              <br />Linux / Windows / Android 三端，GitHub Actions 自动构建发布
            </div>
            <div class="d-flex flex-wrap justify-center ga-2 mt-3">
              <v-btn variant="tonal" color="primary" prepend-icon="mdi-github" @click="openUrlSafe('https://github.com/icenfn/DHThub')">
                GitHub 仓库
              </v-btn>
              <v-btn variant="tonal" color="secondary" prepend-icon="mdi-history" @click="openUrlSafe(RELEASES_PAGE_URL)">
                版本历史
              </v-btn>
            </div>
          </div>
          <v-divider class="my-2" />
          <div class="text-caption text-center text-medium-emphasis pa-2">
            开源项目（MIT License）· 不包含广告与商业追踪
          </div>
        </v-card>
      </div>
    </v-main>
  </div>
</template>
