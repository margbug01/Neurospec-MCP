<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
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

const records = ref<InteractRecord[]>([])
const loading = ref(false)
const expandedId = ref<string | null>(null)

async function loadHistory() {
  loading.value = true
  try {
    const result = await invoke<InteractRecord[]>('get_interact_history_cmd', { count: 10 })
    records.value = result
  } catch (e) {
    console.error('加载历史失败:', e)
  } finally {
    loading.value = false
  }
}

function formatTime(timestamp: string): string {
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

function truncateMessage(msg: string, maxLen = 80): string {
  if (msg.length <= maxLen) return msg
  return msg.substring(0, maxLen) + '...'
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

onMounted(() => {
  loadHistory()
})
</script>

<template>
  <div class="mini-history">
    <div class="panel-header">
      <div class="panel-title">
        <div class="i-carbon-time w-4 h-4" />
        <span>最近交互</span>
      </div>
      <button class="refresh-btn" title="刷新" @click="loadHistory">
        <div class="i-carbon-renew w-3.5 h-3.5" :class="{ 'animate-spin': loading }" />
      </button>
    </div>

    <div class="panel-content custom-scrollbar">
      <!-- 加载状态 -->
      <div v-if="loading && records.length === 0" class="loading-state">
        <BaseSpinner size="small" />
        <span>加载中...</span>
      </div>

      <!-- 空状态 -->
      <div v-else-if="records.length === 0" class="empty-state">
        <div class="i-carbon-document-blank w-8 h-8 opacity-40" />
        <span>暂无记录</span>
      </div>

      <!-- 记录列表 -->
      <div v-else class="records-list">
        <div
          v-for="record in records"
          :key="record.id"
          class="record-item"
          :class="{ expanded: expandedId === record.id }"
          @click="toggleExpand(record.id)"
        >
          <div class="record-header">
            <span class="record-time">{{ formatTime(record.timestamp) }}</span>
            <div class="record-badges">
              <span v-if="record.selected_options.length > 0" class="badge">
                {{ record.selected_options.length }}选
              </span>
            </div>
          </div>
          <div class="record-message">
            {{ expandedId === record.id ? record.request_message : truncateMessage(record.request_message) }}
          </div>
          <div v-if="expandedId === record.id && record.user_response" class="record-response">
            <span class="response-label">回复:</span>
            {{ record.user_response }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mini-history {
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

.records-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.record-item {
  padding: 0.625rem;
  background: white;
  border: 2px solid #e5e7eb;
  cursor: pointer;
  transition: all 0.1s;
}

.record-item:hover {
  border-color: #1f2937;
}

.record-item.expanded {
  border-color: #1f2937;
  box-shadow: 2px 2px 0px 0px rgba(31, 41, 55, 0.2);
}

.record-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.375rem;
}

.record-time {
  font-size: 0.625rem;
  color: #6b7280;
  font-weight: 600;
}

.record-badges {
  display: flex;
  gap: 0.25rem;
}

.badge {
  font-size: 0.5rem;
  padding: 0.0625rem 0.25rem;
  border: 1px solid #0d9488;
  color: #0d9488;
  font-weight: 700;
}

.record-message {
  font-size: 0.75rem;
  line-height: 1.4;
  color: #1f2937;
  white-space: pre-wrap;
  word-break: break-word;
}

.record-response {
  margin-top: 0.5rem;
  padding-top: 0.5rem;
  border-top: 1px dashed #d1d5db;
  font-size: 0.75rem;
  color: #6b7280;
}

.response-label {
  font-weight: 700;
  color: #f97316;
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
