import { createRouter, createWebHistory } from 'vue-router'
import AppLayout from '../components/AppLayout.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      // 首页框架：底部 tab 内嵌 4 页（搜索 / 搜索源 / 历史 / 设置），左右滑动切换。
      // 设置不再是独立路由，作为 swiper 的第 4 页嵌入。
      path: '/',
      component: AppLayout,
    },
    // 搜索结果独立页面（自带返回顶栏，不走首页框架）
    {
      path: '/search',
      name: 'search',
      component: () => import('../views/SearchResultsView.vue'),
      meta: { title: '搜索结果' },
    },
    // 资源详情独立页面（由搜索结果点击进入）
    {
      path: '/res',
      name: 'resource',
      component: () => import('../views/ResourceDetailView.vue'),
      meta: { title: '资源详情' },
    },
  ],
})

router.afterEach((to) => {
  document.title = to.meta.title ? `${String(to.meta.title)} · DHThub` : 'DHThub'
})

export default router
