<script setup lang="ts">
// 资源详情页（重构版）
//
// 布局要点（修复「无法上下滚动 / 卡片显示不全」）：
//   整页只有 **一个** 滚动容器 `.rd-scroll`，顶栏用 position: sticky 吸顶，
//   不再依赖 flex 高度计算与 100dvh 的父子高度传递，因此在 WebView、桌面窗口、
//   以及被 keep-alive 包裹的嵌套场景下都能正常滚动。
//
// 数据要点：
//   - whatslink 成功时 error 字段是 **空字符串**（实测响应 {"error":"",...}）
//   - 只有形如 magnet:?xt=urn:btih: 的才算磁力；站点给的「短磁链」
//     （如 https://0mag.me/!xxxxx）不能当磁力展示，只能作为来源页
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useClipboard } from '@vueuse/core'
import { httpGetText } from '../lib/http'
import { invoke, isTauri } from '../lib/tauri'

const route = useRoute()
const router = useRouter()

const title = ref(String(route.query.t ?? ''))
const size = ref(String(route.query.s ?? ''))
const date = ref(String(route.query.d ?? ''))
const rawMagnet = ref(String(route.query.m ?? ''))
const rawUrl = ref(String(route.query.u ?? ''))
const source = ref(String(route.query.src ?? ''))
const keyword = ref(String(route.query.k ?? ''))

/** 是否是可用的磁力链接（magnet:? 且带 btih） */
function isMagnet(s: string): boolean {
  const t = s.trim().toLowerCase()
  return t.startsWith('magnet:?') && t.includes('xt=urn:btih')
}

/** 真正的磁力链接：只有合法磁力才复用，否则留空 */
const magnet = computed(() => (isMagnet(rawMagnet.value) ? rawMagnet.value.trim() : ''))

/** 来源页面：短链 / 普通地址 */
const pageUrl = computed(() => {
  if (rawUrl.value.trim()) return rawUrl.value.trim()
  // 站点把「短磁链」误填进磁力字段时，这里兜底当作来源页
  const m = rawMagnet.value.trim()
  return m && !isMagnet(m) ? m : ''
})

const hasLink = computed(() => !!magnet.value || !!pageUrl.value)

// ---------- whatslink 解析 ----------
interface WlShot {
  time?: number
  screenshot?: string
}
interface WlInfo {
  error?: number | string | null
  type?: string
  file_type?: string
  name?: string
  size?: number
  count?: number
  screenshots?: WlShot[]
}

const info = ref<WlInfo | null>(null)
const shots = ref<{ url: string; time: number }[]>([])
const loadingInfo = ref(false)
const viewerOpen = ref(false)
const viewerIndex = ref(0)

const WHATSLINK_API = 'https://whatslink.info/api/v1/link'

/** whatslink 成功判定：error 为空 / 缺失 / 0 都算成功 */
function isOkPayload(err: WlInfo['error']): boolean {
  if (err === undefined || err === null) return true
  const s = String(err).trim()
  return s === '' || s === '0'
}

const showShots = computed(() => loadingInfo.value || shots.value.length > 0)

function formatTime(sec?: number): string {
  if (!sec || sec < 0) return ''
  const s = Math.floor(sec)
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const ss = s % 60
  return h > 0
    ? `${h}:${String(m).padStart(2, '0')}:${String(ss).padStart(2, '0')}`
    : `${m}:${String(ss).padStart(2, '0')}`
}

const fileTypeLabel = computed(() => {
  const ft = info.value?.file_type
  const map: Record<string, string> = {
    video: '视频',
    audio: '音频',
    image: '图片',
    document: '文档',
    archive: '压缩包',
    application: '程序',
  }
  return ft ? (map[ft] ?? ft) : ''
})

async function loadInfo() {
  const link = magnet.value || pageUrl.value
  if (!link) return
  loadingInfo.value = true
  shots.value = []
  info.value = null
  try {
    const target = `${WHATSLINK_API}?url=${encodeURIComponent(link)}`
    const text = await httpGetText(target, { timeoutMs: 20000 })
    const data = JSON.parse(text) as WlInfo
    if (data && isOkPayload(data.error)) {
      info.value = data
      const list = Array.isArray(data.screenshots) ? data.screenshots : []
      shots.value = list
        .filter((s) => !!s.screenshot)
        .map((s) => ({ url: String(s.screenshot), time: Number(s.time ?? 0) }))
    }
  } catch {
    // 解析失败：不展示截图区块，也不影响其余信息
  } finally {
    loadingInfo.value = false
  }
}

function openViewer(i: number) {
  viewerIndex.value = i
  viewerOpen.value = true
}

// ---------- 操作 ----------
const toast = ref('')
const toastOpen = ref(false)
const { copy: copyFallback } = useClipboard({ legacy: true })

function flash(msg: string) {
  toast.value = msg
  toastOpen.value = true
}

async function copyText(text: string, label: string) {
  if (!text) return
  let ok = false
  try {
    const mod = await import('@tauri-apps/plugin-clipboard-manager')
    await mod.writeText(text)
    ok = true
  } catch {
    try {
      await copyFallback(text)
      ok = true
    } catch {
      ok = false
    }
  }
  flash(ok ? `${label}已复制` : '复制失败，请长按手动选择复制')
  if (ok && magnet.value) {
    void invoke('add_history', {
      kind: 'copy',
      keyword: keyword.value || title.value,
      magnet: magnet.value,
    }).catch(() => undefined)
  }
}

async function openSource() {
  if (!pageUrl.value) return
  try {
    if (isTauri) {
      const { openUrl } = await import('@tauri-apps/plugin-opener')
      await openUrl(pageUrl.value)
    } else {
      window.open(pageUrl.value, '_blank', 'noopener')
    }
  } catch {
    flash('无法打开来源页面')
  }
}

function back() {
  if (window.history.length > 1) router.back()
  else router.push('/')
}

onMounted(() => {
  // 记录浏览历史：历史页读取的正是 browse 类型
  void invoke('add_history', {
    kind: 'browse',
    keyword: keyword.value || title.value,
    magnet: magnet.value || pageUrl.value,
  }).catch(() => undefined)
  void loadInfo()
})
</script>

<template>
  <div class="rd">
    <!-- 单一滚动容器：顶栏 sticky 吸顶，内容自然上下滚动 -->
    <div class="rd-scroll">
      <header class="rd-bar">
        <v-btn icon="mdi-arrow-left" variant="text" size="small" title="返回" @click="back" />
        <span class="rd-bar__title">资源详情</span>
        <v-spacer />
        <v-chip v-if="source" size="x-small" variant="tonal" color="primary">
          {{ source }}
        </v-chip>
      </header>

      <main class="rd-main">
        <!-- 标题 + 关键操作 -->
        <section class="rd-hero">
          <h1 class="rd-title selectable">{{ title || '未命名资源' }}</h1>

          <div class="rd-badges">
            <v-chip v-if="size" size="small" variant="tonal" prepend-icon="mdi-database-outline">
              {{ size }}
            </v-chip>
            <v-chip v-if="date" size="small" variant="outlined" prepend-icon="mdi-calendar-outline">
              {{ date }}
            </v-chip>
            <v-chip v-if="fileTypeLabel" size="small" color="primary" variant="tonal">
              {{ fileTypeLabel }}
            </v-chip>
          </div>

          <div class="rd-cta">
            <v-btn
              v-if="magnet"
              color="primary"
              variant="flat"
              size="large"
              rounded="lg"
              block
              prepend-icon="mdi-magnet"
              @click="copyText(magnet, '磁力链接')"
            >
              复制磁力链接
            </v-btn>
            <v-btn
              v-if="pageUrl"
              variant="tonal"
              size="large"
              rounded="lg"
              block
              prepend-icon="mdi-open-in-new"
              @click="openSource"
            >
              打开来源页面
            </v-btn>
            <v-alert
              v-if="!hasLink"
              type="info"
              variant="tonal"
              density="compact"
              rounded="lg"
              text="该资源没有提供磁力链接或来源地址"
            />
          </div>
        </section>

        <!-- 磁力链接：等宽字体 + 复制 -->
        <section v-if="magnet" class="rd-block">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-link-variant" size="14" class="mr-1" />磁力链接
            </span>
            <v-btn
              size="x-small"
              variant="text"
              color="primary"
              prepend-icon="mdi-content-copy"
              class="rd-block__copy"
              @click="copyText(magnet, '磁力链接')"
            >
              复制
            </v-btn>
          </div>
          <v-sheet class="rd-code" color="surface-container-low" rounded="lg" border>
            <code class="mono selectable">{{ magnet }}</code>
          </v-sheet>
        </section>

        <!-- 来源页面 -->
        <section v-if="pageUrl" class="rd-block">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-web" size="14" class="mr-1" />来源页面
            </span>
            <v-btn
              size="x-small"
              variant="text"
              color="primary"
              prepend-icon="mdi-content-copy"
              class="rd-block__copy"
              @click="copyText(pageUrl, '来源链接')"
            >
              复制
            </v-btn>
          </div>
          <v-sheet class="rd-code" color="surface-container-low" rounded="lg" border>
            <code class="selectable">{{ pageUrl }}</code>
          </v-sheet>
          <v-btn
            class="rd-inline-link"
            size="small"
            variant="text"
            color="primary"
            append-icon="mdi-arrow-top-right"
            @click="openSource"
          >
            在浏览器中打开
          </v-btn>
        </section>

        <!-- 解析信息（whatslink） -->
        <section v-if="info?.name || info?.size || fileTypeLabel" class="rd-block">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-information-outline" size="14" class="mr-1" />解析信息
            </span>
          </div>
          <div class="rd-rows">
            <div v-if="info?.name" class="rd-row">
              <span class="rd-row__k">文件名</span>
              <span class="rd-row__v selectable">{{ info.name }}</span>
            </div>
            <div v-if="fileTypeLabel" class="rd-row">
              <span class="rd-row__k">类型</span>
              <span class="rd-row__v">{{ fileTypeLabel }}</span>
            </div>
            <div v-if="size" class="rd-row">
              <span class="rd-row__k">大小</span>
              <span class="rd-row__v">{{ size }}</span>
            </div>
            <div v-if="date" class="rd-row">
              <span class="rd-row__k">发布</span>
              <span class="rd-row__v">{{ date }}</span>
            </div>
            <div v-if="source" class="rd-row">
              <span class="rd-row__k">来源</span>
              <span class="rd-row__v">{{ source }}</span>
            </div>
          </div>
        </section>

        <!-- 截图：解析中或确实有截图时才渲染整块 -->
        <section v-if="showShots" class="rd-block">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-image-multiple-outline" size="14" class="mr-1" />资源截图
            </span>
            <span v-if="shots.length" class="rd-block__count">{{ shots.length }} 张</span>
          </div>

          <div v-if="loadingInfo" class="rd-loading">
            <v-progress-circular indeterminate size="18" width="2" color="primary" />
            <span>正在通过 whatslink 解析截图…</span>
          </div>

          <div v-else class="rd-shots">
            <v-btn
              v-for="(s, i) in shots"
              :key="i"
              class="rd-shot"
              variant="text"
              :ripple="false"
              @click="openViewer(i)"
            >
              <img :src="s.url" :alt="`截图 ${i + 1}`" loading="lazy" class="rd-shot__img" />
              <span v-if="s.time" class="rd-shot__time">{{ formatTime(s.time) }}</span>
            </v-btn>
          </div>

          <p class="rd-note">
            截图由
            <a href="https://whatslink.info/" target="_blank" rel="noopener noreferrer">
              whatslink.info
            </a>
            解析提供，仅供参考
          </p>
        </section>
      </main>
    </div>

    <!-- 截图大图查看 -->
    <v-dialog v-model="viewerOpen" max-width="920" scrollable>
      <v-card class="rd-viewer" color="surface" rounded="lg">
        <div class="rd-viewer__bar">
          <span>截图 {{ viewerIndex + 1 }} / {{ shots.length }}</span>
          <v-btn
            icon="mdi-close"
            variant="text"
            size="small"
            title="关闭"
            @click="viewerOpen = false"
          />
        </div>
        <img
          v-if="shots[viewerIndex]"
          :src="shots[viewerIndex].url"
          :alt="`截图 ${viewerIndex + 1}`"
          class="rd-viewer__img"
        />
        <div class="rd-viewer__nav">
          <v-btn
            icon="mdi-chevron-left"
            variant="tonal"
            size="small"
            :disabled="viewerIndex === 0"
            title="上一张"
            @click="viewerIndex -= 1"
          />
          <span v-if="shots[viewerIndex]?.time" class="rd-viewer__time">
            {{ formatTime(shots[viewerIndex].time) }}
          </span>
          <v-btn
            icon="mdi-chevron-right"
            variant="tonal"
            size="small"
            :disabled="viewerIndex >= shots.length - 1"
            title="下一张"
            @click="viewerIndex += 1"
          />
        </div>
      </v-card>
    </v-dialog>

    <v-snackbar v-model="toastOpen" :timeout="1800" location="bottom" rounded="lg">
      {{ toast }}
    </v-snackbar>
  </div>
</template>

<style scoped>
.rd {
  height: 100%;
  height: 100dvh;
  background: rgb(var(--v-theme-surface));
  overflow: hidden;
}

/* 唯一滚动容器 */
.rd-scroll {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-y: contain;
}

/* ---------- 吸顶栏 ---------- */
.rd-bar {
  position: sticky;
  top: 0;
  z-index: 6;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  padding-top: calc(8px + var(--safe-area-inset-top, env(safe-area-inset-top, 0px)));
  padding-left: calc(12px + var(--safe-area-inset-left, env(safe-area-inset-left, 0px)));
  padding-right: calc(12px + var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.rd-bar__title {
  font-size: 0.94rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
}

/* ---------- 内容区 ---------- */
.rd-main {
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 14px 14px
    calc(28px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* ---------- 标题区 ---------- */
.rd-hero {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.rd-title {
  margin: 0;
  font-size: 1.06rem;
  font-weight: 700;
  line-height: 1.45;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-word;
}

.rd-badges {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.rd-cta {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 2px;
}

/* ---------- 区块 ---------- */
.rd-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 13px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 14px;
  background: rgb(var(--v-theme-surface));
}

.rd-block__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.rd-block__copy {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

.rd-block__count {
  font-size: 0.7rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface-variant));
}

/* 代码块：可换行、可选中 */
.rd-code {
  padding: 10px 12px;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
}

.rd-code code {
  display: block;
  font-size: 0.72rem;
  line-height: 1.65;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-all;
}

.rd-inline-link {
  align-self: flex-start;
  text-transform: none;
  letter-spacing: 0;
  padding-left: 0;
}

/* ---------- 信息行 ---------- */
.rd-rows {
  display: flex;
  flex-direction: column;
}

.rd-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 6px 0;
  font-size: 0.76rem;
}

.rd-row + .rd-row {
  border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.rd-row__k {
  flex: 0 0 62px;
  color: rgb(var(--v-theme-on-surface-variant));
  font-weight: 600;
}

.rd-row__v {
  flex: 1 1 auto;
  min-width: 0;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-word;
}

/* ---------- 截图 ---------- */
.rd-loading {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 2px;
  font-size: 0.76rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.rd-shots {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
  gap: 8px;
}

.rd-shot {
  position: relative;
  width: 100%;
  height: 80px;
  min-width: 0;
  padding: 0;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 10px;
  overflow: hidden;
  background: rgb(var(--v-theme-surface-container));
}

.rd-shot :deep(.v-btn__overlay) {
  display: none;
}

.rd-shot__img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.rd-shot__time {
  position: absolute;
  right: 5px;
  bottom: 5px;
  padding: 1px 5px;
  border-radius: 5px;
  background: rgba(0, 0, 0, 0.66);
  color: #fff;
  font-size: 0.63rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.rd-note {
  margin: 0;
  font-size: 0.66rem;
  line-height: 1.55;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.9;
}

.rd-note a {
  color: rgb(var(--v-theme-primary));
  text-decoration: none;
}

/* ---------- 大图查看 ---------- */
.rd-viewer {
  overflow: hidden;
}

.rd-viewer__bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 8px 8px 14px;
  font-size: 0.8rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.rd-viewer__img {
  display: block;
  width: 100%;
  max-height: 66vh;
  object-fit: contain;
  background: #000;
}

.rd-viewer__nav {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 14px;
  padding: 10px;
}

.rd-viewer__time {
  font-size: 0.78rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: rgb(var(--v-theme-on-surface-variant));
}
</style>
