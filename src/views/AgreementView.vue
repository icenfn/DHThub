<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>

<template>
  <div class="px-4 px-sm-8 py-4 mx-auto" style="max-width: 760px">
    <div class="d-flex align-center mt-2 mb-3">
      <v-btn variant="text" icon="mdi-arrow-left" @click="router.back()" />
      <div class="text-h6 font-weight-bold">使用协议</div>
    </div>
    <v-card rounded="lg">
      <v-card-text class="text-body-2">
        <p><strong>DHThub 使用协议</strong></p>
        <p class="mt-2">1. 本应用（以下简称「本软件」）是一款基于开源协议发布的磁力链接聚合搜索工具，仅用于个人技术学习、资料检索与合法信息访问。</p>
        <p class="mt-1">2. 本软件的搜索能力来自第三方公开网页，搜索结果由第三方来源站点实时生成。本软件不存储、不缓存、不托管任何资源文件内容，亦不对搜索结果的内容、完整性与可用性作出任何担保。</p>
        <p class="mt-1">3. 您承诺遵守所在地法律法规，仅检索、传播与下载您拥有合法权利的内容。因使用本软件产生的任何直接或间接法律风险与责任，均由使用者自行承担。</p>
        <p class="mt-1">4. 站点订阅源由使用者自行配置与维护。本软件对订阅源中的站点名单、规则与内容不承担审核、担保或连带责任。</p>
        <p class="mt-1">5. 本软件不包含任何广告、商业追踪或用户画像采集。软件更新检测仅访问 GitHub 官方接口。</p>
        <p class="mt-1">6. 本软件按「现状」提供，作者不对任何功能缺陷或服务中断承担责任。使用即视为同意本协议。</p>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" variant="flat" @click="router.push('/')">返回</v-btn>
      </v-card-actions>
    </v-card>
  </div>
</template>
