<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { settings } from '../stores/settings'
import { useSitesStore } from '../stores/sites'
import { invoke, isTauri } from '../lib/tauri'
import { speedTestMirror, MIRROR_PROBE_URL } from '../lib/mirrors'
import { checkUpdate, updateChecking } from '../lib/update'
import type { GithubMirror, MirrorSpeedResult } from '../types'

const sitesStore = useSitesStore()
const themeMode = ref<'system' | 'light' | 'dark'>('system')
const autoCheck = ref(true)
const dnsServer = ref('')
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
async function saveDns(v: string) {
  dnsServer.value = v.trim()
  await settings.set('dnsServer', dnsServer.value)
}

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
        // 单项完成立即更新对应行
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
    // 重新同步页面状态
    themeMode.value = settings.get('theme')
    autoCheck.value = settings.get('autoCheckUpdate')
    dnsServer.value = settings.get('dnsServer')
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
  await refreshMirrors()
  await sitesStore.load().catch(() => undefined)
})
</script>

<template>
  <div>
    <!-- 设置页独立框架：返回顶栏 + 内容 -->
    <v-app-bar color="surface" border="b" height="52">
      <template #prepend>
        <v-btn icon="mdi-arrow-left" variant="text" title="返回" @click="$router.push('/')" />
      </template>
      <v-app-bar-title>
        <span class="text-subtitle-1 font-weight-bold">设置</span>
      </v-app-bar-title>
    </v-app-bar>

    <v-main>
      <div class="px-3 px-sm-6 py-3 mx-auto" style="max-width: 1040px">
        <!-- 通用：外观 + 自动检测更新 -->
        <v-card class="mb-3" rounded="lg">
          <v-card-item>
            <template #prepend>
              <v-avatar color="primary-container" variant="flat" rounded="lg">
                <v-icon icon="mdi-cog-outline" color="on-primary-container" />
              </v-avatar>
            </template>
            <v-card-title class="text-subtitle-1 font-weight-bold">通用</v-card-title>
          </v-card-item>
          <v-card-text>
            <div class="text-subtitle-2 font-weight-bold mb-1">外观</div>
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
              variant="outlined"
              density="compact"
              hide-details
              class="mb-1"
              @update:model-value="saveTheme($event)"
            />
            <v-divider class="my-2" />
            <div class="d-flex align-center">
              <div class="mr-auto">
                <div class="text-subtitle-2 font-weight-bold">自动检测更新</div>
                <div class="text-caption text-medium-emphasis">启动时静默检查 GitHub Release，发现新版本后通过提示条提醒</div>
              </div>
              <v-switch :model-value="autoCheck" color="primary" hide-details @update:model-value="saveAutoCheck(!!$event)" />
            </div>
            <v-divider class="my-2" />
            <div class="text-subtitle-2 font-weight-bold mb-1">自定义 DNS</div>
            <v-text-field
              :model-value="dnsServer"
              label="DNS 服务器（搜索请求使用）"
              placeholder="223.5.5.5 / 8.8.8.8"
              variant="outlined"
              density="compact"
              hide-details
              hint="留空使用系统默认；可解决个别搜索源域名解析失败问题"
              persistent-hint
              @update:model-value="saveDns($event)"
            />
            <v-btn
              color="primary"
              variant="tonal"
              prepend-icon="mdi-update"
              size="small"
              class="mt-3"
              :loading="updateChecking"
              @click="checkUpdate"
            >
              立即检查更新
            </v-btn>
          </v-card-text>
        </v-card>

        <!-- GitHub 镜像 -->
        <v-card class="mb-3" rounded="lg">
          <v-card-item>
            <template #prepend>
              <v-avatar color="secondary-container" variant="flat" rounded="lg">
                <v-icon icon="mdi-cloud-sync-outline" color="on-secondary-container" />
              </v-avatar>
            </template>
            <v-card-title class="text-subtitle-1 font-weight-bold">GitHub 镜像</v-card-title>
            <template #append>
              <v-btn
                color="primary"
                variant="tonal"
                size="small"
                prepend-icon="mdi-speedometer"
                :loading="testing"
                @click="runSpeedTest"
              >
                全部测速
              </v-btn>
            </template>
          </v-card-item>

          <!-- 镜像列表：仅展示链接；直连行保留「直连」标签，其余不加标签 -->
          <v-list density="compact" class="px-2 pb-2">
            <v-list-item
              v-for="row in mirrorRows"
              :key="row.id"
              :active="selectedMirrorId === row.id"
              rounded="xl"
              class="mb-1"
              @click="selectMirror(row)"
            >
              <template #prepend>
                <v-icon
                  :icon="selectedMirrorId === row.id ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank'"
                  :color="selectedMirrorId === row.id ? 'primary' : 'grey'"
                  size="20"
                />
              </template>
              <v-list-item-title class="text-body-2 font-weight-medium font-family-monospace">
                {{ mirrorLabel(row) }}
                <v-chip v-if="row.id === 'direct'" size="x-small" color="primary" variant="tonal" class="ml-1">
                  直连
                </v-chip>
              </v-list-item-title>
              <template #append>
                <div class="d-flex align-center ga-2">
                  <v-progress-circular
                    v-if="row.testing"
                    indeterminate
                    size="18"
                    width="2"
                    color="primary"
                    class="mr-1"
                  />
                  <v-chip
                    v-else-if="row.speedChip"
                    size="x-small"
                    :color="row.speedChip.color"
                    :variant="row.speedChip.icon === 'mdi-close' ? 'tonal' : 'flat'"
                  >
                    <v-icon :icon="row.speedChip.icon" size="13" class="mr-1" />
                    {{ row.speedChip.text }}
                  </v-chip>
                  <v-btn
                    v-if="!row.builtin"
                    icon="mdi-delete-outline"
                    size="x-small"
                    variant="text"
                    color="error"
                    title="删除镜像"
                    @click.stop="removeMirror(row)"
                  />
                </div>
              </template>
            </v-list-item>
          </v-list>

          <v-card-text class="pt-0">
            <v-btn variant="text" color="secondary" size="small" prepend-icon="mdi-plus" @click="openAddMirror">
              添加自定义镜像
            </v-btn>
            <div class="text-caption text-medium-emphasis mt-1">测速探针：{{ MIRROR_PROBE_URL }}</div>
          </v-card-text>
        </v-card>

        <!-- 数据管理 -->
        <v-card class="mb-3" rounded="lg">
          <v-card-item>
            <template #prepend>
              <v-avatar color="error-container" variant="flat" rounded="lg">
                <v-icon icon="mdi-database-cog-outline" color="on-error-container" />
              </v-avatar>
            </template>
            <v-card-title class="text-subtitle-1 font-weight-bold">数据管理</v-card-title>
          </v-card-item>
          <v-card-text>
            <div class="text-subtitle-2 font-weight-bold mb-1">设置备份 / 恢复</div>
            <div class="d-flex flex-wrap ga-2 mb-4">
              <v-btn variant="tonal" color="primary" prepend-icon="mdi-export" @click="doExportSettings">
                导出设置（JSON）
              </v-btn>
              <v-btn variant="tonal" color="primary" prepend-icon="mdi-import" :loading="importing" @click="doImportSettings">
                导入设置
              </v-btn>
            </div>
            <v-divider class="mb-3" />
            <div class="text-subtitle-2 font-weight-bold mb-1">数据清理</div>
            <div class="d-flex flex-wrap ga-2">
              <v-btn variant="tonal" color="error" prepend-icon="mdi-delete-sweep-outline" :loading="clearing" @click="clearHistory">
                清空全部历史记录
              </v-btn>
              <v-btn variant="tonal" color="error" prepend-icon="mdi-history" @click="clearSearchHistory">
                清空搜索历史
              </v-btn>
              <v-btn variant="tonal" color="warning" prepend-icon="mdi-antenna-off" @click="sitesStore.reset(); notice('站点已重置')">
                重置站点订阅
              </v-btn>
            </div>
          </v-card-text>
        </v-card>

        <!-- 关于 -->
        <v-card class="mb-3" rounded="lg" @click="$router.push('/about')">
          <v-card-item>
            <template #prepend>
              <v-avatar color="success-container" variant="flat" rounded="lg">
                <v-icon icon="mdi-information-outline" color="on-success-container" />
              </v-avatar>
            </template>
            <v-card-title class="text-subtitle-1 font-weight-bold">关于</v-card-title>
            <v-card-subtitle class="text-caption">版本信息与项目主页</v-card-subtitle>
            <template #append>
              <v-icon icon="mdi-chevron-right" color="grey" />
            </template>
          </v-card-item>
        </v-card>

        <!-- 添加自定义镜像弹窗（仅需填写链接，名称自动生成） -->
        <v-dialog v-model="addMirrorDialog" max-width="480">
          <v-card rounded="lg">
            <v-card-title class="text-subtitle-1 font-weight-bold">添加自定义 GitHub 镜像</v-card-title>
            <v-divider />
            <v-card-text>
              <v-text-field
                v-model="newMirrorBase"
                label="镜像前缀地址 *（https:// 开头）"
                hide-details
                placeholder="https://ghproxy.net/"
              />
              <div class="text-caption text-medium-emphasis mt-2">
                前缀代理模式：请求 GitHub 原始地址时自动拼接该前缀，例如
                <span class="font-family-monospace">{{ newMirrorBase || 'https://镜像地址/' }}https://api.github.com/…</span>
              </div>
            </v-card-text>
            <v-card-actions>
              <v-spacer />
              <v-btn variant="text" @click="addMirrorDialog = false">取消</v-btn>
              <v-btn
                color="primary"
                variant="flat"
                :disabled="!newMirrorBase.trim()"
                @click="addMirror"
              >
                保存
              </v-btn>
            </v-card-actions>
          </v-card>
        </v-dialog>

        <v-snackbar v-model="showToast" location="bottom" color="success" timeout="2200">
          {{ toast }}
        </v-snackbar>
      </div>
    </v-main>
  </div>
</template>
