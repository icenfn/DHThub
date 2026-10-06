<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { useSitesStore } from '../stores/sites'
import { settings, DEFAULT_SUBSCRIBE_URL } from '../stores/settings'
import { invoke, isTauri } from '../lib/tauri'
import { httpGetText } from '../lib/http'
import { mirrorUrl } from '../lib/mirrors'
import { pushOverlay } from '../lib/overlay'
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
const testingSite = ref(false)
const siteTest = ref<{ ok: boolean; elapsed_ms: number; items: number; error: string | null; samples: string[] } | null>(null)

// 长按（手机）/ 右键（PC）上下文菜单 + 删除二次确认
// 菜单使用触发点坐标定位（position-x/y），避免锚定元素导致菜单位置偏差
const contextMenu = ref<{ show: boolean; site: SiteConfig | null }>({ show: false, site: null })
const menuPos = ref<{ x: number; y: number } | null>(null)
const deleteDialog = ref(false)
const deletingSite = ref<SiteConfig | null>(null)
let longPressTimer: ReturnType<typeof setTimeout> | null = null

const enabledCount = computed(() => sitesStore.sites.filter((s) => s.enabled).length)

function openContextMenu(site: SiteConfig, e: { clientX: number; clientY: number }) {
  if (longPressTimer) {
    clearTimeout(longPressTimer)
    longPressTimer = null
  }
  menuPos.value = { x: e.clientX, y: e.clientY }
  contextMenu.value = { show: true, site }
}

function onRowContextmenu(site: SiteConfig, e: MouseEvent) {
  openContextMenu(site, e)
}

function onRowTouchstart(site: SiteConfig, e: TouchEvent) {
  const t = e.touches[0]
  if (!t) return
  longPressTimer = setTimeout(() => openContextMenu(site, { clientX: t.clientX, clientY: t.clientY }), 500)
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

// 弹层栈注册：手机返回键 / PC ESC 关闭
watch(subscribeDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { subscribeDialog.value = false }))
})
watch(deleteDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { deleteDialog.value = false }))
})
watch(addDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { addDialog.value = false }))
})
watch(
  () => contextMenu.value.show,
  (v, _o, onCleanup) => {
    if (v) {
      onCleanup(
        pushOverlay(() => {
          contextMenu.value.show = false
          contextMenu.value.site = null
        }),
      )
    }
  },
)

function openAdd() {
  editing.value = null
  form.value = emptyForm()
  syncFormHelpers()
  siteTest.value = null
  addDialog.value = true
}

function openEdit(site: SiteConfig) {
  editing.value = site
  form.value = JSON.parse(JSON.stringify(site)) as SiteConfig
  if (!form.value.request.headers) form.value.request.headers = {}
  syncFormHelpers()
  siteTest.value = null
  addDialog.value = true
}

/** 测试连接：用「test」关键词请求一次并解析，展示耗时/条目/样例 */
async function testSite() {
  await run(async () => {
    collectFormHelpers()
    siteTest.value = null
    const dns = settings.getSelectedDns()?.base ?? ''
    const r = await invoke('test_site', { site: form.value, dns }, 30000)
    siteTest.value = r as { ok: boolean; elapsed_ms: number; items: number; error: string | null; samples: string[] }
    if (siteTest.value.ok) notice(`连接成功：${siteTest.value.items} 条结果 · ${siteTest.value.elapsed_ms}ms`)
    else notice('连接失败：' + (siteTest.value.error ?? '未知错误'))
  })
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

// 导入导出
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
  <div class="page-wrap">
    <div class="page-head">
      <div>
        <h2 class="text-title-large font-weight-bold">搜索源</h2>
        <p class="text-body-small text-medium-emphasis">
          已启用 <strong>{{ enabledCount }}</strong> / {{ sitesStore.sites.length }} 个站点 · 长按或右键可管理
        </p>
      </div>
      <v-spacer />
      <v-btn
        color="secondary"
        variant="tonal"
        prepend-icon="mdi-cloud-download-outline"
        size="small"
        rounded="pill"
        @click="subscribeDialog = true"
      >
        订阅源管理
      </v-btn>
    </div>

    <v-progress-linear v-if="sitesStore.loading" indeterminate color="primary" rounded />

    <v-sheet
      v-if="!sitesStore.loading && sitesStore.sites.length === 0"
      rounded="xl"
      color="surface-container-low"
      border
      class="pa-8 text-center"
    >
      <v-icon icon="mdi-antenna-off" size="48" color="outline" class="mb-2" />
      <div class="text-title-medium font-weight-bold">还没有任何搜索源</div>
      <div class="text-body-small text-medium-emphasis">
        点击「订阅源管理」加载内置订阅源，或右下角「+」添加自定义站点
      </div>
    </v-sheet>

    <div v-else class="site-list">
      <v-card
        v-for="site in sitesStore.sites"
        :key="site.id"
        rounded="xl"
        variant="flat"
        color="surface-container-low"
        class="site-card"
        :class="{ 'site-card--off': !site.enabled }"
        @contextmenu.prevent="onRowContextmenu(site, $event)"
        @touchstart="onRowTouchstart(site, $event)"
        @touchend="cancelLongPress"
        @touchmove="cancelLongPress"
        @touchcancel="cancelLongPress"
      >
        <div class="site-card__row">
          <v-switch
            :model-value="site.enabled"
            color="primary"
            hide-details
            density="compact"
            class="site-switch"
            @touchstart.stop
            @click.stop
            @update:model-value="toggleEnabled(site, !!$event)"
          />
          <div class="site-card__meta">
            <div class="text-body-medium font-weight-bold text-truncate">
              {{ site.name }}
              <v-chip v-if="site.is_custom" size="x-small" color="secondary" variant="tonal" class="ml-1">
                自定义
              </v-chip>
              <v-chip v-if="site.is_default" size="x-small" color="warning" variant="flat" class="ml-1">
                默认
              </v-chip>
            </div>
            <div class="text-body-small text-medium-emphasis text-truncate">
              {{ site.info || '—' }}
              <span v-if="site.update_time" class="ml-1">更新：{{ site.update_time }}</span>
            </div>
          </div>
          <v-btn
            icon="mdi-dots-vertical"
            size="small"
            variant="text"
            rounded="lg"
            class="d-sm-none"
            @click.stop="openContextMenu(site, $event)"
          />
        </div>
      </v-card>
    </div>

    <!-- 悬浮按钮：订阅源管理 / 自定义搜索源 -->
    <v-fab
      icon="mdi-cloud-download-outline"
      color="secondary"
      variant="tonal"
      size="56"
      title="订阅源管理"
      class="fab fab--top"
      @click="subscribeDialog = true"
    />
    <v-fab
      icon="mdi-plus"
      color="primary"
      size="60"
      title="自定义搜索源"
      class="fab fab--main"
      @click="openAdd"
    />

    <!-- 订阅源管理弹窗 -->
    <v-dialog v-model="subscribeDialog" max-width="560">
      <v-card rounded="xl" variant="flat" color="surface-container-low">
        <v-card-item class="pt-4">
          <template #prepend>
            <v-avatar color="secondary-container" rounded="lg">
              <v-icon icon="mdi-cloud-download-outline" color="on-secondary-container" />
            </v-avatar>
          </template>
          <v-card-title class="text-title-medium font-weight-bold">订阅源管理</v-card-title>
          <v-card-subtitle class="text-body-small">从订阅仓库拉取站点配置</v-card-subtitle>
        </v-card-item>

        <v-card-text>
          <v-text-field
            v-model="subscribeUrl"
            label="订阅仓库地址（GitHub Raw / JSON）"
            hide-details
            placeholder="https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json"
            density="comfortable"
            class="mb-3"
          />
          <div class="d-flex flex-wrap ga-2">
            <v-btn
              color="primary"
              variant="flat"
              rounded="pill"
              :loading="subscribing"
              :disabled="busying"
              @click="doSubscribe"
            >
              <v-icon icon="mdi-cloud-download-outline" class="mr-1" />拉取订阅
            </v-btn>
            <v-btn variant="tonal" color="secondary" rounded="pill" prepend-icon="mdi-import" :disabled="busying" @click="doImport">
              导入
            </v-btn>
            <v-btn variant="tonal" color="secondary" rounded="pill" prepend-icon="mdi-export" :disabled="busying" @click="doExport">
              导出
            </v-btn>
            <v-btn variant="text" color="error" rounded="pill" prepend-icon="mdi-restore" :disabled="busying" @click="doReset">
              重置
            </v-btn>
          </div>
          <div v-if="sitesStore.subscribedAt" class="text-label-small text-medium-emphasis mt-3">
            上次更新：{{ sitesStore.subscribedAt }}
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>

    <!-- 长按 / 右键上下文菜单 -->
    <v-menu
      v-model="contextMenu.show"
      :position-x="menuPos?.x ?? 0"
      :position-y="menuPos?.y ?? 0"
      min-width="220"
      rounded="lg"
    >
      <v-list density="compact" nav rounded="lg">
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
      <v-card rounded="xl" variant="flat" color="surface-container-low">
        <v-card-title class="text-title-medium font-weight-bold pa-4 pb-0">删除搜索源</v-card-title>
        <v-card-text class="pt-4">
          确定删除「{{ deletingSite?.name }}」吗？删除后需重新订阅或添加。
        </v-card-text>
        <v-card-actions class="px-4 pb-3">
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" variant="flat" @click="confirmDelete">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 自定义站点表单 -->
    <v-dialog v-model="addDialog" max-width="660">
      <v-card rounded="xl" variant="flat" color="surface-container-low">
        <v-card-title class="text-title-medium font-weight-bold pa-4 pb-0">
          {{ editing?.is_custom ? '编辑自定义站点' : '添加自定义站点' }}
        </v-card-title>
        <v-card-text style="max-height: 66vh; overflow-y: auto" class="pt-4">
          <v-text-field v-model="form.name" label="站点名称 *" hide-details class="mb-3" />
          <v-text-field v-model="form.info" label="站点说明" hide-details class="mb-3" />
          <div class="d-flex ga-2 mb-3">
            <v-select
              v-model="form.request.method"
              :items="['GET', 'POST']"
              label="请求方法"
              hide-details
              style="max-width: 160px"
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
          <v-divider class="my-3" />
          <div class="text-label-large text-medium-emphasis mb-2">解析规则（CSS 选择器）</div>
          <v-text-field v-model="form.expression_model.group" label="结果条目容器 *（如 .search-item）" hide-details class="mb-3" />
          <v-text-field v-model="form.expression_model.title" label="标题选择器" hide-details class="mb-3" />
          <div class="d-flex ga-2 mb-3">
            <v-text-field v-model="form.expression_model.date" label="日期选择器" hide-details />
            <v-text-field v-model="form.expression_model.size" label="大小选择器" hide-details />
          </div>
          <div class="d-flex ga-2 mb-3">
            <v-text-field v-model="urlSel" label="链接选择器" hide-details />
            <v-select v-model="urlAttr" :items="['href', 'value', 'data-clipboard-text']" label="链接属性" hide-details style="max-width: 210px" />
          </div>
          <div class="d-flex ga-2">
            <v-text-field v-model="magnetSel" label="磁力选择器（留空则用链接）" hide-details />
            <v-select v-model="magnetAttr" :items="['href', 'value', 'data-clipboard-text']" label="磁力属性" hide-details style="max-width: 210px" />
          </div>
          <!-- 测试结果 -->
          <template v-if="siteTest">
            <v-divider class="my-3" />
            <div v-if="siteTest.ok" class="text-body-medium">
              <v-icon icon="mdi-check-circle" color="success" size="18" class="mr-1" />
              连接成功 · {{ siteTest.items }} 条结果 · {{ siteTest.elapsed_ms }}ms
              <div v-if="siteTest.samples.length" class="text-body-small text-medium-emphasis mt-2">
                <v-chip v-for="t in siteTest.samples" :key="t" size="x-small" variant="tonal" class="mr-1 mb-1">{{ t }}</v-chip>
              </div>
            </div>
            <div v-else class="text-body-medium text-error">
              <v-icon icon="mdi-close-circle" size="18" class="mr-1" />
              连接失败：{{ siteTest.error || '未知错误' }}
            </div>
          </template>
        </v-card-text>
        <v-card-actions class="px-4 pb-3">
          <v-btn variant="tonal" color="secondary" rounded="pill" prepend-icon="mdi-connection" :loading="testingSite" @click="testSite">
            测试连接
          </v-btn>
          <v-spacer />
          <v-btn variant="text" @click="addDialog = false">取消</v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="!form.name || !form.request.search_url || !form.expression_model.group"
            @click="saveSite"
          >
            保存
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-snackbar v-model="showToast" location="bottom" color="inverse-surface" rounded="lg" timeout="2500">
      {{ toast }}
    </v-snackbar>
  </div>
</template>

<style scoped>
.page-wrap {
  max-width: 1040px;
  margin: 0 auto;
  padding: 16px 20px 96px;
}

.page-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.site-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.site-card {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition: opacity 0.18s ease;
}

.site-card--off {
  opacity: 0.62;
}

.site-card__row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
}

.site-switch {
  flex-shrink: 0;
}

.site-card__meta {
  flex: 1;
  min-width: 0;
}

.fab {
  position: fixed;
  right: 20px;
  z-index: 1200;
}

.fab--main {
  bottom: calc(88px + env(safe-area-inset-bottom));
}

.fab--top {
  bottom: calc(152px + env(safe-area-inset-bottom));
}
</style>
