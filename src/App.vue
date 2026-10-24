<script setup lang="ts">
// 应用外壳：仅承载全局逻辑（主题 / 更新提示 / 返回键），页面框架由各路由页面自行提供
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { useMediaQuery } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import { settings } from './stores/settings'
import { isTauri } from './lib/tauri'
import { closeTopOverlay, pushOverlay } from './lib/overlay'
import { checkUpdate, openReleasePage, updateHasNew, updateMsg, updateToast } from './lib/update'

// 有更新 -> 弹窗通知；无更新/失败 -> snackbar 轻提示
const updateDialog = ref(false)
const updateSnackbar = ref(false)

watch([updateToast, updateHasNew], () => {
  if (!updateToast.value) return
  if (updateHasNew.value) {
    updateDialog.value = true
  } else {
    updateSnackbar.value = true
  }
  updateToast.value = false
})

const theme = useTheme()
const router = useRouter()

// 弹层返回键支持：PC ESC 关闭最上层弹层（Vuetify 自身的 ESC 关闭仍生效）；
// Android 硬件返回键由原生 MainActivity 派发 android:back 事件
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && closeTopOverlay()) {
    e.preventDefault()
  }
}
let onAndroidBack: (() => void) | null = null
function setupAndroidBack() {
  if (!isTauri) return
  const handler = async () => {
    if (closeTopOverlay()) return
    if (window.history.length > 1) {
      router.back()
      return
    }
    try {
      await invoke('exit_app')
    } catch {
      /* 桌面端无此命令时忽略 */
    }
  }
  window.addEventListener('android:back', handler)
  onAndroidBack = () => window.removeEventListener('android:back', handler)
}
onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  setupAndroidBack()
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  onAndroidBack?.()
})

// 更新弹窗本身也注册到弹层栈（关闭时自动注销）
watch(updateDialog, (v, _o, onCleanup) => {
  if (v) {
    const off = pushOverlay(() => {
      updateDialog.value = false
    })
    onCleanup(off)
  }
})

const prefersDark = useMediaQuery('(prefers-color-scheme: dark)')

async function applyTheme() {
  await settings.ready()
  const mode = settings.get('theme')
  if (mode === 'system') {
    theme.change(prefersDark.value ? 'dark' : 'light')
  } else {
    theme.change(mode)
  }
}

watch(prefersDark, (v) => {
  if (settings.get('theme') === 'system') theme.change(v ? 'dark' : 'light')
})

watch(
  () => settings.get('theme'),
  () => applyTheme(),
)

onMounted(async () => {
  await settings.ready()
  applyTheme()
  // 自动检测更新：启动即静默检查（snackbar 提示）
  if (isTauri && settings.get('autoCheckUpdate')) {
    void checkUpdate()
  }
  // 浏览器预览提示
  if (!isTauri) {
    console.info('[DHThub] 浏览器预览模式：搜索/站点功能需在 Tauri 应用中运行')
  }
})
</script>

<template>
  <v-app>
    <!--
      保持「搜索结果」页面缓存：从资源详情返回时不再重新请求，
      列表数据、滚动、筛选与排序状态全部保留。
      资源详情页本身不缓存（每次进入都是新的链接）。
    -->
    <router-view v-slot="{ Component }">
      <keep-alive :include="['SearchResultsView']">
        <component :is="Component" />
      </keep-alive>
    </router-view>

    <!-- 发现新版本弹窗 -->
    <v-dialog v-model="updateDialog" max-width="420">
      <v-card rounded="xl" variant="flat" color="surface-container-low" border>
        <div class="dlg-head">
          <v-avatar color="primary-container" rounded="lg" size="40">
            <v-icon icon="mdi-update" color="on-primary-container" size="21" />
          </v-avatar>
          <div class="dlg-head__text">
            <div class="text-title-small font-weight-bold">发现新版本</div>
            <div class="text-label-small text-medium-emphasis">更新已就绪，前往发布页获取</div>
          </div>
        </div>
        <div class="dlg-body text-body-medium">{{ updateMsg }}</div>
        <div class="dlg-actions">
          <v-btn variant="text" size="small" @click="updateDialog = false">稍后</v-btn>
          <v-btn
            color="primary"
            variant="flat"
            size="small"
            rounded="pill"
            prepend-icon="mdi-download"
            @click="openReleasePage"
          >
            前往下载
          </v-btn>
        </div>
      </v-card>
    </v-dialog>

    <!-- 无更新 / 检查失败：snackbar 轻提示 -->
    <v-snackbar
      v-model="updateSnackbar"
      location="bottom"
      multi-line
      :timeout="2500"
      rounded="lg"
      color="inverse-surface"
    >
      <div class="d-flex align-center ga-2">
        <v-icon icon="mdi-information-outline" color="primary" size="18" />
        <span class="text-body-small">{{ updateMsg }}</span>
      </div>
    </v-snackbar>
  </v-app>
</template>

<style scoped>
.dlg-head {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 18px 8px;
}

.dlg-head__text {
  display: flex;
  flex-direction: column;
  line-height: 1.25;
  min-width: 0;
}

.dlg-body {
  padding: 4px 18px 14px;
  color: rgb(var(--v-theme-on-surface-variant));
}

.dlg-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 14px 14px;
}
</style>
