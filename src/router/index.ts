import { createRouter, createWebHistory } from 'vue-router'
import AppLayout from '../components/AppLayout.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      // 首页框架：顶栏（标题随 tab 变化 + 设置入口）+ 滑动窗口内嵌 3 页（搜索/站点/历史）
      path: '/',
      component: AppLayout,
    },
    // 搜索结果独立页面（自带返回顶栏，不走首页框架）
    { path: '/search', name: 'search', component: () => import('../views/SearchResultsView.vue'), meta: { title: '搜索结果' } },
    // 设置页独立框架
    { path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue'), meta: { title: '设置' } },
  ],
})

router.afterEach((to) => {
  document.title = to.meta.title ? `${String(to.meta.title)} · DHThub` : 'DHThub'
})

export default router
