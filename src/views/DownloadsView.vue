<script setup lang="ts">
// 下载页：右下角悬浮按钮新增任务 → 先解析磁力列出文件 → 用户勾选后开始下载。
// 数据来自 Rust 侧内嵌的 librqbit；进度由 1.2s 轮询拉取。
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke, isTauri } from '../lib/tauri'
import { settings } from '../stores/settings'
import { toast } from '../lib/toast'

const emit = defineEmits<{ (e: 'go-sites'): void }>()

interface RawStats {
  state?: string
  paused?: boolean
  error?: string | null
  progress_bytes?: number
  total_bytes?: number
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

interface TorrentFile {
  id: number
  name: string
  length: number
}

interface ProbeResult {
  info_hash: string
  name: string
  total_length: number
  files: TorrentFile[]
}

const rows = ref<DownloadRow[]>([])
const loading = ref(false)
const message = ref('')
const busyIds = ref<Set<number>>(new Set())

// 新增任务流程
const addDialog = ref(false)
const magnet = ref('')
const probing = ref(false)
const probingInfo = ref('')
const probe = ref<ProbeResult | null>(null)
const selected = ref<Set<number>>(new Set())
const adding = ref(false)

let timer: number | undefined

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

function percent(st: RawStats): number {
  const total = st.total_bytes ?? 0
  const got = st.progress_bytes ?? 0
  if (total <= 0) return 0
  return Math.min(100, Math.round((got / total) * 1000) / 10)
}

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
const selectedBytes = computed(() =>
  probe.value
    ? probe.value.files
        .filter((f) => selected.value.has(f.id))
        .reduce((sum, f) => sum + f.length, 0)
    : 0,
)
const allSelected = computed(
  () => !!probe.value && probe.value.files.length > 0 && selected.value.size === probe.value.files.length,
)

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

function resetAdd() {
  magnet.value = ''
  probe.value = null
  selected.value = new Set()
  probingInfo.value = ''
}

function openAdd() {
  resetAdd()
  addDialog.value = true
}

/** 第一步：解析磁力，列出文件供勾选 */
async function doProbe() {
  const m = magnet.value.trim()
  if (!m) {
    toast.warning('请先粘贴磁力链接')
    return
  }
  if (!m.startsWith('magnet:')) {
    toast.warning('目前仅支持 magnet: 磁力链接')
    return
  }
  probing.value = true
  probingInfo.value = '正在解析种子…'
  try {
    const r = await invoke<ProbeResult>('probe_download', { magnet: m })
    probe.value = r
    // 默认全选，用户可取消不需要的文件
    selected.value = new Set(r.files.map((f) => f.id))
    probingInfo.value = ''
    if (r.files.length === 0) toast.info('该种子未包含文件信息')
  } catch (e) {
    probingInfo.value = ''
    toast.error(String(e))
  } finally {
    probing.value = false
  }
}

function toggleFile(id: number) {
  const next = new Set(selected.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selected.value = next
}

function toggleAll() {
  if (!probe.value) return
  selected.value = allSelected.value ? new Set() : new Set(probe.value.files.map((f) => f.id))
}

/** 第二步：按勾选结果正式添加任务 */
async function confirmAdd() {
  if (!probe.value) return
  if (selected.value.size === 0) {
    toast.warning('请至少选择一个文件')
    return
  }
  adding.value = true
  try {
    await invoke('add_download', {
      magnet: magnet.value.trim(),
      onlyFiles: [...selected.value].sort((a, b) => a - b),
    })
    toast.success('已开始下载')
    addDialog.value = false
    resetAdd()
    await refresh()
  } catch (e) {
    toast.error(String(e))
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
      toast.success('已继续下载')
    } else {
      await invoke('pause_download', { id: row.id })
      toast.info('已暂停下载')
    }
    await refresh()
  } catch (e) {
    toast.error(String(e))
  } finally {
    await markBusy(row.id, false)
  }
}

async function remove(row: DownloadRow) {
  await markBusy(row.id, true)
  try {
    await invoke('remove_download', { id: row.id, deleteFiles: false })
    toast.info('已移除任务（保留已下载文件）')
    await refresh()
  } catch (e) {
    toast.error(String(e))
  } finally {
    await markBusy(row.id, false)
  }
}

async function pushConfig() {
  if (!isTauri) return
  try {
    const c = settings.getDownloadConfig()
    await invoke('configure_downloads', {
      cfg: {
        dht_enabled: c.dhtEnabled,
        upnp_enabled: c.upnpEnabled,
        utp_enabled: c.utpEnabled,
        download_limit_kbps: c.downloadLimitKbps,
        upload_limit_kbps: c.uploadLimitKbps,
        extra_trackers: c.extraTrackers,
      },
    })
  } catch {
    // 配置下发失败不阻塞页面
  }
}

onMounted(async () => {
  await settings.ready()
  await pushConfig()
  await refresh(false)
  if (!isTauri) return
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
    <div class="page-inner dlwrap">
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

      <div v-if="rows.length" class="list">
        <v-card v-for="row in rows" :key="row.id" class="task" variant="outlined" rounded="lg">
          <div class="task__head">
            <div class="task__title" :title="row.name">{{ row.name }}</div>
            <v-chip size="x-small" variant="tonal" :color="stateOf(row).color" class="task__state">
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

      <v-empty-state
        v-else-if="!loading"
        icon="mdi-download-outline"
        title="暂无下载任务"
        text="点击右下角按钮粘贴磁力链接即可添加，下载前可勾选要下载的文件。"
        class="empty"
      >
        <template #actions>
          <v-btn variant="tonal" rounded="pill" size="small" prepend-icon="mdi-antenna" @click="emit('go-sites')">
            管理搜索源
          </v-btn>
        </template>
      </v-empty-state>
    </div>

    <!-- 悬浮新增按钮 -->
    <button class="fab" type="button" aria-label="新增下载任务" @click="openAdd">
      <v-icon icon="mdi-plus" size="26" />
    </button>

    <!-- 新增任务弹窗：先解析，再勾选文件 -->
    <v-dialog v-model="addDialog" max-width="520" scrollable>
      <v-card rounded="lg" class="dlg">
        <div class="dlg__head">
          <span class="dlg__title">{{ probe ? '选择要下载的文件' : '新增下载任务' }}</span>
          <v-btn
            v-if="probe"
            variant="text"
            size="x-small"
            rounded="pill"
            class="pillbtn"
            prepend-icon="mdi-arrow-left"
            @click="probe = null"
          >
            重填
          </v-btn>
        </div>

        <v-divider />

        <div class="dlg__body">
          <!-- 步骤一：输入磁力 -->
          <template v-if="!probe">
            <v-textarea
              v-model="magnet"
              variant="outlined"
              density="compact"
              rows="4"
              no-resize
              hide-details
              auto-focus
              class="mono-input"
              placeholder="magnet:?xt=urn:btih:..."
            />
            <p class="hint">{{ probingInfo || '粘贴磁力后先解析种子内容，确认文件再开始下载。' }}</p>
          </template>

          <!-- 步骤二：勾选文件 -->
          <template v-else>
            <div class="pinfo">
              <span class="pinfo__name">{{ probe.name || '未命名种子' }}</span>
              <span class="pinfo__meta">
                {{ probe.files.length }} 个文件 · 共 {{ fmtBytes(probe.total_length) }}
              </span>
            </div>

            <div class="pall">
              <v-checkbox
                :model-value="allSelected"
                density="compact"
                hide-details
                color="primary"
                label="全选"
                class="pall__box"
                @update:model-value="toggleAll"
              />
              <span class="pall__sum">已选 {{ fmtBytes(selectedBytes) }}</span>
            </div>

            <div class="files">
              <label v-for="f in probe.files" :key="f.id" class="file">
                <v-checkbox
                  :model-value="selected.has(f.id)"
                  density="compact"
                  hide-details
                  color="primary"
                  class="file__box"
                  @update:model-value="toggleFile(f.id)"
                />
                <span class="file__name" :title="f.name">{{ f.name }}</span>
                <span class="file__size">{{ fmtBytes(f.length) }}</span>
              </label>
            </div>
          </template>
        </div>

        <div class="dlg__actions">
          <v-btn variant="text" size="small" rounded="pill" class="pillbtn" @click="addDialog = false">
            取消
          </v-btn>
          <v-btn
            v-if="!probe"
            variant="flat"
            color="primary"
            size="small"
            rounded="pill"
            class="pillbtn"
            :loading="probing"
            @click="doProbe"
          >
            解析
          </v-btn>
          <v-btn
            v-else
            variant="flat"
            color="primary"
            size="small"
            rounded="pill"
            class="pillbtn"
            :loading="adding"
            :disabled="selected.size === 0"
            @click="confirmAdd"
          >
            开始下载（{{ selected.size }}）
          </v-btn>
        </div>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
/* 滚动容器与内容宽度沿用 src/styles/main.scss 的 .page-scroll / .page-inner */
.dlwrap {
  padding-bottom: calc(96px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
}

.summary {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 2px 2px 9px;
  font-size: 0.7rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.dot {
  opacity: 0.5;
}

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

/* ---------- 悬浮按钮 ---------- */
.fab {
  position: fixed;
  right: max(16px, var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
  bottom: calc(74px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
  z-index: 30;
  width: 52px;
  height: 52px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 50%;
  color: rgb(var(--v-theme-on-primary));
  background: rgb(var(--v-theme-primary));
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.28);
  cursor: pointer;
  transition: transform 0.14s ease, box-shadow 0.14s ease;
}

.fab:active {
  transform: scale(0.94);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.24);
}

/* ---------- 弹窗 ---------- */
.dlg {
  background: rgb(var(--v-theme-surface)) !important;
}

.dlg__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 13px 16px 11px;
}

.dlg__title {
  font-size: 0.94rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.dlg__body {
  padding: 12px 14px;
  max-height: 58vh;
  overflow-y: auto;
}

.hint {
  margin: 8px 2px 0;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.pinfo {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-bottom: 8px;
}

.pinfo__name {
  font-size: 0.82rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  word-break: break-all;
}

.pinfo__meta {
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.pall {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(var(--v-border-color), calc(var(--v-border-opacity) * 0.6));
}

.pall__box :deep(.v-label) {
  font-size: 0.76rem;
  font-weight: 600;
}

.pall__sum {
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.files {
  display: flex;
  flex-direction: column;
}

.file {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 38px;
  cursor: pointer;
}

.file__box {
  flex-shrink: 0;
  margin-inline-end: 0;
}

.file__name {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.74rem;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file__size {
  flex-shrink: 0;
  font-size: 0.66rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.dlg__actions {
  display: flex;
  justify-content: flex-end;
  gap: 7px;
  padding: 8px 12px 12px;
}

.pillbtn {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

.empty {
  margin-top: 12vh;
}
</style>
