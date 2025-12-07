<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import BaseSpinner from '../base/Spinner.vue'

interface MemoryEntry {
  id: string
  content: string
  category: string
  created_at: string
  usage_count: number
}

interface MemoryListResponse {
  memories: MemoryEntry[]
  total: number
  page: number
  page_size: number
}

const memories = ref<MemoryEntry[]>([])
const loading = ref(false)
const projectPath = ref<string | null>(null)

const categoryColors: Record<string, { bg: string; border: string; text: string }> = {
  rule: { bg: '#fef2f2', border: '#ef4444', text: '#dc2626' },
  preference: { bg: '#f0fdf4', border: '#22c55e', text: '#16a34a' },
  pattern: { bg: '#eff6ff', border: '#3b82f6', text: '#2563eb' },
  context: { bg: '#fefce8', border: '#eab308', text: '#ca8a04' },
}

const categoryLabels: Record<string, string> = {
  rule: '规则',
  preference: '偏好',
  pattern: '模式',
  context: '上下文',
}

async function detectProject() {
  try {
    const result = await invoke<{ path: string }>('detect_project_agents')
    if (result.path) {
      projectPath.value = result.path
    }
  } catch (e) {
    console.error('检测项目失败:', e)
  }
}

async function loadMemories() {
  if (!projectPath.value) {
    await detectProject()
  }
  
  loading.value = true
  try {
    const result = await invoke<MemoryListResponse>('list_memories', {
      projectPath: projectPath.value,
      category: null,
      page: 1,
      pageSize: 15,
    })
    memories.value = result.memories || []
  } catch (e) {
    console.error('加载记忆失败:', e)
    memories.value = []
  } finally {
    loading.value = false
  }
}

function truncateContent(content: string, maxLen = 100): string {
  if (content.length <= maxLen) return content
  return content.substring(0, maxLen) + '...'
}

function getCategoryStyle(category: string) {
  return categoryColors[category] || categoryColors.context
}

function getCategoryLabel(category: string) {
  return categoryLabels[category] || category
}

// 按分类分组
const groupedMemories = computed(() => {
  const groups: Record<string, MemoryEntry[]> = {}
  memories.value.forEach(m => {
    if (!groups[m.category]) {
      groups[m.category] = []
    }
    groups[m.category].push(m)
  })
  return groups
})

onMounted(() => {
  loadMemories()
})
</script>

<template>
  <div class="mini-memory">
    <div class="panel-header">
      <div class="panel-title">
        <div class="i-carbon-brain w-4 h-4" />
        <span>项目记忆</span>
      </div>
      <button class="refresh-btn" title="刷新" @click="loadMemories">
        <div class="i-carbon-renew w-3.5 h-3.5" :class="{ 'animate-spin': loading }" />
      </button>
    </div>

    <div class="panel-content custom-scrollbar">
      <!-- 加载状态 -->
      <div v-if="loading && memories.length === 0" class="loading-state">
        <BaseSpinner size="small" />
        <span>加载中...</span>
      </div>

      <!-- 空状态 -->
      <div v-else-if="memories.length === 0" class="empty-state">
        <div class="i-carbon-brain w-8 h-8 opacity-40" />
        <span>暂无记忆</span>
        <span class="hint">使用 memory 工具添加</span>
      </div>

      <!-- 记忆列表 -->
      <div v-else class="memory-groups">
        <div
          v-for="(items, category) in groupedMemories"
          :key="category"
          class="memory-group"
        >
          <div
            class="group-header"
            :style="{
              borderColor: getCategoryStyle(category as string).border,
              color: getCategoryStyle(category as string).text,
            }"
          >
            {{ getCategoryLabel(category as string) }} ({{ items.length }})
          </div>
          <div class="group-items">
            <div
              v-for="memory in items"
              :key="memory.id"
              class="memory-item"
              :style="{
                borderLeftColor: getCategoryStyle(memory.category).border,
              }"
            >
              <div class="memory-content">
                {{ truncateContent(memory.content) }}
              </div>
              <div class="memory-meta">
                <span v-if="memory.usage_count > 0" class="usage-badge">
                  已用 {{ memory.usage_count }} 次
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 项目路径显示 -->
    <div v-if="projectPath" class="project-path">
      <div class="i-carbon-folder w-3 h-3" />
      <span>{{ projectPath.split(/[/\\]/).pop() }}</span>
    </div>
  </div>
</template>

<style scoped>
.mini-memory {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem;
  border-bottom: 2px dashed #d1d5db;
  background: #f9fafb;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 800;
  font-size: 0.75rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.refresh-btn {
  width: 1.75rem;
  height: 1.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border: 2px solid #1f2937;
  color: #6b7280;
  cursor: pointer;
  transition: all 0.1s;
}

.refresh-btn:hover {
  background: #1f2937;
  color: white;
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem 0;
  color: #6b7280;
  font-size: 0.75rem;
}

.empty-state .hint {
  font-size: 0.625rem;
  opacity: 0.7;
}

.memory-groups {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.memory-group {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.group-header {
  font-size: 0.625rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding-bottom: 0.25rem;
  border-bottom: 1px solid currentColor;
}

.group-items {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.memory-item {
  padding: 0.5rem;
  background: white;
  border: 1px solid #e5e7eb;
  border-left: 3px solid;
  transition: all 0.1s;
}

.memory-item:hover {
  border-color: #1f2937;
  border-left-color: inherit;
}

.memory-content {
  font-size: 0.75rem;
  line-height: 1.4;
  color: #1f2937;
  word-break: break-word;
}

.memory-meta {
  display: flex;
  gap: 0.25rem;
  margin-top: 0.375rem;
}

.usage-badge {
  font-size: 0.5rem;
  padding: 0.0625rem 0.25rem;
  background: #f3f4f6;
  border: 1px solid #d1d5db;
  color: #6b7280;
  font-weight: 600;
}

.project-path {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.75rem;
  background: #f3f4f6;
  border-top: 1px solid #e5e7eb;
  font-size: 0.625rem;
  color: #6b7280;
  font-family: ui-monospace, monospace;
}

/* 自定义滚动条 */
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(31, 41, 55, 0.2);
  border-radius: 2px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(31, 41, 55, 0.4);
}
</style>
