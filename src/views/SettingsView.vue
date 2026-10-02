<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { settings } from '../stores/settings'
import { useSitesStore } from '../stores/sites'
import { invoke, isTauri } from '../lib/tauri'
import { speedTestMirror, pickFastest, MIRROR_PROBE_URL } from '../lib/mirrors'
import { debugLog } from '../lib/debug'
import UpdateDialog from '../components/UpdateDialog.vue'
import type { GithubMirror, MirrorSpeedResult } from '../types'

const sitesStore = useSitesStore()
const themeMode = ref<'system' | 'light' | 'dark'>('system')
const autoCheck = ref(true)
const debugOn = ref(false)
const toast = ref('')
const showToast = ref(false)
const updateOpen = ref(false)
const clearing = ref(false)
const importing = ref(false)

// ---------- GitHub 镜像 ----------
const mirrors = ref<GithubMirror[]>([])
const mirrorMode = ref<'manual' | 'auto'>('manual')
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
  error: string | null
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
    return { ...m, speedChip: chip, testing: testingIds.value.has(m.id), error: r?.error ?? null }
  }),
)

const selectedMirror = computed(() => mirrors.value.find((m) => m.id === selectedMirrorId.value))
const currentLatency = computed<number | null>(() => {
  const r = selectedMirror.value ? speeds.value[selectedMirror.value.id] : undefined
  if (!r || r.error) return null
  return r.latency ?? null
})

function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

// ---------- 外观 ----------
async function saveTheme(v: 'system' | 'light' | 'dark') {
  themeMode.value = v
  await settings.set('theme', v)
}

// ---------- 更新 / 调试 ----------
async function saveAutoCheck(v: boolean) {
  autoCheck.value = v
  await settings.set('autoCheckUpdate', v)
  notice(v ? '已开启自动检测更新（启动时静默检查）' : '已关闭自动检测更新')
}

async function saveDebug(v: boolean) {
  debugOn.value = v
  await settings.set('debug', v)
  notice(v ? '已开启调试模式' : '已关闭调试模式')
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
  mirrorMode.value = settings.get('githubMirrorMode')
  selectedMirrorId.value = settings.get('githubMirrorId')
}

async function onMirrorModeChange(v: 'manual' | 'auto') {
  mirrorMode.value = v
  await settings.setMirrorMode(v)
  if (v === 'auto') {
    notice('正在测速所有镜像并自动选择最快…')
    await runSpeedTest(true)
  } else {
    notice('已切换为手动选择')
  }
}

async function selectMirror(m: GithubMirror) {
  if (mirrorMode.value !== 'manual') return
  selectedMirrorId.value = m.id
  await settings.selectMirror(m.id)
  notice(`已切换：${mirrorLabel(m)}`)
}

/** 并发测速全部镜像，单项完成立即刷新延迟显示（单项独立超时，永不挂起） */
async function runSpeedTest(autoPick = false) {
  if (testing.value) return
  testing.value = true
  speeds.value = {}
  testingIds.value = new Set(mirrors.value.map((m) => m.id))
  try {
    const results = await Promise.all(
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
    debugLog('[测速] 全部完成：', list)
    if (autoPick || mirrorMode.value === 'auto') {
      const best = pickFastest(list)
      if (best) {
        selectedMirrorId.value = best.id
        await settings.selectMirror(best.id)
        const bm = mirrors.value.find((m) => m.id === best.id)
        notice(`已自动选择最快镜像：${mirrorLabel(bm ?? best)}（${best.latency}ms）`)
      } else {
        notice('测速完成，但所有镜像均不可用')
      }
    } else {
      const ok = list.filter((r) => r.latency != null).length
      notice(`测速完成：${ok}/${list.length} 个镜像可用`)
    }
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
    debugOn.value = settings.get('debug')
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
  debugOn.value = settings.get('debug')
  await refreshMirrors()
  await sitesStore.load().catch(() => undefined)
})

const subscribeUrl = computed(() => settings.get('subscribeUrl'))
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 1040px">
    <!-- 页头 -->
    <div class="d-flex align-center mt-2 mb-4 flex-wrap ga-2">
      <div class="mr-auto">
        <div class="text-h6 font-weight-bold">设置</div>
        <div class="text-caption text-medium-emphasis">外观、GitHub 镜像与数据管理</div>
      </div>
      <v-btn
        color="primary"
        variant="tonal"
        prepend-icon="mdi-import"
        size="small"
        :loading="importing"
        @click="doImportSettings"
      >
        导入
      </v-btn>
      <v-btn color="primary" variant="tonal" prepend-icon="mdi-export" size="small" @click="doExportSettings">
        导出
      </v-btn>
      <v-btn color="primary" variant="flat" prepend-icon="mdi-update" size="small" @click="updateOpen = true">
        检查更新
      </v-btn>
    </div>

    <!-- 外观 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="primary-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-theme-light-dark" color="on-primary-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">外观</v-card-title>
        <v-card-subtitle class="text-caption">主题模式，跟随系统可自动适配深色</v-card-subtitle>
      </v-card-item>
      <v-card-text>
        <v-radio-group v-model="themeMode" inline hide-details @update:model-value="saveTheme(themeMode)">
          <v-radio label="跟随系统" value="system" color="primary" />
          <v-radio label="浅色" value="light" color="primary" />
          <v-radio label="深色" value="dark" color="primary" />
        </v-radio-group>
      </v-card-text>
    </v-card>

    <!-- 更新 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="info-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-update" color="on-info-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">更新</v-card-title>
        <v-card-subtitle class="text-caption">启动时静默检查 GitHub Release，发现新版本自动弹窗提醒</v-card-subtitle>
        <template #append>
          <v-switch :model-value="autoCheck" color="primary" hide-details @update:model-value="saveAutoCheck(!!$event)" />
        </template>
      </v-card-item>
    </v-card>

    <!-- GitHub 镜像 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="secondary-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-cloud-sync-outline" color="on-secondary-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">GitHub 镜像</v-card-title>
        <v-card-subtitle class="text-caption">
          用于检查更新、拉取订阅源等 GitHub 请求；内置 2 个，可自定义添加
        </v-card-subtitle>
        <template #append>
          <v-btn
            color="primary"
            variant="tonal"
            size="small"
            prepend-icon="mdi-speedometer"
            :loading="testing"
            @click="runSpeedTest(false)"
          >
            全部测速
          </v-btn>
        </template>
      </v-card-item>

      <!-- 选择模式 -->
      <v-card-text class="pt-0">
        <div class="d-flex align-center ga-2 flex-wrap">
          <span class="text-subtitle-2 text-medium-emphasis">选择方式</span>
          <v-chip
            :variant="mirrorMode === 'manual' ? 'flat' : 'outlined'"
            color="primary"
            size="small"
            @click="onMirrorModeChange('manual')"
          >
            手动选择
          </v-chip>
          <v-chip
            :variant="mirrorMode === 'auto' ? 'flat' : 'outlined'"
            color="primary"
            size="small"
            @click="onMirrorModeChange('auto')"
          >
            自动选最快
          </v-chip>
          <v-spacer />
          <span v-if="selectedMirror" class="text-caption text-medium-emphasis">
            当前：<strong class="text-primary">{{ mirrorLabel(selectedMirror) }}</strong>
            <span v-if="currentLatency != null" class="ml-1">· {{ currentLatency }}ms</span>
          </span>
        </div>
      </v-card-text>

      <!-- 镜像列表：仅展示链接；直连行保留「直连」标签，其余不加标签 -->
      <v-list density="compact" class="px-2 pb-2">
        <v-list-item
          v-for="row in mirrorRows"
          :key="row.id"
          :active="mirrorMode === 'manual' && selectedMirrorId === row.id"
          :disabled="mirrorMode === 'auto'"
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
          <v-list-item-subtitle v-if="debugOn && row.error" class="text-caption text-error font-family-monospace">
            {{ row.error }}
          </v-list-item-subtitle>
        </v-list-item>
      </v-list>

      <v-card-text class="pt-0">
        <v-btn variant="text" color="secondary" size="small" prepend-icon="mdi-plus" @click="openAddMirror">
          添加自定义镜像
        </v-btn>
        <div class="text-caption text-medium-emphasis mt-1">测速探针：{{ MIRROR_PROBE_URL }}</div>
      </v-card-text>
    </v-card>

    <!-- 高级 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="warning-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-bug-outline" color="on-warning-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">高级</v-card-title>
        <v-card-subtitle class="text-caption">调试模式：输出详细运行日志，并在测速列表显示失败原因</v-card-subtitle>
        <template #append>
          <v-switch :model-value="debugOn" color="primary" hide-details @update:model-value="saveDebug(!!$event)" />
        </template>
      </v-card-item>
    </v-card>

    <!-- 数据管理 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="error-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-database-cog-outline" color="on-error-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">数据管理</v-card-title>
        <v-card-subtitle class="text-caption">设置备份与恢复、历史与订阅数据清理</v-card-subtitle>
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

    <!-- 订阅源 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="info-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-link-variant" color="on-info-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">订阅源</v-card-title>
        <v-card-subtitle class="text-caption">
          当前订阅仓库地址（可在「站点管理」中修改；拉取时自动套用选中镜像）
        </v-card-subtitle>
      </v-card-item>
      <v-card-text class="text-body-2 font-family-monospace text-caption">
        {{ subscribeUrl || '未设置' }}
      </v-card-text>
    </v-card>

    <!-- 关于与法律 -->
    <v-card>
      <v-card-item>
        <template #prepend>
          <v-avatar color="success-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-information-outline" color="on-success-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">关于与法律</v-card-title>
      </v-card-item>
      <v-list density="compact">
        <v-list-item prepend-icon="mdi-shield-check-outline" title="使用协议" @click="$router.push('/agreement')" />
        <v-list-item prepend-icon="mdi-file-document-outline" title="免责声明" @click="$router.push('/disclaimer')" />
        <v-list-item prepend-icon="mdi-github" title="GitHub 仓库" subtitle="github.com/icenfn/DHThub" @click="$router.push('/about')" />
      </v-list>
    </v-card>

    <!-- 添加自定义镜像弹窗（仅需填写链接，名称自动生成） -->
    <v-dialog v-model="addMirrorDialog" max-width="480">
      <v-card>
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

    <UpdateDialog v-model="updateOpen" />
    <v-snackbar v-model="showToast" location="bottom" color="success" timeout="2200">
      {{ toast }}
    </v-snackbar>
  </div>
</template>
