<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { settings } from '../stores/settings'
import { useSitesStore } from '../stores/sites'
import { invoke, isTauri } from '../lib/tauri'
import { speedTestAll, pickFastest, MIRROR_PROBE_URL } from '../lib/mirrors'
import UpdateDialog from '../components/UpdateDialog.vue'
import type { GithubMirror, MirrorSpeedResult } from '../types'

const sitesStore = useSitesStore()
const themeMode = ref<'system' | 'light' | 'dark'>('system')
const hotWordInput = ref('')
const hotWords = ref<string[]>([])
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
const addMirrorDialog = ref(false)
const newMirrorName = ref('')
const newMirrorBase = ref('')

interface MirrorRow extends GithubMirror {
  speedChip: { color: string; icon: string; text: string } | null
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
    return { ...m, speedChip: chip }
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

// ---------- 热词 ----------
async function addHotWord() {
  const w = hotWordInput.value.trim()
  if (!w) return
  const list = [...hotWords.value.filter((x) => x !== w), w]
  hotWords.value = list
  hotWordInput.value = ''
  await settings.set('hotWords', list)
}

async function removeHotWord(w: string) {
  const list = hotWords.value.filter((x) => x !== w)
  hotWords.value = list
  await settings.set('hotWords', list)
}

async function restoreDefaultHotWords() {
  const { DEFAULT_HOT_WORDS } = await import('../stores/settings')
  hotWords.value = [...DEFAULT_HOT_WORDS]
  await settings.set('hotWords', hotWords.value)
  notice('已恢复默认热词')
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
    await runSpeedTest(true)
  } else {
    notice('已切换为手动选择')
  }
}

async function selectMirror(m: GithubMirror) {
  if (mirrorMode.value !== 'manual') return
  selectedMirrorId.value = m.id
  await settings.selectMirror(m.id)
  notice(`已切换：${m.name}`)
}

async function runSpeedTest(autoPick = false) {
  testing.value = true
  try {
    const results = await speedTestAll(mirrors.value)
    const map: Record<string, MirrorSpeedResult> = {}
    results.forEach((r) => (map[r.id] = r))
    speeds.value = map
    if (autoPick || mirrorMode.value === 'auto') {
      const best = pickFastest(results)
      if (best) {
        selectedMirrorId.value = best.id
        await settings.selectMirror(best.id)
        notice(`已自动选择最快镜像：${best.name}（${best.latency}ms）`)
      } else {
        notice('测速完成，但所有镜像均不可用')
      }
    } else {
      notice('测速完成')
    }
  } finally {
    testing.value = false
  }
}

function openAddMirror() {
  newMirrorName.value = ''
  newMirrorBase.value = ''
  addMirrorDialog.value = true
}

async function addMirror() {
  try {
    const m = await settings.addMirror(newMirrorName.value, newMirrorBase.value)
    newMirrorName.value = ''
    newMirrorBase.value = ''
    addMirrorDialog.value = false
    await refreshMirrors()
    notice(`已添加镜像：${m.name}`)
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
    hotWords.value = [...settings.get('hotWords')]
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
  hotWords.value = [...settings.get('hotWords')]
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
        <div class="text-caption text-medium-emphasis">外观、GitHub 镜像、数据与热词管理</div>
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
          用于检查更新、拉取订阅源等 GitHub 请求；内置 3 个，可自定义添加
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
            当前：<strong class="text-primary">{{ selectedMirror.name }}</strong>
            <span v-if="currentLatency != null" class="ml-1">· {{ currentLatency }}ms</span>
          </span>
        </div>
      </v-card-text>

      <!-- 镜像列表 -->
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
          <v-list-item-title class="text-body-2 font-weight-medium">
            {{ row.name }}
            <v-chip v-if="row.builtin" size="x-small" color="primary" variant="tonal" class="ml-1">
              内置
            </v-chip>
          </v-list-item-title>
          <v-list-item-subtitle class="text-caption font-family-monospace">
            {{ row.base || '（官方直连，不加前缀）' }}
          </v-list-item-subtitle>
          <template #append>
            <div class="d-flex align-center ga-2">
              <v-chip
                v-if="row.speedChip"
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

    <!-- 热门推荐热词 -->
    <v-card class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="tertiary-container" variant="flat" rounded="lg">
            <v-icon icon="mdi-fire" color="on-tertiary-container" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">热门推荐热词</v-card-title>
        <v-card-subtitle class="text-caption">首页「热门推荐」与「换一换」的数据来源，本地保存</v-card-subtitle>
      </v-card-item>
      <v-card-text>
        <div class="d-flex ga-2 mb-3">
          <v-text-field v-model="hotWordInput" label="添加热词" hide-details density="comfortable" @keyup.enter="addHotWord" />
          <v-btn color="primary" variant="flat" @click="addHotWord">添加</v-btn>
        </div>
        <div class="d-flex flex-wrap ga-2">
          <v-chip v-for="(w, i) in hotWords" :key="i" closable @click:close="removeHotWord(w)">
            {{ w }}
          </v-chip>
        </div>
        <v-btn variant="text" color="secondary" size="small" class="mt-2" @click="restoreDefaultHotWords">
          恢复默认热词
        </v-btn>
      </v-card-text>
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
            清空全部历史（磁力/复制/浏览）
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

    <!-- 添加自定义镜像弹窗 -->
    <v-dialog v-model="addMirrorDialog" max-width="480">
      <v-card>
        <v-card-title class="text-subtitle-1 font-weight-bold">添加自定义 GitHub 镜像</v-card-title>
        <v-divider />
        <v-card-text>
          <v-text-field v-model="newMirrorName" label="镜像名称 *" hide-details class="mb-3" placeholder="例如：ghproxy.net" />
          <v-text-field
            v-model="newMirrorBase"
            label="前缀地址 *（https:// 开头）"
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
            :disabled="!newMirrorName.trim() || !newMirrorBase.trim()"
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
