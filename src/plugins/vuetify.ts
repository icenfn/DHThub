import { createVuetify } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'

// ============================================================
// DHThub · Vuetify 4 + Material Design 3 主题
//
// ⚠️ 关键约束（v0.5.0 颜色 bug 的根因）：
// Vuetify 生成 CSS 变量时**直接使用 colors 对象的键名**，不做大小写转换。
// 写成 onSurfaceVariant 会生成 `--v-theme-onSurfaceVariant`，而样式里用的是
// `--v-theme-on-surface-variant` —— 取不到值就回落到 Vuetify 默认调色板
// （浅色主题 `#EEEEEE` 近白 → 看起来「发灰」，暗色主题 `#000000` → 看起来很黑）。
// 因此这里所有颜色键名一律使用 **kebab-case**。
//
// 另：只覆盖需要的键，其余由 Vuetify 默认主题补齐（mergeDeep）。
// ============================================================

const light = {
  dark: false,
  colors: {
    primary: '#00695f',
    'on-primary': '#ffffff',
    'primary-darken-1': '#00544c',
    'primary-container': '#9cf2e2',
    'on-primary-container': '#00201c',

    secondary: '#42584f',
    'on-secondary': '#ffffff',
    'secondary-darken-1': '#33473f',
    'secondary-container': '#cce8e2',
    'on-secondary-container': '#06201c',

    tertiary: '#3f6176',
    'on-tertiary': '#ffffff',
    'tertiary-container': '#c7e7ff',
    'on-tertiary-container': '#001e2e',

    error: '#b3261e',
    'on-error': '#ffffff',
    'error-darken-1': '#8c1d18',
    'error-container': '#f9dedc',
    'on-error-container': '#410e0b',

    warning: '#7a5900',
    'on-warning': '#ffffff',
    'warning-container': '#ffdf9e',
    'on-warning-container': '#261a00',

    success: '#2f6a1c',
    'on-success': '#ffffff',
    'success-container': '#b7f295',
    'on-success-container': '#072100',

    info: '#00639b',
    'on-info': '#ffffff',
    'info-container': '#cde5ff',
    'on-info-container': '#001d33',

    // ---- 纯白基底 ----
    background: '#ffffff',
    'on-background': '#14161a',
    surface: '#ffffff',
    'on-surface': '#14161a',
    'surface-variant': '#e6e8ea',
    'on-surface-variant': '#41464b',

    // MD3 surface 色阶（浅色主题靠描边 + 极淡色阶分层）
    'surface-bright': '#ffffff',
    'surface-light': '#f4f5f6',
    'surface-dim': '#dcdee0',
    'surface-container-lowest': '#ffffff',
    'surface-container-low': '#fafafa',
    'surface-container': '#f4f5f6',
    'surface-container-high': '#eceef0',
    'surface-container-highest': '#e3e5e8',

    // 描边加深一档，图标与分隔线不再「浅灰一片」
    outline: '#6a6e73',
    'outline-variant': '#c6cacd',

    'inverse-surface': '#2e3130',
    'inverse-on-surface': '#eff1ef',
    'inverse-primary': '#7fd9c9',
    scrim: '#000000',
    shadow: '#000000',
  },
  variables: {
    'border-color': '#14161a',
    'border-opacity': 0.12,
    'high-emphasis-opacity': 1,
    'medium-emphasis-opacity': 0.72,
    'disabled-opacity': 0.42,
  },
}

const dark = {
  dark: true,
  colors: {
    primary: '#7fd9c9',
    'on-primary': '#00382f',
    'primary-darken-1': '#9cefe0',
    'primary-container': '#005047',
    'on-primary-container': '#9cf2e2',

    secondary: '#b0ccc5',
    'on-secondary': '#1b352f',
    'secondary-darken-1': '#c5e1da',
    'secondary-container': '#324b46',
    'on-secondary-container': '#cce8e2',

    tertiary: '#a8c9e2',
    'on-tertiary': '#0e3144',
    'tertiary-container': '#29485d',
    'on-tertiary-container': '#c7e7ff',

    error: '#ffb4ab',
    'on-error': '#690005',
    'error-darken-1': '#ffcabf',
    'error-container': '#93000a',
    'on-error-container': '#ffdad6',

    warning: '#f6c250',
    'on-warning': '#3f2e00',
    'warning-container': '#5c4400',
    'on-warning-container': '#ffdf9e',

    success: '#9cd578',
    'on-success': '#0b3a00',
    'success-container': '#23540e',
    'on-success-container': '#b7f295',

    info: '#9ccbff',
    'on-info': '#003352',
    'info-container': '#004a75',
    'on-info-container': '#cde5ff',

    background: '#101413',
    'on-background': '#e6e9e7',
    surface: '#101413',
    'on-surface': '#e6e9e7',
    'surface-variant': '#414745',
    'on-surface-variant': '#c2c9c6',

    'surface-bright': '#363a38',
    'surface-light': '#1a1e1d',
    'surface-dim': '#101413',
    'surface-container-lowest': '#0b0f0e',
    'surface-container-low': '#151918',
    'surface-container': '#1a1e1d',
    'surface-container-high': '#242827',
    'surface-container-highest': '#2e3231',

    outline: '#8d9491',
    'outline-variant': '#414745',

    'inverse-surface': '#e6e9e7',
    'inverse-on-surface': '#2e3130',
    'inverse-primary': '#00695f',
    scrim: '#000000',
    shadow: '#000000',
  },
  variables: {
    'border-color': '#e6e9e7',
    'border-opacity': 0.16,
    'high-emphasis-opacity': 1,
    'medium-emphasis-opacity': 0.76,
    'disabled-opacity': 0.44,
  },
}

// 原始调色板快照：切主题时用它重建 colors，
// 避免 Vuetify 复用/合并上一次主题的调色板导致按钮发灰。
export const PRISTINE_COLORS = {
  light: { ...light.colors },
  dark: { ...dark.colors },
}

export default createVuetify({
  theme: {
    defaultTheme: 'light',
    themes: { light, dark },
  },
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: { mdi },
  },
  defaults: {
    global: { ripple: true },
    VBtn: { rounded: 'pill', variant: 'flat', textTransform: 'none' },
    VCard: { rounded: 'lg', variant: 'flat' },
    VSheet: { rounded: 'lg' },
    VTextField: {
      variant: 'outlined',
      density: 'comfortable',
      rounded: 'pill',
      color: 'primary',
      hideDetails: 'auto',
    },
    VSelect: {
      variant: 'outlined',
      density: 'comfortable',
      rounded: 'lg',
      color: 'primary',
      hideDetails: 'auto',
    },
    VTextarea: {
      variant: 'outlined',
      density: 'comfortable',
      rounded: 'lg',
      color: 'primary',
      hideDetails: 'auto',
    },
    VAutocomplete: {
      variant: 'outlined',
      density: 'comfortable',
      rounded: 'lg',
      color: 'primary',
      hideDetails: 'auto',
    },
    VSwitch: { color: 'primary', density: 'compact', hideDetails: 'auto' },
    VCheckbox: { color: 'primary', density: 'compact', hideDetails: 'auto' },
    VList: { rounded: 'lg' },
    VListItem: { rounded: 'lg' },
    VChip: { rounded: 'pill' },
    VChipGroup: { color: 'primary' },
    VAvatar: { rounded: 'lg' },
    VDialog: { scrim: true },
    VSnackbar: { rounded: 'pill', color: 'inverse-surface' },
    VProgressLinear: { rounded: true, color: 'primary' },
    VDivider: { color: 'outline-variant' },
    VMenu: { rounded: 'lg' },
    VTooltip: { location: 'bottom' },
  },
})
