import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import legacy from '@vitejs/plugin-legacy'

// Tauri 开发时固定端口，供 tauri.conf.json 的 devUrl 引用
const host = process.env.TAURI_DEV_HOST

/**
 * 目标运行环境：Android WebView 91+（Chromium 91）。
 *  - build.target = es2017：Tauri 三端 WebView 均支持；比 es2020 更保守，
 *    避免可选链/空值合并等语法在旧内核解析失败（Android 8/9 自带 WebView 可能滞后）。
 *  - cssTarget = chrome91：esbuild 只降级 Chromium 91 不认识的 CSS 语法。
 *  - @vitejs/plugin-legacy：为比目标更旧的引擎（如 Android 7 自带 WebView 61）
 *    额外产出 SystemJS 版产物 + <script nomodule> 兜底，现代引擎仍只加载现代包。
 *  - core-js polyfill 由 src/main.ts 显式引入（见该文件注释），保证现代包内也有。
 */
const BROWSER_TARGETS = ['chrome >= 91', 'and_chr >= 91', 'edge >= 91', 'firefox >= 90', 'safari >= 14', 'ios_saf >= 14']

export default defineConfig({
  plugins: [
    vue(),
    vuetify({ autoImport: true }),
    legacy({
      targets: BROWSER_TARGETS,
      modernTargets: BROWSER_TARGETS,
      // polyfill 由 main.ts 的 core-js 显式导入，避免两套注入重复
      modernPolyfills: false,
      renderLegacyChunks: true,
      polyfills: false,
    }),
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  // 禁止 vite 处理 Tauri 内部资源路径
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: 'es2017',
    cssTarget: 'chrome91',
    // legacy 插件需要 terser 才能产出兼容包
    minify: !process.env.TAURI_ENV_DEBUG ? 'terser' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // 单 chunk 过大时 v8 之外的旧引擎解码更慢，拆出 vendor
    chunkSizeWarningLimit: 1400,
  },
})
