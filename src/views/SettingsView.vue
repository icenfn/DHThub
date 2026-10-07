<script setup lang="ts">
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
const dnsServers = ref<DnsServer[]>([])
const selectedDnsId = ref('')
const dnsSpeeds = ref<Record<string, DnsSpeedResult>>({})
const dnsTesting = ref(false)
const dnsTestingIds = ref<Set<string>>(new Set())
const addDnsDialog = ref(false)
const newDnsBase = ref('')

interface DnsRow extends DnsServer {
  speedChip: { color: string; icon: string; text: string } | null
  testing: boolean
}

const dnsRows = computed<DnsRow[]>(() =>
  dnsServers.value.map((d) => {
    const r = dnsSpeeds.value[d.id]
    let chip: DnsRow['speedChip'] = null
    if (r) {
      chip = r.error
        ? { color: 'error', icon: 'mdi-close', text: '失败' }
        : {
            color: (r.latency ?? 0) < 200 ? 'success' : 'warning',
            icon: 'mdi-speedometer',
            text: `${r.latency}ms`,
          }
    }
    return { ...d, speedChip: chip, testing: dnsTestingIds.value.has(d.id) }
  }),
)
const toast = ref('')
const showToast = ref(false)
const clearing = ref(false)
const importing = ref(false)

// ---------- GitHub 镜像 ----------
const mirrors = ref<GithubMirror[]>([])
const selectedMirrorId = ref('direct')
const speeds = ref<Record<string, MirrorSpeedResult>>({})
const testing = ref(false)
const testingIds = ref<Set<string>>(new Set())
const addMirrorDialog = ref(false)
const newMirrorBase = ref('')

// 弹层栈注册：手机返回键 / PC ESC 关闭
watch(addDnsDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { addDnsDialog.value = false }))
})
watch(addMirrorDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { addMirrorDialog.value = false }))
})

/** 镜像在列表中的展示文本：只显示链接；直连显示官方地址 */
function mirrorLabel(m: GithubMirror): string {
  return m.base || 'https://github.com'
}

interface MirrorRow extends GithubMirror {
  speedChip: { color: string; icon: string; text: string } | null
  testing: boolean
}

const mirrorRows = computed<MirrorRow[]>(() =>
  mirrors.value.map((m) => {
    const r = speeds.value[m.id]
    let chip: MirrorRow['speedChip'] = null
    if (r) {
      chip = r.error
        ? { color: 'error', icon: 'mdi-close', text: '失败' }
        : {
            color: (r.latency ?? 0) < 800 ? 'success' : 'warning',
            icon: 'mdi-speedometer',
            text: `${r.latency}ms`,
          }
    }
    return { ...m, speedChip: chip, testing: testingIds.value.has(m.id) }
  }),
)

function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

// ---------- 外观 ----------
async function saveTheme(v: 'system' | 'light' | 'dark') {
  themeMode.value = v
  await settings.set('theme', v)
}

// ---------- 更新 ----------
async function saveAutoCheck(v: boolean) {
  autoCheck.value = v
  await settings.set('autoCheckUpdate', v)
  notice(v ? '已开启自动检测更新（启动时检查）' : '已关闭自动检测更新')
}

// ---------- 数据 ----------
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
    const list = dnsServers.value.map((d) => {
      const ms = map.get(d.base)
      return {
        id: d.id,
        name: d.name,
        base: d.base,
        latency: ms ?? null,
        error: ms == null ? '测速失败' : null,
      } as DnsSpeedResult
    })
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

// ---------- GitHub 镜像 ----------
async function refreshMirrors() {
  mirrors.value = settings.getMirrors()
  selectedMirrorId.value = settings.get('githubMirrorId')
}

async function selectMirror(m: GithubMirror) {
  selectedMirrorId.value = m.id
  await settings.selectMirror(m.id)
  notice(`已切换：${mirrorLabel(m)}`)
}

/** 并发测速全部镜像，单项完成立即刷新延迟显示（单项独立超时，永不挂起） */
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

// ---------- 设置导出 / 导入 ----------
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
    if (!path) return
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
  await refreshDns()
  await refreshMirrors()
  await sitesStore.load().catch(() => undefined)
})
</script>

<template>
  <div class="settings-page">
    <!-- 设置页独立框架：返回顶栏 + 内容 -->
    <header class="settings-topbar">
      <v-btn icon="mdi-arrow-left" variant="text" size="small" density="comfortable" title="返回" @click="$router.push('/')" />
      <span class="text-title-medium font-weight-bold">设置</span>
    </header>

    <main class="settings-main">
      <!-- 通用：外观 + 自动检测更新（信息密集型：单行并排） -->
      <section class="settings-card">
        <div class="settings-card__head">
          <v-icon icon="mdi-cog-outline" size="18" color="primary" />
          <span class="text-title-small font-weight-bold">通用</span>
        </div>
        <div class="settings-card__body">
          <div class="dense-grid">
            <v-select
              :model-value="themeMode"
              :items="[
                { title: '跟随系统', value: 'system' },
                { title: '浅色', value: 'light' },
                { title: '深色', value: 'dark' },
              ]"
              item-title="title"
              item-value="value"
              label="主题模式"
              density="compact"
              variant="outlined"
              hide-details
              @update:model-value="saveTheme($event)"
            />
            <div class="dense-row">
              <div class="dense-row__label">
                <span class="text-body-small font-weight-medium">自动检测更新</span>
                <span class="text-label-small text-medium-emphasis">启动时静默检查 GitHub Release</span>
              </div>
              <v-switch
                :model-value="autoCheck"
                color="primary"
                hide-details
                density="compact"
                class="flex-shrink-0"
                @update:model-value="saveAutoCheck(!!$event)"
              />
            </div>
            <v-btn
              color="primary"
              variant="tonal"
              prepend-icon="mdi-update"
              size="small"
              rounded="pill"
              density="comfortable"
              :loading="updateChecking"
              class="align-self-start"
              @click="checkUpdate"
            >
              立即检查更新
            </v-btn>
          </div>
        </div>
      </section>

      <!-- DNS 服务器（紧凑列表） -->
      <section class="settings-card">
        <div class="settings-card__head">
          <v-icon icon="mdi-server-network" size="18" color="primary" />
          <span class="text-title-small font-weight-bold">DNS 服务器</span>
          <span class="text-label-small text-medium-emphasis flex-grow-1 text-truncate">
            搜索请求域名解析 · 内置 AliDNS / DNSPod / Cloudflare / 114DNS
          </span>
          <v-btn
            color="primary"
            variant="tonal"
            size="x-small"
            rounded="pill"
            prepend-icon="mdi-speedometer"
            :loading="dnsTesting"
            @click="runDnsSpeedTest"
          >
            全部测速
          </v-btn>
        </div>
        <div class="settings-card__body pt-0">
          <v-list density="compact" class="px-0 bg-transparent py-0">
            <v-list-item
              :active="selectedDnsId === ''"
              rounded="lg"
              density="compact"
              @click="selectDns({ id: '', name: '系统默认', base: '' })"
            >
              <template #prepend>
                <v-icon
                  :icon="selectedDnsId === '' ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
                  :color="selectedDnsId === '' ? 'primary' : 'grey'"
                  size="16"
                />
              </template>
              <v-list-item-title class="text-body-small">
                系统默认
                <v-chip v-if="selectedDnsId === ''" size="x-small" color="primary" variant="tonal" class="ml-1">当前</v-chip>
              </v-list-item-title>
              <v-list-item-subtitle class="text-label-small mono">使用系统 DNS</v-list-item-subtitle>
            </v-list-item>
            <v-list-item
              v-for="row in dnsRows"
              :key="row.id"
              :active="selectedDnsId === row.id"
              rounded="lg"
              density="compact"
              @click="selectDns(row)"
            >
              <template #prepend>
                <v-icon
                  :icon="selectedDnsId === row.id ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
                  :color="selectedDnsId === row.id ? 'primary' : 'grey'"
                  size="16"
                />
              </template>
              <v-list-item-title class="text-body-small">
                {{ row.name }}
                <v-chip v-if="row.builtin" size="x-small" color="primary" variant="tonal" class="ml-1">内置</v-chip>
              </v-list-item-title>
              <v-list-item-subtitle class="text-label-small mono">{{ row.base }}</v-list-item-subtitle>
              <template #append>
                <div class="d-flex align-center ga-1">
                  <v-progress-circular v-if="row.testing" indeterminate size="14" width="2" color="primary" />
                  <v-chip
                    v-else-if="row.speedChip"
                    size="x-small"
                    :color="row.speedChip.color"
                    :variant="row.speedChip.icon === 'mdi-close' ? 'tonal' : 'flat'"
                  >
                    {{ row.speedChip.text }}
                  </v-chip>
                  <v-btn
                    v-if="!row.builtin"
                    icon="mdi-delete-outline"
                    size="x-small"
                    variant="text"
                    color="error"
                    title="删除"
                    density="comfortable"
                    @click.stop="removeDns(row)"
                  />
                </div>
              </template>
            </v-list-item>
          </v-list>
          <v-btn variant="text" color="secondary" size="x-small" rounded="pill" prepend-icon="mdi-plus" @click="openAddDns">
            添加自定义 DNS
          </v-btn>
        </div>
      </section>

      <!-- GitHub 镜像（紧凑列表） -->
      <section class="settings-card">
        <div class="settings-card__head">
          <v-icon icon="mdi-cloud-sync-outline" size="18" color="primary" />
          <span class="text-title-small font-weight-bold">GitHub 镜像</span>
          <span class="text-label-small text-medium-emphasis flex-grow-1 text-truncate">拉取订阅与检测更新的加速通道</span>
          <v-btn
            color="primary"
            variant="tonal"
            size="x-small"
            rounded="pill"
            prepend-icon="mdi-speedometer"
            :loading="testing"
            @click="runSpeedTest"
          >
            全部测速
          </v-btn>
        </div>
        <div class="settings-card__body pt-0">
          <v-list density="compact" class="px-0 bg-transparent py-0">
            <v-list-item
              v-for="row in mirrorRows"
              :key="row.id"
              :active="selectedMirrorId === row.id"
              rounded="lg"
              density="compact"
              @click="selectMirror(row)"
            >
              <template #prepend>
                <v-icon
                  :icon="selectedMirrorId === row.id ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
                  :color="selectedMirrorId === row.id ? 'primary' : 'grey'"
                  size="16"
                />
              </template>
              <v-list-item-title class="text-body-small mono">
                {{ mirrorLabel(row) }}
                <v-chip v-if="row.id === 'direct'" size="x-small" color="primary" variant="tonal" class="ml-1">直连</v-chip>
              </v-list-item-title>
              <template #append>
                <div class="d-flex align-center ga-1">
                  <v-progress-circular v-if="row.testing" indeterminate size="14" width="2" color="primary" />
                  <v-chip
                    v-else-if="row.speedChip"
                    size="x-small"
                    :color="row.speedChip.color"
                    :variant="row.speedChip.icon === 'mdi-close' ? 'tonal' : 'flat'"
                  >
                    {{ row.speedChip.text }}
                  </v-chip>
                  <v-btn
                    v-if="!row.builtin"
                    icon="mdi-delete-outline"
                    size="x-small"
                    variant="text"
                    color="error"
                    title="删除镜像"
                    density="comfortable"
                    @click.stop="removeMirror(row)"
                  />
                </div>
              </template>
            </v-list-item>
          </v-list>
          <div class="d-flex align-center flex-wrap ga-2">
            <v-btn variant="text" color="secondary" size="x-small" rounded="pill" prepend-icon="mdi-plus" @click="openAddMirror">
              添加自定义镜像
            </v-btn>
            <span class="text-label-small text-medium-emphasis">测速探针：{{ MIRROR_PROBE_URL }}</span>
          </div>
        </div>
      </section>

      <!-- 数据管理（信息密集型：单行操作条） -->
      <section class="settings-card">
        <div class="settings-card__head">
          <v-icon icon="mdi-database-cog-outline" size="18" color="primary" />
          <span class="text-title-small font-weight-bold">数据管理</span>
        </div>
        <div class="settings-card__body">
          <div class="dense-row">
            <span class="text-label-medium text-medium-emphasis dense-row__label-text">备份 / 恢复</span>
            <div class="d-flex flex-wrap ga-2">
              <v-btn variant="tonal" color="primary" size="small" rounded="pill" density="comfortable" prepend-icon="mdi-export" @click="doExportSettings">
                导出设置
              </v-btn>
              <v-btn variant="tonal" color="primary" size="small" rounded="pill" density="comfortable" prepend-icon="mdi-import" :loading="importing" @click="doImportSettings">
                导入设置
              </v-btn>
            </div>
          </div>
          <v-divider class="my-2" />
          <div class="dense-row">
            <span class="text-label-medium text-medium-emphasis dense-row__label-text">数据清理</span>
            <div class="d-flex flex-wrap ga-2">
              <v-btn variant="tonal" color="error" size="small" rounded="pill" density="comfortable" prepend-icon="mdi-delete-sweep-outline" :loading="clearing" @click="clearHistory">
                清空全部历史
              </v-btn>
              <v-btn variant="tonal" color="error" size="small" rounded="pill" density="comfortable" prepend-icon="mdi-history" @click="clearSearchHistory">
                清空搜索历史
              </v-btn>
              <v-btn
                variant="tonal"
                color="warning"
                size="small"
                rounded="pill"
                density="comfortable"
                prepend-icon="mdi-antenna-off"
                @click="sitesStore.reset(); notice('站点已重置')"
              >
                重置站点订阅
              </v-btn>
            </div>
          </div>
        </div>
      </section>

      <!-- 添加自定义 DNS 弹窗 -->
      <v-dialog v-model="addDnsDialog" max-width="420">
        <v-card rounded="xl" variant="flat" color="surface-container-low">
          <v-card-title class="text-title-small font-weight-bold pa-4 pb-0">添加自定义 DNS</v-card-title>
          <v-card-text class="pt-3">
            <v-text-field
              v-model="newDnsBase"
              label="DNS 服务器地址 *（IP 或 IP:端口）"
              density="compact"
              hide-details
              placeholder="223.6.6.6"
            />
          </v-card-text>
          <v-card-actions class="px-4 pb-3">
            <v-spacer />
            <v-btn variant="text" size="small" @click="addDnsDialog = false">取消</v-btn>
            <v-btn color="primary" variant="flat" size="small" :disabled="!newDnsBase.trim()" @click="addDns">保存</v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <!-- 添加自定义镜像弹窗 -->
      <v-dialog v-model="addMirrorDialog" max-width="480">
        <v-card rounded="xl" variant="flat" color="surface-container-low">
          <v-card-title class="text-title-small font-weight-bold pa-4 pb-0">添加自定义 GitHub 镜像</v-card-title>
          <v-card-text class="pt-3">
            <v-text-field
              v-model="newMirrorBase"
              label="镜像前缀地址 *（https:// 开头）"
              density="compact"
              hide-details
              placeholder="https://ghproxy.net/"
            />
            <div class="text-label-small text-medium-emphasis mt-2">
              前缀代理模式：请求 GitHub 原始地址时自动拼接该前缀，例如
              <span class="mono">{{ newMirrorBase || 'https://镜像地址/' }}https://api.github.com/…</span>
            </div>
          </v-card-text>
          <v-card-actions class="px-4 pb-3">
            <v-spacer />
            <v-btn variant="text" size="small" @click="addMirrorDialog = false">取消</v-btn>
            <v-btn color="primary" variant="flat" size="small" :disabled="!newMirrorBase.trim()" @click="addMirror">保存</v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <v-snackbar v-model="showToast" location="bottom" color="inverse-surface" rounded="lg" timeout="2200">
        {{ toast }}
      </v-snackbar>
    </main>
  </div>
</template>

<style scoped>
.settings-page {
  min-height: 100dvh;
  background: rgb(var(--v-theme-surface));
  display: flex;
  flex-direction: column;
}

.settings-topbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  background: rgb(var(--v-theme-surface-container-low));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  position: sticky;
  top: 0;
  z-index: 10;
}

/* 信息密集型：更小间距、更宽内容区、多列布局 */
.settings-main {
  flex: 1;
  width: 100%;
  max-width: 1080px;
  margin: 0 auto;
  padding: 10px 14px 28px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.settings-card {
  background: rgb(var(--v-theme-surface-container-low));
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 16px;
  overflow: hidden;
}

.settings-card__head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px 6px;
}

.settings-card__body {
  padding: 8px 14px 12px;
}

/* 密集型网格：桌面两列，移动单列 */
.dense-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 8px;
}
@media (min-width: 600px) {
  .dense-grid {
    grid-template-columns: 1fr 1fr;
    align-items: center;
  }
}

.dense-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 36px;
}

.dense-row__label {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.dense-row__label-text {
  flex-shrink: 0;
  min-width: 72px;
}

.mono {
  font-family: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', Menlo, monospace;
}
</style>
