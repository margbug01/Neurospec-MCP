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
  fromPopup?: boolean
}

interface Emits {
  toggleAlwaysOnTop: []
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
  configReloaded: []
  closeToPopup: []
}

const props = withDefaults(defineProps<Props>(), {
  fromPopup: false
})
const emit = defineEmits<Emits>()
</script>

<template>
  <MainLayout
    :current-theme="props.appConfig.theme"
    :always-on-top="props.appConfig.window.alwaysOnTop"
    :window-width="props.appConfig.window.width"
    :window-height="props.appConfig.window.height"
    :fixed-window-size="props.appConfig.window.fixed"
    :initial-tab="props.initialTab"
    :from-popup="props.fromPopup"
    @toggle-always-on-top="emit('toggleAlwaysOnTop')"
    @update-window-size="emit('updateWindowSize', $event)"
    @config-reloaded="emit('configReloaded')"
    @close-to-popup="emit('closeToPopup')"
  />
</template>
