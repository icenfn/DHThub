import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'

// Tauri 开发时固定端口，供 tauri.conf.json 的 devUrl 引用
const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [vue(), vuetify({ autoImport: true })],
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
    // 现代 WebView（Linux WebKitGTK / Windows WebView2 / Android System WebView / WKWebView）均支持 ES2020
    target: 'es2020',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
