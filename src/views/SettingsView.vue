<script setup lang="ts">
// 设置页：外观 / 更新 / DNS / GitHub 镜像 / 数据管理
// 版式取向：信息密集 —— 每个分区用「标题行 + 紧凑列表」承载，同屏可见更多条目；
// 单选行用自绘 radio 指示，避免 Vuetify 列表在旧内核下的高度抖动。
import { computed, onMounted, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { settings } from '../stores/settings'
import { useSitesStore } from '../stores/sites'
import { invoke, isTauri } from '../lib/tauri'
import { speedTestMirror, MIRROR_PROBE_URL } from '../lib/mirrors'
import { pushOverlay } from '../lib/overlay'
import { checkUpdate, updateChecking } from '../lib/update'
import type { DnsServer, DnsSpeedResult, GithubMirror, MirrorSpeedResult } from '../types'

const sitesStore = useSitesStore()
const themeMode = ref<'system' | 'light' | 'dark'>('system')
const autoCheck = ref(true)

// ---------- DNS ----------
const dnsServers = ref<DnsServer[]>([])
const selectedDnsId = ref('')
const dnsSpeeds = ref<Record<string, DnsSpeedResult>>({})
const dnsTesting = ref(false)
const dnsTestingIds = ref<Set<string>>(new Set())
const addDnsDialog = ref(false)
const newDnsBase = ref('')

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

// ---------- GitHub 镜像 ----------
const mirrors = ref<GithubMirror[]>([])
const selectedMirrorId = ref('direct')
const speeds = ref<Record<string, MirrorSpeedResult>>({})
const testing = ref(false)
const testingIds = ref<Set<string>>(new Set())
const addMirrorDialog = ref(false)
const newMirrorBase = ref('')

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

// ---------- 弹层栈 ----------
watch(addDnsDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { addDnsDialog.value = false }))
})
watch(addMirrorDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { addMirrorDialog.value = false }))
})

// ---------- 轻提示 ----------
const toast = ref('')
const showToast = ref(false)
function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

// ---------- 外观 / 更新 ----------
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

// ---------- DNS ----------
async function refreshDns() {
  dnsServers.value = settings.getDnsServers()
  selectedDnsId.value = settings.get('dnsId') ?? ''
}

async function selectDns(d: DnsServer) {
  selectedDnsId.value = d.id
  await settings.selectDns(d.id)
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
    const ok = list.filter((r) => r.latency != null).length
    notice(`测速完成：${ok}/${list.length} 个 DNS 可用`)
  } catch (e) {
    notice(String(e))
  } finally {
    dnsTesting.value = false
    dnsTestingIds.value = new Set()
  }
}

function openAddDns() {
  newDnsBase.value = ''
  addDnsDialog.value = true
}

async function addDns() {
  try {
    const d = await settings.addDns(newDnsBase.value)
    newDnsBase.value = ''
    addDnsDialog.value = false
    await refreshDns()
    notice(`已添加 DNS：${d.name}（${d.base}）`)
  } catch (e) {
    notice(String(e))
  }
}

async function removeDns(d: DnsServer) {
  if (d.builtin) return
  await settings.removeDns(d.id)
  delete dnsSpeeds.value[d.id]
  await refreshDns()
  notice('已删除自定义 DNS')
}

// ---------- 镜像 ----------
async function refreshMirrors() {
  mirrors.value = settings.getMirrors()
  selectedMirrorId.value = settings.get('githubMirrorId')
}

async function selectMirror(m: GithubMirror) {
  selectedMirrorId.value = m.id
  await settings.selectMirror(m.id)
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
    const ok = list.filter((r) => r.latency != null).length
    notice(`测速完成：${ok}/${list.length} 个镜像可用`)
  } finally {
    testing.value = false
    testingIds.value = new Set()
  }
}

function openAddMirror() {
  newMirrorBase.value = ''
  addMirrorDialog.value = true
}

async function addMirror() {
  try {
    const m = await settings.addMirror(newMirrorBase.value)
    newMirrorBase.value = ''
    addMirrorDialog.value = false
    await refreshMirrors()
    notice(`已添加镜像：${mirrorLabel(m)}`)
  } catch (e) {
    notice(String(e))
  }
}

async function removeMirror(m: GithubMirror) {
  if (m.builtin) return
  await settings.removeMirror(m.id)
  delete speeds.value[m.id]
  await refreshMirrors()
  notice('已删除自定义镜像')
}

// ---------- 导入 / 导出 ----------
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

const THEME_OPTIONS: { value: 'system' | 'light' | 'dark'; label: string; icon: string }[] = [
  { value: 'system', label: '跟随系统', icon: 'mdi-theme-light-dark' },
  { value: 'light', label: '浅色', icon: 'mdi-white-balance-sunny' },
  { value: 'dark', label: '深色', icon: 'mdi-weather-night' },
]

onMounted(async () => {
  await settings.ready()
  themeMode.value = settings.get('theme')
  autoCheck.value = settings.get('autoCheckUpdate')
  await refreshDns()
  await refreshMirrors()
  await sitesStore.load().catch(() => undefined)
})
</script>

<template>
  <div class="settings">
    <!-- 设置已作为首页框架 swiper 的第 4 页嵌入，不再有独立顶栏与返回按钮 -->
    <main class="stmain">
      <!-- ===== 外观 ===== -->
      <v-card class="card" variant="outlined" rounded="lg">
        <div class="card__head">
          <v-icon icon="mdi-palette-outline" size="16" color="primary" />
          <span class="card__title">外观</span>
        </div>

        <v-btn-toggle
          class="seg"
          :model-value="themeMode"
          mandatory
          divided
          variant="outlined"
          density="comfortable"
          rounded="pill"
          color="primary"
          @update:model-value="(v: unknown) => onThemePick(v)"
        >
          <v-btn
            v-for="opt in THEME_OPTIONS"
            :key="opt.value"
            :value="opt.value"
            :prepend-icon="opt.icon"
            class="seg__item"
          >
            {{ opt.label }}
          </v-btn>
        </v-btn-toggle>

        <div class="row">
          <div class="row__label">
            <span class="row__title">自动检测更新</span>
            <span class="row__sub">启动时静默检查 GitHub Release</span>
          </div>
          <v-switch
            :model-value="autoCheck"
            color="primary"
            density="compact"
            hide-details
            inset
            class="row__switch"
            aria-label="自动检测更新"
            @update:model-value="(v: unknown) => saveAutoCheck(v === true)"
          />
        </div>

        <v-btn
          class="pillbtn pillbtn--tonal self-start"
          variant="tonal"
          size="small"
          rounded="pill"
          :loading="updateChecking"
          :prepend-icon="updateChecking ? undefined : 'mdi-update'"
          @click="checkUpdate"
        >
          立即检查更新
        </v-btn>
      </v-card>

      <!-- ===== DNS ===== -->
      <v-card class="card" variant="outlined" rounded="lg">
        <div class="card__head">
          <v-icon icon="mdi-server-network" size="16" color="primary" />
          <span class="card__title">DNS 服务器</span>
          <span class="card__hint">搜索请求域名解析</span>
          <v-btn
            variant="tonal"
            size="x-small"
            rounded="pill"
            class="card__action"
            :loading="dnsTesting"
            :prepend-icon="dnsTesting ? undefined : 'mdi-speedometer'"
            @click="runDnsSpeedTest"
          >
            全部测速
          </v-btn>
        </div>

        <div class="rows">
          <!-- 系统默认 -->
          <div class="row row--click" @click="selectDns({ id: '', name: '系统默认', base: '' })">
            <v-icon
              :icon="selectedDnsId === '' ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
              :color="selectedDnsId === '' ? 'primary' : undefined"
              size="18"
              class="radio"
            />
            <div class="row__label">
              <span class="row__title">系统默认</span>
              <span class="row__sub mono">使用系统 DNS</span>
            </div>
            <v-chip v-if="selectedDnsId === ''" size="x-small" color="primary" variant="tonal">
              当前
            </v-chip>
          </div>

          <div v-for="row in dnsRows" :key="row.id" class="row row--click" @click="selectDns(row)">
            <v-icon
              :icon="selectedDnsId === row.id ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
              :color="selectedDnsId === row.id ? 'primary' : undefined"
              size="18"
              class="radio"
            />
            <div class="row__label">
              <span class="row__title">
                {{ row.name }}
                <v-chip v-if="row.builtin" size="x-small" color="primary" variant="tonal">
                  内置
                </v-chip>
              </span>
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
              <v-btn
                v-if="!row.builtin"
                icon="mdi-delete-outline"
                variant="text"
                size="x-small"
                title="删除"
                @click.stop="removeDns(row)"
              />
            </div>
          </div>
        </div>

        <v-btn
          class="addbtn"
          variant="text"
          size="small"
          rounded="pill"
          color="primary"
          prepend-icon="mdi-plus"
          @click="openAddDns"
        >
          添加自定义 DNS
        </v-btn>
      </v-card>

      <!-- ===== GitHub 镜像 ===== -->
      <v-card class="card" variant="outlined" rounded="lg">
        <div class="card__head">
          <v-icon icon="mdi-cloud-sync-outline" size="16" color="primary" />
          <span class="card__title">GitHub 镜像</span>
          <span class="card__hint">订阅与更新的加速通道</span>
          <v-btn
            variant="tonal"
            size="x-small"
            rounded="pill"
            class="card__action"
            :loading="testing"
            :prepend-icon="testing ? undefined : 'mdi-speedometer'"
            @click="runSpeedTest"
          >
            全部测速
          </v-btn>
        </div>

        <div class="rows">
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
              <span class="row__title mono">
                {{ mirrorLabel(row) }}
                <v-chip v-if="row.id === 'direct'" size="x-small" color="primary" variant="tonal">
                  直连
                </v-chip>
              </span>
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
              <v-btn
                v-if="!row.builtin"
                icon="mdi-delete-outline"
                variant="text"
                size="x-small"
                title="删除镜像"
                @click.stop="removeMirror(row)"
              />
            </div>
          </div>
        </div>

        <div class="card__foot">
          <v-btn
            class="addbtn"
            variant="text"
            size="small"
            rounded="pill"
            color="primary"
            prepend-icon="mdi-plus"
            @click="openAddMirror"
          >
            添加自定义镜像
          </v-btn>
          <span class="card__probe">探针：{{ MIRROR_PROBE_URL }}</span>
        </div>
      </v-card>

      <!-- ===== 数据管理 ===== -->
      <v-card class="card" variant="outlined" rounded="lg">
        <div class="card__head">
          <v-icon icon="mdi-database-cog-outline" size="16" color="primary" />
          <span class="card__title">数据管理</span>
        </div>

        <div class="btnrow">
          <v-btn
            variant="tonal"
            size="small"
            rounded="pill"
            prepend-icon="mdi-export"
            @click="doExportSettings"
          >
            导出设置
          </v-btn>
          <v-btn
            variant="tonal"
            size="small"
            rounded="pill"
            :loading="importing"
            :prepend-icon="importing ? undefined : 'mdi-import'"
            @click="doImportSettings"
          >
            导入设置
          </v-btn>
        </div>

        <v-divider class="my-2" />

        <div class="btnrow">
          <v-btn
            variant="tonal"
            color="error"
            size="small"
            rounded="pill"
            :loading="clearing"
            :prepend-icon="clearing ? undefined : 'mdi-delete-sweep-outline'"
            @click="clearHistory"
          >
            清空全部历史
          </v-btn>
          <v-btn
            variant="tonal"
            color="error"
            size="small"
            rounded="pill"
            prepend-icon="mdi-history"
            @click="clearSearchHistory"
          >
            清空搜索历史
          </v-btn>
          <v-btn
            variant="tonal"
            color="warning"
            size="small"
            rounded="pill"
            prepend-icon="mdi-antenna-off"
            @click="resetSites"
          >
            重置站点订阅
          </v-btn>
        </div>
      </v-card>

      <!-- 关于信息（原「关于页」已删除，仅保留版本号于页脚） -->
      <footer class="stfoot">DHThub · 多源磁力聚合 · v0.5.0</footer>

      <!-- 添加 DNS -->
      <v-dialog v-model="addDnsDialog" max-width="400">
        <v-card rounded="xl" variant="flat" color="surface-container-low" border>
          <div class="dlgbody dlgbody--tight">
            <div class="dlgtitle">添加自定义 DNS</div>
            <v-text-field
              v-model="newDnsBase"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="223.6.6.6 或 223.6.6.6:53"
              class="mono-input"
            />
            <div class="dlghint">支持 IP 或 IP:端口，保存后可参与全部测速</div>
          </div>
          <v-card-actions class="dlgactions">
            <v-btn variant="text" size="small" @click="addDnsDialog = false">取消</v-btn>
            <v-btn
              color="primary"
              variant="flat"
              size="small"
              rounded="pill"
              :disabled="!newDnsBase.trim()"
              @click="addDns"
            >
              保存
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <!-- 添加镜像 -->
      <v-dialog v-model="addMirrorDialog" max-width="460">
        <v-card rounded="xl" variant="flat" color="surface-container-low" border>
          <div class="dlgbody dlgbody--tight">
            <div class="dlgtitle">添加自定义 GitHub 镜像</div>
            <v-text-field
              v-model="newMirrorBase"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="https://ghproxy.net/"
              class="mono-input"
            />
            <div class="dlghint">
              前缀代理模式：请求 GitHub 地址时自动拼接该前缀，例如
              <span class="mono"
                >{{ newMirrorBase || 'https://镜像地址/' }}https://api.github.com/…</span
              >
            </div>
          </div>
          <v-card-actions class="dlgactions">
            <v-btn variant="text" size="small" @click="addMirrorDialog = false">取消</v-btn>
            <v-btn
              color="primary"
              variant="flat"
              size="small"
              rounded="pill"
              :disabled="!newMirrorBase.trim()"
              @click="addMirror"
            >
              保存
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <v-snackbar v-model="showToast" location="bottom" color="inverse-surface" rounded="lg" :timeout="2200">
        {{ toast }}
      </v-snackbar>
    </main>
  </div>
</template>

<style scoped>
/* 设置作为 swiper 的第 4 页嵌在 .pane 里，滚动交给外层 .pane。
   这里不再自建 height:100% + overflow:hidden 的嵌套滚动容器，
   否则内容会被内层容器裁剪成「折叠 / 显示不全」。 */
.settings {
  display: block;
  min-height: 100%;
  background: rgb(var(--v-theme-surface));
}

/* ---------- 主体：窄屏单列，宽屏双列（信息密集） ---------- */
.stmain {
  width: 100%;
  max-width: 1080px;
  margin: 0 auto;
  padding: 10px 12px calc(20px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
  display: flex;
  flex-direction: column;
  gap: 10px;
}

@media (min-width: 760px) {
  .stmain {
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: start;
  }
}

/* ---------- 卡片 ---------- */
.card {
  display: flex;
  flex-direction: column;
  padding: 11px 13px 13px;
  background: rgb(var(--v-theme-surface-container-low));
  border-color: rgba(var(--v-border-color), var(--v-border-opacity));
}

.card__head {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 9px;
}

.card__title {
  font-size: 0.84rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
}

.card__hint {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.66rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 卡片头部右侧的测速按钮：交给 v-btn，这里只压尺寸 */
.card__action {
  flex-shrink: 0;
  text-transform: none;
  letter-spacing: 0;
}

.card__foot {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 2px;
}

.card__probe {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.62rem;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.85;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ---------- 主题分段选择 ---------- */
.seg {
  align-self: flex-start;
  margin-bottom: 10px;
  flex-wrap: wrap;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.seg__item {
  text-transform: none;
  letter-spacing: 0;
  font-size: 0.74rem;
  font-weight: 600;
  min-width: 0;
}

/* ---------- 行 ---------- */
.rows {
  display: flex;
  flex-direction: column;
}

.row {
  display: flex;
  align-items: center;
  gap: 9px;
  min-height: 40px;
  padding: 5px 2px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.row:last-child {
  border-bottom: none;
}

.row--click {
  cursor: pointer;
  border-radius: 8px;
  transition: background-color 0.14s ease;
}

.row--click:hover {
  background: rgba(var(--v-theme-on-surface), 0.04);
}

/* 单选图标：保持与原 radio 一致的视觉尺寸 */
.radio {
  flex-shrink: 0;
  opacity: 1;
}

.row__label {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  line-height: 1.25;
}

.row__title {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.79rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row__sub {
  margin-top: 1px;
  font-size: 0.65rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row__side {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 5px;
}

/* 行内开关：遵循 MD3 尺寸（track 32dp / thumb 20dp），
   仅收紧上下留白，比 MD3 略紧凑，但不压扁控件本体 */
.row__switch {
  flex-shrink: 0;
  margin: 0;
}

.row__switch :deep(.v-selection-control) {
  min-height: 40px;
}

.row__switch :deep(.v-switch__track) {
  height: 32px;
  min-height: 32px;
  width: 52px;
}

.row__switch :deep(.v-switch__thumb) {
  height: 24px;
  width: 24px;
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

/* ---------- 按钮组 ---------- */
.btnrow {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}

/* 主题样式里的 tonal 按钮：去掉 v-btn 默认的 letter-spacing / 大写 */
.pillbtn {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

.self-start {
  align-self: flex-start;
  margin-top: 10px;
}

.addbtn {
  align-self: flex-start;
  margin-top: 8px;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

/* ---------- 生成式输入框（等宽） ---------- */
.mono-input :deep(input),
.mono-input :deep(textarea) {
  font-family: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace;
  font-size: 0.78rem;
}

/* ---------- 弹窗 ---------- */
.dlgbody {
  padding: 16px 16px 6px;
}

.dlgbody--tight {
  padding-bottom: 2px;
}

.dlgtitle {
  font-size: 0.92rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
  margin-bottom: 11px;
}

.dlghint {
  margin-top: 8px;
  font-size: 0.66rem;
  line-height: 1.55;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.9;
}

.dlgactions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 7px;
  padding: 10px 16px 15px;
}

/* ---------- 页脚 ---------- */
.stfoot {
  text-align: center;
  padding: 6px 0 2px;
  font-size: 0.66rem;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.75;
}

@media (min-width: 760px) {
  .stfoot {
    grid-column: 1 / -1;
  }
}
</style>
