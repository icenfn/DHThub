<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { useSitesStore } from '../stores/sites'
import { settings, DEFAULT_SUBSCRIBE_URL } from '../stores/settings'
import { invoke, isTauri } from '../lib/tauri'
import type { SiteConfig } from '../types'

const sitesStore = useSitesStore()
const subscribeUrl = ref(settings.get('subscribeUrl'))
const subscribing = ref(false)
const toast = ref('')
const showToast = ref(false)
const busying = ref(false)

const addDialog = ref(false)
const editing = ref<SiteConfig | null>(null)
const form = ref<SiteConfig>(emptyForm())

function emptyForm(): SiteConfig {
  return {
    name: '',
    info: '',
    request: {
      method: 'GET',
      search_url: '',
      headers: {},
      timeout_ms: 12000,
    },
    expression_model: {
      group: '',
      title: '',
      date: '',
      size: '',
      url: { sel: '', attr: 'href' },
      magnet: { sel: '', attr: 'href' },
    },
  }
}

// 表单辅助字段：请求头/链接/磁力 拆分编辑
const headersText = ref('')
const urlSel = ref('')
const urlAttr = ref('href')
const magnetSel = ref('')
const magnetAttr = ref('href')

function syncFormHelpers() {
  const expr = form.value.expression_model
  headersText.value = Object.keys(form.value.request.headers || {}).length
    ? JSON.stringify(form.value.request.headers, null, 0)
    : ''
  const u = expr.url as { sel?: string; attr?: string } | undefined
  urlSel.value = u?.sel ?? ''
  urlAttr.value = u?.attr ?? 'href'
  const m = expr.magnet as { sel?: string; attr?: string } | undefined
  magnetSel.value = m?.sel ?? ''
  magnetAttr.value = m?.attr ?? 'href'
}

function collectFormHelpers() {
  try {
    const parsed = headersText.value.trim() ? JSON.parse(headersText.value) : {}
    form.value.request.headers = typeof parsed === 'object' ? parsed : {}
  } catch {
    notice('请求头 JSON 格式错误，已忽略')
    form.value.request.headers = {}
  }
  form.value.expression_model.url = urlSel.value
    ? { sel: urlSel.value, attr: urlAttr.value }
    : { sel: '', attr: 'href' }
  form.value.expression_model.magnet = magnetSel.value
    ? { sel: magnetSel.value, attr: magnetAttr.value }
    : { sel: '', attr: 'href' }
}

function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

async function run(fn: () => Promise<void>) {
  busying.value = true
  try {
    await fn()
  } catch (e) {
    notice(String(e))
  } finally {
    busying.value = false
  }
}

async function doSubscribe() {
  await run(async () => {
    const url = subscribeUrl.value.trim() || DEFAULT_SUBSCRIBE_URL
    subscribeUrl.value = url
    await settings.set('subscribeUrl', url)
    subscribing.value = true
    try {
      const sites = await sitesStore.subscribe(url)
      notice(`订阅成功：${sites.length} 个搜索源`)
    } finally {
      subscribing.value = false
    }
  })
}

async function toggleEnabled(site: SiteConfig) {
  await run(async () => {
    await sitesStore.setEnabled(site.id!, !site.enabled)
  })
}

async function setDefault(site: SiteConfig) {
  await run(async () => {
    await sitesStore.setDefault(site.is_default ? null : (site.id ?? null))
  })
}

async function removeSite(site: SiteConfig) {
  await run(async () => {
    await sitesStore.remove(site.id!)
    notice(site.is_custom ? '已删除自定义站点' : '已停用该站点（订阅源站点不可删除）')
  })
}

function openAdd() {
  editing.value = null
  form.value = emptyForm()
  syncFormHelpers()
  addDialog.value = true
}

function openEdit(site: SiteConfig) {
  editing.value = site
  form.value = JSON.parse(JSON.stringify(site)) as SiteConfig
  if (!form.value.request.headers) form.value.request.headers = {}
  syncFormHelpers()
  addDialog.value = true
}

async function saveSite() {
  await run(async () => {
    collectFormHelpers()
    if (editing.value?.is_custom) {
      await sitesStore.updateCustom(form.value)
      notice('站点已更新')
    } else {
      await sitesStore.addCustom(form.value)
      notice('自定义站点已添加，请开启开关后使用')
    }
    addDialog.value = false
  })
}

// B8 导入导出
async function doExport() {
  await run(async () => {
    const json = await sitesStore.exportJson()
    if (!isTauri) {
      notice('浏览器预览模式不支持文件导出')
      return
    }
    const path = await save({
      title: '导出站点配置',
      defaultPath: 'dhthub-sites.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path) return
    await writeTextFile(path, json)
    notice('已导出到 ' + path)
  })
}

async function doImport() {
  await run(async () => {
    if (!isTauri) {
      notice('浏览器预览模式不支持文件导入')
      return
    }
    const path = await open({
      title: '导入站点配置',
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path) return
    const text = await readTextFile(path)
    await sitesStore.importJson(text)
    notice('导入成功')
  })
}

async function doReset() {
  await run(async () => {
    await sitesStore.reset()
    notice('已恢复出厂：清空订阅与自定义站点')
  })
}

async function reloadSites() {
  await run(async () => {
    await sitesStore.load()
  })
}

const customCount = computed(() => sitesStore.customSites.length)
const subscribedCount = computed(() => sitesStore.subscribedSites.length)

onMounted(async () => {
  await settings.ready()
  subscribeUrl.value = settings.get('subscribeUrl')
  await reloadSites()
  // 首次自动订阅默认仓库
  if (sitesStore.subscribedSites.length === 0 && isTauri) {
    await doSubscribe()
  }
})
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 1040px">
    <div class="d-flex align-center mt-2 mb-4">
      <div>
        <div class="text-h6 font-weight-bold">站点管理</div>
        <div class="text-caption text-medium-emphasis">订阅源 {{ subscribedCount }} 个 · 自定义 {{ customCount }} 个</div>
      </div>
      <v-spacer />
      <v-btn variant="tonal" color="secondary" prepend-icon="mdi-import" size="small" @click="doImport">导入</v-btn>
      <v-btn variant="tonal" color="secondary" prepend-icon="mdi-export" size="small" class="ml-2" @click="doExport">导出</v-btn>
      <v-btn variant="text" color="error" prepend-icon="mdi-restore" size="small" class="ml-2" @click="doReset">重置</v-btn>
    </div>

    <!-- 订阅仓库 -->
    <v-card rounded="lg" class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="primary" variant="tonal">
            <v-icon icon="mdi-download-circle-outline" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">订阅在线搜索仓库</v-card-title>
        <v-card-subtitle class="text-caption">
          输入任意 GitHub Raw / JSON 地址，拉取站点列表（仓库内置：sites/default.json）
          <span v-if="sitesStore.subscribedAt" class="ml-1 text-medium-emphasis">上次更新：{{ sitesStore.subscribedAt }}</span>
        </v-card-subtitle>
      </v-card-item>
      <v-card-text>
        <div class="d-flex flex-column flex-sm-row ga-2">
          <v-text-field
            v-model="subscribeUrl"
            label="订阅源地址"
            hide-details
            placeholder="https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json"
            density="comfortable"
          />
          <v-btn
            color="primary"
            variant="flat"
            :loading="subscribing"
            :disabled="busying"
            @click="doSubscribe"
          >
            <v-icon icon="mdi-cloud-download-outline" class="mr-1" />拉取订阅
          </v-btn>
        </div>
      </v-card-text>
    </v-card>

    <!-- 站点列表 -->
    <v-card rounded="lg">
      <v-card-item>
        <template #prepend>
          <v-avatar color="secondary" variant="tonal">
            <v-icon icon="mdi-antenna" />
          </v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">搜索源列表</v-card-title>
        <v-card-subtitle class="text-caption">开启的站点才会参与搜索；点击 ☆ 设为默认引擎</v-card-subtitle>
        <template #append>
          <v-btn color="primary" variant="tonal" prepend-icon="mdi-plus" size="small" @click="openAdd">
            添加自定义
          </v-btn>
        </template>
      </v-card-item>

      <v-divider />
      <v-progress-linear v-if="sitesStore.loading" indeterminate color="primary" />
      <v-empty-state
        v-if="!sitesStore.loading && sitesStore.sites.length === 0"
        icon="mdi-antenna-off"
        title="还没有任何搜索源"
        text="点击上方「拉取订阅」加载内置订阅源，或添加自定义站点"
      />
      <v-list v-else>
        <v-list-item v-for="site in sitesStore.sites" :key="site.id">
          <template #prepend>
            <v-btn
              :icon="site.is_default ? 'mdi-star' : 'mdi-star-outline'"
              size="small"
              variant="text"
              :color="site.is_default ? 'warning' : 'grey'"
              title="设为默认搜索源"
              @click="setDefault(site)"
            />
          </template>
          <v-list-item-title class="text-body-2 font-weight-bold">
            {{ site.name }}
            <v-chip v-if="site.is_custom" size="x-small" color="secondary" variant="tonal" class="ml-1">自定义</v-chip>
            <v-chip v-if="site.is_default" size="x-small" color="warning" variant="flat" class="ml-1">默认</v-chip>
          </v-list-item-title>
          <v-list-item-subtitle class="text-caption">
            {{ site.info || '—' }}
            <span v-if="site.update_time" class="ml-2 text-medium-emphasis">更新：{{ site.update_time }}</span>
          </v-list-item-subtitle>
          <template #append>
            <div class="d-flex align-center ga-1">
              <v-btn
                v-if="site.is_custom"
                icon="mdi-pencil-outline"
                size="small"
                variant="text"
                title="编辑"
                @click="openEdit(site)"
              />
              <v-btn
                :icon="site.is_custom ? 'mdi-delete-outline' : 'mdi-power'"
                size="small"
                variant="text"
                :color="site.is_custom ? 'error' : 'grey'"
                :title="site.is_custom ? '删除' : '停用'"
                @click="removeSite(site)"
              />
              <v-switch
                :model-value="site.enabled"
                color="primary"
                hide-details
                density="compact"
                @update:model-value="toggleEnabled(site)"
              />
            </div>
          </template>
        </v-list-item>
      </v-list>
    </v-card>

    <!-- 自定义站点表单 -->
    <v-dialog v-model="addDialog" max-width="640">
      <v-card>
        <v-card-title class="text-subtitle-1 font-weight-bold">
          {{ editing?.is_custom ? '编辑自定义站点' : '添加自定义站点' }}
        </v-card-title>
        <v-divider />
        <v-card-text style="max-height: 66vh; overflow-y: auto">
          <v-text-field v-model="form.name" label="站点名称 *" hide-details class="mb-3" />
          <v-text-field v-model="form.info" label="站点说明" hide-details class="mb-3" />
          <div class="d-flex ga-2 mb-3">
            <v-select
              v-model="form.request.method"
              :items="['GET', 'POST']"
              label="请求方法"
              hide-details
              style="max-width: 140px"
            />
            <v-text-field v-model.number="form.request.timeout_ms" label="超时(ms)" type="number" hide-details />
          </div>
          <v-text-field
            v-model="form.request.search_url"
            label="搜索地址模板 *（支持 [keyword] [page] 占位符）"
            hide-details
            class="mb-3"
            placeholder="https://example.com/s/[keyword]?page=[page]"
          />
          <v-textarea
            v-model="headersText"
            label="请求头（JSON，可选）"
            hide-details
            auto-grow
            rows="2"
            class="mb-3"
          />
          <v-divider class="my-2" />
          <div class="text-subtitle-2 font-weight-bold mb-2">解析规则（CSS 选择器）</div>
          <v-text-field v-model="form.expression_model.group" label="结果条目容器 *（如 .search-item）" hide-details class="mb-3" />
          <v-text-field v-model="form.expression_model.title" label="标题选择器" hide-details class="mb-3" />
          <div class="d-flex ga-2 mb-3">
            <v-text-field v-model="form.expression_model.date" label="日期选择器" hide-details />
            <v-text-field v-model="form.expression_model.size" label="大小选择器" hide-details />
          </div>
          <div class="d-flex ga-2 mb-3">
            <v-text-field v-model="urlSel" label="链接选择器" hide-details />
            <v-select v-model="urlAttr" :items="['href', 'value', 'data-clipboard-text']" label="链接属性" hide-details style="max-width: 200px" />
          </div>
          <div class="d-flex ga-2">
            <v-text-field v-model="magnetSel" label="磁力选择器（留空则用链接）" hide-details />
            <v-select v-model="magnetAttr" :items="['href', 'value', 'data-clipboard-text']" label="磁力属性" hide-details style="max-width: 200px" />
          </div>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="addDialog = false">取消</v-btn>
          <v-btn color="primary" variant="flat" :disabled="!form.name || !form.request.search_url || !form.expression_model.group" @click="saveSite">
            保存
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-snackbar v-model="showToast" location="bottom" color="success" timeout="2500">
      {{ toast }}
    </v-snackbar>
  </div>
</template>
