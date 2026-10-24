// 运行环境兼容层：必须在其它模块之前求值，因此作为入口第一段导入。
//
// 1) core-js polyfill
//    Android WebView 91（Chromium 91）缺失部分 ES2018+/ES2021 API
//    （Object.fromEntries / Array.prototype.flat / String.prototype.replaceAll /
//     Promise.allSettled / structuredClone 等），Vuetify 4 与 @vueuse/core 会用到。
//    这里显式引入所需命名空间，而不是 'core-js/stable' 全量导入，
//    控制在可接受的包体积内；@vitejs/plugin-legacy 负责更老内核的整包兜底。
import 'core-js/es/symbol'
import 'core-js/es/object'
import 'core-js/es/function'
import 'core-js/es/array'
import 'core-js/es/string'
import 'core-js/es/number'
import 'core-js/es/math'
import 'core-js/es/json'
import 'core-js/es/promise'
import 'core-js/es/reflect'
import 'core-js/es/map'
import 'core-js/es/set'
import 'core-js/es/weak-map'
import 'core-js/es/weak-set'
import 'core-js/web/url'
import 'core-js/web/url-search-params'
import 'core-js/web/dom-collections'
import 'core-js/web/timers'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import vuetify from './plugins/vuetify'
import { installCascadeOrderCompat } from './compat/legacy-css'
import { vViewer } from 'v-viewer'
// 安全区 / 输入法内边距桥接：须在首帧前安装，避免启动瞬间内容被系统栏或键盘遮住
import { installInsetsBridge } from './lib/insets'
// 视口高度桥接：旧内核不支持 dvh，注入 --app-height 像素高度
import { installViewportBridge } from './lib/viewport'
// 截图预览（v-viewer / viewerjs）样式：必须在全局样式之前引入
import 'v-viewer/style.css'
// 全局设计系统样式（须在 vuetify/styles 之后，才能覆盖其默认值）
import './styles/main.scss'

// 2) CSS 级联层兼容：Vuetify 4 的运行时主题表带 @layer，
//    Chromium 91 会整段丢弃（--v-theme-* 全部失效）；新版引擎则出现
//    「未分层规则压过分层规则」的主题色失效问题。统一展平并置后。
installCascadeOrderCompat()
// 原生注入 --safe-area-inset-* / --keyboard-height；浏览器下自动降级为 env() 兜底
installInsetsBridge()
installViewportBridge()

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(vuetify)
// v-viewer：截图缩略图点击后由它接管为大图预览（缩放 / 拖动 / 左右切换）
// 注意 vViewer 是「指令」而非插件，必须用 app.directive 注册
app.directive('viewer', vViewer)
app.mount('#app')
