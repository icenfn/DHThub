<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke, isTauri } from '../lib/tauri'
import UpdateDialog from '../components/UpdateDialog.vue'

const version = ref('0.2.0')
const updateOpen = ref(false)

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

async function openRepo() {
  const url = 'https://github.com/icenfn/DHThub'
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 760px">
    <v-card rounded="xl" class="mt-4 pa-4">
      <div class="d-flex flex-column align-center text-center pa-4">
        <v-avatar color="primary" size="72" rounded="lg">
          <v-icon icon="mdi-flash-outline" size="40" />
        </v-avatar>
        <div class="text-h6 font-weight-bold mt-3">DHThub</div>
        <div class="text-body-2 text-medium-emphasis">多源磁力链接聚合搜索</div>
        <v-chip size="small" variant="tonal" color="primary" class="mt-2">v{{ version }}</v-chip>
        <div class="text-caption text-medium-emphasis mt-3" style="max-width: 480px">
          Tauri 2.12 · Vue 3 · Vuetify 4（MD3）· Rust
          <br />Linux / Windows / Android 三端，GitHub Actions 自动构建发布
        </div>
        <div class="d-flex ga-2 mt-4">
          <v-btn variant="tonal" color="primary" prepend-icon="mdi-update" @click="updateOpen = true">
            检查更新
          </v-btn>
          <v-btn variant="tonal" color="secondary" prepend-icon="mdi-github" @click="openRepo">
            GitHub 仓库
          </v-btn>
        </div>
      </div>
      <v-divider class="my-2" />
      <div class="text-caption text-center text-medium-emphasis pa-2">
        开源项目 · 不包含广告与商业追踪 · 使用前请阅读
        <a class="text-primary" href="javascript:void(0)" @click="$router.push('/disclaimer')">免责声明</a>
      </div>
    </v-card>
    <UpdateDialog v-model="updateOpen" />
  </div>
</template>
