import type { GlobalTheme } from 'naive-ui'
import { lightTheme } from 'naive-ui'
import { functionalColors, primaryColors, themeColors } from './colors'

// Custom light theme - Apple Design Language
export const customLightTheme: GlobalTheme = {
  ...lightTheme,
  common: {
    ...lightTheme.common,
    // Apple System Blue
    primaryColor: primaryColors[500],
    primaryColorHover: primaryColors[400],
    primaryColorPressed: primaryColors[600],
    primaryColorSuppl: primaryColors[400],

    // Background colors - Apple style
    bodyColor: themeColors.light.surface,
    popoverColor: themeColors.light.container,
    cardColor: themeColors.light.container,
    modalColor: themeColors.light.container,

    // Border colors
    borderColor: themeColors.light.border,
    dividerColor: themeColors.light.divider,

    // Text colors (Apple HIG)
    textColorBase: themeColors.light.onSurface,
    textColor1: themeColors.light.onSurface,
    textColor2: themeColors.light.onSurfaceSecondary,
    textColor3: themeColors.light.onSurfaceMuted,
    textColorDisabled: themeColors.light.onSurfaceDisabled,

    // Input colors
    inputColor: themeColors.light.containerSecondary,
    inputColorDisabled: themeColors.light.surface100,

    // Interaction colors (subtle)
    hoverColor: 'rgba(0, 0, 0, 0.04)',
    pressedColor: 'rgba(0, 0, 0, 0.08)',

    // Functional colors (iOS style)
    successColor: functionalColors.success,
    warningColor: functionalColors.warning,
    errorColor: functionalColors.error,
    infoColor: functionalColors.info,

    // Border radius (Apple style - larger, more friendly)
    borderRadius: '12px',
    borderRadiusSmall: '8px',

    // Box shadows (subtle, layered like macOS)
    boxShadow1: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
    boxShadow2: '0 2px 8px 0 rgba(0, 0, 0, 0.08), 0 1px 2px 0 rgba(0, 0, 0, 0.05)',
    boxShadow3: '0 4px 16px 0 rgba(0, 0, 0, 0.10), 0 2px 4px 0 rgba(0, 0, 0, 0.06)',
  },
}

// 主题工具函数 - 始终返回浅色主题
export function getTheme(): GlobalTheme {
  return customLightTheme
}

// CSS 变量映射 - 始终使用浅色主题
export function applyThemeVariables() {
  const root = document.documentElement

  const colors = themeColors.light

  // 设置语义化 CSS 变量 - 用于 UnoCSS
  root.style.setProperty('--color-surface', colors.surface)
  root.style.setProperty('--color-surface-50', colors.surface50)
  root.style.setProperty('--color-surface-100', colors.surface100)
  root.style.setProperty('--color-surface-200', colors.surface200)
  root.style.setProperty('--color-surface-300', colors.surface300)
  root.style.setProperty('--color-surface-400', colors.surface400)
  root.style.setProperty('--color-surface-500', colors.surface500)
  root.style.setProperty('--color-surface-600', colors.surface600)
  root.style.setProperty('--color-surface-700', colors.surface700)
  root.style.setProperty('--color-surface-800', colors.surface800)
  root.style.setProperty('--color-surface-900', colors.surface900)
  root.style.setProperty('--color-surface-950', colors.surface950)
  root.style.setProperty('--color-on-surface', colors.onSurface)

  // 设置其他语义化变量
  root.style.setProperty('--color-on-surface-secondary', colors.onSurfaceSecondary)
  root.style.setProperty('--color-on-surface-muted', colors.onSurfaceMuted)
  root.style.setProperty('--color-container', colors.container)
  root.style.setProperty('--color-border', colors.border)
  root.style.setProperty('--color-divider', colors.divider)

  // 设置body和text颜色变量（兼容旧CSS）
  root.style.setProperty('--body-color', colors.surface)
  root.style.setProperty('--text-color', colors.onSurface)

  // 强制设置为浅色主题
  root.classList.remove('dark')
  root.classList.add('light')
  root.setAttribute('data-theme', 'light')
}
