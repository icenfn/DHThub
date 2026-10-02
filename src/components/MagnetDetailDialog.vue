<script setup lang="ts">
import { ref, watch } from 'vue'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { openUrl } from '@tauri-apps/plugin-opener'
import { isTauri } from '../lib/tauri'
import type { MagnetItem } from '../types'

const props = defineProps<{
  item: MagnetItem | null
  siteName: string
  keyword: string
}>()

const model = defineModel<boolean>({ required: true })
const toast = ref('')
const showToast = ref(false)

watch(
  () => model.value,
  (v) => {
    if (v) toast.value = ''
  },
)

async function copy() {
  if (!props.item) return
  try {
    await writeText(props.item.magnet || props.item.url)
    toast.value = '磁力链接已复制到剪贴板'
    showToast.value = true
  } catch (e) {
    // 浏览器降级
    try {
      await navigator.clipboard.writeText(props.item.magnet || props.item.url)
      toast.value = '磁力链接已复制'
      showToast.value = true
    } catch {
      toast.value = `复制失败：${e}`
      showToast.value = true
    }
  }
}

async function openMagnet() {
  if (!props.item) return
  const target = props.item.magnet || props.item.url
  try {
    if (isTauri) {
      await openUrl(target)
      toast.value = '已交给系统处理'
      showToast.value = true
    } else {
      window.open(target, '_blank')
    }
  } catch (e) {
    toast.value = `无法打开：${e}`
    showToast.value = true
  }
}

async function share() {
  if (!props.item) return
  const text = `${props.item.title}\n${props.item.magnet || props.item.url}`
  try {
    if (typeof navigator.share === 'function') {
      await navigator.share({ title: props.item.title, text })
    } else {
      await writeText(text)
      toast.value = '已复制（当前环境不支持系统分享）'
      showToast.value = true
    }
  } catch (e) {
    toast.value = `分享失败：${e}`
    showToast.value = true
  }
}
</script>

<template>
  <v-dialog v-model="model" max-width="640">
    <v-card v-if="item">
      <v-card-title class="d-flex align-center pr-3">
        <v-icon icon="mdi-magnet" color="primary" class="mr-2" />
        <span class="text-subtitle-1 font-weight-bold">磁力链接详情</span>
        <v-spacer />
        <v-chip size="small" variant="tonal" color="secondary">{{ siteName }}</v-chip>
      </v-card-title>
      <v-divider />
      <v-card-text>
        <div class="text-subtitle-2 font-weight-bold mb-1">标题</div>
        <p class="text-body-2 mb-3">{{ item.title }}</p>
        <div class="d-flex flex-wrap ga-4 text-body-2 mb-3">
          <span v-if="item.size"><v-icon icon="mdi-database-outline" size="16" class="mr-1" />{{ item.size }}</span>
          <span v-if="item.date"><v-icon icon="mdi-calendar-outline" size="16" class="mr-1" />{{ item.date }}</span>
        </div>
        <div class="text-subtitle-2 font-weight-bold mb-1">磁力地址</div>
        <v-textarea
          :model-value="item.magnet || item.url"
          readonly
          variant="outlined"
          rows="3"
          auto-grow
          hide-details
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="tonal" color="secondary" prepend-icon="mdi-share-variant-outline" @click="share">
          分享
        </v-btn>
        <v-btn variant="tonal" color="info" prepend-icon="mdi-open-in-new" @click="openMagnet">
          打开
        </v-btn>
        <v-btn color="primary" prepend-icon="mdi-content-copy" @click="copy">复制</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-snackbar v-model="showToast" location="bottom" color="success" timeout="2000">
    {{ toast }}
  </v-snackbar>
</template>
