<script setup lang="ts">
import type { SiteOutcome } from '../types'

defineProps<{
  outcomes: SiteOutcome[]
  keyword: string
}>()

const model = defineModel<boolean>({ required: true })

function fmt(ms: number) {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)}s` : `${ms}ms`
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
          <span class="text-medium-emphasis">（{{ outcomes.filter((o) => o.success).length }}/{{ outcomes.length }} 站成功）</span>
        </p>
        <v-table density="compact">
          <thead>
            <tr>
              <th>#</th>
              <th>站点</th>
              <th>结果数</th>
              <th>耗时</th>
              <th>状态</th>
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
            </tr>
          </tbody>
        </v-table>
        <div v-if="outcomes.length === 0" class="text-center text-medium-emphasis mt-4">暂无统计数据</div>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" variant="flat" @click="model = false">关闭</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
