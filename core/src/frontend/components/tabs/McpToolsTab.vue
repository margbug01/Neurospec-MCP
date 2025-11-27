<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useToast } from '../../composables/useToast'
import BaseCard from '../base/Card.vue'
import BaseButton from '../base/Button.vue'
import BaseModal from '../base/Modal.vue'
import BaseInput from '../base/Input.vue'
import BaseTextarea from '../base/Textarea.vue'
import BaseSpinner from '../base/Spinner.vue'

const emit = defineEmits<{
  navigateTo: [tab: string]
}>()

const toast = useToast()
const isLoading = ref(true)

// 核心工具列表
const coreTools = ref([
  {
    id: 'interact',
    name: 'interact',
    description: '智能交互入口，自动检测意图',
    icon: 'i-carbon-chat',
    iconColor: 'text-blue-500',
    iconBg: 'bg-blue-500/10',
    status: 'active',
    stats: { calls: 0, lastCall: null as Date | null },
  },
  {
    id: 'memory',
    name: 'memory',
    description: '记忆管理，存储规则/偏好/模式',
    icon: 'i-carbon-data-base',
    iconColor: 'text-purple-500',
    iconBg: 'bg-purple-500/10',
    status: 'active',
    stats: { items: 0, rules: 0, preferences: 0 },
  },
  {
    id: 'search',
    name: 'search',
    description: '代码搜索，全文/符号搜索',
    icon: 'i-carbon-search',
    iconColor: 'text-green-500',
    iconBg: 'bg-green-500/10',
    status: 'active',
    stats: { indexedFiles: 0, indexReady: false },
  },
])

// 搜索调试状态
const showDebugModal = ref(false)
const debugProjectRoot = ref('')
const debugQuery = ref('')
const debugMode = ref<'text' | 'symbol'>('text')
const debugResult = ref('')
const debugLoading = ref(false)

// 加载工具状态
async function loadToolStats() {
  try {
    // 获取项目路径
    const projectResult = await invoke<{ path: string }>('detect_project_agents')
    debugProjectRoot.value = projectResult.path

    // 获取记忆统计
    try {
      const memories = await invoke<any[]>('memory_list', {
        projectPath: projectResult.path,
        page: 1,
        pageSize: 100
      })
      const memoryTool = coreTools.value.find(t => t.id === 'memory')
      if (memoryTool) {
        const items = memories || []
        memoryTool.stats.items = items.length
        memoryTool.stats.rules = items.filter((m: any) => m.category === 'rule').length
        memoryTool.stats.preferences = items.filter((m: any) => m.category === 'preference').length
      }
    } catch {
      // 忽略记忆加载错误
    }

    // 模拟索引状态
    const searchTool = coreTools.value.find(t => t.id === 'search')
    if (searchTool) {
      searchTool.stats.indexedFiles = 1234
      searchTool.stats.indexReady = true
    }

  } catch (error) {
    console.error('加载工具状态失败:', error)
  } finally {
    isLoading.value = false
  }
}

// 运行搜索调试
async function runSearchDebug() {
  if (!debugProjectRoot.value || !debugQuery.value) {
    toast.warning('请填写项目路径和查询语句')
    return
  }

  debugLoading.value = true
  debugResult.value = ''

  try {
    const result = await invoke('debug_acemcp_search', {
      projectRootPath: debugProjectRoot.value,
      query: debugQuery.value,
    }) as { success: boolean, result?: string, error?: string }

    if (result.success && result.result) {
      debugResult.value = result.result
      toast.success('搜索完成')
    } else {
      debugResult.value = result.error || '搜索无结果'
      toast.warning('搜索无结果')
    }
  } catch (e: any) {
    debugResult.value = `错误: ${e?.message || e}`
    toast.error('搜索失败')
  } finally {
    debugLoading.value = false
  }
}

// 清除索引缓存
async function clearIndexCache() {
  try {
    const result = await invoke('clear_acemcp_cache') as string
    toast.success(result)
  } catch (err) {
    toast.error(`清除缓存失败: ${err}`)
  }
}

// 格式化最后调用时间
function formatLastCall(date: Date | null): string {
  if (!date) return '从未'
  const diff = Date.now() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  return `${Math.floor(minutes / 60)}小时前`
}

onMounted(() => {
  loadToolStats()
})
</script>

<template>
  <div class="tools-page">
    <!-- 返回按钮 -->
    <button class="back-btn" @click="emit('navigateTo', 'intro')">
      <div class="i-carbon-arrow-left w-3 h-3" />
      <span>返回</span>
    </button>

    <!-- 标题 -->
    <div class="header-retro mb-6">
      <div class="text-xl font-mono font-bold tracking-wider">
        MCP TOOLS
      </div>
      <div class="text-sm font-mono opacity-60 mt-1">
        // Core tools for AI-powered development
      </div>
    </div>

    <!-- 加载状态 -->
    <div v-if="isLoading" class="text-center py-12">
      <BaseSpinner size="medium" />
      <div class="mt-3 text-sm font-mono opacity-60">LOADING...</div>
    </div>

    <!-- 工具列表 -->
    <div v-else class="space-y-4">
      <BaseCard
        v-for="tool in coreTools"
        :key="tool.id"
        class="tool-card"
        padding="none"
      >
        <div class="p-4">
          <!-- 工具头部 -->
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <!-- 图标 -->
              <div
                class="w-10 h-10 rounded-lg flex items-center justify-center"
                :class="tool.iconBg"
              >
                <div :class="[tool.icon, tool.iconColor]" class="text-xl" />
              </div>
              <!-- 名称 -->
              <div>
                <div class="font-mono font-semibold text-lg">
                  {{ tool.name }}
                </div>
                <div class="text-sm opacity-60">
                  {{ tool.description }}
                </div>
              </div>
            </div>
            <!-- 状态 -->
            <div class="flex items-center gap-2">
              <div class="status-dot bg-green-500" />
              <span class="text-xs font-mono uppercase text-green-600">ACTIVE</span>
            </div>
          </div>

          <!-- 工具统计 -->
          <div class="tool-stats">
            <!-- interact 统计 -->
            <template v-if="tool.id === 'interact'">
              <div class="stat-item">
                <span class="stat-label">CALLS:</span>
                <span class="stat-value">{{ tool.stats.calls }}</span>
              </div>
              <div class="stat-item">
                <span class="stat-label">LAST:</span>
                <span class="stat-value">{{ formatLastCall(tool.stats.lastCall) }}</span>
              </div>
            </template>

            <!-- memory 统计 -->
            <template v-else-if="tool.id === 'memory'">
              <div class="stat-item">
                <span class="stat-label">ITEMS:</span>
                <span class="stat-value">{{ tool.stats.items }}</span>
              </div>
              <div class="stat-item">
                <span class="stat-label">RULES:</span>
                <span class="stat-value">{{ tool.stats.rules }}</span>
              </div>
              <div class="stat-item">
                <span class="stat-label">PREFS:</span>
                <span class="stat-value">{{ tool.stats.preferences }}</span>
              </div>
            </template>

            <!-- search 统计 -->
            <template v-else-if="tool.id === 'search'">
              <div class="stat-item">
                <span class="stat-label">INDEX:</span>
                <span class="stat-value" :class="tool.stats.indexReady ? 'text-green-500' : 'text-yellow-500'">
                  {{ tool.stats.indexReady ? 'READY' : 'BUILDING' }}
                </span>
              </div>
              <div class="stat-item">
                <span class="stat-label">FILES:</span>
                <span class="stat-value">{{ tool.stats.indexedFiles }}</span>
              </div>
              <div class="stat-item">
                <BaseButton size="small" variant="ghost" @click="showDebugModal = true">
                  <div class="i-carbon-debug w-4 h-4 mr-1" />
                  DEBUG
                </BaseButton>
              </div>
            </template>
          </div>
        </div>
      </BaseCard>
    </div>

    <!-- 底部装饰 -->
    <div class="mt-6 text-center">
      <div class="font-mono text-xs opacity-40">
        ─────────────────────────────────────
      </div>
      <div class="font-mono text-xs opacity-50 mt-2">
        3 / 3 TOOLS ACTIVE
      </div>
    </div>

    <!-- 搜索调试弹窗 -->
    <BaseModal
      v-model:show="showDebugModal"
      title="Search Debug Console"
      :closable="true"
    >
      <div class="space-y-4">
        <!-- 项目路径 -->
        <div>
          <label class="block text-sm font-mono mb-2 opacity-70">PROJECT_ROOT:</label>
          <BaseInput
            v-model="debugProjectRoot"
            placeholder="/path/to/project"
            :clearable="true"
          />
        </div>

        <!-- 查询语句 -->
        <div>
          <label class="block text-sm font-mono mb-2 opacity-70">QUERY:</label>
          <BaseTextarea
            v-model="debugQuery"
            placeholder="输入搜索查询..."
            :rows="2"
          />
        </div>

        <!-- 搜索模式 -->
        <div class="flex gap-4">
          <label class="flex items-center gap-2 cursor-pointer">
            <input v-model="debugMode" type="radio" value="text" class="accent-blue-500">
            <span class="text-sm font-mono">TEXT</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input v-model="debugMode" type="radio" value="symbol" class="accent-blue-500">
            <span class="text-sm font-mono">SYMBOL</span>
          </label>
        </div>

        <!-- 操作按钮 -->
        <div class="flex gap-2">
          <BaseButton
            variant="primary"
            :loading="debugLoading"
            @click="runSearchDebug"
          >
            <div class="i-carbon-play w-4 h-4 mr-1" />
            RUN
          </BaseButton>
          <BaseButton @click="clearIndexCache">
            <div class="i-carbon-trash-can w-4 h-4 mr-1" />
            CLEAR CACHE
          </BaseButton>
        </div>

        <!-- 结果显示 -->
        <div v-if="debugResult">
          <label class="block text-sm font-mono mb-2 opacity-70">RESULT:</label>
          <BaseTextarea
            v-model="debugResult"
            :rows="8"
            readonly
            class="font-mono text-xs bg-gray-900 text-green-400"
          />
        </div>
      </div>
    </BaseModal>
  </div>
</template>

<style scoped>
.tools-page {
  max-width: 700px;
  margin: 0 auto;
}

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

.header-retro {
  border-bottom: 2px solid currentColor;
  padding-bottom: 0.75rem;
  opacity: 0.9;
}

.tool-card {
  transition: transform 0.2s, box-shadow 0.2s;
}

.tool-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.tool-stats {
  display: flex;
  gap: 1rem;
  padding: 0.75rem;
  background: rgba(128, 128, 128, 0.05);
  border-radius: 0.5rem;
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.stat-label {
  opacity: 0.6;
}

.stat-value {
  font-weight: 600;
}

/* 深色模式 */
:deep(.dark) .tool-stats {
  background: rgba(255, 255, 255, 0.05);
}
</style>
