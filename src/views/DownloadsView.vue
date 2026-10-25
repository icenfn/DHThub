<script setup lang="ts">
// 下载页：对接 Rust 侧内嵌的 BitTorrent 内核（librqbit / rqbit）
// 交互：粘贴磁力链接添加任务；任务卡展示进度 / 速度 / 状态，可暂停、继续、删除。
// 数据：命令返回 librqbit 的 TorrentStats 原样 JSON，这里只做展示层取值。
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke, isTauri } from '../lib/tauri'
import { settings } from '../stores/settings'

const emit = defineEmits<{ (e: 'go-sites'): void }>()

/** librqbit TorrentStats 的前端最小视图 */
interface RawStats {
  state?: string
  paused?: boolean
  error?: string | null
  progress_bytes?: number
  total_bytes?: number
  uploaded_bytes?: number
  finished?: boolean
  live?: {
    download_speed?: { mbps?: number }
    upload_speed?: { mbps?: number }
  } | null
}

interface DownloadRow {
  id: number
  name: string
  paused: boolean
  stats: RawStats
}

const rows = ref<DownloadRow[]>([])
const magnet = ref('')
const adding = ref(false)
const loading = ref(false)
const message = ref('')
const busyIds = ref<Set<number>>(new Set())

let timer: number | undefined

const toast = ref('')
const showToast = ref(false)
function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

/** 字节数格式化（B / KB / MB / GB） */
function fmtBytes(n?: number): string {
  const v = n ?? 0
  if (v <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  let x = v
  while (x >= 1024 && i < units.length - 1) {
    x /= 1024
    i += 1
  }
  return `${x.toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}

function fmtSpeed(mbps?: number): string {
  const v = mbps ?? 0
  if (v <= 0) return '—'
  if (v < 1) return `${(v * 1024).toFixed(0)} KB/s`
  return `${v.toFixed(2)} MB/s`
}

/** 进度百分比（0-100），用已校验字节 / 总字节 */
function percent(st: RawStats): number {
  const total = st.total_bytes ?? 0
  const got = st.progress_bytes ?? 0
  if (total <= 0) return 0
  return Math.min(100, Math.round((got / total) * 1000) / 10)
}

/** 状态文案与色板 */
function stateOf(row: DownloadRow): { text: string; color: string } {
  const st = row.stats
  if (st.error) return { text: '出错', color: 'error' }
  if (st.finished) return { text: '已完成', color: 'success' }
  if (st.state === 'initializing') return { text: '准备中', color: 'info' }
  if (row.paused || st.state === 'paused') return { text: '已暂停', color: 'warning' }
  if (st.state === 'live') return { text: '下载中', color: 'primary' }
  return { text: '等待中', color: 'info' }
}

const activeCount = computed(() => rows.value.filter((r) => !r.stats.finished && !r.paused).length)
const doneCount = computed(() => rows.value.filter((r) => r.stats.finished).length)

async function refresh(quiet = true) {
  if (!isTauri) return
  if (!quiet) loading.value = true
  try {
    rows.value = await invoke<DownloadRow[]>('list_downloads')
    message.value = ''
  } catch (e) {
    message.value = String(e)
  } finally {
    if (!quiet) loading.value = false
  }
}

async function addTask() {
  const m = magnet.value.trim()
  if (!m) {
    notice('请先粘贴磁力链接')
    return
  }
  if (!m.startsWith('magnet:')) {
    notice('目前仅支持 magnet: 磁力链接')
    return
  }
  adding.value = true
  try {
    await invoke('add_download', { magnet: m })
    magnet.value = ''
    notice('已添加下载任务')
    await refresh()
  } catch (e) {
    notice(String(e))
  } finally {
    adding.value = false
  }
}

async function markBusy(id: number, on: boolean) {
  const next = new Set(busyIds.value)
  if (on) next.add(id)
  else next.delete(id)
  busyIds.value = next
}

async function toggle(row: DownloadRow) {
  await markBusy(row.id, true)
  try {
    if (row.paused || row.stats.state === 'paused') {
      await invoke('resume_download', { id: row.id })
      notice('已继续下载')
    } else {
      await invoke('pause_download', { id: row.id })
      notice('已暂停下载')
    }
    await refresh()
  } catch (e) {
    notice(String(e))
  } finally {
    await markBusy(row.id, false)
  }
}

async function remove(row: DownloadRow) {
  await markBusy(row.id, true)
  try {
    await invoke('remove_download', { id: row.id, deleteFiles: false })
    notice('已移除任务（保留已下载文件）')
    await refresh()
  } catch (e) {
    notice(String(e))
  } finally {
    await markBusy(row.id, false)
  }
}

/** 把设置页的下载内核配置下发给内核（未创建会话时生效于首次初始化） */
async function pushConfig() {
  if (!isTauri) return
  try {
    const c = settings.getDownloadConfig()
    await invoke('configure_downloads', {
      cfg: {
        dht_enabled: c.dhtEnabled,
        dht_port: c.dhtPort,
        download_limit_kbps: c.downloadLimitKbps,
        upload_limit_kbps: c.uploadLimitKbps,
        peer_limit: c.peerLimit,
        concurrent_init_limit: c.concurrentInitLimit,
        disable_trackers: c.disableTrackers,
        ipv4_only: c.ipv4Only,
        extra_trackers: c.extraTrackers,
      },
    })
  } catch {
    // 配置下发失败不阻塞页面（添加任务时内核会用当前配置初始化并报错）
  }
}

onMounted(async () => {
  await settings.ready()
  await pushConfig()
  await refresh(false)
  if (!isTauri) return
  // 轮询刷新进度：下载速度/进度都在内核侧计算，这里只做拉取
  timer = window.setInterval(() => {
    void refresh()
  }, 1200)
})

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner">
      <!-- 添加任务 -->
      <section class="add">
        <div class="add__field">
          <v-icon icon="mdi-magnet" size="18" class="add__icon" />
          <input
            v-model="magnet"
            class="add__input"
            type="text"
            inputmode="text"
            autocomplete="off"
            spellcheck="false"
            placeholder="粘贴 magnet: 磁力链接"
            @keyup.enter="addTask"
          />
        </div>
        <v-btn
          class="add__btn"
          color="primary"
          variant="flat"
          rounded="pill"
          size="small"
          :loading="adding"
          @click="addTask"
        >
          添加
        </v-btn>
      </section>

      <div v-if="rows.length" class="summary">
        <span>共 {{ rows.length }} 个任务</span>
        <span class="dot">·</span>
        <span>下载中 {{ activeCount }}</span>
        <span class="dot">·</span>
        <span>已完成 {{ doneCount }}</span>
      </div>

      <v-alert v-if="message" type="error" variant="tonal" density="compact" class="mb-3">
        {{ message }}
      </v-alert>

      <!-- 任务列表 -->
      <div v-if="rows.length" class="list">
        <v-card
          v-for="row in rows"
          :key="row.id"
          class="task"
          variant="outlined"
          rounded="lg"
        >
          <div class="task__head">
            <div class="task__title" :title="row.name">{{ row.name }}</div>
            <v-chip
              size="x-small"
              variant="tonal"
              :color="stateOf(row).color"
              class="task__state"
            >
              {{ stateOf(row).text }}
            </v-chip>
          </div>

          <v-progress-linear
            :model-value="percent(row.stats)"
            :color="stateOf(row).color"
            height="6"
            rounded
            class="task__bar"
          />

          <div class="task__meta">
            <span class="task__pct">{{ percent(row.stats) }}%</span>
            <span class="task__size">
              {{ fmtBytes(row.stats.progress_bytes) }} / {{ fmtBytes(row.stats.total_bytes) }}
            </span>
            <span class="task__spacer" />
            <span class="task__speed">
              <v-icon icon="mdi-arrow-down" size="13" />
              {{ fmtSpeed(row.stats.live?.download_speed?.mbps) }}
            </span>
            <span class="task__speed">
              <v-icon icon="mdi-arrow-up" size="13" />
              {{ fmtSpeed(row.stats.live?.upload_speed?.mbps) }}
            </span>
          </div>

          <div v-if="row.stats.error" class="task__error">{{ row.stats.error }}</div>

          <div class="task__actions">
            <v-btn
              v-if="!row.stats.finished"
              variant="tonal"
              size="x-small"
              rounded="pill"
              class="pillbtn"
              :loading="busyIds.has(row.id)"
              :prepend-icon="row.paused ? 'mdi-play' : 'mdi-pause'"
              @click="toggle(row)"
            >
              {{ row.paused ? '继续' : '暂停' }}
            </v-btn>
            <v-btn
              variant="text"
              color="error"
              size="x-small"
              rounded="pill"
              class="pillbtn"
              :disabled="busyIds.has(row.id)"
              prepend-icon="mdi-delete-outline"
              @click="remove(row)"
            >
              移除
            </v-btn>
          </div>
        </v-card>
      </div>

      <!-- 空态 -->
      <v-empty-state
        v-else-if="!loading"
        icon="mdi-download-outline"
        title="暂无下载任务"
        text="在上方粘贴磁力链接即可开始下载。文件保存在应用数据目录的 downloads 下。"
        class="empty"
      >
        <template #actions>
          <v-btn
            variant="tonal"
            rounded="pill"
            size="small"
            prepend-icon="mdi-antenna"
            @click="emit('go-sites')"
          >
            管理搜索源
          </v-btn>
        </template>
      </v-empty-state>
    </div>

    <v-snackbar v-model="showToast" :timeout="2200" location="bottom">
      {{ toast }}
    </v-snackbar>
  </div>
</template>

<style scoped>
/* 滚动容器与内容宽度沿用 src/styles/main.scss 的 .page-scroll / .page-inner */

/* ---------- 添加任务 ---------- */
.add {
  display: flex;
  align-items: center;
  gap: 8px;
}

.add__field {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
  height: 42px;
  padding: 0 12px;
  border-radius: 999px;
  background: rgb(var(--v-theme-surface-container-high));
}

.add__icon {
  flex-shrink: 0;
  opacity: 0.6;
}

.add__input {
  flex: 1 1 auto;
  min-width: 0;
  border: 0;
  outline: none;
  background: transparent;
  font-size: 0.82rem;
  color: rgb(var(--v-theme-on-surface));
}

.add__input::placeholder {
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.8;
}

.add__btn {
  flex-shrink: 0;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

/* ---------- 汇总 ---------- */
.summary {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 12px 2px 8px;
  font-size: 0.7rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.dot {
  opacity: 0.5;
}

/* ---------- 任务卡 ---------- */
.list {
  display: flex;
  flex-direction: column;
  gap: 9px;
}

.task {
  padding: 11px 12px 9px;
}

.task__head {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.task__title {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.83rem;
  font-weight: 600;
  line-height: 1.35;
  color: rgb(var(--v-theme-on-surface));
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.task__state {
  flex-shrink: 0;
}

.task__bar {
  margin: 9px 0 7px;
}

.task__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.task__pct {
  font-weight: 700;
  color: rgb(var(--v-theme-primary));
}

.task__spacer {
  flex: 1 1 auto;
}

.task__speed {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  white-space: nowrap;
}

.task__error {
  margin-top: 7px;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-error));
  word-break: break-all;
}

.task__actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  margin-top: 8px;
}

.pillbtn {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

.empty {
  margin-top: 14vh;
}
</style>
