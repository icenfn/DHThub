<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 760px">
    <div class="d-flex align-center mt-2 mb-3">
      <v-btn variant="text" icon="mdi-arrow-left" @click="router.back()" />
      <div class="text-h6 font-weight-bold">免责声明</div>
    </div>
    <v-card rounded="lg">
      <v-card-text class="text-body-2">
        <p><strong>免责声明</strong></p>
        <p class="mt-2">1. DHThub 仅提供技术检索能力，不对任何搜索结果指向的内容负责。您访问、下载或传播的任何内容，其版权与合法性由您自行确认并承担全部责任。</p>
        <p class="mt-1">2. 搜索结果来自第三方站点，可能包含不准确、过期或违反您所在地法律的信息。请勿在未确认权利归属的情况下传播或下载受版权保护的作品。</p>
        <p class="mt-1">3. 订阅源由社区或使用者提供，DHThub 无法保证订阅源站点长期可用，也不对订阅源内容的合规性作任何担保。</p>
        <p class="mt-1">4. 若您所在国家或地区禁止使用本类工具，请立即停止使用。使用本软件即表示您已阅读并理解本免责声明。</p>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" variant="flat" @click="router.push('/')">返回</v-btn>
      </v-card-actions>
    </v-card>
  </div>
</template>
