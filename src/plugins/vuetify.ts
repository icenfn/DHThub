import { createVuetify } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'

// DHThub 品牌色：青绿主色 + 深色模式
const dhthubTheme = {
  colors: {
    primary: '#0f766e',
    secondary: '#0e7490',
    accent: '#14b8a6',
    error: '#dc2626',
    info: '#0284c7',
    success: '#16a34a',
    warning: '#d97706',
    surface: '#ffffff',
    background: '#f6f8f7',
  },
}

const dhthubDarkTheme = {
  colors: {
    primary: '#2dd4bf',
    secondary: '#38bdf8',
    accent: '#5eead4',
    error: '#f87171',
    info: '#38bdf8',
    success: '#4ade80',
    warning: '#fbbf24',
    surface: '#1e2a28',
    background: '#131b1a',
  },
}

export default createVuetify({
  theme: {
    defaultTheme: 'light',
    themes: {
      light: dhthubTheme,
      dark: dhthubDarkTheme,
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
    VBtn: { rounded: 'lg' },
    VCard: { rounded: 'lg' },
    VTextField: { rounded: 'lg', variant: 'outlined', density: 'comfortable' },
    VSelect: { rounded: 'lg', variant: 'outlined', density: 'comfortable' },
  },
})
