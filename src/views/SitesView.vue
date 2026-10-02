<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { useSitesStore } from '../stores/sites'
import { settings, DEFAULT_SUBSCRIBE_URL } from '../stores/settings'
import { invoke, isTauri } from '../lib/tauri'
import { httpGetText } from '../lib/http'
import { mirrorUrl } from '../lib/mirrors'
import type { SiteConfig } from '../types'

const sitesStore = useSitesStore()
const subscribeUrl = ref(settings.get('subscribeUrl'))
const subscribing = ref(false)
const toast = ref('')
const showToast = ref(false)
const busying = ref(false)

const subscribeDialog = ref(false)
const addDialog = ref(false)
const editing = ref<SiteConfig | null>(null)
const form = ref<SiteConfig>(emptyForm())

// 长按（手机）/ 右键（PC）上下文菜单 + 删除二次确认
// 菜单用 activator 锚定到触发的卡片元素，保证弹出位置正确
const contextMenu = ref<{ show: boolean; site: SiteConfig | null }>({ show: false, site: null })
const menuActivator = ref<Element | null>(null)
const deleteDialog = ref(false)
const deletingSite = ref<SiteConfig | null>(null)
let longPressTimer: ReturnType<typeof setTimeout> | null = null

const rowEls = new Map<string, Element>()

function collectRowEl(siteId: string | undefined, el: Element | null) {
  if (!siteId) return
  if (el) rowEls.set(siteId, el)
  else rowEls.delete(siteId)
}

function openContextMenu(site: SiteConfig, el: Element) {
  if (longPressTimer) {
    clearTimeout(longPressTimer)
    longPressTimer = null
  }
  menuActivator.value = rowEls.get(site.id ?? '') ?? el
  contextMenu.value = { show: true, site }
}

function onRowContextmenu(site: SiteConfig, e: MouseEvent) {
  openContextMenu(site, e.currentTarget as Element)
}

function onRowTouchstart(site: SiteConfig, e: TouchEvent) {
  const t = e.touches[0]
  if (!t) return
  longPressTimer = setTimeout(() => openContextMenu(site, e.currentTarget as Element), 500)
}

function cancelLongPress() {
  if (longPressTimer) {
    clearTimeout(longPressTimer)
    longPressTimer = null
  }
}

function editFromMenu() {
  const site = contextMenu.value.site
  contextMenu.value.show = false
  if (site) openEdit(site)
}

function askDelete() {
  const site = contextMenu.value.site
  contextMenu.value.show = false
  if (site) {
    deletingSite.value = site
    deleteDialog.value = true
  }
}

async function confirmDelete() {
  const site = deletingSite.value
  deleteDialog.value = false
  deletingSite.value = null
  if (site) await removeSite(site)
}

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
      // 拉取走前端 plugin-http（套用选中镜像，显式超时不转圈），存储仍为规范地址
      const mirror = settings.getSelectedMirror()
      const realUrl = mirrorUrl(mirror, url)
      const text = await httpGetText(realUrl, { timeoutMs: 15000 })
      const sites = await sitesStore.subscribeFromText(text, url)
      notice(`订阅成功：${sites.length} 个搜索源${mirror.base ? `（经 ${mirror.base}）` : ''}`)
    } finally {
      subscribing.value = false
    }
  })
}

async function toggleEnabled(site: SiteConfig, v: boolean) {
  await run(async () => {
    await sitesStore.setEnabled(site.id!, v)
  })
}

async function setDefaultFromMenu() {
  const site = contextMenu.value.site
  if (!site) return
  contextMenu.value.show = false
  await setDefault(site)
}

async function setDefault(site: SiteConfig) {
  await run(async () => {
    await sitesStore.setDefault(site.is_default ? null : (site.id ?? null))
  })
}

async function removeSite(site: SiteConfig) {
  await run(async () => {
    await sitesStore.remove(site.id!)
    notice(site.is_custom ? '已删除自定义站点' : '已删除搜索源')
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
    if (editing.value) {
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
  <div class="px-3 px-sm-6 pt-2 pb-3 mx-auto" style="max-width: 1040px">

    <!-- 站点列表：逐个 v-card（长按 / 右键菜单不变） -->
    <v-progress-linear v-if="sitesStore.loading" indeterminate color="primary" />
    <v-empty-state
      v-if="!sitesStore.loading && sitesStore.sites.length === 0"
      icon="mdi-antenna-off"
      title="还没有任何搜索源"
      text="点击右下角「订阅源管理」加载内置订阅源，或「自定义搜索源」添加站点"
    />
    <v-card
      v-for="site in sitesStore.sites"
      :key="site.id"
      :ref="(el) => collectRowEl(site.id, el as Element | null)"
      rounded="lg"
      class="mb-2"
      @contextmenu.prevent="onRowContextmenu(site, $event)"
      @touchstart="onRowTouchstart(site, $event)"
      @touchend="cancelLongPress"
      @touchmove="cancelLongPress"
      @touchcancel="cancelLongPress"
    >
      <div class="d-flex align-center pa-2">
        <v-switch
          :model-value="site.enabled"
          color="primary"
          hide-details
          density="compact"
          class="mr-2"
          @update:model-value="toggleEnabled(site, !!$event)"
        />
        <div class="flex-grow-1 mr-2" style="min-width: 0">
          <div class="text-body-2 font-weight-bold text-truncate">
            {{ site.name }}
            <v-chip v-if="site.is_custom" size="x-small" color="secondary" variant="tonal" class="ml-1">自定义</v-chip>
            <v-chip v-if="site.is_default" size="x-small" color="warning" variant="flat" class="ml-1">默认</v-chip>
          </div>
          <div class="text-caption text-medium-emphasis text-truncate">
            {{ site.info || '—' }}
            <span v-if="site.update_time" class="ml-1">更新：{{ site.update_time }}</span>
          </div>
        </div>
      </div>
    </v-card>

    <!-- 订阅源管理弹窗 -->
    <v-dialog v-model="subscribeDialog" width="auto" max-width="560">
      <v-card rounded="lg">
        <v-card-title class="text-subtitle-1 font-weight-bold">订阅源管理</v-card-title>
        <v-divider />
        <v-card-text style="min-width: min(80vw, 320px)">
          <v-text-field
            v-model="subscribeUrl"
            label="订阅仓库地址（GitHub Raw / JSON）"
            hide-details
            placeholder="https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json"
            density="compact"
            class="mb-3"
          />
          <div class="d-flex flex-wrap ga-2">
            <v-btn
              color="primary"
              variant="flat"
              :loading="subscribing"
              :disabled="busying"
              @click="doSubscribe"
            >
              <v-icon icon="mdi-cloud-download-outline" class="mr-1" />拉取订阅
            </v-btn>
            <v-btn variant="tonal" color="secondary" prepend-icon="mdi-import" :disabled="busying" @click="doImport">导入</v-btn>
            <v-btn variant="tonal" color="secondary" prepend-icon="mdi-export" :disabled="busying" @click="doExport">导出</v-btn>
            <v-btn variant="text" color="error" prepend-icon="mdi-restore" :disabled="busying" @click="doReset">重置</v-btn>
          </div>
          <div v-if="sitesStore.subscribedAt" class="text-caption text-medium-emphasis mt-3">
            上次更新：{{ sitesStore.subscribedAt }}
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>

    <!-- 悬浮按钮：自定义搜索源 / 订阅源管理 -->
    <v-fab
      icon="mdi-plus"
      color="primary"
      title="自定义搜索源"
      style="position: fixed; right: 20px; bottom: calc(84px + env(safe-area-inset-bottom)); z-index: 1200"
      @click="openAdd"
    />
    <v-fab
      icon="mdi-cloud-download-outline"
      color="secondary"
      title="订阅源管理"
      style="position: fixed; right: 20px; bottom: calc(140px + env(safe-area-inset-bottom)); z-index: 1200"
      @click="subscribeDialog = true"
    />

    <!-- 长按 / 右键上下文菜单 -->
    <v-menu v-model="contextMenu.show" :activator="menuActivator || undefined" min-width="200">
      <v-list density="compact" nav>
        <v-list-item prepend-icon="mdi-pencil-outline" title="修改" @click="editFromMenu" />
        <v-list-item
          :prepend-icon="contextMenu.site?.is_default ? 'mdi-star-off-outline' : 'mdi-star-outline'"
          :title="contextMenu.site?.is_default ? '取消默认搜索源' : '设为默认搜索源'"
          @click="setDefaultFromMenu"
        />
        <v-list-item prepend-icon="mdi-delete-outline" title="删除搜索源" color="error" @click="askDelete" />
      </v-list>
    </v-menu>

    <!-- 删除二次确认 -->
    <v-dialog v-model="deleteDialog" max-width="360">
      <v-card rounded="lg">
        <v-card-title class="text-subtitle-1 font-weight-bold">删除搜索源</v-card-title>
        <v-divider />
        <v-card-text class="pt-4">
          确定删除「{{ deletingSite?.name }}」吗？删除后需重新订阅或添加。
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" variant="flat" @click="confirmDelete">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

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
