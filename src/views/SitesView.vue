<script setup lang="ts">
// 搜索源：订阅拉取 / 自定义站点增删改 / 启停开关 / 设默认 / 导入导出 / 重置
// 列表每行右侧为操作按钮（点击弹出菜单），不再使用长按或右键。
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
const siteTest = ref<{
  ok: boolean
  elapsed_ms: number
  items: number
  error: string | null
  samples: string[]
} | null>(null)

// 每行右侧操作按钮 -> 弹出菜单
// 修复「点一行按钮弹出一堆菜单」：
// 原实现用单个 menuOpen 布尔量给每一行都绑定了 v-menu，任意一行打开都会让
// 所有行的菜单同时展开。改为记录「当前打开菜单的站点 id」，逐行独立判定。
const menuOpenId = ref<string | null>(null)
const menuSite = ref<SiteConfig | null>(null)

function isMenuOpen(site: SiteConfig): boolean {
  return menuOpenId.value === (site.id ?? null)
}

function setMenuOpen(site: SiteConfig, open: boolean): void {
  if (open) {
    menuOpenId.value = site.id ?? null
    menuSite.value = site
  } else if (isMenuOpen(site)) {
    menuOpenId.value = null
  }
}
const deleteDialog = ref(false)
const deletingSite = ref<SiteConfig | null>(null)

const METHOD_ITEMS = ['GET', 'POST']
const ATTR_ITEMS = ['href', 'value', 'data-clipboard-text']

const enabledCount = computed(() => sitesStore.sites.filter((s) => s.enabled).length)

function emptyForm(): SiteConfig {
  return {
    name: '',
    info: '',
    request: { method: 'GET', search_url: '', headers: {}, timeout_ms: 12000 },
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

// 表单辅助字段：请求头 / 链接 / 磁力 拆分编辑
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
    form.value.request.headers = typeof parsed === 'object' && parsed ? parsed : {}
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
      const mirror = settings.getSelectedMirror()
      const realUrl = mirrorUrl(mirror, url)
      const text = await httpGetText(realUrl, { timeoutMs: 15000 })
      const sites = await sitesStore.subscribeFromText(text, url)
      notice(`订阅成功：${sites.length} 个搜索源${mirror.base ? `（经 ${mirror.base}）` : ''}`)
      subscribeDialog.value = false
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

async function setDefault(site: SiteConfig) {
  await run(async () => {
    await sitesStore.setDefault(site.is_default ? null : (site.id ?? null))
    notice(site.is_default ? '已取消默认搜索源' : `已将「${site.name}」设为默认`)
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

function askDelete(site: SiteConfig) {
  deletingSite.value = site
  deleteDialog.value = true
}

async function confirmDelete() {
  const site = deletingSite.value
  deleteDialog.value = false
  deletingSite.value = null
  if (site) await removeSite(site)
}

/** 测试连接：用「test」关键词请求一次并解析 */
async function testSite() {
  await run(async () => {
    collectFormHelpers()
    siteTest.value = null
    testingSite.value = true
    try {
      const dns = settings.getSelectedDns()?.base ?? ''
      const r = await invoke('test_site', { site: form.value, dns }, 30000)
      siteTest.value = r as {
        ok: boolean
        elapsed_ms: number
        items: number
        error: string | null
        samples: string[]
      }
      if (siteTest.value.ok) notice(`连接成功：${siteTest.value.items} 条 · ${siteTest.value.elapsed_ms}ms`)
      else notice('连接失败：' + (siteTest.value.error ?? '未知错误'))
    } finally {
      testingSite.value = false
    }
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

// ---------- 导入 / 导出 / 重置 ----------
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
    if (!path || typeof path !== 'string') return
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

onMounted(async () => {
  await settings.ready()
  subscribeUrl.value = settings.get('subscribeUrl')
  await sitesStore.load()
  // 首次自动订阅默认仓库
  if (sitesStore.subscribedSites.length === 0 && isTauri) {
    await doSubscribe()
  }
  if (!isTauri) {
    // 浏览器预览：用空列表提示，避免调用不存在的 IPC
    sitesStore.loading = false
  }
})
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner">
      <!-- 页头：标题/副标题已移除（顶栏已显示页名），只保留订阅入口 -->
      <div class="head head--actions">
        <v-btn
          variant="outlined"
          size="small"
          rounded="pill"
          color="primary"
          prepend-icon="mdi-cloud-download-outline"
          class="head__btn"
          @click="subscribeDialog = true"
        >
          订阅
        </v-btn>
      </div>

      <v-progress-linear v-if="sitesStore.loading" indeterminate color="primary" rounded height="2" />

      <!-- 空态 -->
      <v-empty-state
        v-if="!sitesStore.loading && sitesStore.sites.length === 0"
        icon="mdi-antenna-off"
        title="还没有任何搜索源"
        text="点击右上「订阅」加载内置订阅源，或用右下角 + 添加自定义站点"
        class="empty"
      />

      <!-- 站点列表 -->
      <v-card v-else class="list" variant="outlined" rounded="lg">
        <div
          v-for="site in sitesStore.sites"
          :key="site.id"
          class="row"
          :class="{ 'row--off': !site.enabled }"
        >
          <!-- 启停开关 -->
          <v-switch
            :model-value="site.enabled"
            color="primary"
            density="compact"
            hide-details
            inset
            class="row__switch"
            :aria-label="site.enabled ? '停用该站点' : '启用该站点'"
            @update:model-value="(v: unknown) => toggleEnabled(site, v === true)"
          />

          <div class="row__body" @click="openEdit(site)">
            <div class="row__title">
              <span class="row__name">{{ site.name }}</span>
              <v-chip v-if="site.is_default" size="x-small" color="warning" variant="tonal">
                默认
              </v-chip>
              <v-chip v-if="site.is_custom" size="x-small" color="secondary" variant="tonal">
                自定义
              </v-chip>
            </div>
            <div class="row__meta">
              <span class="row__info">{{ site.info || '—' }}</span>
              <span v-if="site.update_time" class="row__time">更新 {{ site.update_time }}</span>
            </div>
          </div>

          <!-- 右侧操作菜单 -->
          <v-menu
            :model-value="isMenuOpen(site)"
            location="bottom end"
            @update:model-value="(v: boolean) => setMenuOpen(site, v)"
          >
            <template #activator="{ props: menuProps }">
              <v-btn
                v-bind="menuProps"
                icon="mdi-dots-vertical"
                variant="text"
                size="small"
                title="更多操作"
                @click.stop
              />
            </template>
            <v-list density="compact" rounded="lg" nav>
              <v-list-item prepend-icon="mdi-pencil-outline" title="修改" @click="openEdit(site)" />
              <v-list-item
                :prepend-icon="site.is_default ? 'mdi-star-off-outline' : 'mdi-star-outline'"
                :title="site.is_default ? '取消默认' : '设为默认'"
                @click="setDefault(site)"
              />
              <v-list-item
                prepend-icon="mdi-delete-outline"
                title="删除"
                base-color="error"
                @click="askDelete(site)"
              />
            </v-list>
          </v-menu>
        </div>
      </v-card>

      <!-- 悬浮新增按钮 -->
      <v-btn
        class="fab"
        icon="mdi-plus"
        color="primary"
        size="large"
        title="添加自定义站点"
        @click="openAdd"
      />

      <!-- 订阅源管理 -->
      <v-dialog v-model="subscribeDialog" max-width="520">
        <v-card rounded="xl" variant="flat" color="surface-container-low" border>
          <div class="dlghead">
            <v-avatar color="secondary-container" rounded="lg" size="36">
              <v-icon icon="mdi-cloud-download-outline" color="on-secondary-container" size="19" />
            </v-avatar>
            <div class="dlghead__text">
              <div class="dlghead__title">订阅源管理</div>
              <div class="dlghead__sub">从订阅仓库拉取站点配置</div>
            </div>
          </div>

          <div class="dlgbody">
            <div class="section-label mb-1">订阅仓库地址</div>
            <v-textarea
              v-model="subscribeUrl"
              variant="outlined"
              density="compact"
              rows="2"
              hide-details
              placeholder="https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json"
              class="mb-1"
            />
            <div class="hintline">
              支持 GitHub Raw 直链；将按「设置 → GitHub 镜像」所选前缀代理抓取
            </div>

            <div class="btnrow">
              <v-btn
                color="primary"
                variant="flat"
                size="small"
                rounded="pill"
                prepend-icon="mdi-cloud-download-outline"
                :loading="subscribing"
                :disabled="busying || subscribing"
                @click="doSubscribe"
              >
                拉取订阅
              </v-btn>
              <v-btn
                variant="tonal"
                size="small"
                rounded="pill"
                prepend-icon="mdi-import"
                :disabled="busying"
                @click="doImport"
              >
                导入
              </v-btn>
              <v-btn
                variant="tonal"
                size="small"
                rounded="pill"
                prepend-icon="mdi-export"
                :disabled="busying"
                @click="doExport"
              >
                导出
              </v-btn>
              <v-btn
                variant="tonal"
                size="small"
                rounded="pill"
                color="error"
                prepend-icon="mdi-restore"
                :disabled="busying"
                @click="doReset"
              >
                重置
              </v-btn>
            </div>
          </div>

          <v-card-actions class="dlgactions">
            <v-btn variant="text" size="small" @click="subscribeDialog = false">关闭</v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <!-- 删除确认 -->
      <v-dialog v-model="deleteDialog" max-width="340">
        <v-card rounded="xl" variant="flat" color="surface-container-low" border>
          <div class="dlgbody dlgbody--tight">
            <div class="dlghead__title">删除搜索源</div>
            <div class="confirm__text">
              确定删除「{{ deletingSite?.name }}」吗？删除后需重新订阅或添加。
            </div>
          </div>
          <v-card-actions class="dlgactions">
            <v-btn variant="text" size="small" @click="deleteDialog = false">取消</v-btn>
            <v-btn color="error" variant="flat" size="small" rounded="pill" @click="confirmDelete">
              删除
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <!-- 自定义站点表单 -->
      <v-dialog v-model="addDialog" max-width="620" scrollable>
        <v-card rounded="xl" variant="flat" color="surface-container-low" border>
          <div class="dlghead">
            <v-avatar color="primary-container" rounded="lg" size="36">
              <v-icon icon="mdi-pencil-plus-outline" color="on-primary-container" size="19" />
            </v-avatar>
            <div class="dlghead__text">
              <div class="dlghead__title">
                {{ editing?.is_custom ? '编辑自定义站点' : '添加自定义站点' }}
              </div>
              <div class="dlghead__sub">配置请求模板与 CSS 解析规则</div>
            </div>
          </div>

          <div class="dlgbody">
            <div class="section-label mb-1">基本信息</div>
            <v-text-field
              v-model="form.name"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="站点名称 *"
              class="mb-2"
            />
            <v-text-field
              v-model="form.info"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="站点说明（可选）"
              class="mb-2"
            />

            <div class="grid2">
              <v-select
                v-model="form.request.method"
                :items="METHOD_ITEMS"
                variant="outlined"
                density="compact"
                hide-details
              />
              <v-text-field
                v-model.number="form.request.timeout_ms"
                type="number"
                variant="outlined"
                density="compact"
                hide-details
                placeholder="超时(ms)"
              />
            </div>

            <v-text-field
              v-model="form.request.search_url"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="搜索地址模板 *（支持 [keyword] [page]）"
              class="mono-input mb-2"
            />
            <v-textarea
              v-model="headersText"
              variant="outlined"
              density="compact"
              rows="2"
              hide-details
              placeholder='请求头 JSON（可选），如 {"Referer":"..."}'
              class="mono-input"
            />

            <div class="section-label mt-4 mb-1">解析规则（CSS 选择器）</div>
            <v-text-field
              v-model="form.expression_model.group"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="结果条目容器 *（如 .search-item）"
              class="mono-input mb-2"
            />
            <v-text-field
              v-model="form.expression_model.title"
              variant="outlined"
              density="compact"
              hide-details
              placeholder="标题选择器"
              class="mono-input mb-2"
            />
            <div class="grid2">
              <v-text-field
                v-model="form.expression_model.date"
                variant="outlined"
                density="compact"
                hide-details
                placeholder="日期选择器"
                class="mono-input"
              />
              <v-text-field
                v-model="form.expression_model.size"
                variant="outlined"
                density="compact"
                hide-details
                placeholder="大小选择器"
                class="mono-input"
              />
            </div>

            <div class="section-label mt-3 mb-1">链接</div>
            <div class="grid2">
              <v-text-field
                v-model="urlSel"
                variant="outlined"
                density="compact"
                hide-details
                placeholder="链接选择器"
                class="mono-input"
              />
              <v-select
                v-model="urlAttr"
                :items="ATTR_ITEMS"
                variant="outlined"
                density="compact"
                hide-details
              />
            </div>

            <div class="section-label mt-3 mb-1">磁力（留空则使用链接）</div>
            <div class="grid2">
              <v-text-field
                v-model="magnetSel"
                variant="outlined"
                density="compact"
                hide-details
                placeholder="磁力选择器"
                class="mono-input"
              />
              <v-select
                v-model="magnetAttr"
                :items="ATTR_ITEMS"
                variant="outlined"
                density="compact"
                hide-details
              />
            </div>

            <!-- 测试结果 -->
            <div
              v-if="siteTest"
              class="testbox"
              :class="siteTest.ok ? 'testbox--ok' : 'testbox--err'"
            >
              <div class="testbox__head">
                <v-icon
                  :icon="siteTest.ok ? 'mdi-check-circle' : 'mdi-close-circle'"
                  size="16"
                  :color="siteTest.ok ? 'success' : 'error'"
                />
                <span v-if="siteTest.ok">
                  连接成功 · {{ siteTest.items }} 条 · {{ siteTest.elapsed_ms }}ms
                </span>
                <span v-else>连接失败：{{ siteTest.error || '未知错误' }}</span>
              </div>
              <div v-if="siteTest.samples?.length" class="testbox__samples">
                <v-chip
                  v-for="t in siteTest.samples"
                  :key="t"
                  size="x-small"
                  variant="tonal"
                  class="sample"
                >
                  {{ t }}
                </v-chip>
              </div>
            </div>
          </div>

          <v-card-actions class="dlgactions">
            <v-btn
              variant="tonal"
              size="small"
              rounded="pill"
              prepend-icon="mdi-connection"
              :loading="testingSite"
              :disabled="busying || testingSite"
              @click="testSite"
            >
              测试连接
            </v-btn>
            <v-spacer />
            <v-btn variant="text" size="small" @click="addDialog = false">取消</v-btn>
            <v-btn
              color="primary"
              variant="flat"
              size="small"
              rounded="pill"
              :disabled="!form.name || !form.request.search_url || !form.expression_model.group"
              @click="saveSite"
            >
              保存
            </v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>

      <v-snackbar
        v-model="showToast"
        location="bottom"
        color="inverse-surface"
        rounded="lg"
        :timeout="2500"
      >
        {{ toast }}
      </v-snackbar>
    </div>
  </div>
</template>
<style scoped>
/* ---------- 页头 ---------- */
.head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}

.head__text {
  flex: 1 1 auto;
  min-width: 0;
}

.head__title {
  font-size: 1.02rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.head__sub {
  margin-top: 2px;
  font-size: 0.72rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.head--actions {
  justify-content: flex-end;
  margin-bottom: 10px;
}

.head__btn {
  flex-shrink: 0;
  text-transform: none;
  letter-spacing: 0;
}

/* 表单里的等宽输入（选择器 / 模板地址） */
.mono-input :deep(input),
.mono-input :deep(textarea) {
  font-family: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace;
  font-size: 0.78rem;
}

/* 行内开关：压缩 v-switch 默认占位 */
.row__switch {
  flex-shrink: 0;
  margin: 0;
}

.row__switch :deep(.v-switch__track) {
  height: 20px;
  min-height: 20px;
  width: 36px;
  opacity: 1;
}

.row__switch :deep(.v-switch__thumb) {
  height: 14px;
  width: 14px;
}

.row__switch :deep(.v-selection-control) {
  min-height: 24px;
}



/* ---------- 空态 ---------- */
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 5px;
  padding: 44px 16px;
  border-radius: 16px;
  background: rgb(var(--v-theme-surface-container-low));
  border: 1px dashed rgba(var(--v-border-color), calc(var(--v-border-opacity) * 2));
}

.empty__title {
  margin-top: 6px;
  font-size: 0.9rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.empty__text {
  font-size: 0.74rem;
  line-height: 1.55;
  color: rgb(var(--v-theme-on-surface-variant));
}

/* ---------- 列表 ---------- */
.list {
  border-radius: 16px;
  overflow: hidden;
  background: rgb(var(--v-theme-surface-container-low));
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition: opacity 0.18s ease;
}

.row:last-child {
  border-bottom: none;
}

.row--off {
  opacity: 0.58;
}

/* 自绘开关：避免 v-switch 在紧凑行里占位过大 */




.row__body {
  flex: 1 1 auto;
  min-width: 0;
  cursor: pointer;
}

.row__title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.row__name {
  font-size: 0.84rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}




.row__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
  min-width: 0;
}

.row__info {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row__time {
  flex-shrink: 0;
  font-size: 0.62rem;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.8;
  white-space: nowrap;
}



/* ---------- 菜单 ---------- */




/* ---------- FAB ---------- */
.fab {
  position: fixed;
  right: 18px;
  bottom: calc(86px + env(safe-area-inset-bottom));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 54px;
  height: 54px;
  border: none;
  border-radius: 18px;
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
  cursor: pointer;
  z-index: 20;
  transition: filter 0.16s ease, transform 0.16s ease;
}

.fab:hover {
  filter: brightness(1.07);
  transform: translateY(-1px);
}

@media (min-width: 600px) {
  .fab {
    bottom: 26px;
  }
}

/* ---------- 弹窗骨架 ---------- */
.dlghead {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 15px 16px 10px;
}

.dlghead__text {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.dlghead__title {
  font-size: 0.94rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.dlghead__sub {
  margin-top: 2px;
  font-size: 0.68rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.dlgbody {
  padding: 2px 16px 10px;
  max-height: 62vh;
  overflow-y: auto;
}

.dlgbody--tight {
  padding: 16px 16px 6px;
}

.dlgactions {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 16px 15px;
}


/* ---------- 表单控件（原生，避免 Vuetify 输入框在旧内核的高度问题） ---------- */





.grid2 {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 118px);
  gap: 8px;
}

/* 表单内分段按钮 */







.btnrow {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 10px;
}

.hintline {
  margin: -2px 0 4px;
  font-size: 0.66rem;
  line-height: 1.5;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.85;
}

.confirm__text {
  margin-top: 5px;
  font-size: 0.79rem;
  line-height: 1.6;
  color: rgb(var(--v-theme-on-surface-variant));
}

/* ---------- 测试结果 ---------- */
.testbox {
  margin-top: 12px;
  padding: 9px 11px;
  border-radius: 11px;
  font-size: 0.75rem;
  line-height: 1.5;
}

.testbox--ok {
  background: rgba(var(--v-theme-success), 0.1);
  color: rgb(var(--v-theme-on-surface));
}

.testbox--err {
  background: rgba(var(--v-theme-error), 0.1);
  color: rgb(var(--v-theme-on-surface));
}

.testbox__head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
}

.testbox__samples {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 7px;
}

.sample {
  max-width: 100%;
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(var(--v-theme-on-surface), 0.07);
  font-size: 0.66rem;
  color: rgb(var(--v-theme-on-surface-variant));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mt-3 {
  margin-top: 12px;
}
.mt-4 {
  margin-top: 16px;
}
.mb-1 {
  margin-bottom: 4px;
}
</style>
