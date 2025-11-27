<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref, watch } from 'vue'
import { useToast } from '../../composables/useToast'

interface MemorySuggestion {
  id: string
  content: string
  category: 'rule' | 'preference' | 'pattern' | 'context'
  confidence: number
  reason: string
  keywords: string[]
  suggested_at: string
}

interface Props {
  visible?: boolean
  messages?: string[]
  projectPath?: string
  mockMode?: boolean
}

interface Emits {
  confirm: [suggestions: MemorySuggestion[]]
  cancel: []
  addMemory: [suggestion: MemorySuggestion]
}

const props = withDefaults(defineProps<Props>(), {
  visible: false,
  messages: () => [],
  projectPath: '',
  mockMode: false,
})

const emit = defineEmits<Emits>()
const { showToast } = useToast()

// 状态
const loading = ref(false)
const suggestions = ref<MemorySuggestion[]>([])
const selectedSuggestions = ref<Set<string>>(new Set())

// 加载记忆建议
async function loadSuggestions() {
  if (props.mockMode) {
    // 模拟数据
    suggestions.value = [
      {
        id: 'std_indent',
        content: '项目编码规范 - 4空格缩进',
        category: 'rule',
        confidence: 0.85,
        reason: '检测到编码规范相关讨论',
        keywords: ['空格', '缩进', 'indent'],
        suggested_at: new Date().toISOString(),
      },
      {
        id: 'config_info',
        content: '项目配置信息',
        category: 'context',
        confidence: 0.72,
        reason: '检测到配置相关讨论',
        keywords: ['配置', 'config', 'settings'],
        suggested_at: new Date().toISOString(),
      },
    ]
    return
  }

  loading.value = true
  try {
    // 使用新的 analyze_memory_suggestions 命令
    const result = await invoke<MemorySuggestion[]>('analyze_memory_suggestions', {
      messages: props.messages,
      projectPath: props.projectPath || null,
    })

    suggestions.value = result || []
  }
  catch (error) {
    console.error('Failed to load memory suggestions:', error)
    showToast('加载记忆建议失败', 'error')
  }
  finally {
    loading.value = false
  }
}

// 切换选择状态
function toggleSelection(id: string) {
  if (selectedSuggestions.value.has(id)) {
    selectedSuggestions.value.delete(id)
  }
  else {
    selectedSuggestions.value.add(id)
  }
}

// 全选/取消全选
function toggleSelectAll() {
  if (selectedSuggestions.value.size === suggestions.value.length) {
    selectedSuggestions.value.clear()
  }
  else {
    suggestions.value.forEach(s => selectedSuggestions.value.add(s.id))
  }
}

// 添加选中的记忆
function addSelectedMemories() {
  const selected = suggestions.value.filter(s => selectedSuggestions.value.has(s.id))
  emit('confirm', selected)

  selected.forEach((suggestion) => {
    emit('addMemory', suggestion)
  })

  showToast(`已添加 ${selected.length} 条记忆建议`, 'success')
  close()
}

// 关闭弹窗
function close() {
  emit('cancel')
  selectedSuggestions.value.clear()
}

// 获取分类标签样式
function getCategoryStyle(category: string) {
  const styles = {
    rule: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    preference: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    pattern: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    context: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400',
  }
  return styles[category as keyof typeof styles] || styles.context
}

// 获取分类名称
function getCategoryName(category: string) {
  const names = {
    rule: '规则',
    preference: '偏好',
    pattern: '模式',
    context: '上下文',
  }
  return names[category as keyof typeof names] || category
}

// 组件挂载时加载建议
onMounted(() => {
  if (props.visible) {
    loadSuggestions()
  }
})

// 监听可见性变化
watch(() => props.visible, (newVisible) => {
  if (newVisible) {
    loadSuggestions()
  }
})
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="w-full max-w-2xl max-h-[80vh] bg-white dark:bg-gray-800 rounded-lg shadow-xl overflow-hidden">
      <!-- 头部 -->
      <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
            🧠 AI 记忆建议
          </h2>
          <button
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            @click="close"
          >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
          AI 检测到以下可能需要记忆化的信息
        </p>
      </div>

      <!-- 内容区域 -->
      <div class="px-6 py-4 overflow-y-auto" style="max-height: calc(80vh - 200px)">
        <!-- 加载状态 -->
        <div v-if="loading" class="flex items-center justify-center py-12">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" />
          <span class="ml-3 text-gray-600 dark:text-gray-400">正在分析对话...</span>
        </div>

        <!-- 空状态 -->
        <div v-else-if="suggestions.length === 0" class="text-center py-12">
          <div class="text-6xl mb-4">
            🤔
          </div>
          <p class="text-gray-600 dark:text-gray-400">
            暂无记忆建议。系统正在学习您的对话模式...
          </p>
        </div>

        <!-- 建议列表 -->
        <div v-else class="space-y-4">
          <!-- 全选按钮 -->
          <div class="flex items-center justify-between">
            <label class="flex items-center cursor-pointer">
              <input
                type="checkbox"
                :checked="selectedSuggestions.size === suggestions.length && suggestions.length > 0"
                :indeterminate="selectedSuggestions.size > 0 && selectedSuggestions.size < suggestions.length"
                class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                @change="toggleSelectAll"
              >
              <span class="ml-2 text-sm text-gray-700 dark:text-gray-300">
                全选 ({{ selectedSuggestions.size }}/{{ suggestions.length }})
              </span>
            </label>

            <span class="text-xs text-gray-500 dark:text-gray-400">
              点击卡片或复选框选择
            </span>
          </div>

          <!-- 记忆建议卡片 -->
          <div
            v-for="suggestion in suggestions"
            :key="suggestion.id"
            class="border border-gray-200 dark:border-gray-700 rounded-lg p-4 cursor-pointer hover:border-blue-300 dark:hover:border-blue-600 transition-colors"
            :class="{
              'ring-2 ring-blue-500 border-blue-500': selectedSuggestions.has(suggestion.id),
              'bg-gray-50 dark:bg-gray-750': !selectedSuggestions.has(suggestion.id),
            }"
            @click="toggleSelection(suggestion.id)"
          >
            <div class="flex items-start space-x-3">
              <!-- 复选框 -->
              <input
                type="checkbox"
                :checked="selectedSuggestions.has(suggestion.id)"
                class="mt-1 w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                @click.stop
              >

              <div class="flex-1 min-w-0">
                <!-- 标题和分类 -->
                <div class="flex items-center justify-between mb-2">
                  <h3 class="text-lg font-medium text-gray-900 dark:text-white">
                    {{ suggestion.content }}
                  </h3>
                  <span
                    class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium"
                    :class="getCategoryStyle(suggestion.category)"
                  >
                    {{ getCategoryName(suggestion.category) }}
                  </span>
                </div>

                <!-- 置信度 -->
                <div class="flex items-center space-x-2 mb-2">
                  <span class="text-sm text-gray-600 dark:text-gray-400">置信度:</span>
                  <div class="flex-1 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                    <div
                      class="bg-blue-500 h-2 rounded-full transition-all"
                      :style="{ width: `${suggestion.confidence * 100}%` }"
                    />
                  </div>
                  <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                    {{ Math.round(suggestion.confidence * 100) }}%
                  </span>
                </div>

                <!-- 原因 -->
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">
                  {{ suggestion.reason }}
                </p>

                <!-- 关键词 -->
                <div class="flex flex-wrap gap-1">
                  <span
                    v-for="keyword in suggestion.keywords"
                    :key="keyword"
                    class="inline-flex items-center px-2 py-1 rounded text-xs bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300"
                  >
                    {{ keyword }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 底部操作栏 -->
      <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-750">
        <div class="flex items-center justify-between">
          <div class="text-sm text-gray-600 dark:text-gray-400">
            已选择 {{ selectedSuggestions.size }} 条建议
          </div>
          <div class="flex space-x-3">
            <button
              class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
              @click="close"
            >
              取消
            </button>
            <button
              :disabled="selectedSuggestions.size === 0"
              class="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
              @click="addSelectedMemories"
            >
              添加选中记忆 ({{ selectedSuggestions.size }})
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 自定义滚动条 */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.5);
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.7);
}
</style>
