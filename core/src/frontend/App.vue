<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import AppContent from './components/AppContent.vue'
import ToastContainer from './components/base/ToastContainer.vue'
import { useAppManager } from './composables/useAppManager'
import { useEventHandlers } from './composables/useEventHandlers'

// 使用封装的应用管理器
const {
  mcpRequest,
  showMcpPopup,
  appConfig,
  isInitializing,
  actions,
} = useAppManager()

// 创建事件处理器
const handlers = useEventHandlers(actions)

// 主题应用由useTheme统一管理，移除重复的主题应用逻辑

// 初始化
onMounted(async () => {
  try {
    await actions.app.initialize()
  }
  catch (error) {
    console.error('应用初始化失败:', error)
  }
})

// 清理
onUnmounted(() => {
  actions.app.cleanup()
})
</script>

<template>
  <div class="min-h-screen bg-surface transition-colors duration-200">
    <AppContent
      :mcp-request="mcpRequest" :show-mcp-popup="showMcpPopup" :app-config="appConfig"
      :is-initializing="isInitializing" @mcp-response="handlers.onMcpResponse" @mcp-cancel="handlers.onMcpCancel"
      @toggle-always-on-top="handlers.onToggleAlwaysOnTop"
      @update-window-size="handlers.onUpdateWindowSize"
      @update-reply-config="handlers.onUpdateReplyConfig" @message-ready="handlers.onMessageReady"
      @config-reloaded="handlers.onConfigReloaded"
    />
    <ToastContainer />
  </div>
</template>
