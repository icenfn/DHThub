import { createVuetify } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'

// ============================================================
// DHThub · Vuetify 4 + Material Design 3 主题
// 完整 MD3 色彩 token（light / dark），品牌主色为青绿（Teal）
// 设计取向：MD3 基调（tonal 分层、大圆角、pill 控件）+ 轻量描边卡片，
// 不做教条化的 elevation 堆叠，强调留白与信息层级。
// ============================================================

const md3Light = {
  dark: false,
  colors: {
    primary: '#006a60',
    onPrimary: '#ffffff',
    primaryContainer: '#9cf2e2',
    onPrimaryContainer: '#00201c',
    primaryDarken1: '#00564e',

    secondary: '#4a635e',
    onSecondary: '#ffffff',
    secondaryContainer: '#cce8e2',
    onSecondaryContainer: '#06201c',
    secondaryDarken1: '#375049',

    tertiary: '#426278',
    onTertiary: '#ffffff',
    tertiaryContainer: '#c7e7ff',
    onTertiaryContainer: '#001e2e',

    error: '#ba1a1a',
    onError: '#ffffff',
    errorContainer: '#ffdad6',
    onErrorContainer: '#410002',
    errorDarken1: '#a80707',

    warning: '#7a5900',
    onWarning: '#ffffff',
    warningContainer: '#ffdf9e',
    onWarningContainer: '#261a00',

    success: '#386a20',
    onSuccess: '#ffffff',
    successContainer: '#b7f295',
    onSuccessContainer: '#072100',

    info: '#00639b',
    onInfo: '#ffffff',
    infoContainer: '#cde5ff',
    onInfoContainer: '#001d33',

    surface: '#f8faf8',
    onSurface: '#191c1b',
    surfaceVariant: '#dae5e1',
    onSurfaceVariant: '#3f4946',
    surfaceContainer: '#edf0ee',
    surfaceContainerHigh: '#e7eae8',
    surfaceContainerHighest: '#e1e5e2',
    surfaceContainerLow: '#f3f5f4',
    surfaceContainerLowest: '#ffffff',
    surfaceBright: '#f8faf8',
    surfaceDim: '#d8dbd9',

    background: '#f8faf8',
    onBackground: '#191c1b',

    outline: '#6f7976',
    outlineVariant: '#bec9c5',
    outlineInverse: '#899390',
    inverseSurface: '#2e3130',
    inverseOnSurface: '#eff1ef',
    inversePrimary: '#7fd9c9',
    scrim: '#000000',
    shadow: '#000000',
  },
}

const md3Dark = {
  dark: true,
  colors: {
    primary: '#7fd9c9',
    onPrimary: '#00382f',
    primaryContainer: '#005047',
    onPrimaryContainer: '#9cf2e2',
    primaryDarken1: '#9cefe0',

    secondary: '#b0ccc5',
    onSecondary: '#1b352f',
    secondaryContainer: '#324b46',
    onSecondaryContainer: '#cce8e2',
    secondaryDarken1: '#c5e1da',

    tertiary: '#a8c9e2',
    onTertiary: '#0e3144',
    tertiaryContainer: '#29485d',
    onTertiaryContainer: '#c7e7ff',

    error: '#ffb4ab',
    onError: '#690005',
    errorContainer: '#93000a',
    onErrorContainer: '#ffdad6',
    errorDarken1: '#ffcabf',

    warning: '#f6c250',
    onWarning: '#3f2e00',
    warningContainer: '#5c4400',
    onWarningContainer: '#ffdf9e',

    success: '#9cd578',
    onSuccess: '#0b3a00',
    successContainer: '#23540e',
    onSuccessContainer: '#b7f295',

    info: '#9ccbff',
    onInfo: '#003352',
    infoContainer: '#004a75',
    onInfoContainer: '#cde5ff',

    surface: '#101413',
    onSurface: '#e0e3e1',
    surfaceVariant: '#3f4946',
    onSurfaceVariant: '#bec9c5',
    surfaceContainer: '#151918',
    surfaceContainerHigh: '#1f2322',
    surfaceContainerHighest: '#2a2e2c',
    surfaceContainerLow: '#111514',
    surfaceContainerLowest: '#0b0f0e',
    surfaceBright: '#363a38',
    surfaceDim: '#101413',

    background: '#101413',
    onBackground: '#e0e3e1',

    outline: '#899390',
    outlineVariant: '#3f4946',
    outlineInverse: '#d8dbd9',
    inverseSurface: '#e0e3e1',
    inverseOnSurface: '#2e3130',
    inversePrimary: '#006a60',
    scrim: '#000000',
    shadow: '#000000',
  },
}

export default createVuetify({
  theme: {
    defaultTheme: 'light',
    themes: {
      light: md3Light,
      dark: md3Dark,
    },
  },
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
  defaults: {
    // MD3 基调：pill 按钮、大圆角卡片、描边输入控件；阴影交由 surface 分层承担
    VBtn: { rounded: 'pill', variant: 'flat', textTransform: 'none' },
    VCard: { rounded: 'xl', variant: 'flat' },
    VSheet: { rounded: 'xl' },
    VTextField: { variant: 'outlined', density: 'comfortable', rounded: 'lg', color: 'primary' },
    VSelect: { variant: 'outlined', density: 'comfortable', rounded: 'lg', color: 'primary' },
    VTextarea: { variant: 'outlined', density: 'comfortable', rounded: 'lg', color: 'primary' },
    VAutocomplete: { variant: 'outlined', density: 'comfortable', rounded: 'lg', color: 'primary' },
    VSwitch: { color: 'primary' },
    VCheckbox: { color: 'primary' },
    VList: { rounded: 'xl' },
    VListItem: { rounded: 'lg' },
    VChip: { rounded: 'pill' },
    VAvatar: { rounded: 'lg' },
    VNavigationDrawer: { elevation: 0 },
    VAppBar: { elevation: 0 },
    VToolbar: { color: 'surface' },
    VDialog: { scrim: true },
  },
})
