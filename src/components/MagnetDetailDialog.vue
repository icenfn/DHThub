<script setup lang="ts">
// 磁力详情弹窗：标题 / 大小 / 日期 / 来源 + 磁力地址，支持复制、打开、分享
import { ref, watch } from 'vue'
import { useClipboard } from '@vueuse/core'
import { pushOverlay } from '../lib/overlay'
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
const copied = ref(false)
const { copy: copyToClipboard } = useClipboard({ legacy: true })

// 弹层栈注册：手机返回键 / PC ESC 关闭
watch(model, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { model.value = false }))
  if (v) {
    toast.value = ''
    copied.value = false
  }
})

function flash(msg: string) {
  toast.value = msg
  showToast.value = true
}

async function copy() {
  if (!props.item) return
  const text = props.item.magnet || props.item.url
  if (!text) return
  try {
    const mod = await import('@tauri-apps/plugin-clipboard-manager')
    await mod.writeText(text)
    copied.value = true
    flash('磁力链接已复制到剪贴板')
  } catch {
    try {
      await copyToClipboard(text)
      copied.value = true
      flash('磁力链接已复制')
    } catch (e) {
      flash(`复制失败：${e}`)
    }
  }
}

async function openMagnet() {
  if (!props.item) return
  const target = props.item.magnet || props.item.url
  try {
    if (isTauri) {
      await openUrl(target)
      flash('已交给系统处理')
    } else {
      window.open(target, '_blank')
    }
  } catch (e) {
    flash(`无法打开：${e}`)
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
      await copyToClipboard(text)
      flash('已复制（当前环境不支持系统分享）')
    }
  } catch (e) {
    flash(`分享失败：${e}`)
  }
}

const magnetText = () => props.item?.magnet || props.item?.url || ''
</script>

<template>
  <v-dialog v-model="model" max-width="560" scrollable>
    <v-card v-if="item" rounded="xl" variant="flat" color="surface-container-low" border>
      <!-- 头部 -->
      <div class="md-head">
        <v-avatar color="primary-container" rounded="lg" size="38">
          <v-icon icon="mdi-magnet" color="on-primary-container" size="20" />
        </v-avatar>
        <div class="md-head__text">
          <div class="md-head__title">磁力链接详情</div>
          <div class="md-head__sub">来源 · {{ siteName }}</div>
        </div>
        <v-btn
          icon="mdi-close"
          variant="text"
          size="small"
          title="关闭"
          @click="model = false"
        />
      </div>

      <div class="md-body">
        <!-- 标题 -->
        <div class="section-label">标题</div>
        <p class="md-title selectable">{{ item.title }}</p>

        <!-- 元信息 -->
        <div class="md-meta">
          <v-chip
            v-if="item.size"
            size="x-small"
            variant="text"
            prepend-icon="mdi-database-outline"
          >
            {{ item.size }}
          </v-chip>
          <v-chip
            v-if="item.date"
            size="x-small"
            variant="text"
            prepend-icon="mdi-calendar-outline"
          >
            {{ item.date }}
          </v-chip>
        </div>

        <!-- 磁力地址 -->
        <div class="section-label mt-4">磁力地址</div>
        <v-sheet class="md-magnet" color="surface-container-highest" rounded="lg" border>
          <code class="md-magnet__text mono selectable">{{ magnetText() }}</code>
        </v-sheet>
      </div>

      <!-- 操作 -->
      <div class="md-actions">
        <v-btn
          icon="mdi-share-variant-outline"
          variant="outlined"
          size="small"
          title="分享"
          @click="share"
        />
        <v-btn
          icon="mdi-open-in-new"
          variant="outlined"
          size="small"
          title="用系统打开"
          @click="openMagnet"
        />
        <div class="md-actions__spacer" />
        <v-btn
          color="primary"
          variant="flat"
          rounded="pill"
          class="md-primary"
          :prepend-icon="copied ? 'mdi-check' : 'mdi-content-copy'"
          @click="copy"
        >
          {{ copied ? '已复制' : '复制磁力' }}
        </v-btn>
      </div>
    </v-card>
  </v-dialog>

  <v-snackbar v-model="showToast" location="bottom" color="inverse-surface" rounded="lg" :timeout="2000">
    {{ toast }}
  </v-snackbar>
</template>

<style scoped>
.md-head {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 15px 16px 10px;
}

.md-head__text {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.md-head__title {
  font-size: 0.95rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.md-head__sub {
  margin-top: 2px;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.md-body {
  padding: 4px 16px 8px;
  max-height: 56vh;
  overflow-y: auto;
}

.md-title {
  margin: 4px 0 0;
  font-size: 0.85rem;
  line-height: 1.55;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-word;
}

.md-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  margin-top: 8px;
}

.md-magnet {
  margin-top: 5px;
  padding: 10px 12px;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
}

.md-magnet__text {
  display: block;
  font-size: 0.71rem;
  line-height: 1.6;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-all;
  max-height: 7.2em;
  overflow-y: auto;
}

.md-actions {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 16px 16px;
}

.md-actions__spacer {
  flex: 1 1 auto;
}

.md-primary {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}
</style>
