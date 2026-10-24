// 更新检测（纯前端）：直连 GitHub releases/latest → 解析最新版本号，
// 结果用全局 snackbar 提示（v-snackbar，无弹窗）；网络走 plugin-http + 显式超时，不转圈。

import { ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { isTauri } from './tauri'
import { httpGetRaw } from './http'


export const RELEASES_PAGE_URL = 'https://github.com/icenfn/DHThub/releases/latest'
export const UPDATE_TIMEOUT_MS = 15000

/** 全局更新提示状态（App.vue 渲染 snackbar，任意页面可触发） */
export const updateToast = ref(false)
export const updateMsg = ref('')
export const updateHasNew = ref(false)
export const updateChecking = ref(false)

/** 从 URL 或页面标题解析版本号（支持重定向后 /releases/tag/vX.Y.Z 与镜像站地址） */
function extractVersion(text: string, finalUrl: string): string {
  const fromUrl = finalUrl
    .split('/')
    .filter((s) => /^v?\d+\.\d+/.test(s))
    .pop()
  if (fromUrl) return fromUrl.replace(/^v/, '')
  const m = text.match(/Release\s+v?(\d+\.\d+(?:\.\d+)?)/i)
  return m ? m[1] : ''
}

/** 简单版本比较（x.y.z） */
function versionGt(a: string, b: string): boolean {
  const pa = a.split('.').map(Number)
  const pb = b.split('.').map(Number)
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = pa[i] ?? 0
    const y = pb[i] ?? 0
    if (x !== y) return x > y
  }
  return false
}

/** 检查更新（自动检测与手动按钮共用），结果通过 snackbar 展示 */
export async function checkUpdate(): Promise<void> {
  if (!isTauri) return
  if (updateChecking.value) return
  updateChecking.value = true
  try {
    // 直连 GitHub，不套用镜像（releases/latest 会 302 到 tag 页面，plugin-http 自动跟随）
    const res = await httpGetRaw(RELEASES_PAGE_URL, { timeoutMs: UPDATE_TIMEOUT_MS })
    const finalUrl = res.url || RELEASES_PAGE_URL
    const text = await res.text()
    const latest = extractVersion(text, finalUrl)
    const current = await getCurrentVersion()
    if (!latest) throw new Error('无法解析最新版本号')
    if (versionGt(latest, current)) {
      updateHasNew.value = true
      updateMsg.value = `发现新版本 v${latest}（当前 v${current}），点击查看下载`
    } else {
      updateHasNew.value = false
      updateMsg.value = `已是最新版本 v${current}`
    }
  } catch (e) {
    updateHasNew.value = false
    updateMsg.value = `检查更新失败：${e instanceof Error ? e.message : String(e)}`
  } finally {
    updateChecking.value = false
    updateToast.value = true
  }
}

async function getCurrentVersion(): Promise<string> {
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    return await getVersion()
  } catch {
    return '0.0.0'
  }
}

/** 打开 GitHub Releases latest 页面 */
export async function openReleasePage(): Promise<void> {
  try {
    await openUrl(RELEASES_PAGE_URL)
  } catch {
    window.open(RELEASES_PAGE_URL, '_blank')
  }
}
