<script setup lang="ts">
import type { McpRequest } from '../../types/popup'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'
import { useToast } from '../../composables/useToast'

import PopupContent from './PopupContent.vue'
import PopupInput from './PopupInput.vue'

interface AppConfig {
  theme: string
  window: {
    alwaysOnTop: boolean
    width: number
    height: number
    fixed: boolean
  }
  audio: {
    enabled: boolean
    url: string
  }
  reply: {
    enabled: boolean
    prompt: string
  }
}

interface Props {
  request: McpRequest | null
  appConfig: AppConfig
  mockMode?: boolean
  testMode?: boolean
}

interface Emits {
  response: [response: any]
  cancel: []
  openMainLayout: []
  openHistory: []
  toggleAlwaysOnTop: []
  toggleAudioNotification: []
  updateAudioUrl: [url: string]
  testAudio: []
  stopAudio: []
  testAudioError: [error: any]
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
}

const props = withDefaults(defineProps<Props>(), {
  mockMode: false,
  testMode: false,
})

const emit = defineEmits<Emits>()

// 使用消息提示
const { success: showSuccess, error: showError } = useToast()

// 响应式状态
const loading = ref(false)
const submitting = ref(false)
const selectedOptions = ref<string[]>([])
const userInput = ref('')
const draggedImages = ref<string[]>([])
const inputRef = ref()

// 继续回复配置
const continueReplyEnabled = ref(true)
const continuePrompt = ref('请按照最佳实践继续')

// 计算属性
const isVisible = computed(() => !!props.request)
const hasOptions = computed(() => (props.request?.predefined_options?.length ?? 0) > 0)
const canSubmit = computed(() => {
  if (hasOptions.value) {
    return selectedOptions.value.length > 0 || userInput.value.trim().length > 0 || draggedImages.value.length > 0
  }
  return userInput.value.trim().length > 0 || draggedImages.value.length > 0
})

// 获取输入组件的状态文本
const inputStatusText = computed(() => {
  return inputRef.value?.statusText || '等待输入...'
})

// 加载继续回复配置
async function loadReplyConfig() {
  try {
    const config = await invoke('get_reply_config')
    if (config) {
      const replyConfig = config as any
      continueReplyEnabled.value = replyConfig.enable_continue_reply ?? true
      continuePrompt.value = replyConfig.continue_prompt ?? '请按照最佳实践继续'
    }
  }
  catch (error) {
    console.log('加载继续回复配置失败，使用默认值:', error)
  }
}

// 监听配置变化（当从设置页面切换回来时）
watch(() => props.appConfig.reply, (newReplyConfig) => {
  if (newReplyConfig) {
    continueReplyEnabled.value = newReplyConfig.enabled
    continuePrompt.value = newReplyConfig.prompt
  }
}, { deep: true, immediate: true })

// Telegram事件监听器（已废弃，保留代码以防需要）
// let telegramUnlisten: (() => void) | null = null

// 监听请求变化
watch(() => props.request, (newRequest) => {
  if (newRequest) {
    resetForm()
    loading.value = true
    // 每次显示弹窗时重新加载配置
    loadReplyConfig()
    setTimeout(() => {
      loading.value = false
    }, 300)
  }
}, { immediate: true })

// 组件挂载时加载配置
onMounted(() => {
  loadReplyConfig()
})

// 重置表单
function resetForm() {
  selectedOptions.value = []
  userInput.value = ''
  draggedImages.value = []
  submitting.value = false

  // 重置子组件状态
  inputRef.value?.reset()
}

// 处理提交
async function handleSubmit() {
  if (!canSubmit.value || submitting.value)
    return

  submitting.value = true

  try {
    // 使用新的结构化数据格式
    const response = {
      user_input: userInput.value.trim() || null,
      selected_options: selectedOptions.value,
      images: draggedImages.value.map(imageData => ({
        data: imageData.split(',')[1], // 移除 data:image/png;base64, 前缀
        media_type: 'image/png',
        filename: null,
      })),
      metadata: {
        timestamp: new Date().toISOString(),
        request_id: props.request?.id || null,
        source: 'popup',
      },
    }

    // 如果没有任何有效内容，设置默认用户输入
    if (!response.user_input && response.selected_options.length === 0 && response.images.length === 0) {
      response.user_input = '用户确认继续'
    }

    if (props.mockMode) {
      // 模拟模式下的延迟
      await new Promise(resolve => setTimeout(resolve, 1000))
      showSuccess('模拟响应发送成功')
    }

    // 统一交给父级事件处理，避免重复发送或提前退出
    emit('response', response)
  }
  catch (error) {
    console.error('提交响应失败:', error)
    showError('提交失败，请重试')
  }
  finally {
    submitting.value = false
  }
}

// 处理输入更新
function handleInputUpdate(data: { userInput: string, selectedOptions: string[], draggedImages: string[] }) {
  userInput.value = data.userInput
  selectedOptions.value = data.selectedOptions
  draggedImages.value = data.draggedImages
}

// 处理图片添加 - 移除重复逻辑，避免双重添加
function handleImageAdd(_image: string) {
  // 这个函数现在只是为了保持接口兼容性，实际添加在PopupInput中完成
}

// 处理图片移除
function handleImageRemove(index: number) {
  draggedImages.value.splice(index, 1)
}

// 处理继续按钮点击
async function handleContinue() {
  if (submitting.value)
    return

  submitting.value = true

  try {
    // 使用新的结构化数据格式
    const response = {
      user_input: continuePrompt.value,
      selected_options: [],
      images: [],
      metadata: {
        timestamp: new Date().toISOString(),
        request_id: props.request?.id || null,
        source: 'popup_continue',
      },
    }

    if (props.mockMode) {
      // 模拟模式下的延迟
      await new Promise(resolve => setTimeout(resolve, 1000))
      showSuccess('继续请求发送成功')
    }

    // 统一交给父级事件处理，避免重复发送或提前退出
    emit('response', response)
  }
  catch (error) {
    console.error('发送继续请求失败:', error)
    showError('继续请求失败，请重试')
  }
  finally {
    submitting.value = false
  }
}

// 处理引用消息
function handleQuoteMessage(messageContent: string) {
  if (inputRef.value) {
    inputRef.value.handleQuoteMessage(messageContent)
  }
}

// 返回主界面（打开设置）
function handleBackToMain() {
  emit('openMainLayout')
}
</script>

<template>
  <div v-if="isVisible" class="retro-popup">
    <!-- 噪点纹理 -->
    <div class="noise-overlay" />

    <!-- 磁带卡带容器 -->
    <div class="cassette-popup">
      <!-- 顶部三色条 -->
      <div class="top-stripe">
        <div class="stripe-orange" />
        <div class="stripe-teal" />
        <div class="stripe-dark" />
      </div>

      <!-- 头部标题 -->
      <div class="popup-header">
        <div class="header-left">
          <div class="i-carbon-chat text-lg" />
          <span class="header-title">MCP INTERACT</span>
        </div>
        <div class="header-right">
          <button class="header-btn" title="交互历史" @click="emit('openHistory')">
            <div class="i-carbon-time w-3.5 h-3.5" />
          </button>
          <button class="header-btn" title="返回主界面" @click="handleBackToMain">
            <div class="i-carbon-settings w-3.5 h-3.5" />
          </button>
          <div class="header-badge">SIDE B</div>
        </div>
      </div>

      <!-- 主内容区 -->
      <div class="popup-content custom-scrollbar">
        <!-- 消息卡片 -->
        <div class="message-card">
          <PopupContent
            :request="request"
            :loading="loading"
            :current-theme="props.appConfig.theme"
            @quote-message="handleQuoteMessage"
          />
        </div>

        <!-- 输入区域 -->
        <PopupInput
          ref="inputRef"
          :request="request"
          :loading="loading"
          :submitting="submitting"
          :can-submit="canSubmit"
          :continue-reply-enabled="continueReplyEnabled"
          :input-status-text="inputStatusText"
          @update="handleInputUpdate"
          @image-add="handleImageAdd"
          @image-remove="handleImageRemove"
          @submit="handleSubmit"
          @continue="handleContinue"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.retro-popup {
  min-height: 100vh;
  background-color: #e8e4d9;
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
  color: #1f2937;
  padding: 0.5rem;
  position: relative;
}

.noise-overlay {
  position: fixed;
  inset: 0;
  pointer-events: none;
  opacity: 0.03;
  z-index: 50;
  mix-blend-mode: multiply;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
}

.cassette-popup {
  background-color: #fbfaf8;
  border: 3px solid #1f2937;
  box-shadow: 8px 8px 0px 0px rgba(31, 41, 55, 1);
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 1rem);
  position: relative;
  overflow: hidden;
}

.top-stripe {
  height: 12px;
  width: 100%;
  display: flex;
  border-bottom: 3px solid #1f2937;
  flex-shrink: 0;
}

.stripe-orange { flex: 1; background-color: #f97316; }
.stripe-teal { flex: 1; background-color: #0d9488; }
.stripe-dark { flex: 1; background-color: #1f2937; }

.popup-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 2px dashed #d1d5db;
  background: #f9fafb;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.header-title {
  font-weight: 800;
  font-size: 0.875rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.header-btn {
  width: 1.5rem;
  height: 1.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border: 2px solid #1f2937;
  color: #6b7280;
  cursor: pointer;
  transition: all 0.1s;
}

.header-btn:hover {
  background: #1f2937;
  color: white;
}

.header-badge {
  border: 2px solid #1f2937;
  padding: 0.125rem 0.5rem;
  background: #f3f4f6;
  font-weight: 700;
  font-size: 0.625rem;
}

.popup-content {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  padding-bottom: 8rem;
}

.message-card {
  background: white;
  border: 2px solid #1f2937;
  padding: 1rem;
  margin-bottom: 1rem;
  box-shadow: 4px 4px 0px 0px rgba(200, 200, 200, 1);
}

/* 自定义滚动条 */
.custom-scrollbar::-webkit-scrollbar {
  width: 10px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: #e8e4d9;
  border-left: 2px solid #1f2937;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border: 2px solid #e8e4d9;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #ea580c;
}
</style>
