<script setup lang="ts">
import type { CustomPrompt, McpRequest } from '../../types/popup'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useSortable } from '@vueuse/integrations/useSortable'
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { useKeyboard } from '../../composables/useKeyboard'
import { useToast } from '../../composables/useToast'
import BaseButton from '../base/Button.vue'
import ImagePreview from '../base/ImagePreview.vue'
import BaseModal from '../base/Modal.vue'
import BaseSwitch from '../base/Switch.vue'
import BaseTextarea from '../base/Textarea.vue'

interface Props {
  request: McpRequest | null
  loading?: boolean
  submitting?: boolean
  canSubmit?: boolean
  continueReplyEnabled?: boolean
  inputStatusText?: string
}

interface Emits {
  update: [data: {
    userInput: string
    selectedOptions: string[]
    draggedImages: string[]
  }]
  imageAdd: [image: string]
  imageRemove: [index: number]
  submit: []
  continue: []
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  submitting: false,
  canSubmit: false,
  continueReplyEnabled: true,
})

const emit = defineEmits<Emits>()

// 响应式数据 - 基础状态
const showSettings = ref(false)
const userInput = ref('')
const selectedOptions = ref<string[]>([])
const uploadedImages = ref<string[]>([])
const imageFingerprints = ref<Set<string>>(new Set())
const imageFingerprintMap = ref<Map<number, string>>(new Map()) // index -> fingerprint
const textareaRef = ref<HTMLTextAreaElement | null>(null)


// 自定义prompt相关状态 (移动到 toggleSettings 之前)
const customPrompts = ref<CustomPrompt[]>([])
const customPromptEnabled = ref(true)
const showInsertDialog = ref(false)
const pendingPromptContent = ref('')

// 分离普通prompt和条件性prompt
const normalPrompts = computed(() =>
  customPrompts.value.filter(prompt => prompt.type === 'normal' || !prompt.type),
)

const conditionalPrompts = computed(() =>
  customPrompts.value.filter(prompt => prompt.type === 'conditional'),
)

// 拖拽排序相关状态
const promptContainer = ref<HTMLElement | null>(null)
const sortablePrompts = shallowRef<CustomPrompt[]>([])
const { start, stop } = useSortable(promptContainer, sortablePrompts, {
  animation: 200,
  ghostClass: 'sortable-ghost',
  chosenClass: 'sortable-chosen',
  dragClass: 'sortable-drag',
  handle: '.drag-handle',
  forceFallback: true,
  fallbackTolerance: 3,
  onEnd: (evt) => {
    if (evt.oldIndex !== evt.newIndex && evt.oldIndex !== undefined && evt.newIndex !== undefined) {
      const newList = [...sortablePrompts.value]
      const [movedItem] = newList.splice(evt.oldIndex, 1)
      newList.splice(evt.newIndex, 0, movedItem)
      sortablePrompts.value = newList

      const conditionalPromptsList = customPrompts.value.filter(prompt => prompt.type === 'conditional')
      customPrompts.value = [...sortablePrompts.value, ...conditionalPromptsList]
      savePromptOrder()
    }
  },
})

function toggleSettings() {
  showSettings.value = !showSettings.value
  // 如果打开设置，重新初始化拖拽
  if (showSettings.value && customPrompts.value.length > 0) {
    nextTick(() => initializeDragSort())
  }
}

function handleEnterKey(e: KeyboardEvent) {
  if (!e.shiftKey && props.canSubmit && !props.submitting) {
    e.preventDefault()
    emit('submit')
  }
}

// 移除了 unused pasteShortcut
useKeyboard()
const { showSuccess, showError, showWarning } = useToast()

// 计算属性
const hasOptions = computed(() => (props.request?.predefined_options?.length ?? 0) > 0)
const canSubmit = computed(() => {
  const hasOptionsSelected = selectedOptions.value.length > 0
  const hasInputText = userInput.value.trim().length > 0
  const hasImages = uploadedImages.value.length > 0

  if (hasOptions.value) {
    return hasOptionsSelected || hasInputText || hasImages
  }
  return hasInputText || hasImages
})

// 工具栏状态文本
const statusText = computed(() => {
  if (props.submitting)
    return 'Submitting...'

  const hasInput = selectedOptions.value.length > 0
    || uploadedImages.value.length > 0
    || userInput.value.trim().length > 0

  if (hasInput)
    return ''
  return '等待输入...'
})

// 发送更新事件
function emitUpdate() {
  const conditionalContent = generateConditionalContent()
  const finalUserInput = userInput.value + conditionalContent

  emit('update', {
    userInput: finalUserInput,
    selectedOptions: selectedOptions.value,
    draggedImages: uploadedImages.value,
  })
}

// 处理选项变化
function handleOptionChange(option: string, checked: boolean) {
  if (checked) {
    selectedOptions.value.push(option)
  }
  else {
    const idx = selectedOptions.value.indexOf(option)
    if (idx > -1)
      selectedOptions.value.splice(idx, 1)
  }
  emitUpdate()
}

function handleOptionToggle(option: string) {
  const idx = selectedOptions.value.indexOf(option)
  if (idx > -1) {
    selectedOptions.value.splice(idx, 1)
  }
  else {
    selectedOptions.value.push(option)
  }
  emitUpdate()
}

function handleImagePaste(event: ClipboardEvent) {
  const items = event.clipboardData?.items
  let hasImage = false

  if (items) {
    for (const item of items) {
      if (item.type.includes('image')) {
        hasImage = true
        const file = item.getAsFile()
        if (file) {
          handleImageFiles([file])
        }
      }
    }
  }

  if (hasImage) {
    event.preventDefault()
  }
}

async function handleImageFiles(files: FileList | File[]): Promise<void> {
  for (const file of files) {
    if (file.type.startsWith('image/')) {
      // Check file size (10MB limit)
      if (file.size > 10 * 1024 * 1024) {
        showError(`图片 ${file.name} 太大 (最大 10MB)`)
        continue
      }

      // Check fingerprint for deduplication before reading file
      const fingerprint = `${file.name}-${file.size}`
      if (imageFingerprints.value.has(fingerprint)) {
        showWarning(`图片 ${file.name} 已存在`)
        continue
      }

      try {
        const base64 = await fileToBase64(file)
        if (!uploadedImages.value.includes(base64)) {
          const newIndex = uploadedImages.value.length
          uploadedImages.value.push(base64)
          imageFingerprints.value.add(fingerprint)
          imageFingerprintMap.value.set(newIndex, fingerprint)
          showSuccess(`图片 ${file.name} 已添加`)
          emitUpdate()
        }
        else {
          // Fallback check if fingerprint didn't catch it
          imageFingerprints.value.add(fingerprint)
          showWarning(`图片 ${file.name} 已存在`)
        }
      }
      catch (error) {
        console.error('图片处理失败:', error)
        showError(`图片 ${file.name} 处理失败`)
      }
    }
  }
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

function removeImage(index: number) {
  // Remove fingerprint from set
  const fingerprint = imageFingerprintMap.value.get(index)
  if (fingerprint) {
    imageFingerprints.value.delete(fingerprint)
    imageFingerprintMap.value.delete(index)
  }

  // Update map indices for remaining images
  const updatedMap = new Map<number, string>()
  imageFingerprintMap.value.forEach((fp, idx) => {
    if (idx > index) {
      updatedMap.set(idx - 1, fp)
    }
    else if (idx < index) {
      updatedMap.set(idx, fp)
    }
  })
  imageFingerprintMap.value = updatedMap

  uploadedImages.value.splice(index, 1)
  emit('imageRemove', index)
  emitUpdate()
}

async function loadCustomPrompts() {
  try {
    const config = await invoke('get_custom_prompt_config')
    if (config) {
      const promptConfig = config as any
      customPrompts.value = (promptConfig.prompts || []).sort((a: CustomPrompt, b: CustomPrompt) => a.sort_order - b.sort_order)
      customPromptEnabled.value = promptConfig.enabled ?? true
      sortablePrompts.value = [...normalPrompts.value]

      if (customPrompts.value.length > 0 && showSettings.value) {
        initializeDragSort()
      }
    }
  }
  catch (error) {
    console.error('PopupInput: 加载自定义prompt失败:', error)
  }
}

function handlePromptClick(prompt: CustomPrompt) {
  if (!prompt.content || prompt.content.trim() === '') {
    userInput.value = ''
    emitUpdate()
    return
  }

  if (userInput.value.trim()) {
    pendingPromptContent.value = prompt.content
    showInsertDialog.value = true
  }
  else {
    insertPromptContent(prompt.content)
  }
}

function handleQuoteMessage(messageContent: string) {
  if (userInput.value.trim()) {
    pendingPromptContent.value = messageContent
    showInsertDialog.value = true
  }
  else {
    insertPromptContent(messageContent)
    showSuccess('原文内容已引用到输入框')
  }
}

function insertPromptContent(content: string, mode: 'replace' | 'append' = 'replace') {
  if (mode === 'replace') {
    userInput.value = content
  }
  else {
    userInput.value = userInput.value.trim() + (userInput.value.trim() ? '\n\n' : '') + content
  }

  setTimeout(() => {
    if (textareaRef.value) {
      textareaRef.value.focus()
    }
  }, 100)

  emitUpdate()
}

function handleInsertMode(mode: 'replace' | 'append') {
  insertPromptContent(pendingPromptContent.value, mode)
  showInsertDialog.value = false
  pendingPromptContent.value = ''
}

async function handleConditionalToggle(promptId: string, value: boolean) {
  const prompt = customPrompts.value.find(p => p.id === promptId)
  if (prompt) {
    prompt.current_state = value
  }

  try {
    await invoke('update_conditional_prompt_state', {
      promptId,
      newState: value,
    })
    emitUpdate() // 状态改变后立即更新生成的content
  }
  catch (error) {
    console.error('保存条件性prompt状态失败:', error)
    showError(`保存设置失败: ${(error as any)?.message}` || error)
    if (prompt) {
      prompt.current_state = !value
    }
  }
}

function generateConditionalContent(): string {
  const conditionalTexts: string[] = []
  conditionalPrompts.value.forEach((prompt) => {
    const isEnabled = prompt.current_state ?? false
    const template = isEnabled ? prompt.template_true : prompt.template_false
    if (template && template.trim()) {
      conditionalTexts.push(template.trim())
    }
  })
  return conditionalTexts.length > 0 ? `\n\n${conditionalTexts.join('\n')}` : ''
}

function getConditionalDescription(prompt: CustomPrompt): string {
  const isEnabled = prompt.current_state ?? false
  const template = isEnabled ? prompt.template_true : prompt.template_false
  if (template && template.trim()) {
    return template.trim()
  }
  return prompt.description || ''
}

async function initializeDragSort() {
  await nextTick()
  setTimeout(() => {
    let targetContainer = promptContainer.value
    if (!targetContainer) {
      targetContainer = document.querySelector('[data-prompt-container]') as HTMLElement
    }

    if (targetContainer) {
      promptContainer.value = targetContainer
      start()
    }
  }, 500)
}

async function savePromptOrder() {
  try {
    const promptIds = sortablePrompts.value.map(p => p.id)
    await invoke('update_custom_prompt_order', { promptIds })
  }
  catch (error) {
    console.error('保存排序失败:', error)
    loadCustomPrompts()
  }
}

watch(userInput, () => {
  emitUpdate()
})

onMounted(async () => {
  await loadCustomPrompts()
  const unlistenCustomPromptUpdate = await listen('custom-prompt-updated', () => {
    loadCustomPrompts()
  })

  onUnmounted(() => {
    unlistenCustomPromptUpdate()
    stop()
  })
})

function reset() {
  userInput.value = ''
  selectedOptions.value = []
  uploadedImages.value = []
  imageFingerprints.value.clear()
  imageFingerprintMap.value.clear()
  emitUpdate()
}

function updateData(data: { userInput?: string, selectedOptions?: string[], draggedImages?: string[] }) {
  if (data.userInput !== undefined)
    userInput.value = data.userInput
  if (data.selectedOptions !== undefined)
    selectedOptions.value = data.selectedOptions
  if (data.draggedImages !== undefined)
    uploadedImages.value = data.draggedImages
  emitUpdate()
}

defineExpose({
  reset,
  canSubmit,
  statusText,
  updateData,
  handleQuoteMessage,
})
</script>

<template>
  <!-- 增加底部 padding 防止内容被 fixed 输入框遮挡 -->
  <div class="pb-32">
    <!-- 预定义选项 - 复古风格 -->
    <div v-if="!loading && hasOptions" class="retro-options-section" data-guide="predefined-options">
      <div class="retro-options-grid">
        <button
          v-for="(option, index) in request!.predefined_options"
          :key="`option-${index}`"
          class="retro-option-btn"
          :class="{ selected: selectedOptions.includes(option) }"
          @click="handleOptionToggle(option)"
        >
          <div class="option-checkbox">
            <div v-if="selectedOptions.includes(option)" class="i-carbon-checkmark w-3 h-3" />
          </div>
          <span class="option-text">{{ option }}</span>
        </button>
      </div>
    </div>

    <!-- 图片预览区域 (保持在文档流中) -->
    <div v-if="!loading && uploadedImages.length > 0" class="mb-4">
      <div class="flex flex-wrap gap-3">
        <div
          v-for="(image, index) in uploadedImages"
          :key="`image-${index}`"
          class="relative group"
        >
          <ImagePreview
            :src="image"
            :width="80"
            :height="80"
            object-fit="cover"
            class="rounded-xl border border-gray-200 shadow-sm"
          />
          <button
            class="absolute -top-2 -right-2 w-6 h-6 bg-white rounded-full shadow-md flex items-center justify-center hover:bg-red-50 text-gray-400 hover:text-red-500 transition-colors opacity-0 group-hover:opacity-100"
            @click="removeImage(index)"
          >
            <div class="i-carbon-close w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>

    <!-- 底部悬浮输入区 -->
    <div class="fixed bottom-4 left-0 right-0 z-50 px-3 pointer-events-none">
      <div class="max-w-2xl mx-auto relative pointer-events-auto">
        <!-- 状态指示器 -->
        <transition
          enter-active-class="transition-all duration-200"
          leave-active-class="transition-all duration-200"
          enter-from-class="opacity-0 -translate-y-2"
          leave-to-class="opacity-0 -translate-y-2"
        >
          <div
            v-if="submitting"
            class="absolute bottom-full mb-2 left-4 right-4 flex items-center justify-center"
          >
            <div class="retro-status-badge">
              <div class="i-carbon-circle-dash animate-spin w-3.5 h-3.5" />
              <span>SUBMITTING...</span>
            </div>
          </div>
        </transition>

        <!-- 设置面板 (可折叠) -->
        <transition
          enter-active-class="transition duration-200 ease-out"
          enter-from-class="translate-y-4 opacity-0"
          enter-to-class="translate-y-0 opacity-100"
          leave-active-class="transition duration-150 ease-in"
          leave-from-class="translate-y-0 opacity-100"
          leave-to-class="translate-y-4 opacity-0"
        >
          <div v-if="showSettings" class="retro-settings-panel">
            <!-- 快捷模板 -->
            <div v-if="customPromptEnabled && sortablePrompts.length > 0" class="space-y-2">
              <div class="panel-label">TEMPLATES</div>
              <div ref="promptContainer" data-prompt-container class="flex flex-wrap gap-2">
                <div
                  v-for="prompt in sortablePrompts"
                  :key="prompt.id"
                  class="retro-template-chip"
                >
                  <div class="drag-handle cursor-grab active:cursor-grabbing">
                    <div class="i-carbon-drag-horizontal w-3 h-3" />
                  </div>
                  <span class="cursor-pointer" @click="handlePromptClick(prompt)">{{ prompt.name }}</span>
                </div>
              </div>
            </div>

            <!-- 上下文追加 -->
            <div v-if="customPromptEnabled && conditionalPrompts.length > 0" class="space-y-2">
              <div class="panel-label">CONTEXT</div>
              <div class="grid grid-cols-2 gap-2">
                <div
                  v-for="prompt in conditionalPrompts"
                  :key="prompt.id"
                  class="retro-context-item"
                >
                  <div class="flex-1 min-w-0 mr-3">
                    <div class="context-title">
                      {{ prompt.condition_text || prompt.name }}
                    </div>
                    <div class="context-desc">
                      {{ getConditionalDescription(prompt) }}
                    </div>
                  </div>
                  <BaseSwitch
                    :model-value="prompt.current_state ?? false"
                    size="small"
                    @update:model-value="(value: boolean) => handleConditionalToggle(prompt.id, value)"
                  />
                </div>
              </div>
            </div>
          </div>
        </transition>

        <!-- 输入框本体 - 复古风格 -->
        <div class="retro-input-box">
          <!-- 设置按钮 -->
          <button
            class="retro-icon-btn"
            :class="{ active: showSettings }"
            title="设置与模板"
            @click="toggleSettings"
          >
            <div class="i-carbon-settings-adjust w-4 h-4" />
          </button>

          <!-- 文本输入框 -->
          <div class="input-wrapper">
            <BaseTextarea
              ref="textareaRef"
              v-model="userInput"
              size="small"
              :placeholder="hasOptions ? 'ADD COMMENT...' : 'TYPE RESPONSE...'"
              :disabled="submitting"
              :autosize="{ minRows: 3, maxRows: 12 }"
              class="retro-textarea"
              @paste="handleImagePaste"
              @keydown.enter="handleEnterKey"
            />
          </div>

          <!-- 操作按钮组 -->
          <div class="flex items-center gap-1 shrink-0">
            <!-- 继续按钮 -->
            <button
              v-if="continueReplyEnabled"
              class="retro-icon-btn"
              title="继续生成"
              @click="emit('continue')"
            >
              <div class="i-carbon-play w-4 h-4" />
            </button>

            <!-- 发送按钮 -->
            <button
              class="retro-send-btn"
              :class="{ disabled: !canSubmit || submitting }"
              :disabled="!canSubmit || submitting"
              @click="emit('submit')"
            >
              <div v-if="submitting" class="i-carbon-circle-dash animate-spin w-4 h-4" />
              <div v-else class="i-carbon-arrow-up w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 插入模式选择对话框 -->
    <BaseModal v-model:show="showInsertDialog" title="插入Prompt">
      <template #header>
        <div class="flex items-center gap-2">
          <div class="i-carbon-text-creation w-4 h-4" />
          <span>插入Prompt</span>
        </div>
      </template>
      <div class="space-y-4">
        <p class="text-sm text-gray-600">
          输入框中已有内容，请选择插入模式：
        </p>
        <div class="bg-gray-50 p-3 rounded-xl text-sm border border-gray-100">
          {{ pendingPromptContent }}
        </div>
      </div>
      <template #footer>
        <div class="flex gap-2 justify-end">
          <BaseButton ghost @click="showInsertDialog = false">
            取消
          </BaseButton>
          <BaseButton secondary @click="handleInsertMode('replace')">
            替换
          </BaseButton>
          <BaseButton type="primary" @click="handleInsertMode('append')">
            追加
          </BaseButton>
        </div>
      </template>
    </BaseModal>

  </div>
</template>

<style scoped>
/* 复古选项样式 */
.retro-options-section {
  margin-bottom: 1rem;
}

.retro-options-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.5rem;
}

.retro-option-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 0.75rem;
  background: #fbfaf8;
  border: 2px solid #1f2937;
  box-shadow: 2px 2px 0px 0px rgba(31, 41, 55, 1);
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
  font-weight: 600;
  color: #1f2937;
  text-align: left;
  cursor: pointer;
  transition: all 0.1s;
}

.retro-option-btn:hover {
  background: #f3f4f6;
}

.retro-option-btn:active {
  box-shadow: none;
  transform: translate(2px, 2px);
}

.retro-option-btn.selected {
  background: #1f2937;
  color: white;
}

.retro-option-btn.selected .option-checkbox {
  background: white;
  color: #1f2937;
}

.option-checkbox {
  width: 1.125rem;
  height: 1.125rem;
  border: 2px solid currentColor;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.option-text {
  flex: 1;
  line-height: 1.3;
}

/* 复古风格输入框 */
.retro-input-box {
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
  padding: 0.5rem;
  background: #fbfaf8;
  border: 2px solid #1f2937;
  box-shadow: 4px 4px 0px 0px rgba(31, 41, 55, 1);
}

.input-wrapper {
  flex: 1;
  min-width: 0;
  padding: 0.25rem 0.5rem;
  overflow: hidden;
}

.retro-textarea {
  background: transparent !important;
  border: none !important;
  box-shadow: none !important;
  padding: 0 !important;
  font-family: ui-monospace, monospace !important;
  font-size: 0.875rem !important;
  caret-color: #1f2937 !important;
  outline: none !important;
  border-radius: 0 !important;
}

.retro-textarea:focus {
  outline: none !important;
  box-shadow: none !important;
  border: none !important;
}

.retro-textarea::placeholder {
  color: #9ca3af !important;
  opacity: 1 !important;
}

/* 移除 BaseTextarea 组件的聚焦样式 */
:deep(.retro-textarea),
:deep(.retro-textarea textarea),
:deep(.retro-textarea input) {
  outline: none !important;
  box-shadow: none !important;
  border: none !important;
  border-radius: 0 !important;
}

:deep(.retro-textarea:focus),
:deep(.retro-textarea textarea:focus),
:deep(.retro-textarea input:focus) {
  outline: none !important;
  box-shadow: none !important;
  border: none !important;
}

/* 移除任何 focus-visible 样式 */
:deep(*:focus-visible) {
  outline: none !important;
}

.input-wrapper :deep(*) {
  outline: none !important;
  box-shadow: none !important;
}

.retro-icon-btn {
  width: 2rem;
  height: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border: 2px solid #1f2937;
  color: #6b7280;
  transition: all 0.1s;
  flex-shrink: 0;
}

.retro-icon-btn:hover {
  background: #f3f4f6;
  color: #1f2937;
}

.retro-icon-btn.active {
  background: #1f2937;
  color: white;
}

.retro-icon-btn.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.retro-send-btn {
  width: 2rem;
  height: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #1f2937;
  border: 2px solid #1f2937;
  color: white;
  box-shadow: 2px 2px 0px 0px rgba(31, 41, 55, 0.5);
  transition: all 0.1s;
  flex-shrink: 0;
}

.retro-send-btn:hover {
  background: #374151;
}

.retro-send-btn:active {
  box-shadow: none;
  transform: translate(2px, 2px);
}

.retro-send-btn.disabled {
  background: #d1d5db;
  border-color: #d1d5db;
  box-shadow: none;
  cursor: not-allowed;
}

/* 状态徽章 */
.retro-status-badge {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  background: #fbfaf8;
  border: 2px solid #1f2937;
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

/* 设置面板 */
.retro-settings-panel {
  position: absolute;
  bottom: 100%;
  margin-bottom: 0.75rem;
  left: 0;
  right: 0;
  background: #fbfaf8;
  border: 2px solid #1f2937;
  box-shadow: 4px 4px 0px 0px rgba(31, 41, 55, 1);
  padding: 1rem;
  max-height: 60vh;
  overflow-y: auto;
}

.panel-label {
  font-size: 0.625rem;
  font-weight: 700;
  color: #6b7280;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  margin-bottom: 0.5rem;
}

.retro-template-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  font-weight: 600;
  background: white;
  border: 2px solid #1f2937;
  color: #1f2937;
  transition: all 0.1s;
}

.retro-template-chip:hover {
  background: #f3f4f6;
}

.retro-context-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.625rem;
  background: white;
  border: 2px solid #e5e7eb;
}

.context-title {
  font-size: 0.75rem;
  font-weight: 700;
  color: #1f2937;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-desc {
  font-size: 0.625rem;
  color: #6b7280;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 0.125rem;
}

/* 自定义滚动条隐藏 */
:deep(textarea) {
  padding: 0 !important;
  background-color: transparent !important;
  box-shadow: none !important;
  border: none !important;
}

:deep(textarea:focus) {
  box-shadow: none !important;
}

/* Sortable.js 拖拽样式 */
.sortable-ghost {
  opacity: 0.5;
  background: #f3f4f6;
}

.sortable-chosen {
  cursor: grabbing !important;
}

.sortable-drag {
  opacity: 0.9;
  transform: scale(1.02);
  box-shadow: 4px 4px 0px 0px rgba(31, 41, 55, 0.3);
  z-index: 100;
}
</style>
