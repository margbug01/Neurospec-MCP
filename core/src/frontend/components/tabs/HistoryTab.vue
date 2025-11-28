<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref, withDefaults } from 'vue'
import { useToast } from '../../composables/useToast'
import BaseButton from '../base/Button.vue'
import BaseCard from '../base/Card.vue'
import BaseInput from '../base/Input.vue'
import BaseModal from '../base/Modal.vue'
import BaseSpinner from '../base/Spinner.vue'

interface InteractRecord {
  id: string
  timestamp: string
  request_message: string
  predefined_options: string[]
  user_response: string | null
  selected_options: string[]
  project_path: string | null
}

const toast = useToast()

interface Props {
  fromPopup?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  fromPopup: false
})

const emit = defineEmits<{
  navigateTo: [tab: string]
  closeToPopup: []
}>()

// 状态
const records = ref<InteractRecord[]>([])
const loading = ref(false)
const searchQuery = ref('')
const showClearModal = ref(false)
const expandedId = ref<string | null>(null)

// 加载历史记录
async function loadHistory() {
  loading.value = true
  try {
    const result = await invoke<InteractRecord[]>('get_interact_history_cmd', { count: 50 })
    records.value = result
  } catch (e: any) {
    toast.error(`加载失败: ${e}`)
  } finally {
    loading.value = false
  }
}

// 搜索历史
async function handleSearch() {
  if (!searchQuery.value.trim()) {
    await loadHistory()
    return
  }
  
  loading.value = true
  try {
    const result = await invoke<InteractRecord[]>('search_interact_history_cmd', { query: searchQuery.value })
    records.value = result
  } catch (e: any) {
    toast.error(`搜索失败: ${e}`)
  } finally {
    loading.value = false
  }
}

// 清空历史
async function handleClear() {
  try {
    await invoke('clear_interact_history_cmd')
    records.value = []
    toast.success('历史记录已清空')
    showClearModal.value = false
  } catch (e: any) {
    toast.error(`清空失败: ${e}`)
  }
}

// 格式化时间
function formatTime(timestamp: string): string {
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

// 截断消息
function truncateMessage(msg: string, maxLen = 100): string {
  if (msg.length <= maxLen) return msg
  return msg.substring(0, maxLen) + '...'
}

// 切换展开
function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

// 处理返回按钮点击
function handleBack() {
  if (props.fromPopup) {
    emit('closeToPopup')
  } else {
    emit('navigateTo', 'intro')
  }
}

onMounted(() => {
  loadHistory()
})
</script>

<template>
  <div class="history-tab">
    <div class="header">
      <button class="back-btn" title="返回" @click="handleBack">
        <div class="i-carbon-arrow-left w-4 h-4" />
      </button>
      <h2 class="title">
        <div class="i-carbon-time w-5 h-5" />
        INTERACT HISTORY
      </h2>
      <div class="actions">
        <BaseButton 
          size="small" 
          variant="secondary"
          :disabled="records.length === 0"
          @click="showClearModal = true"
        >
          <div class="i-carbon-trash-can w-3.5 h-3.5" />
          清空
        </BaseButton>
        <BaseButton size="small" @click="loadHistory">
          <div class="i-carbon-renew w-3.5 h-3.5" />
          刷新
        </BaseButton>
      </div>
    </div>

    <!-- 搜索栏 -->
    <div class="search-bar">
      <BaseInput
        v-model="searchQuery"
        placeholder="搜索历史记录..."
        @keyup.enter="handleSearch"
      />
      <BaseButton size="small" @click="handleSearch">搜索</BaseButton>
    </div>

    <!-- 加载状态 -->
    <div v-if="loading" class="loading-state">
      <BaseSpinner size="medium" />
      <p>加载中...</p>
    </div>

    <!-- 空状态 -->
    <div v-else-if="records.length === 0" class="empty-state">
      <div class="i-carbon-document-blank w-12 h-12 text-gray-400" />
      <p>暂无交互记录</p>
    </div>

    <!-- 记录列表 -->
    <div v-else class="records-list">
      <BaseCard 
        v-for="record in records" 
        :key="record.id" 
        class="record-card"
        @click="toggleExpand(record.id)"
      >
        <div class="record-header">
          <span class="record-time">{{ formatTime(record.timestamp) }}</span>
          <div class="record-badges">
            <span v-if="record.selected_options.length > 0" class="badge badge-option">
              {{ record.selected_options.length }} 选项
            </span>
            <span v-if="record.user_response" class="badge badge-input">
              有输入
            </span>
          </div>
        </div>
        
        <div class="record-message">
          {{ expandedId === record.id ? record.request_message : truncateMessage(record.request_message) }}
        </div>

        <div v-if="expandedId === record.id" class="record-details">
          <!-- 选中的选项 -->
          <div v-if="record.selected_options.length > 0" class="detail-section">
            <span class="detail-label">选中选项:</span>
            <div class="options-list">
              <span v-for="opt in record.selected_options" :key="opt" class="option-tag">
                {{ opt }}
              </span>
            </div>
          </div>
          
          <!-- 用户输入 -->
          <div v-if="record.user_response" class="detail-section">
            <span class="detail-label">用户输入:</span>
            <div class="user-input">{{ record.user_response }}</div>
          </div>

          <!-- 项目路径 -->
          <div v-if="record.project_path" class="detail-section">
            <span class="detail-label">项目:</span>
            <span class="project-path">{{ record.project_path }}</span>
          </div>
        </div>

        <div class="expand-hint">
          {{ expandedId === record.id ? '点击收起' : '点击展开详情' }}
        </div>
      </BaseCard>
    </div>

    <!-- 清空确认弹窗 -->
    <BaseModal
      :visible="showClearModal"
      title="确认清空"
      @close="showClearModal = false"
    >
      <p>确定要清空所有交互历史记录吗？此操作无法撤销。</p>
      <template #footer>
        <BaseButton variant="secondary" @click="showClearModal = false">取消</BaseButton>
        <BaseButton variant="danger" @click="handleClear">确认清空</BaseButton>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.history-tab {
  padding: 0.5rem 0;
}

.header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
  padding-bottom: 0.75rem;
  border-bottom: 2px solid #1f2937;
}

.back-btn {
  width: 2rem;
  height: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border: 2px solid #1f2937;
  color: #1f2937;
  cursor: pointer;
  transition: all 0.1s;
  flex-shrink: 0;
}

.back-btn:hover {
  background: #1f2937;
  color: white;
}

.title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 1rem;
  font-weight: 800;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  margin: 0;
  flex: 1;
}

.actions {
  display: flex;
  gap: 0.5rem;
}

.search-bar {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 0;
  color: #6b7280;
}

.records-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.record-card {
  cursor: pointer;
  transition: all 0.1s;
}

.record-card:hover {
  transform: translateX(2px);
}

.record-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.record-time {
  font-size: 0.75rem;
  color: #6b7280;
  font-weight: 600;
}

.record-badges {
  display: flex;
  gap: 0.25rem;
}

.badge {
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  border: 1px solid;
  font-weight: 700;
  text-transform: uppercase;
}

.badge-option {
  border-color: #0d9488;
  color: #0d9488;
}

.badge-input {
  border-color: #f97316;
  color: #f97316;
}

.record-message {
  font-size: 0.875rem;
  line-height: 1.5;
  color: #1f2937;
  white-space: pre-wrap;
}

.record-details {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px dashed #d1d5db;
}

.detail-section {
  margin-bottom: 0.5rem;
}

.detail-label {
  font-size: 0.75rem;
  font-weight: 700;
  color: #6b7280;
  display: block;
  margin-bottom: 0.25rem;
}

.options-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}

.option-tag {
  font-size: 0.75rem;
  padding: 0.125rem 0.5rem;
  background: #f3f4f6;
  border: 1px solid #1f2937;
}

.user-input {
  font-size: 0.875rem;
  padding: 0.5rem;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  white-space: pre-wrap;
}

.project-path {
  font-size: 0.75rem;
  color: #6b7280;
  font-family: monospace;
}

.expand-hint {
  font-size: 0.625rem;
  color: #9ca3af;
  text-align: center;
  margin-top: 0.5rem;
}
</style>
