<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { pushOverlay } from '../lib/overlay'
import type { SiteOutcome } from '../types'

const model = defineModel<boolean>({ required: true })

// 弹层栈注册：手机返回键 / PC ESC 关闭
watch(model, (v, _o, onCleanup) => {
  if (v) onCleanup(pushOverlay(() => { model.value = false }))
})

function fmt(ms: number) {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)}s` : `${ms}ms`
}

const { outcomes, keyword } = defineProps<{
  outcomes: SiteOutcome[]
  keyword: string
}>()

const totalElapsed = computed(() => outcomes.reduce((a, b) => a + b.elapsed_ms, 0))
const successCount = computed(() => outcomes.filter((o) => o.success).length)

async function copyStats() {
  const lines = outcomes.map(
    (o, i) =>
      `${i + 1}. ${o.site_name} [${o.success ? '成功' : '失败'}] ${o.items.length}条 ${fmt(o.elapsed_ms)}${o.error ? ` ${o.error}` : ''}`,
  )
  const text = `DHThub 搜索统计（${keyword}）：${successCount.value}/${outcomes.length} 站成功，${outcomes.reduce((a, o) => a + o.items.length, 0)} 条结果，总耗时 ${fmt(totalElapsed.value)}\n${lines.join('\n')}`
  try {
    const { writeText } = await import('@tauri-apps/plugin-clipboard-manager')
    await writeText(text)
  } catch {
    /* 浏览器预览忽略 */
  }
}
</script>

<template>
  <v-dialog v-model="model" max-width="560">
    <v-card>
      <v-card-title class="text-subtitle-1 font-weight-bold">
        <v-icon icon="mdi-chart-bar" color="primary" class="mr-2" />搜索统计（B2）
      </v-card-title>
      <v-divider />
      <v-card-text>
        <p class="text-body-2 mb-3">
          关键词：<strong>{{ keyword }}</strong>
          <span class="text-medium-emphasis">（{{ successCount }}/{{ outcomes.length }} 站成功 · 总耗时 {{ fmt(totalElapsed) }}）</span>
        </p>
        <v-table density="compact">
          <thead>
            <tr>
              <th>#</th>
              <th>站点</th>
              <th>结果数</th>
              <th>耗时</th>
              <th>状态</th>
              <th>错误信息</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(o, i) in outcomes" :key="o.site_id">
              <td>{{ i + 1 }}</td>
              <td>{{ o.site_name }}</td>
              <td>{{ o.items.length }}</td>
              <td>{{ fmt(o.elapsed_ms) }}</td>
              <td>
                <v-chip :color="o.success ? 'success' : 'error'" size="x-small" variant="tonal">
                  {{ o.success ? '成功' : '失败' }}
                </v-chip>
              </td>
              <td class="text-caption text-medium-emphasis" style="max-width: 220px">
                <span class="text-truncate d-block" :title="o.error || ''">{{ o.success ? '—' : o.error }}</span>
              </td>
            </tr>
          </tbody>
        </v-table>
        <div v-if="outcomes.length === 0" class="text-center text-medium-emphasis mt-4">暂无统计数据</div>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="tonal" color="secondary" prepend-icon="mdi-content-copy" @click="copyStats">复制统计</v-btn>
        <v-btn color="primary" variant="flat" @click="model = false">关闭</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
