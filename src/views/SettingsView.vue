<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { settings } from '../stores/settings'
import { useSitesStore } from '../stores/sites'
import { invoke, isTauri } from '../lib/tauri'
import UpdateDialog from '../components/UpdateDialog.vue'

const sitesStore = useSitesStore()
const themeMode = ref<'system' | 'light' | 'dark'>('system')
const hotWordInput = ref('')
const hotWords = ref<string[]>([])
const toast = ref('')
const showToast = ref(false)
const updateOpen = ref(false)
const clearing = ref(false)

function notice(msg: string) {
  toast.value = msg
  showToast.value = true
}

async function saveTheme(v: 'system' | 'light' | 'dark') {
  themeMode.value = v
  await settings.set('theme', v)
}

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

onMounted(async () => {
  await settings.ready()
  themeMode.value = settings.get('theme')
  hotWords.value = [...settings.get('hotWords')]
  await sitesStore.load().catch(() => undefined)
})

const subscribeUrl = computed(() => settings.get('subscribeUrl'))
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 1040px">
    <div class="d-flex align-center mt-2 mb-4">
      <div>
        <div class="text-h6 font-weight-bold">设置</div>
        <div class="text-caption text-medium-emphasis">外观、更新、数据与热词管理</div>
      </div>
      <v-spacer />
      <v-btn color="primary" variant="flat" prepend-icon="mdi-update" @click="updateOpen = true">
        检查更新
      </v-btn>
    </div>

    <v-card rounded="lg" class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="primary" variant="tonal"><v-icon icon="mdi-theme-light-dark" /></v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">外观（B7）</v-card-title>
      </v-card-item>
      <v-card-text>
        <v-radio-group v-model="themeMode" inline hide-details @update:model-value="saveTheme(themeMode)">
          <v-radio label="跟随系统" value="system" />
          <v-radio label="浅色" value="light" />
          <v-radio label="深色" value="dark" />
        </v-radio-group>
      </v-card-text>
    </v-card>

    <v-card rounded="lg" class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="secondary" variant="tonal"><v-icon icon="mdi-fire" /></v-avatar>
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

    <v-card rounded="lg" class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="warning" variant="tonal"><v-icon icon="mdi-database-cog-outline" /></v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">数据</v-card-title>
      </v-card-item>
      <v-card-text class="d-flex flex-wrap ga-2">
        <v-btn variant="tonal" color="error" prepend-icon="mdi-delete-sweep-outline" :loading="clearing" @click="clearHistory">
          清空全部历史（磁力/复制/浏览）
        </v-btn>
        <v-btn variant="tonal" color="error" prepend-icon="mdi-history" @click="clearSearchHistory">
          清空搜索历史
        </v-btn>
        <v-btn variant="tonal" color="warning" prepend-icon="mdi-antenna-off" @click="sitesStore.reset(); notice('站点已重置')">
          重置站点订阅
        </v-btn>
      </v-card-text>
    </v-card>

    <v-card rounded="lg" class="mb-4">
      <v-card-item>
        <template #prepend>
          <v-avatar color="info" variant="tonal"><v-icon icon="mdi-link-variant" /></v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">订阅源</v-card-title>
        <v-card-subtitle class="text-caption">当前订阅仓库地址（可在「站点管理」中修改）</v-card-subtitle>
      </v-card-item>
      <v-card-text class="text-body-2 font-family-monospace text-caption">
        {{ subscribeUrl || '未设置' }}
      </v-card-text>
    </v-card>

    <v-card rounded="lg">
      <v-card-item>
        <template #prepend>
          <v-avatar color="success" variant="tonal"><v-icon icon="mdi-information-outline" /></v-avatar>
        </template>
        <v-card-title class="text-subtitle-1 font-weight-bold">关于与法律</v-card-title>
      </v-card-item>
      <v-list density="compact">
        <v-list-item prepend-icon="mdi-shield-check-outline" title="使用协议" @click="$router.push('/agreement')" />
        <v-list-item prepend-icon="mdi-file-document-outline" title="免责声明" @click="$router.push('/disclaimer')" />
        <v-list-item prepend-icon="mdi-github" title="GitHub 仓库" subtitle="github.com/icenfn/DHThub" @click="$router.push('/about')" />
      </v-list>
    </v-card>

    <UpdateDialog v-model="updateOpen" />
    <v-snackbar v-model="showToast" location="bottom" color="success" timeout="2200">
      {{ toast }}
    </v-snackbar>
  </div>
</template>
