import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'home', component: HomeView, meta: { title: '搜索' } },
    { path: '/sites', name: 'sites', component: () => import('../views/SitesView.vue'), meta: { title: '站点管理' } },
    { path: '/history', name: 'history', component: () => import('../views/HistoryView.vue'), meta: { title: '历史记录' } },
    { path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue'), meta: { title: '设置' } },
    { path: '/about', name: 'about', component: () => import('../views/AboutView.vue'), meta: { title: '关于' } },
    { path: '/agreement', name: 'agreement', component: () => import('../views/AgreementView.vue'), meta: { title: '使用协议' } },
    { path: '/disclaimer', name: 'disclaimer', component: () => import('../views/DisclaimerView.vue'), meta: { title: '免责声明' } },
  ],
})

router.afterEach((to) => {
  document.title = to.meta.title ? `${String(to.meta.title)} · DHThub` : 'DHThub'
})

export default router
