<script setup lang="ts">
// 全局唯一的轻提示宿主：由 lib/toast.ts 驱动，挂在 App.vue 根部。
// 移动端底部导航高 58px，提示条需抬到导航之上，并额外让出安全区。
import { computed } from 'vue'
import { toastState } from '../lib/toast'

const model = computed({
  get: () => toastState.visible,
  set: (v: boolean) => {
    toastState.visible = v
  },
})

const color = computed(() =>
  toastState.color === 'default' ? 'inverse-surface' : toastState.color,
)
</script>

<template>
  <v-snackbar
    v-model="model"
    class="appsnack"
    :color="color"
    :timeout="toastState.timeout"
    location="bottom"
    rounded="lg"
    elevation="6"
  >
    {{ toastState.text }}
  </v-snackbar>
</template>

<style>
/* 根级样式（非 scoped）：v-snackbar 渲染在 overlay 容器中 */
.appsnack .v-snackbar__wrapper {
  /* 底部安全区 + 移动端底部导航高度，避免被 tab 栏遮挡 */
  margin-bottom: calc(var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)) + 62px);
  margin-left: max(12px, var(--safe-area-inset-left, env(safe-area-inset-left, 0px)));
  margin-right: max(12px, var(--safe-area-inset-right, env(safe-area-inset-right, 0px)));
}

@media (min-width: 600px) {
  .appsnack .v-snackbar__wrapper {
    margin-bottom: calc(var(--safe-area-inset-bottom, env(safe-area-inset-bottom, 0px)) + 18px);
  }
}
</style>
