<script setup lang="ts">
import MainLayout from './MainLayout.vue'

interface AppConfig {
  theme: string
  window: {
    alwaysOnTop: boolean
    width: number
    height: number
    fixed: boolean
  }
  reply: {
    enabled: boolean
    prompt: string
  }
}

interface Props {
  appConfig: AppConfig
  initialTab?: string
}

interface Emits {
  toggleAlwaysOnTop: []
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
  configReloaded: []
}

defineProps<Props>()
defineEmits<Emits>()
</script>

<template>
  <MainLayout
    :current-theme="appConfig.theme"
    :always-on-top="appConfig.window.alwaysOnTop"
    :window-width="appConfig.window.width"
    :window-height="appConfig.window.height"
    :fixed-window-size="appConfig.window.fixed"
    :initial-tab="initialTab"
    @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
    @update-window-size="$emit('updateWindowSize', $event)"
    @config-reloaded="$emit('configReloaded')"
  />
</template>
