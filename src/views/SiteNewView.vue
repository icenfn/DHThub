<script setup lang="ts">
// 添加自定义搜索源：独立的整页表单（不再使用弹窗）
// 移动端整页填写比弹窗更好用，也更适合字段较多的表达式配置。
import { computed, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useSitesStore } from '../stores/sites'
import type { SiteConfig } from '../types'

const router = useRouter()
const sitesStore = useSitesStore()

const METHOD_ITEMS = ['GET', 'POST']
const ATTR_ITEMS = ['href', 'text', 'src', 'data-clipboard-text']

const form = reactive<SiteConfig>({
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
})

const urlSel = ref('')
const urlAttr = ref('href')
const magnetSel = ref('')
const magnetAttr = ref('href')
const headersText = ref('')

const saving = ref(false)
const err = ref('')

const canSave = computed(() => !!form.name.trim() && !!form.request.search_url.trim())

function back() {
  router.back()
}

async function save() {
  err.value = ''
  if (!form.name.trim()) {
    err.value = '请填写搜索源名称'
    return
  }
  if (!form.request.search_url.trim()) {
    err.value = '请填写搜索地址（需包含 {keyword} 占位符）'
    return
  }
  // 请求头 JSON 校验：格式错误直接提示，不静默丢弃
  let headers: Record<string, string> = {}
  if (headersText.value.trim()) {
    try {
      const parsed = JSON.parse(headersText.value)
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('bad')
      headers = parsed as Record<string, string>
    } catch {
      err.value = '请求头必须是合法的 JSON 对象'
      return
    }
  }
  saving.value = true
  try {
    const site: SiteConfig = {
      ...form,
      request: { ...form.request, headers },
      expression_model: {
        ...form.expression_model,
        url: urlSel.value ? { sel: urlSel.value, attr: urlAttr.value } : { sel: '', attr: 'href' },
        magnet: magnetSel.value
          ? { sel: magnetSel.value, attr: magnetAttr.value }
          : { sel: '', attr: 'href' },
      },
    }
    await sitesStore.addCustom(site)
    router.back()
  } catch (e) {
    err.value = String(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner">
      <!-- 顶部：返回 + 标题 + 保存 -->
      <div class="nf-bar">
        <v-btn icon="mdi-arrow-left" variant="text" size="small" title="返回" @click="back" />
        <span class="nf-bar__title">添加自定义站点</span>
        <v-btn
          variant="text"
          color="primary"
          size="small"
          class="nf-bar__save"
          :loading="saving"
          :disabled="!canSave"
          @click="save"
        >
          保存
        </v-btn>
      </div>

      <v-alert v-if="err" type="error" variant="tonal" density="compact" class="mb-3">
        {{ err }}
      </v-alert>

      <!-- 基本信息 -->
      <section class="grp">
        <div class="grp__title">基本信息</div>
        <v-text-field
          v-model="form.name"
          label="站点名称"
          placeholder="例如：我的磁力站"
          variant="outlined"
          density="comfortable"
          rounded="lg"
          hide-details="auto"
          class="mb-3"
        />
        <v-text-field
          v-model="form.info"
          label="备注（可选）"
          placeholder="简单描述这个站点"
          variant="outlined"
          density="comfortable"
          rounded="lg"
          hide-details="auto"
        />
      </section>

      <!-- 请求配置 -->
      <section class="grp">
        <div class="grp__title">请求配置</div>
        <v-text-field
          v-model="form.request.search_url"
          label="搜索地址"
          placeholder="https://example.com/search?q={keyword}"
          hint="用 {keyword} 表示关键词占位符"
          persistent-hint
          variant="outlined"
          density="comfortable"
          rounded="lg"
          class="mb-4"
        />
        <div class="row">
          <div class="row__label">
            <span class="row__title">请求方式</span>
            <span class="row__sub">多数站点使用 GET</span>
          </div>
          <v-select
            v-model="form.request.method"
            :items="METHOD_ITEMS"
            density="compact"
            variant="outlined"
            rounded="lg"
            hide-details
            class="row__ctrl"
          />
        </div>
        <div class="row">
          <div class="row__label">
            <span class="row__title">超时时间</span>
            <span class="row__sub">单位毫秒，建议 8000–20000</span>
          </div>
          <v-text-field
            v-model.number="form.request.timeout_ms"
            type="number"
            density="compact"
            variant="outlined"
            rounded="lg"
            hide-details
            class="row__ctrl"
          />
        </div>
        <v-textarea
          v-model="headersText"
          label="请求头（可选，JSON）"
          placeholder='{"User-Agent": "Mozilla/5.0"}'
          variant="outlined"
          density="comfortable"
          rounded="lg"
          rows="2"
          auto-grow
          hide-details="auto"
          class="mt-3 mono"
        />
      </section>

      <!-- 结果字段 -->
      <section class="grp">
        <div class="grp__title">结果字段选择器</div>
        <v-text-field
          v-model="form.expression_model.title"
          label="标题选择器"
          placeholder=".item .title"
          variant="outlined"
          density="comfortable"
          rounded="lg"
          hide-details="auto"
          class="mb-3 mono"
        />
        <div class="two">
          <v-text-field
            v-model="form.expression_model.date"
            label="日期选择器"
            placeholder=".item .date"
            variant="outlined"
            density="comfortable"
            rounded="lg"
            hide-details="auto"
            class="mono"
          />
          <v-text-field
            v-model="form.expression_model.size"
            label="大小选择器"
            placeholder=".item .size"
            variant="outlined"
            density="comfortable"
            rounded="lg"
            hide-details="auto"
            class="mono"
          />
        </div>
      </section>

      <!-- 链接与磁力 -->
      <section class="grp">
        <div class="grp__title">链接与磁力</div>
        <div class="two">
          <v-text-field
            v-model="urlSel"
            label="详情链接选择器"
            placeholder=".item a"
            variant="outlined"
            density="comfortable"
            rounded="lg"
            hide-details="auto"
            class="mono"
          />
          <v-select
            v-model="urlAttr"
            :items="ATTR_ITEMS"
            label="取属性"
            variant="outlined"
            density="comfortable"
            rounded="lg"
            hide-details
          />
        </div>
        <div class="two mt-3">
          <v-text-field
            v-model="magnetSel"
            label="磁力选择器（可选）"
            placeholder="a[href^='magnet:']"
            variant="outlined"
            density="comfortable"
            rounded="lg"
            hide-details="auto"
            class="mono"
          />
          <v-select
            v-model="magnetAttr"
            :items="ATTR_ITEMS"
            label="取属性"
            variant="outlined"
            density="comfortable"
            rounded="lg"
            hide-details
          />
        </div>
        <p class="tip">
          若列表页不直接给出磁力链接（磁力在详情页里），只需填好「详情链接选择器」，
          应用会在打开资源详情时回源解析真实磁力。
        </p>
      </section>

      <v-btn
        block
        color="primary"
        rounded="pill"
        class="savebtn"
        :loading="saving"
        :disabled="!canSave"
        @click="save"
      >
        保存站点
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
.page-inner {
  padding-bottom: calc(24px + var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)));
}

.nf-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 12px;
}

.nf-bar__title {
  flex: 1 1 auto;
  font-size: 0.98rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.nf-bar__save {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 700;
}

.grp {
  padding: 12px 0 14px;
  border-bottom: 1px solid rgba(var(--v-border-color), calc(var(--v-border-opacity) * 0.75));
}

.grp:last-of-type {
  border-bottom: 0;
}

.grp__title {
  margin-bottom: 10px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: rgb(var(--v-theme-primary));
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 7px 0;
}

.row__label {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.row__title {
  font-size: 0.84rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.row__sub {
  font-size: 0.68rem;
  line-height: 1.4;
  color: rgb(var(--v-theme-on-surface-variant));
}

.row__ctrl {
  flex: 0 0 auto;
  width: 130px;
  max-width: 46%;
}

.two {
  display: grid;
  grid-template-columns: 1fr 96px;
  gap: 8px;
  align-items: start;
}

.tip {
  margin: 10px 0 0;
  font-size: 0.7rem;
  line-height: 1.6;
  color: rgb(var(--v-theme-on-surface-variant));
}

.savebtn {
  margin-top: 16px;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 700;
}

.mono :deep(input),
.mono :deep(textarea) {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.78rem;
}
</style>
