<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke, isAndroid, isTauri } from '../lib/tauri'
import { settings } from '../stores/settings'
import type { UpdateInfo } from '../types'

const model = defineModel<boolean>({ required: true })

const checking = ref(false)
const info = ref<UpdateInfo | null>(null)
const error = ref('')
const downloading = ref(false)
const progress = ref<{ percent: number; speed: number; received: number; total: number } | null>(null)
const desktopUpdating = ref(false)
let unlisten: UnlistenFn | null = null

watch(
  () => model.value,
  async (v) => {
    if (v) await check()
  },
)

async function check() {
  checking.value = true
  error.value = ''
  info.value = null
  try {
    const mirror = settings.getSelectedMirror()
    info.value = await invoke<UpdateInfo>('check_update', { mirrorBase: mirror.base })
  } catch (e) {
    error.value = String(e)
  } finally {
    checking.value = false
  }
}
function releasePage(): string {
  return `https://github.com/icenfn/DHThub/releases/latest`
}

async function openDownloadPage() {
  try {
    await openUrl(releasePage())
  } catch {
    window.open(releasePage(), '_blank')
  }
}

async function startDownload() {
  const apk = info.value?.assets?.android?.[0]
  if (!apk) {
    await openDownloadPage()
    return
  }
  downloading.value = true
  progress.value = null
  try {
    unlisten = await listen<{ percent: number; speed: number; received: number; total: number }>(
      'apk-download-progress',
      (e) => {
        progress.value = e.payload
      },
    )
    const mirror = settings.getSelectedMirror()
    const path = await invoke<string>('download_apk', { url: apk.url, mirrorBase: mirror.base })
    if (isAndroid()) {
      await invoke('install_apk', { path })
      error.value = ''
    } else {
      error.value = `安装包已下载：${path}`
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    downloading.value = false
    unlisten?.()
    unlisten = null
  }
}

async function desktopAutoUpdate() {
  desktopUpdating.value = true
  error.value = ''
  try {
    if (!isTauri) throw new Error('浏览器预览模式')
    const { check } = await import('@tauri-apps/plugin-updater')
    const update = await check()
    if (!update) {
      error.value = '桌面端暂未发现可自动更新的版本（需配置签名密钥）'
      return
    }
    error.value = '开始下载并安装更新…'
    await update.downloadAndInstall()
    error.value = '更新完成，请重启应用'
  } catch (e) {
    error.value = `自动更新不可用（${e}）。请改用「打开下载页」手动更新。`
  } finally {
    desktopUpdating.value = false
  }
}

function fmtSize(n: number) {
  if (!n) return ''
  return n > 1024 ** 2 ? `${(n / 1024 ** 2).toFixed(1)}MB` : `${(n / 1024).toFixed(1)}KB`
}

onBeforeUnmount(() => unlisten?.())
</script>

<template>
  <v-dialog v-model="model" max-width="560">
    <v-card>
      <v-card-title class="text-subtitle-1 font-weight-bold">
        <v-icon icon="mdi-update" color="primary" class="mr-2" />检查更新
      </v-card-title>
      <v-divider />
      <v-card-text>
        <div v-if="checking" class="text-center pa-6">
          <v-progress-circular indeterminate color="primary" />
          <div class="mt-3 text-medium-emphasis">
            正在检查更新（经 {{ settings.getSelectedMirror().name }}）…
          </div>
        </div>

        <v-alert v-else-if="error && !info" type="error" variant="tonal">{{ error }}</v-alert>

        <template v-else-if="info">
          <v-alert
            :type="info.has_update ? 'info' : 'success'"
            variant="tonal"
            class="mb-3"
          >
            <template #title>
              当前 v{{ info.current_version }} → {{ info.has_update ? `发现新版本 v${info.latest_version}` : '已是最新版本' }}
            </template>
            <div class="text-caption mt-1" v-if="info.published_at">发布于 {{ info.published_at }}</div>
          </v-alert>

          <div v-if="info.notes" class="mb-3">
            <div class="text-subtitle-2 font-weight-bold mb-1">更新说明</div>
            <pre class="text-body-2 update-notes">{{ info.notes }}</pre>
          </div>

          <div v-if="Object.keys(info.assets).length" class="mb-2">
            <div class="text-subtitle-2 font-weight-bold mb-1">可用安装包</div>
            <v-list density="compact">
              <v-list-item v-for="(list, platform) in info.assets" :key="platform">
                <template #prepend>
                  <v-icon :icon="platform === 'android' ? 'mdi-cellphone' : platform === 'windows' ? 'mdi-microsoft-windows' : 'mdi-linux'" />
                </template>
                <v-list-item-title class="text-body-2">{{ platform }}</v-list-item-title>
                <v-list-item-subtitle class="text-caption">
                  {{ list.map((a) => `${a.name}${a.size ? `（${fmtSize(a.size)}）` : ''}`).join('、') }}
                </v-list-item-subtitle>
              </v-list-item>
            </v-list>
          </div>

          <!-- Android 下载进度 -->
          <div v-if="downloading" class="mt-3">
            <v-progress-linear :model-value="progress?.percent ?? 0" color="primary" striped />
            <div class="text-caption mt-1 text-medium-emphasis">
              下载中 {{ progress?.percent ?? 0 }}%
              <span v-if="progress?.speed"> · {{ progress.speed }} KB/s</span>
            </div>
          </div>

          <v-alert v-if="error && info" type="warning" variant="tonal" class="mt-2">{{ error }}</v-alert>
        </template>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="model = false">关闭</v-btn>
        <template v-if="info?.has_update">
          <v-btn v-if="isAndroid()" color="primary" variant="flat" :loading="downloading" :disabled="desktopUpdating" @click="startDownload">
            <v-icon icon="mdi-download" class="mr-1" />下载并安装
          </v-btn>
          <template v-else>
            <v-btn variant="tonal" color="secondary" :disabled="downloading || desktopUpdating" @click="openDownloadPage">
              打开下载页
            </v-btn>
            <v-btn color="primary" variant="flat" :loading="desktopUpdating" :disabled="downloading" @click="desktopAutoUpdate">
              自动更新
            </v-btn>
          </template>
        </template>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.update-notes {
  white-space: pre-wrap;
  max-height: 200px;
  overflow-y: auto;
  background: rgba(127, 127, 127, 0.08);
  padding: 8px 10px;
  border-radius: 8px;
}
</style>
