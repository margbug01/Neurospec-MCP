<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMemory, categoryConfig, type MemoryCategory, type MemoryEntry } from '../../composables/useMemory'
import { useEmbedding, providerOptions } from '../../composables/useEmbedding'
import { useToast } from '../../composables/useToast'
import BaseAlert from '../base/Alert.vue'
import BaseButton from '../base/Button.vue'
import BaseCard from '../base/Card.vue'
import BaseInput from '../base/Input.vue'
import BaseModal from '../base/Modal.vue'
import BaseSpinner from '../base/Spinner.vue'
import BaseSwitch from '../base/Switch.vue'
import BaseTag from '../base/Tag.vue'
import BaseTextarea from '../base/Textarea.vue'

const {
  memories,
  loading,
  error,
  currentPage,
  totalPages,
  totalCount,
  selectedCategory,
  projectPath,
  hasMemories,
  hasPrevPage,
  hasNextPage,
  setProjectPath,
  detectProjectPath,
  loadMemories,
  addMemory,
  updateMemory,
  deleteMemory,
  filterByCategory,
  goToPage,
} = useMemory()

const emit = defineEmits<{
  navigateTo: [tab: string]
}>()

const toast = useToast()

// 项目路径输入
const projectPathInput = ref('')

// 添加记忆弹窗
const showAddModal = ref(false)
const newMemoryContent = ref('')
const newMemoryCategory = ref<MemoryCategory>('context')

// 编辑记忆弹窗
const showEditModal = ref(false)
const editingMemory = ref<MemoryEntry | null>(null)
const editContent = ref('')

// 删除确认弹窗
const showDeleteModal = ref(false)
const deletingMemory = ref<MemoryEntry | null>(null)

// 嵌入配置
const embedding = useEmbedding()
const showEmbeddingModal = ref(false)
const testResult = ref<{ success: boolean; message: string } | null>(null)

// 打开嵌入配置弹窗
async function openEmbeddingModal() {
  await embedding.loadConfig()
  testResult.value = null
  showEmbeddingModal.value = true
}

// 保存嵌入配置
async function handleSaveEmbedding() {
  const success = await embedding.saveConfig()
  if (success) {
    toast.success('配置保存成功')
    showEmbeddingModal.value = false
  } else {
    toast.error(embedding.error.value || '保存失败')
  }
}

// 测试嵌入连接
async function handleTestEmbedding() {
  testResult.value = await embedding.testConnection()
  if (testResult.value.success) {
    toast.success('连接成功')
  } else {
    toast.error(testResult.value.message)
  }
}

// 设置项目路径并加载
async function handleSetProject() {
  if (!projectPathInput.value.trim()) {
    toast.warning('请输入项目路径')
    return
  }
  setProjectPath(projectPathInput.value.trim())
  await loadMemories()
}

// 打开添加弹窗
function openAddModal() {
  newMemoryContent.value = ''
  newMemoryCategory.value = 'context'
  showAddModal.value = true
}

// 添加记忆
async function handleAddMemory() {
  if (!newMemoryContent.value.trim()) {
    toast.warning('请输入记忆内容')
    return
  }
  try {
    await addMemory(newMemoryContent.value.trim(), newMemoryCategory.value)
    toast.success('记忆添加成功')
    showAddModal.value = false
  } catch (e: any) {
    toast.error(`添加失败: ${e}`)
  }
}

// 打开编辑弹窗
function openEditModal(memory: MemoryEntry) {
  editingMemory.value = memory
  editContent.value = memory.content
  showEditModal.value = true
}

// 更新记忆
async function handleUpdateMemory() {
  if (!editingMemory.value || !editContent.value.trim()) {
    toast.warning('请输入记忆内容')
    return
  }
  try {
    await updateMemory(editingMemory.value.id, editContent.value.trim())
    toast.success('记忆更新成功')
    showEditModal.value = false
  } catch (e: any) {
    toast.error(`更新失败: ${e}`)
  }
}

// 打开删除确认
function openDeleteModal(memory: MemoryEntry) {
  deletingMemory.value = memory
  showDeleteModal.value = true
}

// 删除记忆
async function handleDeleteMemory() {
  if (!deletingMemory.value) return
  try {
    await deleteMemory(deletingMemory.value.id)
    toast.success('记忆删除成功')
    showDeleteModal.value = false
  } catch (e: any) {
    toast.error(`删除失败: ${e}`)
  }
}

// 获取分类样式
function getCategoryStyle(category: MemoryCategory) {
  const config = categoryConfig[category]
  return {
    icon: config.icon,
    label: config.label,
    variant: config.color as 'blue' | 'green' | 'yellow' | 'default',
  }
}

// 自动检测中状态
const detecting = ref(false)

onMounted(async () => {
  // 自动检测项目路径
  detecting.value = true
  try {
    const detected = await detectProjectPath()
    if (detected) {
      projectPathInput.value = detected
      await loadMemories()
    }
  } finally {
    detecting.value = false
  }
})
</script>

<template>
  <div class="max-w-3xl mx-auto tab-content">
    <!-- 返回按钮 -->
    <button class="back-btn" @click="emit('navigateTo', 'intro')">
      <div class="i-carbon-arrow-left w-3 h-3" />
      <span>返回</span>
    </button>

    <div class="space-y-4">
      <!-- 项目路径设置 -->
      <BaseCard v-if="!projectPath" padding="medium" shadow="sm">
        <div class="space-y-3">
          <div class="text-lg font-medium">
            📚 项目记忆管理
          </div>
          <div v-if="detecting" class="flex items-center gap-2 text-sm opacity-60">
            <BaseSpinner size="small" />
            正在自动检测项目路径...
          </div>
          <template v-else>
            <div class="text-sm opacity-60">
              请输入项目根路径（Git仓库目录）以管理该项目的记忆
            </div>
            <div class="flex gap-2">
              <BaseInput
                v-model="projectPathInput"
                placeholder="C:/path/to/your/project"
                class="flex-1"
                @keyup.enter="handleSetProject"
              />
              <BaseButton variant="primary" @click="handleSetProject">
                加载项目
              </BaseButton>
            </div>
          </template>
        </div>
      </BaseCard>

      <!-- 已加载项目 -->
      <template v-else>
        <!-- 头部操作栏 -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="text-sm opacity-60">项目:</span>
            <span class="text-sm font-medium truncate max-w-xs">{{ projectPath }}</span>
            <BaseButton size="small" variant="ghost" @click="projectPath = ''">
              <div class="i-carbon-close w-4 h-4" />
            </BaseButton>
          </div>
          <div class="flex items-center gap-2">
            <BaseButton variant="ghost" size="small" @click="openEmbeddingModal">
              <div class="i-carbon-settings w-4 h-4 mr-1" />
              嵌入配置
            </BaseButton>
            <BaseButton variant="primary" size="small" @click="openAddModal">
              <div class="i-carbon-add w-4 h-4 mr-1" />
              添加记忆
            </BaseButton>
          </div>
        </div>

        <!-- 分类筛选 -->
        <div class="flex gap-2 flex-wrap">
          <BaseButton
            :variant="selectedCategory === 'all' ? 'primary' : 'ghost'"
            size="small"
            @click="filterByCategory('all')"
          >
            全部
          </BaseButton>
          <BaseButton
            v-for="(config, key) in categoryConfig"
            :key="key"
            :variant="selectedCategory === key ? 'primary' : 'ghost'"
            size="small"
            @click="filterByCategory(key as MemoryCategory)"
          >
            {{ config.icon }} {{ config.label }}
          </BaseButton>
        </div>

        <!-- 加载状态 -->
        <div v-if="loading" class="text-center py-8">
          <BaseSpinner size="medium" />
          <div class="mt-2 text-sm opacity-60">
            加载记忆中...
          </div>
        </div>

        <!-- 错误提示 -->
        <BaseAlert v-else-if="error" type="error" :title="error" />

        <!-- 空状态 -->
        <div v-else-if="!hasMemories" class="text-center py-8">
          <div class="text-4xl mb-2">
            📭
          </div>
          <div class="text-sm opacity-60">
            暂无记忆，点击"添加记忆"开始
          </div>
        </div>

        <!-- 记忆列表 -->
        <div v-else class="space-y-3">
          <BaseCard
            v-for="memory in memories"
            :key="memory.id"
            padding="small"
            shadow="sm"
            class="hover:shadow-md transition-shadow"
          >
            <div class="flex items-start gap-3">
              <!-- 分类图标 -->
              <div class="text-xl flex-shrink-0">
                {{ getCategoryStyle(memory.category).icon }}
              </div>

              <!-- 内容 -->
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <BaseTag :variant="getCategoryStyle(memory.category).variant" size="small">
                    {{ getCategoryStyle(memory.category).label }}
                  </BaseTag>
                  <span class="text-xs opacity-40 truncate">
                    ID: {{ memory.id.slice(0, 16) }}...
                  </span>
                </div>
                <div class="text-sm">
                  {{ memory.content }}
                </div>
              </div>

              <!-- 操作按钮 -->
              <div class="flex gap-1 flex-shrink-0">
                <BaseButton size="small" variant="ghost" @click="openEditModal(memory)">
                  <div class="i-carbon-edit w-4 h-4" />
                </BaseButton>
                <BaseButton size="small" variant="ghost" @click="openDeleteModal(memory)">
                  <div class="i-carbon-trash-can w-4 h-4 text-red-500" />
                </BaseButton>
              </div>
            </div>
          </BaseCard>
        </div>

        <!-- 分页 -->
        <div v-if="totalPages > 1" class="flex items-center justify-center gap-2 pt-4">
          <BaseButton size="small" :disabled="!hasPrevPage" @click="goToPage(currentPage - 1)">
            上一页
          </BaseButton>
          <span class="text-sm">
            {{ currentPage }} / {{ totalPages }}
          </span>
          <BaseButton size="small" :disabled="!hasNextPage" @click="goToPage(currentPage + 1)">
            下一页
          </BaseButton>
        </div>

        <!-- 统计 -->
        <div class="text-center text-sm opacity-60">
          共 {{ totalCount }} 条记忆
        </div>
      </template>
    </div>

    <!-- 添加记忆弹窗 -->
    <BaseModal v-model:show="showAddModal" title="添加记忆" :closable="true">
      <div class="space-y-4">
        <div>
          <label class="block text-sm font-medium mb-2">分类</label>
          <div class="flex gap-2 flex-wrap">
            <BaseButton
              v-for="(config, key) in categoryConfig"
              :key="key"
              :variant="newMemoryCategory === key ? 'primary' : 'ghost'"
              size="small"
              @click="newMemoryCategory = key as MemoryCategory"
            >
              {{ config.icon }} {{ config.label }}
            </BaseButton>
          </div>
        </div>
        <div>
          <label class="block text-sm font-medium mb-2">内容</label>
          <BaseTextarea
            v-model="newMemoryContent"
            placeholder="输入要记住的内容..."
            :rows="4"
          />
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <BaseButton @click="showAddModal = false">
            取消
          </BaseButton>
          <BaseButton variant="primary" @click="handleAddMemory">
            添加
          </BaseButton>
        </div>
      </template>
    </BaseModal>

    <!-- 编辑记忆弹窗 -->
    <BaseModal v-model:show="showEditModal" title="编辑记忆" :closable="true">
      <div class="space-y-4">
        <div v-if="editingMemory">
          <BaseTag :variant="getCategoryStyle(editingMemory.category).variant" size="small" class="mb-2">
            {{ getCategoryStyle(editingMemory.category).icon }} {{ getCategoryStyle(editingMemory.category).label }}
          </BaseTag>
        </div>
        <div>
          <label class="block text-sm font-medium mb-2">内容</label>
          <BaseTextarea
            v-model="editContent"
            placeholder="输入新内容..."
            :rows="4"
          />
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <BaseButton @click="showEditModal = false">
            取消
          </BaseButton>
          <BaseButton variant="primary" @click="handleUpdateMemory">
            保存
          </BaseButton>
        </div>
      </template>
    </BaseModal>

    <!-- 删除确认弹窗 -->
    <BaseModal v-model:show="showDeleteModal" title="确认删除" :closable="true">
      <div v-if="deletingMemory" class="space-y-2">
        <p>确定要删除这条记忆吗？</p>
        <div class="p-3 bg-surface-100 dark:bg-surface-800 rounded text-sm">
          {{ deletingMemory.content }}
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <BaseButton @click="showDeleteModal = false">
            取消
          </BaseButton>
          <BaseButton variant="primary" class="bg-red-500 hover:bg-red-600" @click="handleDeleteMemory">
            删除
          </BaseButton>
        </div>
      </template>
    </BaseModal>

    <!-- 嵌入配置弹窗 -->
    <BaseModal v-model:show="showEmbeddingModal" title="🧮 嵌入模型配置" :closable="true">
      <div class="space-y-4">
        <!-- Provider 选择 -->
        <div>
          <label class="block text-sm font-medium mb-2">Provider</label>
          <div class="flex gap-2 flex-wrap">
            <BaseButton
              v-for="option in providerOptions"
              :key="option.value"
              :variant="embedding.config.provider === option.value ? 'primary' : 'ghost'"
              size="small"
              @click="embedding.onProviderChange(option.value)"
            >
              {{ option.label }}
            </BaseButton>
          </div>
        </div>

        <!-- API Key -->
        <div>
          <label class="block text-sm font-medium mb-2">API Key</label>
          <BaseInput
            v-model="embedding.config.api_key"
            type="password"
            placeholder="输入 API Key..."
          />
        </div>

        <!-- Model -->
        <div>
          <label class="block text-sm font-medium mb-2">Model</label>
          <BaseInput
            v-model="embedding.config.model"
            placeholder="模型名称"
          />
        </div>

        <!-- Base URL -->
        <div>
          <label class="block text-sm font-medium mb-2">Base URL</label>
          <BaseInput
            v-model="embedding.config.base_url"
            placeholder="API Base URL"
          />
        </div>

        <!-- 启用缓存 -->
        <div class="flex items-center justify-between">
          <span class="text-sm">启用向量缓存</span>
          <BaseSwitch v-model="embedding.config.cache_enabled" />
        </div>

        <!-- 测试结果 -->
        <div v-if="testResult" class="p-3 rounded text-sm" :class="testResult.success ? 'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300' : 'bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300'">
          {{ testResult.message }}
        </div>
      </div>
      <template #footer>
        <div class="flex justify-between">
          <BaseButton @click="handleTestEmbedding" :disabled="embedding.loading.value">
            {{ embedding.loading.value ? '测试中...' : '测试连接' }}
          </BaseButton>
          <div class="flex gap-2">
            <BaseButton @click="showEmbeddingModal = false">
              取消
            </BaseButton>
            <BaseButton variant="primary" @click="handleSaveEmbedding" :disabled="embedding.loading.value">
              保存
            </BaseButton>
          </div>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  margin-bottom: 0.75rem;
  background: white;
  border: 2px solid #1f2937;
  font-weight: 700;
  font-size: 0.625rem;
  letter-spacing: 0.05em;
  cursor: pointer;
  transition: all 0.1s;
  font-family: ui-monospace, monospace;
}

.back-btn:hover {
  background: #1f2937;
  color: white;
}
</style>
