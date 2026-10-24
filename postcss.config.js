/**
 * PostCSS 配置 —— 面向 Android WebView 91+（Chromium 91）的构建期兼容。
 *
 * 两个必须处理的点：
 *  1. `@layer` 级联层：Chromium 91 不认识 `@layer`，整段 at-rule 会被丢弃，
 *     连带 Vuetify 的 `--v-theme-*` 变量一起消失（按钮发黑、卡片没阴影）。
 *     这里用 postcss-preset-env 的 cascade-layers 特性把静态 CSS 的层展平。
 *     运行时生成的主题样式表不过 PostCSS，由 src/compat/legacy-css.ts 处理。
 *  2. 厂商前缀与新版 CSS 语法：autoprefixer + preset-env 按 browserslist 补齐。
 */
export default {
  plugins: {
    'postcss-preset-env': {
      // 显式开启级联层展平：stage 2 的 cascade-layers 插件会把 @layer 内联展开，
      // 使老引擎拿到「无层级」的普通规则（与运行时去层逻辑一致）。
      stage: 3,
      features: {
        'cascade-layers': true,
        'nesting-rules': true,
        'custom-properties': false,
      },
      autoprefixer: {
        grid: 'autoplace',
      },
    },
  },
}
