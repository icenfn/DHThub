<script setup lang="ts">
// 资源详情页
//
// 布局要点（保证可上下滚动、卡片不被裁切）：
//   整页只有 **一个** 滚动容器 `.rd-scroll`，顶栏 position: sticky 吸顶。
//
// 数据要点：
//   - whatslink 成功时 error 字段是 **空字符串**（实测 {"error":"",...}）
//   - 只有形如 magnet:?xt=urn:btih: 的才算磁力；站点给的「短磁链」
//     （如 https://0mag.me/!xxxxx）只作为来源页，不当作磁力展示
//   - 来源地址由后端补全为绝对地址（磁力狗的 /information/xxx →
//     http://clg38.sbs/information/xxx），因此可以直接外部打开
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useClipboard, useEventListener } from '@vueuse/core'
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

/** 真正的磁力链接：只有合法磁力才采用 */
const magnet = computed(() => (isMagnet(rawMagnet.value) ? rawMagnet.value.trim() : ''))

/** 来源页面：短链 / 普通地址（后端已补全为绝对地址） */
const pageUrl = computed(() => {
  if (rawUrl.value.trim()) return rawUrl.value.trim()
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
const shotsRef = ref<HTMLElement | null>(null)

const WHATSLINK_API = 'https://whatslink.info/api/v1/link'

/** whatslink 成功判定：error 为空 / 缺失 / 0 都算成功 */
function isOkPayload(err: WlInfo['error']): boolean {
  if (err === undefined || err === null) return true
  const s = String(err).trim()
  return s === '' || s === '0'
}

const showShots = computed(() => loadingInfo.value || shots.value.length > 0)

/** whatslink 返回的文件大小（字节 → 可读） */
const parsedSize = computed(() => {
  const n = Number(info.value?.size ?? 0)
  if (!n || n <= 0) return ''
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = n
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i += 1
  }
  return `${v >= 100 ? Math.round(v) : v.toFixed(1)} ${units[i]}`
})

/** 有任一解析字段时才展示「解析信息」卡片 */
const hasParsedInfo = computed(
  () =>
    !!info.value?.name ||
    !!parsedSize.value ||
    !!info.value?.count ||
    shots.value.length > 0 ||
    !!size.value ||
    !!date.value ||
    !!source.value,
)

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
    // 解析失败：不展示截图区块，不影响其余信息
  } finally {
    loadingInfo.value = false
  }
}

// ---------- 截图预览（v-viewer） ----------
// 截图改成横向单行列表；点击任意缩略图由 v-viewer 接管为大图预览，
// 支持左右切换、缩放、拖动。viewer 实例挂在容器上的 __viewer 属性。
interface ViewerLike {
  view: (index?: number) => void
  update: () => void
  destroy: () => void
}
type ViewerHost = HTMLElement & { $viewer?: ViewerLike; __viewer?: ViewerLike }

function getHostViewer(): ViewerLike | undefined {
  const el = shotsRef.value as ViewerHost | null
  return el?.$viewer ?? el?.__viewer
}

function openShot(i: number) {
  const v = getHostViewer()
  if (v) {
    v.view(i)
    return
  }
  // 兜底：viewer 尚未就绪时新窗口打开
  const s = shots.value[i]
  if (s) window.open(s.url, '_blank', 'noopener')
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

// ---------- 分享（@vnidrop/tauri-plugin-share） ----------
// 分享内容：标题 + 磁力链接（或来源页地址）+ 大小/日期摘要
const sharing = ref(false)

function shareText(): string {
  const lines: string[] = []
  const t = (title.value || '未命名资源').trim()
  lines.push(t)
  if (size.value) lines.push(`大小：${size.value}`)
  if (date.value) lines.push(`发布：${date.value}`)
  if (magnet.value) lines.push(magnet.value)
  else if (pageUrl.value) lines.push(pageUrl.value)
  return lines.join('\n')
}

async function doShare() {
  if (sharing.value) return
  const text = shareText()
  if (!text.trim()) return
  sharing.value = true
  try {
    // 磁力链接不是 http(s) URL，只能作为文本分享
    const payload = { title: title.value || 'DHThub 资源', text }
    const mod = await import('@vnidrop/tauri-plugin-share')
    if (await mod.canShare(payload)) {
      await mod.share(payload)
      return
    }
    // 不支持系统分享面板：退回 Web Share API / 剪贴板
    const nav = navigator as Navigator & {
      share?: (d: { title?: string; text?: string }) => Promise<void>
    }
    if (nav.share) {
      await nav.share({ title: payload.title, text })
      return
    }
    await copyText(text, '资源信息')
  } catch (e) {
    // 用户取消分享不提示
    const msg = String(e ?? '')
    if (!/cancel|abort|dismiss/i.test(msg)) flash('分享失败，请稍后重试')
  } finally {
    sharing.value = false
  }
}

function back() {
  if (window.history.length > 1) router.back()
  else router.push('/')
}

// 桌面端 ESC 返回，与移动端返回手势一致
useEventListener(window, 'keydown', (e: KeyboardEvent) => {
  if (e.key === 'Escape') back()
})

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
        <v-btn
          icon="mdi-share-variant-outline"
          variant="text"
          size="small"
          title="分享"
          aria-label="分享该资源"
          :loading="sharing"
          @click="doShare"
        />
        <v-btn
          v-if="pageUrl"
          icon="mdi-open-in-new"
          variant="text"
          size="small"
          title="在外部浏览器打开来源页面"
          aria-label="在外部浏览器打开来源页面"
          @click="openSource"
        />
      </header>

      <main class="rd-main">
        <!-- 标题 + 元信息 + 主操作 -->
        <section class="rd-hero">
          <h1 class="rd-title selectable">{{ title || '未命名资源' }}</h1>

          <div class="rd-badges">
            <v-chip
              v-if="size"
              size="small"
              variant="tonal"
              prepend-icon="mdi-database-outline"
            >
              {{ size }}
            </v-chip>
            <v-chip
              v-if="date"
              size="small"
              variant="outlined"
              prepend-icon="mdi-calendar-outline"
            >
              {{ date }}
            </v-chip>
            <v-chip v-if="fileTypeLabel" size="small" color="primary" variant="tonal">
              {{ fileTypeLabel }}
            </v-chip>
            <v-chip v-if="source" size="small" variant="text" prepend-icon="mdi-antenna">
              {{ source }}
            </v-chip>
          </div>

          <v-btn
            v-if="magnet"
            color="primary"
            variant="flat"
            size="large"
            rounded="lg"
            block
            prepend-icon="mdi-magnet"
            class="rd-copy"
            @click="copyText(magnet, '磁力链接')"
          >
            复制磁力链接
          </v-btn>

          <v-alert
            v-if="!hasLink"
            type="info"
            variant="tonal"
            density="compact"
            rounded="lg"
            text="该资源没有提供磁力链接或来源地址"
          />
        </section>

        <!-- 截图：放在磁力链接卡片上方；横向单行列表 + v-viewer 大图预览 -->
        <section v-if="showShots" class="rd-block">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-image-multiple-outline" size="15" class="mr-1" />资源截图
            </span>
            <span v-if="shots.length" class="rd-block__count">共 {{ shots.length }} 张</span>
          </div>

          <div v-if="loadingInfo" class="rd-loading">
            <v-progress-circular indeterminate size="18" width="2" color="primary" />
            <span>正在通过 whatslink 解析截图…</span>
          </div>

          <!-- v-viewer 指令接管：点击缩略图打开可缩放的全屏预览 -->
          <div v-else ref="shotsRef" v-viewer class="rd-shots">
            <button
              v-for="(s, i) in shots"
              :key="i"
              type="button"
              class="rd-shot"
              :aria-label="`查看第 ${i + 1} 张截图`"
              @click="openShot(i)"
            >
              <img :src="s.url" :alt="`截图 ${i + 1}`" loading="lazy" class="rd-shot__img" />
              <span v-if="s.time" class="rd-shot__time">{{ formatTime(s.time) }}</span>
            </button>
          </div>

        </section>

        <!-- 磁力链接 -->
        <section v-if="magnet" class="rd-block">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-link-variant" size="15" class="mr-1" />磁力链接
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

        <!-- 解析信息：放在页面最底部，展示 whatslink 返回的结构化字段 -->
        <section v-if="hasParsedInfo" class="rd-block rd-info">
          <div class="rd-block__head">
            <span class="section-label">
              <v-icon icon="mdi-information-outline" size="15" class="mr-1" />解析信息
            </span>
            <v-chip v-if="fileTypeLabel" size="x-small" variant="tonal" color="primary">
              {{ fileTypeLabel }}
            </v-chip>
          </div>

          <dl class="rd-kv">
            <div v-if="info?.name" class="rd-kv__row">
              <dt>文件名</dt>
              <dd class="selectable">{{ info.name }}</dd>
            </div>
            <div v-if="parsedSize" class="rd-kv__row">
              <dt>文件大小</dt>
              <dd>{{ parsedSize }}</dd>
            </div>
            <div v-if="info?.count" class="rd-kv__row">
              <dt>文件数量</dt>
              <dd>{{ info.count }}</dd>
            </div>
            <div v-if="shots.length" class="rd-kv__row">
              <dt>截图数量</dt>
              <dd>{{ shots.length }}</dd>
            </div>
            <div v-if="size" class="rd-kv__row">
              <dt>列表标注</dt>
              <dd>{{ size }}</dd>
            </div>
            <div v-if="date" class="rd-kv__row">
              <dt>发布时间</dt>
              <dd>{{ date }}</dd>
            </div>
            <div v-if="source" class="rd-kv__row">
              <dt>来源站点</dt>
              <dd>{{ source }}</dd>
            </div>
          </dl>
        </section>
      </main>
    </div>

    <v-snackbar v-model="toastOpen" :timeout="1800" location="bottom" rounded="lg">
      {{ toast }}
    </v-snackbar>
  </div>
</template>

<style scoped>
.rd {
  /* 旧内核无 dvh，统一用 --app-height 兜底，避免高度退化为 auto 后无法滚动 */
  height: 100%;
  height: var(--app-height, 100%);
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
  padding: 9px 14px;
  padding-top: calc(9px + var(--safe-area-inset-top, env(safe-area-inset-top, 0px)));
  padding-left: calc(14px + var(--safe-area-inset-left, env(safe-area-inset-left, 0px)));
  padding-right: calc(14px + var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.rd-bar__title {
  font-size: 0.95rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
}

/* ---------- 内容区 ---------- */
.rd-main {
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 16px 16px
    calc(32px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ---------- 标题区 ---------- */
.rd-hero {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.rd-title {
  margin: 0;
  font-size: 1.08rem;
  font-weight: 700;
  line-height: 1.5;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-word;
}

.rd-badges {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.rd-copy {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

/* ---------- 区块 ---------- */
.rd-block {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 15px 16px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 16px;
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
  font-size: 0.72rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface-variant));
}

/* 代码块：可换行、可选中 */
.rd-code {
  padding: 12px 14px;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
}

.rd-code code {
  display: block;
  font-size: 0.74rem;
  line-height: 1.7;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-all;
}

/* ---------- 截图：横向单行，可左右滚动 ---------- */
.rd-loading {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 2px;
  font-size: 0.78rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.rd-shots {
  display: flex;
  flex-wrap: nowrap;
  gap: 10px;
  overflow-x: auto;
  overflow-y: hidden;
  padding-bottom: 6px;
  scrollbar-width: thin;
  -webkit-overflow-scrolling: touch;
}

.rd-shot {
  position: relative;
  flex: 0 0 auto;
  width: 148px;
  height: 84px;
  padding: 0;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 12px;
  overflow: hidden;
  background: rgb(var(--v-theme-surface-container));
  cursor: zoom-in;
}

.rd-shot__img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.rd-shot__time {
  position: absolute;
  right: 6px;
  bottom: 6px;
  padding: 1px 6px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.68);
  color: #fff;
  font-size: 0.64rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

/* ---------- 解析信息 ---------- */
.rd-kv {
  display: flex;
  flex-direction: column;
  gap: 0;
  margin: 0;
}

.rd-kv__row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 9px 0;
  border-bottom: 1px solid rgba(var(--v-border-color), calc(var(--v-border-opacity) * 0.6));
}

.rd-kv__row:last-child {
  border-bottom: 0;
  padding-bottom: 0;
}

.rd-kv dt {
  flex: 0 0 auto;
  min-width: 68px;
  font-size: 0.74rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface-variant));
}

.rd-kv dd {
  flex: 1 1 auto;
  min-width: 0;
  margin: 0;
  font-size: 0.78rem;
  line-height: 1.6;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-word;
}
</style>
