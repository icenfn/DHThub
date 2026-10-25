<script setup lang="ts">
// 设置页：外观 / 网络 / 数据
// 版式取向：分类标题 + 分隔线 + 多列表；每行左「标题 + 副标题」，右控件（开关 / 值 / 按钮）。
// DNS 与 GitHub 镜像不再在页内平铺全部候选，改为「当前值 + 选择」弹窗，
// 弹窗内可一键测速并自动选中延迟最低的源。
import { computed, onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { settings, DEFAULT_DOWNLOAD_CONFIG, type DownloadConfig } from '../stores/settings'
import { useSitesStore } from '../stores/sites'
import { invoke, isTauri } from '../lib/tauri'
import { speedTestMirror, MIRROR_PROBE_URL } from '../lib/mirrors'
import { checkUpdate, updateChecking } from '../lib/update'
import type { DnsServer, DnsSpeedResult, GithubMirror, MirrorSpeedResult } from '../types'

const sitesStore = useSitesStore()
const themeMode = ref<'system' | 'light' | 'dark'>('system')
const autoCheck = ref(true)

// ---------- 轻提示 ----------
const toast = ref('')
const showToast = ref(false)
function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

// ---------- 外观 / 更新 ----------
const THEME_OPTIONS: { value: 'system' | 'light' | 'dark'; label: string; icon: string }[] = [
  { value: 'system', label: '跟随系统', icon: 'mdi-theme-light-dark' },
  { value: 'light', label: '浅色', icon: 'mdi-white-balance-sunny' },
  { value: 'dark', label: '深色', icon: 'mdi-weather-night' },
]

function onThemePick(v: unknown) {
  if (v === 'system' || v === 'light' || v === 'dark') void saveTheme(v)
}

async function saveTheme(v: 'system' | 'light' | 'dark') {
  themeMode.value = v
  await settings.set('theme', v)
}

async function saveAutoCheck(v: boolean) {
  autoCheck.value = v
  await settings.set('autoCheckUpdate', v)
  notice(v ? '已开启自动检测更新（启动时检查）' : '已关闭自动检测更新')
}

// ---------- DNS ----------
const dnsServers = ref<DnsServer[]>([])
const selectedDnsId = ref('')
const dnsSpeeds = ref<Record<string, DnsSpeedResult>>({})
const dnsTesting = ref(false)
const dnsTestingIds = ref<Set<string>>(new Set())
const dnsDialog = ref(false)

interface DnsRow extends DnsServer {
  latencyText: string
  latencyKind: 'ok' | 'warn' | 'err' | ''
  testing: boolean
}

const dnsRows = computed<DnsRow[]>(() =>
  dnsServers.value.map((d) => {
    const r = dnsSpeeds.value[d.id]
    let latencyText = ''
    let latencyKind: DnsRow['latencyKind'] = ''
    if (r) {
      if (r.error) {
        latencyText = '失败'
        latencyKind = 'err'
      } else {
        latencyText = `${r.latency}ms`
        latencyKind = (r.latency ?? 0) < 200 ? 'ok' : 'warn'
      }
    }
    return { ...d, latencyText, latencyKind, testing: dnsTestingIds.value.has(d.id) }
  }),
)

/** 当前生效的 DNS 展示文案（系统默认 / 指定服务器） */
const currentDnsLabel = computed(() => {
  if (!selectedDnsId.value) return '系统默认'
  return dnsServers.value.find((d) => d.id === selectedDnsId.value)?.name ?? '系统默认'
})
const currentDnsSub = computed(() => {
  if (!selectedDnsId.value) return '跟随系统解析'
  return dnsServers.value.find((d) => d.id === selectedDnsId.value)?.base ?? ''
})

async function refreshDns() {
  dnsServers.value = settings.getDnsServers()
  selectedDnsId.value = settings.get('dnsId') ?? ''
}

async function selectDns(d: DnsServer) {
  selectedDnsId.value = d.id
  await settings.selectDns(d.id)
  dnsDialog.value = false
  notice(d.id ? `已切换 DNS：${d.name}（${d.base}）` : '已切换为系统默认 DNS')
}

async function runDnsSpeedTest() {
  if (dnsTesting.value) return
  dnsTesting.value = true
  dnsSpeeds.value = {}
  dnsTestingIds.value = new Set(dnsServers.value.map((d) => d.id))
  try {
    const servers = dnsServers.value.map((d) => d.base)
    const results = await invoke<[string, number][]>('test_dns_latencies', { servers })
    const map = new Map(results)
    const list = dnsServers.value.map(
      (d) =>
        ({
          id: d.id,
          name: d.name,
          base: d.base,
          latency: map.get(d.base) ?? null,
          error: map.get(d.base) == null ? '测速失败' : null,
        }) as DnsSpeedResult,
    )
    dnsSpeeds.value = Object.fromEntries(list.map((r) => [r.id, r]))
    const ok = list.filter((r) => r.latency != null)
    // 测速后自动选中延迟最低的服务器，无需再手动挑
    const best = ok.slice().sort((a, b) => (a.latency ?? 1e9) - (b.latency ?? 1e9))[0]
    if (best) {
      const target = dnsServers.value.find((d) => d.id === best.id)
      if (target) {
        selectedDnsId.value = target.id
        await settings.selectDns(target.id)
      }
    }
    notice(
      `测速完成：${ok.length}/${list.length} 个 DNS 可用${best ? `，已自动选择最快：${best.name}` : ''}`,
    )
  } catch (e) {
    notice(String(e))
  } finally {
    dnsTesting.value = false
    dnsTestingIds.value = new Set()
  }
}

// ---------- GitHub 镜像 ----------
const mirrors = ref<GithubMirror[]>([])
const selectedMirrorId = ref('direct')
const speeds = ref<Record<string, MirrorSpeedResult>>({})
const testing = ref(false)
const testingIds = ref<Set<string>>(new Set())
const mirrorDialog = ref(false)

interface MirrorRow extends GithubMirror {
  latencyText: string
  latencyKind: 'ok' | 'warn' | 'err' | ''
  testing: boolean
}

const mirrorRows = computed<MirrorRow[]>(() =>
  mirrors.value.map((m) => {
    const r = speeds.value[m.id]
    let latencyText = ''
    let latencyKind: MirrorRow['latencyKind'] = ''
    if (r) {
      if (r.error) {
        latencyText = '失败'
        latencyKind = 'err'
      } else {
        latencyText = `${r.latency}ms`
        latencyKind = (r.latency ?? 0) < 800 ? 'ok' : 'warn'
      }
    }
    return { ...m, latencyText, latencyKind, testing: testingIds.value.has(m.id) }
  }),
)

function mirrorLabel(m: GithubMirror): string {
  return m.base || 'https://github.com'
}

const currentMirrorLabel = computed(() => {
  const m = mirrors.value.find((x) => x.id === selectedMirrorId.value)
  return m ? mirrorLabel(m) : 'https://github.com'
})

async function refreshMirrors() {
  mirrors.value = settings.getMirrors()
  selectedMirrorId.value = settings.get('githubMirrorId')
}

async function selectMirror(m: GithubMirror) {
  selectedMirrorId.value = m.id
  await settings.selectMirror(m.id)
  mirrorDialog.value = false
  notice(`已切换：${mirrorLabel(m)}`)
}

async function runSpeedTest() {
  if (testing.value) return
  testing.value = true
  speeds.value = {}
  testingIds.value = new Set(mirrors.value.map((m) => m.id))
  try {
    await Promise.all(
      mirrors.value.map(async (m) => {
        const r = await speedTestMirror(m)
        speeds.value = { ...speeds.value, [r.id]: r }
        const next = new Set(testingIds.value)
        next.delete(r.id)
        testingIds.value = next
        return r
      }),
    )
    const list = mirrors.value
      .map((m) => speeds.value[m.id])
      .filter((r): r is MirrorSpeedResult => !!r)
    const ok = list.filter((r) => r.latency != null)
    const best = ok.slice().sort((a, b) => (a.latency ?? 1e9) - (b.latency ?? 1e9))[0]
    if (best) {
      const target = mirrors.value.find((m) => m.id === best.id)
      if (target) {
        selectedMirrorId.value = target.id
        await settings.selectMirror(target.id)
      }
    }
    notice(`测速完成：${ok.length}/${list.length} 个镜像可用${best ? '，已自动选择最快' : ''}`)
  } catch (e) {
    notice(String(e))
  } finally {
    testing.value = false
    testingIds.value = new Set()
  }
}

// ---------- 下载内核（rqbit）配置 ----------
const dlCfg = ref<DownloadConfig>({ ...DEFAULT_DOWNLOAD_CONFIG })
const dlApplying = ref(false)
/** 修改后需要重启会话才生效的标记 */
const dlPendingRestart = ref(false)
/** 追踪器输入框（每行一个 URL） */
const trackerText = ref('')

/** 把当前配置下发给内核；会话已运行时提示需重启 */
async function applyDownloadConfig(silent = false) {
  dlApplying.value = true
  try {
    const cfg = dlCfg.value
    cfg.extraTrackers = trackerText.value
      .split('\n')
      .map((t) => t.trim())
      .filter((t) => t.length > 0)
    await settings.saveDownloadConfig(cfg)
    if (isTauri) {
      const needsRestart = await invoke<boolean>('configure_downloads', {
        cfg: {
          dht_enabled: cfg.dhtEnabled,
          dht_port: cfg.dhtPort,
          download_limit_kbps: cfg.downloadLimitKbps,
          upload_limit_kbps: cfg.uploadLimitKbps,
          peer_limit: cfg.peerLimit,
          concurrent_init_limit: cfg.concurrentInitLimit,
          disable_trackers: cfg.disableTrackers,
          ipv4_only: cfg.ipv4Only,
          extra_trackers: cfg.extraTrackers,
        },
      })
      dlPendingRestart.value = needsRestart
      if (!silent) notice(needsRestart ? '配置已保存，重启下载会话后生效' : '下载配置已保存')
    } else if (!silent) {
      notice('下载配置已保存')
    }
  } catch (e) {
    notice(String(e))
  } finally {
    dlApplying.value = false
  }
}

/** 立即重启下载会话，使新配置生效（会丢弃当前任务列表） */
async function restartSession() {
  dlApplying.value = true
  try {
    await applyDownloadConfig(true)
    await invoke('restart_download_session')
    dlPendingRestart.value = false
    notice('下载会话已重启，配置生效')
  } catch (e) {
    notice(String(e))
  } finally {
    dlApplying.value = false
  }
}

/** 单项开关变更即保存 */
async function onDlToggle() {
  await applyDownloadConfig(true)
  notice('下载配置已保存')
}

// ---------- 数据 ----------
const clearing = ref(false)
const importing = ref(false)

async function clearHistory() {
  clearing.value = true
  try {
    await invoke('clear_all_history')
    notice('已清空全部历史记录')
  } finally {
    clearing.value = false
  }
}

async function clearSearchHistory() {
  await settings.clearSearchHistory()
  notice('已清空搜索历史')
}

async function resetSites() {
  await sitesStore.reset()
  notice('站点已重置')
}

async function doExportSettings() {
  if (!isTauri) {
    notice('浏览器预览模式不支持文件导出')
    return
  }
  try {
    const json = await settings.exportJson()
    const path = await save({
      title: '导出设置',
      defaultPath: 'dhthub-settings.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path) return
    await writeTextFile(path, json)
    notice('设置已导出')
  } catch (e) {
    notice(String(e))
  }
}

async function doImportSettings() {
  if (!isTauri) {
    notice('浏览器预览模式不支持文件导入')
    return
  }
  importing.value = true
  try {
    const path = await open({
      title: '导入设置',
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path || typeof path !== 'string') return
    const text = await readTextFile(path)
    const count = await settings.importJson(text)
    themeMode.value = settings.get('theme')
    autoCheck.value = settings.get('autoCheckUpdate')
    await refreshDns()
    await refreshMirrors()
    notice(`设置导入成功（${count} 项）`)
  } catch (e) {
    notice(String(e))
  } finally {
    importing.value = false
  }
}

onMounted(async () => {
  await settings.ready()
  themeMode.value = settings.get('theme')
  autoCheck.value = settings.get('autoCheckUpdate')
  dlCfg.value = settings.getDownloadConfig()
  trackerText.value = dlCfg.value.extraTrackers.join('\n')
  await refreshDns()
  await refreshMirrors()
  await sitesStore.load().catch(() => undefined)
  // 挂载即把当前配置下发给内核，确保首次添加任务前配置已就位
  if (isTauri) await applyDownloadConfig(true)
})
</script>

<template>
  <div class="settings">
    <main class="stmain">
      <!-- ===== 外观 ===== -->
      <section class="group">
        <h2 class="group__title">外观</h2>

        <div class="list">
          <div class="row">
            <div class="row__label">
              <span class="row__title">主题</span>
              <span class="row__sub">跟随系统或强制指定明暗</span>
            </div>
            <v-btn-toggle
              class="seg"
              :model-value="themeMode"
              mandatory
              divided
              variant="outlined"
              density="compact"
              rounded="pill"
              color="primary"
              @update:model-value="(v: unknown) => onThemePick(v)"
            >
              <v-btn
                v-for="opt in THEME_OPTIONS"
                :key="opt.value"
                :value="opt.value"
                :icon="opt.icon"
                size="small"
                :aria-label="opt.label"
                class="seg__item"
              />
            </v-btn-toggle>
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">自动检测更新</span>
              <span class="row__sub">启动时静默检查 GitHub Release</span>
            </div>
            <v-switch
              :model-value="autoCheck"
              color="primary"
              density="compact"
              size="small"
              hide-details
              inset
              class="row__switch"
              aria-label="自动检测更新"
              @update:model-value="(v: unknown) => saveAutoCheck(v === true)"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">更新检查</span>
              <span class="row__sub">立即请求最新版本信息</span>
            </div>
            <v-btn
              variant="tonal"
              size="small"
              rounded="pill"
              class="pillbtn"
              :loading="updateChecking"
              :prepend-icon="updateChecking ? undefined : 'mdi-update'"
              @click="checkUpdate"
            >
              检查
            </v-btn>
          </div>
        </div>
      </section>

      <!-- ===== 网络 ===== -->
      <section class="group">
        <h2 class="group__title">网络</h2>

        <div class="list">
          <div class="row row--click" @click="dnsDialog = true">
            <div class="row__label">
              <span class="row__title">DNS 服务器</span>
              <span class="row__sub mono">{{ currentDnsSub || '跟随系统解析' }}</span>
            </div>
            <div class="row__side">
              <span class="row__value">{{ currentDnsLabel }}</span>
              <v-icon icon="mdi-chevron-right" size="18" class="chev" />
            </div>
          </div>

          <div class="row row--click" @click="mirrorDialog = true">
            <div class="row__label">
              <span class="row__title">GitHub 镜像</span>
              <span class="row__sub">订阅与更新的加速通道</span>
            </div>
            <div class="row__side">
              <span class="row__value mono">{{ currentMirrorLabel }}</span>
              <v-icon icon="mdi-chevron-right" size="18" class="chev" />
            </div>
          </div>
        </div>
      </section>

      <!-- ===== 下载 / BitTorrent ===== -->
      <section class="group">
        <h2 class="group__title">下载内核（BitTorrent）</h2>

        <v-banner
          v-if="dlPendingRestart"
          density="compact"
          color="warning"
          variant="tonal"
          rounded="lg"
          class="dlbanner"
        >
          配置已修改，重启下载会话后生效
          <template #actions>
            <v-btn
              variant="tonal"
              color="warning"
              size="x-small"
              rounded="pill"
              class="pillbtn"
              :loading="dlApplying"
              @click="restartSession"
            >
              立即重启
            </v-btn>
          </template>
        </v-banner>

        <div class="list">
          <div class="row">
            <div class="row__label">
              <span class="row__title">启用 DHT</span>
              <span class="row__sub">无 tracker 时通过分布式哈希表发现 peer</span>
            </div>
            <v-switch
              v-model="dlCfg.dhtEnabled"
              color="primary"
              density="compact"
              size="small"
              hide-details
              inset
              class="row__switch"
              aria-label="启用 DHT"
              @update:model-value="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">DHT 监听端口</span>
              <span class="row__sub">0 = 随机分配（一般无需指定）</span>
            </div>
            <v-text-field
              v-model.number="dlCfg.dhtPort"
              type="number"
              variant="outlined"
              density="compact"
              hide-details
              class="num"
              :min="0"
              :max="65535"
              @change="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">下载限速</span>
              <span class="row__sub">KB/s，0 = 不限速</span>
            </div>
            <v-text-field
              v-model.number="dlCfg.downloadLimitKbps"
              type="number"
              variant="outlined"
              density="compact"
              hide-details
              class="num"
              :min="0"
              @change="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">上传限速</span>
              <span class="row__sub">KB/s，0 = 不限速</span>
            </div>
            <v-text-field
              v-model.number="dlCfg.uploadLimitKbps"
              type="number"
              variant="outlined"
              density="compact"
              hide-details
              class="num"
              :min="0"
              @change="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">Peer 上限</span>
              <span class="row__sub">每个任务的最大连接数，0 = 内核默认</span>
            </div>
            <v-text-field
              v-model.number="dlCfg.peerLimit"
              type="number"
              variant="outlined"
              density="compact"
              hide-details
              class="num"
              :min="0"
              @change="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">并发初始化</span>
              <span class="row__sub">同时建立连接的任务数，0 = 内核默认</span>
            </div>
            <v-text-field
              v-model.number="dlCfg.concurrentInitLimit"
              type="number"
              variant="outlined"
              density="compact"
              hide-details
              class="num"
              :min="0"
              @change="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">禁用 Tracker</span>
              <span class="row__sub">仅使用 DHT 发现 peer（需开启 DHT）</span>
            </div>
            <v-switch
              v-model="dlCfg.disableTrackers"
              color="primary"
              density="compact"
              size="small"
              hide-details
              inset
              class="row__switch"
              aria-label="禁用 Tracker"
              @update:model-value="onDlToggle"
            />
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">仅 IPv4</span>
              <span class="row__sub">网络不支持 IPv6 时可开启</span>
            </div>
            <v-switch
              v-model="dlCfg.ipv4Only"
              color="primary"
              density="compact"
              size="small"
              hide-details
              inset
              class="row__switch"
              aria-label="仅 IPv4"
              @update:model-value="onDlToggle"
            />
          </div>
        </div>

        <div class="trackerbox">
          <span class="trackerbox__title">追加 Tracker（每行一个 URL）</span>
          <v-textarea
            v-model="trackerText"
            variant="outlined"
            density="compact"
            rows="3"
            auto-grow
            hide-details
            class="mono-input"
            placeholder="udp://tracker.openbittorrent.com:6969/announce"
            @blur="onDlToggle"
          />
        </div>
      </section>

      <!-- ===== 数据 ===== -->
      <section class="group">
        <h2 class="group__title">数据</h2>

        <div class="list">
          <div class="row">
            <div class="row__label">
              <span class="row__title">导出设置</span>
              <span class="row__sub">保存为 JSON 文件</span>
            </div>
            <v-btn variant="tonal" size="small" rounded="pill" class="pillbtn" @click="doExportSettings">
              导出
            </v-btn>
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">导入设置</span>
              <span class="row__sub">从 JSON 文件恢复</span>
            </div>
            <v-btn
              variant="tonal"
              size="small"
              rounded="pill"
              class="pillbtn"
              :loading="importing"
              @click="doImportSettings"
            >
              导入
            </v-btn>
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">清空全部历史</span>
              <span class="row__sub">磁力 / 复制 / 浏览记录</span>
            </div>
            <v-btn
              variant="tonal"
              color="error"
              size="small"
              rounded="pill"
              class="pillbtn"
              :loading="clearing"
              @click="clearHistory"
            >
              清空
            </v-btn>
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">清空搜索历史</span>
              <span class="row__sub">输入框的联想词记录</span>
            </div>
            <v-btn variant="tonal" color="error" size="small" rounded="pill" class="pillbtn" @click="clearSearchHistory">
              清空
            </v-btn>
          </div>

          <div class="row">
            <div class="row__label">
              <span class="row__title">重置站点订阅</span>
              <span class="row__sub">恢复内置搜索源列表</span>
            </div>
            <v-btn variant="tonal" color="warning" size="small" rounded="pill" class="pillbtn" @click="resetSites">
              重置
            </v-btn>
          </div>
        </div>
      </section>

      <footer class="stfoot">DHThub · 多源磁力聚合 · v0.5.0</footer>
    </main>

    <!-- ===== DNS 选择弹窗 ===== -->
    <v-dialog v-model="dnsDialog" max-width="460" scrollable>
      <v-card rounded="lg" class="dlg">
        <div class="dlg__head">
          <div class="dlg__heading">
            <span class="dlg__title">选择 DNS 服务器</span>
            <span class="dlg__hint">测速后自动选中延迟最低的一项</span>
          </div>
          <v-btn
            variant="tonal"
            size="x-small"
            rounded="pill"
            class="pillbtn"
            :loading="dnsTesting"
            :prepend-icon="dnsTesting ? undefined : 'mdi-speedometer'"
            @click="runDnsSpeedTest"
          >
            测速
          </v-btn>
        </div>

        <v-divider />

        <div class="dlg__list">
          <div class="row row--click" @click="selectDns({ id: '', name: '系统默认', base: '' })">
            <v-icon
              :icon="selectedDnsId === '' ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
              :color="selectedDnsId === '' ? 'primary' : undefined"
              size="18"
              class="radio"
            />
            <div class="row__label">
              <span class="row__title">系统默认</span>
              <span class="row__sub">使用系统 DNS 解析</span>
            </div>
          </div>

          <div v-for="row in dnsRows" :key="row.id" class="row row--click" @click="selectDns(row)">
            <v-icon
              :icon="selectedDnsId === row.id ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
              :color="selectedDnsId === row.id ? 'primary' : undefined"
              size="18"
              class="radio"
            />
            <div class="row__label">
              <span class="row__title">{{ row.name }}</span>
              <span class="row__sub mono">{{ row.base }}</span>
            </div>
            <div class="row__side">
              <v-progress-circular
                v-if="row.testing"
                indeterminate
                size="13"
                width="2"
                color="primary"
              />
              <span
                v-else-if="row.latencyText"
                class="latency"
                :class="`latency--${row.latencyKind}`"
              >
                {{ row.latencyText }}
              </span>
            </div>
          </div>
        </div>

        <div class="dlg__actions">
          <v-btn variant="text" size="small" rounded="pill" class="pillbtn" @click="dnsDialog = false">
            关闭
          </v-btn>
        </div>
      </v-card>
    </v-dialog>

    <!-- ===== GitHub 镜像选择弹窗 ===== -->
    <v-dialog v-model="mirrorDialog" max-width="480" scrollable>
      <v-card rounded="lg" class="dlg">
        <div class="dlg__head">
          <div class="dlg__heading">
            <span class="dlg__title">选择 GitHub 镜像</span>
            <span class="dlg__hint">探针：{{ MIRROR_PROBE_URL }}</span>
          </div>
          <v-btn
            variant="tonal"
            size="x-small"
            rounded="pill"
            class="pillbtn"
            :loading="testing"
            :prepend-icon="testing ? undefined : 'mdi-speedometer'"
            @click="runSpeedTest"
          >
            测速
          </v-btn>
        </div>

        <v-divider />

        <div class="dlg__list">
          <div
            v-for="row in mirrorRows"
            :key="row.id"
            class="row row--click"
            @click="selectMirror(row)"
          >
            <v-icon
              :icon="selectedMirrorId === row.id ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
              :color="selectedMirrorId === row.id ? 'primary' : undefined"
              size="18"
              class="radio"
            />
            <div class="row__label">
              <span class="row__title mono">{{ mirrorLabel(row) }}</span>
              <span v-if="row.id === 'direct'" class="row__sub">官方直连</span>
            </div>
            <div class="row__side">
              <v-progress-circular
                v-if="row.testing"
                indeterminate
                size="13"
                width="2"
                color="primary"
              />
              <span
                v-else-if="row.latencyText"
                class="latency"
                :class="`latency--${row.latencyKind}`"
              >
                {{ row.latencyText }}
              </span>
            </div>
          </div>
        </div>

        <div class="dlg__actions">
          <v-btn variant="text" size="small" rounded="pill" class="pillbtn" @click="mirrorDialog = false">
            关闭
          </v-btn>
        </div>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
/* 设置作为 swiper 的一页嵌在 .pane 里，滚动交给外层 .pane；
   这里不自建 height:100% + overflow 的嵌套滚动容器，避免内容被裁剪。 */
.settings {
  display: block;
  min-height: 100%;
  background: rgb(var(--v-theme-surface));
}

.stmain {
  width: 100%;
  max-width: 1080px;
  margin: 0 auto;
  padding: 6px 12px calc(20px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* ---------- 分类标题 ---------- */
.group {
  display: flex;
  flex-direction: column;
  padding-top: 12px;
}

.group__title {
  margin: 0 0 6px;
  padding: 0 2px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgb(var(--v-theme-primary));
}

/* 分类之间的分隔线（最后一项除外） */
.group + .group {
  margin-top: 4px;
  border-top: 1px solid rgba(var(--v-border-color), calc(var(--v-border-opacity) * 0.8));
}

/* ---------- 列表 ---------- */
.list {
  display: flex;
  flex-direction: column;
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 7px 4px;
  border-bottom: 1px solid rgba(var(--v-border-color), calc(var(--v-border-opacity) * 0.6));
}

.row:last-child {
  border-bottom: none;
}

.row--click {
  cursor: pointer;
  border-radius: 10px;
  transition: background-color 0.14s ease;
}

.row--click:hover {
  background: rgba(var(--v-theme-on-surface), 0.04);
}

.row--click:active {
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.radio {
  flex-shrink: 0;
}

.row__label {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  line-height: 1.3;
}

.row__title {
  font-size: 0.86rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row__sub {
  margin-top: 2px;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row__side {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
}

.row__value {
  max-width: 42vw;
  font-size: 0.76rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chev {
  opacity: 0.55;
}

.row__switch {
  flex-shrink: 0;
  margin: 0;
}

.row__switch :deep(.v-selection-control) {
  min-height: 40px;
}

/* ---------- 主题分段选择 ---------- */
.seg {
  flex-shrink: 0;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.seg__item {
  min-width: 0;
}

/* ---------- 延迟标签 ---------- */
.latency {
  padding: 1px 7px;
  border-radius: 999px;
  font-size: 0.63rem;
  font-weight: 700;
  white-space: nowrap;
}

.latency--ok {
  background: rgba(var(--v-theme-success), 0.15);
  color: rgb(var(--v-theme-success));
}

.latency--err {
  background: rgba(var(--v-theme-error), 0.15);
  color: rgb(var(--v-theme-error));
}

.latency--warn {
  background: rgba(var(--v-theme-warning), 0.17);
  color: rgb(var(--v-theme-warning));
}

/* ---------- 弹窗 ---------- */
.dlg {
  background: rgb(var(--v-theme-surface)) !important;
}

.dlg__head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px 11px;
}

.dlg__heading {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.dlg__title {
  font-size: 0.94rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.dlg__hint {
  margin-top: 3px;
  font-size: 0.66rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dlg__list {
  display: flex;
  flex-direction: column;
  padding: 4px 10px;
  max-height: 56vh;
  overflow-y: auto;
}

.dlg__actions {
  display: flex;
  justify-content: flex-end;
  gap: 7px;
  padding: 8px 12px 12px;
}

/* ---------- 下载内核配置 ---------- */
.dlbanner {
  margin-bottom: 8px;
}

.num {
  flex-shrink: 0;
  width: 110px;
}

.num :deep(input) {
  text-align: right;
  font-size: 0.8rem;
  font-family: 'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace;
}

.trackerbox {
  margin-top: 6px;
  padding: 0 4px;
}

.trackerbox__title {
  display: block;
  margin-bottom: 5px;
  font-size: 0.72rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface-variant));
}

/* ---------- 通用 ---------- */
.pillbtn {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

.mono {
  font-family: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace;
}

/* ---------- 页脚 ---------- */
.stfoot {
  text-align: center;
  padding: 16px 0 2px;
  font-size: 0.66rem;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.75;
}

/* ---------- 宽屏：双列 ---------- */
@media (min-width: 760px) {
  .stmain {
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: start;
    column-gap: 18px;
  }

  .stfoot {
    grid-column: 1 / -1;
  }
}
</style>
