<script setup lang="ts">
// 搜索源：订阅拉取 / 自定义站点增删改 / 启停开关 / 导入导出 / 重置
// 列表直接平铺展示（不再套一层卡片），每行右侧只有「修改」「删除」两个操作。
// 修改与新增都进入独立的 /site/new 页面填写，避免弹窗内字段拥挤。
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { useSitesStore } from '../stores/sites'
import { settings, DEFAULT_SUBSCRIBE_URL } from '../stores/settings'
import { isTauri } from '../lib/tauri'
import { httpGetText } from '../lib/http'
import { mirrorUrl } from '../lib/mirrors'
import { pushOverlay } from '../lib/overlay'
import { toast } from '../lib/toast'
import type { SiteConfig } from '../types'

const sitesStore = useSitesStore()
const router = useRouter()
const subscribeUrl = ref(settings.get('subscribeUrl'))
const subscribing = ref(false)
const busying = ref(false)

const subscribeDialog = ref(false)
const deleteDialog = ref(false)
const deletingSite = ref<SiteConfig | null>(null)

const enabledCount = computed(() => sitesStore.sites.filter((s) => s.enabled).length)

async function run(fn: () => Promise<void>) {
  busying.value = true
  try {
    await fn()
  } catch (e) {
    toast.error(String(e))
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
      toast.success(`订阅成功：${sites.length} 个搜索源${mirror.base ? `（经 ${mirror.base}）` : ''}`)
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

async function removeSite(site: SiteConfig) {
  await run(async () => {
    await sitesStore.remove(site.id!)
    toast.info(site.is_custom ? '已删除自定义站点' : '已删除搜索源')
  })
}

// 弹层栈注册：手机返回键 / PC ESC 关闭
watch(subscribeDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { subscribeDialog.value = false }))
})
watch(deleteDialog, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { deleteDialog.value = false }))
})

/** 新增：进入独立页面 */
function openAdd() {
  void router.push('/site/new')
}

/** 修改：同样进入独立页面，带上站点 id */
function openEdit(site: SiteConfig) {
  if (!site.id) {
    void router.push('/site/new')
    return
  }
  void router.push({ path: '/site/new', query: { id: site.id } })
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

// ---------- 导入 / 导出 / 重置 ----------
async function doExport() {
  await run(async () => {
    const json = await sitesStore.exportJson()
    if (!isTauri) {
      toast.warning('浏览器预览模式不支持文件导出')
      return
    }
    const path = await save({
      title: '导出站点配置',
      defaultPath: 'dhthub-sites.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!path) return
    await writeTextFile(path, json)
    toast.success('已导出到 ' + path)
  })
}

async function doImport() {
  await run(async () => {
    if (!isTauri) {
      toast.warning('浏览器预览模式不支持文件导入')
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
    toast.success('导入成功')
  })
}

async function doReset() {
  await run(async () => {
    await sitesStore.reset()
    toast.info('已恢复出厂：清空订阅与自定义站点')
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
    sitesStore.loading = false
  }
})
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner sitwrap">
      <div class="head">
        <v-chip size="small" variant="tonal" color="primary" class="head__count">
          启用 {{ enabledCount }} / {{ sitesStore.sites.length }}
        </v-chip>
        <span class="head__spacer" />
        <v-btn
          variant="outlined"
          size="small"
          rounded="pill"
          color="primary"
          prepend-icon="mdi-cloud-download-outline"
          class="pillbtn"
          @click="subscribeDialog = true"
        >
          订阅
        </v-btn>
      </div>

      <v-progress-linear v-if="sitesStore.loading" indeterminate color="primary" rounded height="2" />

      <v-empty-state
        v-if="!sitesStore.loading && sitesStore.sites.length === 0"
        icon="mdi-antenna-off"
        title="还没有任何搜索源"
        text="点击上方「订阅」加载内置订阅源，或用右下角 + 添加自定义站点"
        class="empty"
      />

      <div v-else class="list">
        <div
          v-for="site in sitesStore.sites"
          :key="site.id"
          class="row"
          :class="{ 'row--off': !site.enabled }"
        >
          <v-switch
            :model-value="site.enabled"
            color="primary"
            density="compact"
            size="small"
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

          <div class="row__ops">
            <v-btn
              icon="mdi-pencil-outline"
              variant="text"
              size="small"
              title="修改"
              class="row__op"
              @click.stop="openEdit(site)"
            />
            <v-btn
              icon="mdi-delete-outline"
              variant="text"
              size="small"
              color="error"
              title="删除"
              class="row__op"
              @click.stop="askDelete(site)"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- 悬浮新增按钮 -->
    <button class="fab" type="button" title="添加自定义站点" aria-label="添加自定义站点" @click="openAdd">
      <v-icon icon="mdi-plus" size="26" />
    </button>

    <!-- 订阅源管理 -->
    <v-dialog v-model="subscribeDialog" max-width="520">
      <v-card rounded="lg" class="dlg">
        <div class="dlg__head">
          <span class="dlg__title">订阅源管理</span>
          <v-btn icon="mdi-close" variant="text" size="small" @click="subscribeDialog = false" />
        </div>
        <v-divider />
        <div class="dlg__body">
          <div class="label">订阅仓库地址</div>
          <v-textarea
            v-model="subscribeUrl"
            variant="outlined"
            density="compact"
            rows="3"
            no-resize
            hide-details
            placeholder="https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json"
            class="mono-input"
          />
          <p class="hint">支持 GitHub Raw 直链；将按「设置 → GitHub 镜像」所选前缀代理抓取</p>

          <div class="btnrow">
            <v-btn
              color="primary"
              variant="flat"
              size="small"
              rounded="pill"
              class="pillbtn"
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
              class="pillbtn"
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
              class="pillbtn"
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
              class="pillbtn"
              color="error"
              prepend-icon="mdi-restore"
              :disabled="busying"
              @click="doReset"
            >
              重置
            </v-btn>
          </div>
        </div>
      </v-card>
    </v-dialog>

    <!-- 删除确认 -->
    <v-dialog v-model="deleteDialog" max-width="340">
      <v-card rounded="lg" class="dlg">
        <div class="dlg__body">
          <div class="dlg__title">删除搜索源</div>
          <p class="confirm">确定删除「{{ deletingSite?.name }}」吗？删除后需重新订阅或添加。</p>
        </div>
        <div class="dlg__actions">
          <v-btn variant="text" size="small" class="pillbtn" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" variant="flat" size="small" rounded="pill" class="pillbtn" @click="confirmDelete">
            删除
          </v-btn>
        </div>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.sitwrap {
  padding-bottom: calc(96px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
}

.head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}

.head__count {
  font-weight: 600;
}

.head__spacer {
  flex: 1 1 auto;
}

/* ---------- 列表（平铺，不再套卡片） ---------- */
.list {
  display: flex;
  flex-direction: column;
  border-radius: 14px;
  overflow: hidden;
  background: rgb(var(--v-theme-surface-container-low));
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 8px 9px 6px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition: opacity 0.18s ease;
}

.row:last-child {
  border-bottom: none;
}

.row--off {
  opacity: 0.55;
}

.row__switch {
  flex-shrink: 0;
  margin: 0;
}

.row__switch :deep(.v-selection-control) {
  min-height: 40px;
}

.row__body {
  flex: 1 1 auto;
  min-width: 0;
  cursor: pointer;
  padding: 2px 0;
}

.row__title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.row__name {
  font-size: 0.83rem;
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
  font-size: 0.67rem;
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

.row__ops {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 0;
}

.row__op {
  margin: 0;
}

/* ---------- 悬浮按钮 ---------- */
.fab {
  position: fixed;
  right: max(16px, var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
  bottom: calc(74px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
  z-index: 30;
  width: 52px;
  height: 52px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 50%;
  color: rgb(var(--v-theme-on-primary));
  background: rgb(var(--v-theme-primary));
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.28);
  cursor: pointer;
  transition: transform 0.14s ease;
}

.fab:active {
  transform: scale(0.94);
}

/* ---------- 弹窗 ---------- */
.dlg {
  background: rgb(var(--v-theme-surface)) !important;
}

.dlg__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 10px 10px 16px;
}

.dlg__title {
  font-size: 0.94rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.dlg__body {
  padding: 12px 16px;
}

.dlg__actions {
  display: flex;
  justify-content: flex-end;
  gap: 7px;
  padding: 4px 12px 12px;
}

.label {
  margin-bottom: 5px;
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: rgb(var(--v-theme-primary));
}

.hint {
  margin: 6px 2px 0;
  font-size: 0.66rem;
  line-height: 1.5;
  color: rgb(var(--v-theme-on-surface-variant));
}

.btnrow {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 12px;
}

.confirm {
  margin-top: 6px;
  font-size: 0.79rem;
  line-height: 1.6;
  color: rgb(var(--v-theme-on-surface-variant));
}

.mono-input :deep(input),
.mono-input :deep(textarea) {
  font-family: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace;
  font-size: 0.75rem;
}

.pillbtn {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 600;
}

.empty {
  margin-top: 12vh;
}
</style>
