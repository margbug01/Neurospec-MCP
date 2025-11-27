<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { setupExitWarningListener } from '../composables/useExitWarning'
import { useKeyboard } from '../composables/useKeyboard'
import { useToast } from '../composables/useToast'
import { useVersionCheck } from '../composables/useVersionCheck'
import UpdateModal from './common/UpdateModal.vue'
import LayoutWrapper from './layout/LayoutWrapper.vue'
import McpPopup from './popup/McpPopup.vue'

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
  mcpRequest: any
  showMcpPopup: boolean
  appConfig: AppConfig
  isInitializing: boolean
}

interface Emits {
  mcpResponse: [response: any]
  mcpCancel: []
  toggleAlwaysOnTop: []
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
  updateReplyConfig: [config: { enable_continue_reply?: boolean, continue_prompt?: string }]
  messageReady: [message: any]
  configReloaded: []
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// 版本检查相关
const { versionInfo, showUpdateModal } = useVersionCheck()

// 弹窗中的设置显示控制
const showPopupSettings = ref(false)
const popupSettingsTab = ref<string | undefined>(undefined)

// 初始化 Toast 实例
const toast = useToast()

// 键盘快捷键处理
const { handleExitShortcut } = useKeyboard()

// 切换弹窗设置显示
function togglePopupSettings() {
  popupSettingsTab.value = undefined
  showPopupSettings.value = !showPopupSettings.value
}

// 打开历史记录 Tab
function openHistoryTab() {
  popupSettingsTab.value = 'history'
  showPopupSettings.value = true
}

// 监听 MCP 请求变化，当有新请求时重置设置页面状态
watch(() => props.mcpRequest, (newRequest) => {
  if (newRequest && showPopupSettings.value) {
    showPopupSettings.value = false
  }
}, { immediate: true })

// 全局键盘事件处理器
function handleGlobalKeydown(event: KeyboardEvent) {
  handleExitShortcut(event)
}

onMounted(() => {
  // 将 toast 实例传递给父组件
  emit('messageReady', toast)
  // 设置退出警告监听器（统一处理主界面和弹窗）
  setupExitWarningListener(toast)

  // 添加全局键盘事件监听器
  document.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  // 移除键盘事件监听器
  document.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <div class="min-h-screen bg-gray-50">
    <!-- MCP弹窗模式 -->
    <template v-if="props.showMcpPopup && props.mcpRequest">
      <!-- 设置界面 -->
      <LayoutWrapper
        v-if="showPopupSettings"
        :app-config="props.appConfig"
        :initial-tab="popupSettingsTab"
        @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
        @update-window-size="$emit('updateWindowSize', $event)"
      />
      <!-- 弹窗内容 -->
      <McpPopup
        v-else
        :request="props.mcpRequest"
        :app-config="props.appConfig"
        @response="$emit('mcpResponse', $event)"
        @cancel="$emit('mcpCancel')"
        @open-main-layout="togglePopupSettings"
        @open-history="openHistoryTab"
      />
    </template>

    <!-- 弹窗加载骨架屏 或 初始化骨架屏 -->
    <div
      v-else-if="props.showMcpPopup || props.isInitializing"
      class="flex flex-col w-full h-screen bg-white text-primary"
    >
      <!-- 头部骨架 -->
      <div class="flex-shrink-0 bg-white border-b-2 border-gray-200 px-4 py-3">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="w-3 h-3 rounded-full bg-gray-200 animate-pulse" />
            <div class="w-64 h-4 rounded bg-gray-200 animate-pulse" />
          </div>
          <div class="flex gap-2">
            <div class="w-8 h-8 rounded-full bg-gray-200 animate-pulse" />
            <div class="w-8 h-8 rounded-full bg-gray-200 animate-pulse" />
          </div>
        </div>
      </div>

      <!-- 内容骨架 -->
      <div class="flex-1 p-4">
        <div class="bg-gray-50 rounded-lg p-4 mb-4 space-y-2">
          <div class="h-4 bg-gray-200 rounded animate-pulse w-full" />
          <div class="h-4 bg-gray-200 rounded animate-pulse w-full" />
          <div class="h-4 bg-gray-200 rounded animate-pulse w-3/4" />
        </div>

        <div class="space-y-3">
          <div class="h-4 bg-gray-200 rounded animate-pulse w-32" />
          <div class="h-4 bg-gray-200 rounded animate-pulse w-full" />
          <div class="h-4 bg-gray-200 rounded animate-pulse w-full" />
          <div class="h-4 bg-gray-200 rounded animate-pulse w-5/6" />
        </div>
      </div>

      <!-- 底部骨架 -->
      <div class="flex-shrink-0 bg-white border-t-2 border-gray-200 p-4">
        <div class="flex justify-between items-center">
          <div class="h-4 bg-gray-200 rounded animate-pulse w-24" />
          <div class="flex gap-2">
            <div class="h-8 bg-gray-200 rounded animate-pulse w-16" />
            <div class="h-8 bg-gray-200 rounded animate-pulse w-16" />
          </div>
        </div>
      </div>
    </div>

    <!-- 主界面 - 只在非弹窗模式且非初始化时显示 -->
    <LayoutWrapper
      v-else
      :app-config="props.appConfig"
      @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
      @update-window-size="$emit('updateWindowSize', $event)"
      @config-reloaded="$emit('configReloaded')"
    />

    <!-- 更新弹窗 -->
    <UpdateModal
      v-model:show="showUpdateModal"
      :version-info="versionInfo"
    />
  </div>
</template>
