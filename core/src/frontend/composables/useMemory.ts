import { invoke } from '@tauri-apps/api/core'
import { ref, computed } from 'vue'

// 记忆分类
export type MemoryCategory = 'rule' | 'preference' | 'pattern' | 'context'

// 记忆条目
export interface MemoryEntry {
  id: string
  content: string
  category: MemoryCategory
  created_at: string
  updated_at: string
}

// 记忆建议
export interface MemorySuggestion {
  id: string
  content: string
  category: MemoryCategory
  confidence: number
  reason: string
  keywords: string[]
  suggested_at: string
}

// 分页结果
export interface MemoryListResult {
  memories: MemoryEntry[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

// 记忆管理状态
const memories = ref<MemoryEntry[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const currentPage = ref(1)
const pageSize = ref(20)
const totalPages = ref(1)
const totalCount = ref(0)
const selectedCategory = ref<MemoryCategory | 'all'>('all')
const projectPath = ref('')

// 分类配置
export const categoryConfig = {
  rule: { label: '规则', icon: '🔵', color: 'blue' },
  preference: { label: '偏好', icon: '🟢', color: 'green' },
  pattern: { label: '模式', icon: '🟡', color: 'yellow' },
  context: { label: '上下文', icon: '⚪', color: 'gray' },
}

export function useMemory() {
  // 设置项目路径
  function setProjectPath(path: string) {
    projectPath.value = path
  }

  // 自动检测项目路径
  async function detectProjectPath(): Promise<string | null> {
    try {
      const path = await invoke<string>('detect_project_path')
      if (path) {
        projectPath.value = path
        return path
      }
      return null
    } catch (e) {
      console.log('自动检测项目路径失败:', e)
      return null
    }
  }

  // 加载记忆列表
  async function loadMemories(page = 1, category: MemoryCategory | 'all' = 'all') {
    if (!projectPath.value) {
      error.value = '请先设置项目路径'
      return
    }

    loading.value = true
    error.value = null

    try {
      const result = await invoke<MemoryListResult>('memory_list', {
        projectPath: projectPath.value,
        category: category === 'all' ? '' : category,
        page,
        pageSize: pageSize.value,
      })

      memories.value = result.memories
      currentPage.value = result.page
      totalPages.value = result.total_pages
      totalCount.value = result.total
      selectedCategory.value = category
    } catch (e: any) {
      error.value = typeof e === 'string' ? e : e?.message || '加载记忆失败'
      console.error('加载记忆失败:', e)
    } finally {
      loading.value = false
    }
  }

  // 添加记忆
  async function addMemory(content: string, category: MemoryCategory) {
    if (!projectPath.value) {
      throw new Error('请先设置项目路径')
    }

    const result = await invoke<{ id: string }>('memory_add', {
      projectPath: projectPath.value,
      content,
      category,
    })

    // 刷新列表
    await loadMemories(currentPage.value, selectedCategory.value)
    return result.id
  }

  // 更新记忆
  async function updateMemory(id: string, content: string) {
    if (!projectPath.value) {
      throw new Error('请先设置项目路径')
    }

    await invoke('memory_update', {
      projectPath: projectPath.value,
      id,
      content,
    })

    // 刷新列表
    await loadMemories(currentPage.value, selectedCategory.value)
  }

  // 删除记忆
  async function deleteMemory(id: string) {
    if (!projectPath.value) {
      throw new Error('请先设置项目路径')
    }

    await invoke('memory_delete', {
      projectPath: projectPath.value,
      id,
    })

    // 刷新列表
    await loadMemories(currentPage.value, selectedCategory.value)
  }

  // 切换分类
  async function filterByCategory(category: MemoryCategory | 'all') {
    await loadMemories(1, category)
  }

  // 翻页
  async function goToPage(page: number) {
    await loadMemories(page, selectedCategory.value)
  }

  // 分析对话获取记忆建议
  async function analyzeMemorySuggestions(messages: string[]): Promise<MemorySuggestion[]> {
    try {
      const result = await invoke<MemorySuggestion[]>('analyze_memory_suggestions', {
        messages,
        projectPath: projectPath.value || null,
      })
      return result || []
    } catch (e) {
      console.error('分析记忆建议失败:', e)
      return []
    }
  }

  // 计算属性
  const hasMemories = computed(() => memories.value.length > 0)
  const hasPrevPage = computed(() => currentPage.value > 1)
  const hasNextPage = computed(() => currentPage.value < totalPages.value)

  return {
    // 状态
    memories,
    loading,
    error,
    currentPage,
    pageSize,
    totalPages,
    totalCount,
    selectedCategory,
    projectPath,

    // 计算属性
    hasMemories,
    hasPrevPage,
    hasNextPage,

    // 方法
    setProjectPath,
    detectProjectPath,
    loadMemories,
    addMemory,
    updateMemory,
    deleteMemory,
    filterByCategory,
    goToPage,
    analyzeMemorySuggestions,
  }
}
