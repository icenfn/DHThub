<script setup lang="ts">
// 下载页：占位页面（底部 tab 的第二组第一项）。
// 当前版本尚未接入下载能力，这里给出空态说明，避免出现空白页。
const emit = defineEmits<{ (e: 'go-sites'): void }>()
</script>

<template>
  <div class="page-scroll">
    <div class="page-inner">
      <v-empty-state
        icon="mdi-download-outline"
        title="暂无下载任务"
        text="当前版本还没有接入下载能力，后续会在这里展示进行中与已完成的下载。"
        class="empty"
      >
        <template #actions>
          <v-btn
            variant="tonal"
            rounded="pill"
            size="small"
            prepend-icon="mdi-antenna"
            @click="emit('go-sites')"
          >
            管理搜索源
          </v-btn>
        </template>
      </v-empty-state>
    </div>
  </div>
</template>

<style scoped>
.empty {
  margin-top: 18vh;
}
</style>
