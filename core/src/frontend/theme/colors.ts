/**
 * NeuroSpec Color System - Apple Design Inspired
 *
 * Design Principles:
 * 1. Apple-style light theme as default
 * 2. Semantic color naming for clarity
 * 3. CSS variables for theme switching
 * 4. Subtle, elegant color palette
 */

// Apple-inspired primary colors (System Blue)
export const primaryColors = {
  50: '#e6f2ff',
  100: '#cce5ff',
  200: '#99cbff',
  300: '#66b0ff',
  400: '#3396ff',
  500: '#007AFF', // iOS/macOS System Blue
  600: '#0062cc',
  700: '#004999',
  800: '#003166',
  900: '#001933',
  950: '#000c1a',
}

// Functional colors (iOS/macOS style)
export const functionalColors = {
  success: '#34C759', // iOS Green
  warning: '#FF9500', // iOS Orange
  error: '#FF3B30', // iOS Red
  info: '#007AFF', // iOS Blue
}

// Semantic color system - Redefined UnoCSS base colors
export const semanticColors = {
  // Primary colors
  primary: primaryColors,
  // Functional colors
  ...functionalColors,
}

// Theme color definitions - Apple Design Language
export const themeColors = {
  light: {
    // Surface colors (macOS Big Sur/Monterey inspired)
    surface: '#F5F5F7', // Main background (Apple gray)
    surface50: '#FAFAFA', // Lightest surface
    surface100: '#FFFFFF', // White cards
    surface200: '#F0F0F2', // Secondary surface
    surface300: '#E5E5E7', // Tertiary surface
    surface400: '#D2D2D7', // Border light
    surface500: '#AEAEB2', // Separator
    surface600: '#8E8E93', // Tertiary label
    surface700: '#636366', // Secondary label
    surface800: '#48484A', // Label
    surface900: '#1D1D1F', // Primary text
    surface950: '#000000', // Pure black
    // Text colors (Apple HIG)
    onSurface: '#1D1D1F', // Primary text
    onSurfaceSecondary: '#6E6E73', // Secondary text
    onSurfaceMuted: '#AEAEB2', // Tertiary text
    onSurfaceDisabled: '#D2D2D7', // Disabled text
    // Container colors
    container: '#FFFFFF', // White cards
    containerSecondary: '#FAFAFA', // Light background
    // Border colors
    border: '#D2D2D7', // Subtle borders
    divider: '#E5E5E7', // Dividers
  },
}
