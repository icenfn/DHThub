<script setup lang="ts">
import { ref, watch } from 'vue'
import { pushOverlay } from '../lib/overlay'
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

// 弹层栈注册：手机返回键 / PC ESC 关闭
watch(model, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { model.value = false }))
  if (v) toast.value = ''
})

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
  const text = `${props.item.title}
${props.item.magnet || props.item.url}`
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
  <v-dialog v-model="model" max-width="620">
    <v-card v-if="item" rounded="xl" variant="flat" color="surface-container-low">
      <v-card-item class="pt-4">
        <template #prepend>
          <v-avatar color="primary-container" rounded="lg">
            <v-icon icon="mdi-magnet" color="on-primary-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-title-medium font-weight-bold">磁力链接详情</v-card-title>
        <v-card-subtitle class="text-body-small">
          <v-chip size="x-small" variant="tonal" color="secondary">{{ siteName }}</v-chip>
        </v-card-subtitle>
      </v-card-item>

      <v-card-text class="pt-2">
        <div class="text-label-large text-medium-emphasis mb-1">标题</div>
        <p class="text-body-medium mb-4">{{ item.title }}</p>

        <div class="d-flex flex-wrap ga-2 mb-4">
          <v-chip v-if="item.size" size="small" variant="tonal" prepend-icon="mdi-database-outline">
            {{ item.size }}
          </v-chip>
          <v-chip v-if="item.date" size="small" variant="tonal" prepend-icon="mdi-calendar-outline">
            {{ item.date }}
          </v-chip>
        </div>

        <div class="text-label-large text-medium-emphasis mb-1">磁力地址</div>
        <v-textarea
          :model-value="item.magnet || item.url"
          readonly
          variant="outlined"
          rounded="lg"
          rows="3"
          auto-grow
          hide-details
        />
      </v-card-text>

      <v-card-actions class="px-4 pb-3">
        <v-btn variant="text" color="secondary" prepend-icon="mdi-share-variant-outline" @click="share">
          分享
        </v-btn>
        <v-btn variant="text" color="info" prepend-icon="mdi-open-in-new" @click="openMagnet">
          打开
        </v-btn>
        <v-spacer />
        <v-btn color="primary" variant="flat" prepend-icon="mdi-content-copy" @click="copy">复制</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-snackbar v-model="showToast" location="bottom" color="inverse-surface" rounded="lg" timeout="2000">
    {{ toast }}
  </v-snackbar>
</template>
