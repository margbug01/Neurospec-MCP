<script setup lang="ts">
import { ref, watch } from 'vue'
import BaseButton from '../base/Button.vue'
import BaseModal from '../base/Modal.vue'
import BaseTextarea from '../base/Textarea.vue'

interface Props {
  show?: boolean
  originalText: string
  enhancedText: string
}

interface Emits {
  'update:show': [value: boolean]
  'use-enhanced': [text: string]
  'cancel': []
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
})

const emit = defineEmits<Emits>()

// 可编辑的增强文本
const editableEnhanced = ref('')

// 监听增强文本变化，更新可编辑副本
watch(() => props.enhancedText, (newValue) => {
  editableEnhanced.value = newValue
}, { immediate: true })

function handleUseEnhanced() {
  emit('use-enhanced', editableEnhanced.value)
  emit('update:show', false)
}

function handleCancel() {
  emit('cancel')
  emit('update:show', false)
}
</script>

<template>
  <BaseModal
    :show="show"
    title="增强预览"
    :closable="true"
    :mask-closable="false"
    @update:show="(val: boolean) => emit('update:show', val)"
  >
    <template #header>
      <div class="flex items-center gap-2">
        <div class="i-carbon-magic-wand w-4 h-4 text-purple-600" />
        <span>增强预览</span>
      </div>
    </template>

    <div class="space-y-4">
      <!-- 原始文本 -->
      <div>
        <div class="text-xs font-medium text-gray-500 mb-2">
          原始文本
        </div>
        <div class="bg-gray-50 p-3 rounded-xl text-sm border border-gray-100 max-h-32 overflow-y-auto">
          {{ originalText }}
        </div>
      </div>

      <!-- 增强后的文本 (可编辑) -->
      <div>
        <div class="text-xs font-medium text-gray-500 mb-2">
          增强文本 (可编辑)
        </div>
        <BaseTextarea
          v-model="editableEnhanced"
          :autosize="{ minRows: 4, maxRows: 12 }"
          placeholder="增强后的文本..."
          class="text-sm"
        />
      </div>

      <!-- 提示信息 -->
      <div class="flex items-start gap-2 p-3 bg-blue-50 rounded-lg border border-blue-100">
        <div class="i-carbon-information w-4 h-4 text-blue-600 mt-0.5 shrink-0" />
        <p class="text-xs text-blue-700">
          您可以在上方编辑增强后的文本，点击"使用增强"将其应用到输入框。
        </p>
      </div>
    </div>

    <template #footer>
      <div class="flex gap-2 justify-end">
        <BaseButton ghost @click="handleCancel">
          取消
        </BaseButton>
        <BaseButton type="primary" @click="handleUseEnhanced">
          使用增强
        </BaseButton>
      </div>
    </template>
  </BaseModal>
</template>

<style scoped>
/* 自定义滚动条样式 */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: #94a3b8;
}
</style>
