/**
 * 系统状态监控 Composable
 * 
 * 提供统一的系统状态管理，包括：
 * - MCP 连接状态
 * - 索引状态
 * - 记忆统计
 * - 项目信息
 * - 运行时间
 */

import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, reactive, ref, toRefs } from 'vue'

// 系统状态接口
export interface SystemStatus {
  // MCP 状态
  mcpConnected: boolean
  
  // 索引状态
  indexReady: boolean
  indexFileCount: number
  indexBuilding: boolean
  lastIndexTime: Date | null
  
  // 记忆状态
  memoryCount: number
  memoryRules: number
  memoryPreferences: number
  memoryPatterns: number
  
  // 项目信息
  projectPath: string
  hasAgents: boolean
  
  // 系统信息
  version: string
  uptime: number
}

// 工具状态接口
export interface ToolStatus {
  id: string
  name: string
  active: boolean
  calls: number
  lastCall: Date | null
}

// 默认状态
const defaultStatus: SystemStatus = {
  mcpConnected: true,
  indexReady: false,
  indexFileCount: 0,
  indexBuilding: false,
  lastIndexTime: null,
  memoryCount: 0,
  memoryRules: 0,
  memoryPreferences: 0,
  memoryPatterns: 0,
  projectPath: '',
  hasAgents: false,
  version: '0.4.0',
  uptime: 0,
}

// 全局状态（单例）
const globalStatus = reactive<SystemStatus>({ ...defaultStatus })
const isLoading = ref(true)
const error = ref<string | null>(null)
let uptimeInterval: number | null = null
let refreshInterval: number | null = null

/**
 * 系统状态监控 Composable
 */
export function useSystemStatus() {
  // 格式化运行时间
  const formattedUptime = computed(() => {
    const seconds = globalStatus.uptime
    const h = Math.floor(seconds / 3600)
    const m = Math.floor((seconds % 3600) / 60)
    const s = seconds % 60
    return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  })

  // 格式化最后索引时间
  const formattedLastIndex = computed(() => {
    if (!globalStatus.lastIndexTime) return '从未'
    const diff = Date.now() - globalStatus.lastIndexTime.getTime()
    const minutes = Math.floor(diff / 60000)
    if (minutes < 1) return '刚刚'
    if (minutes < 60) return `${minutes} 分钟前`
    const hours = Math.floor(minutes / 60)
    if (hours < 24) return `${hours} 小时前`
    return `${Math.floor(hours / 24)} 天前`
  })

  // 索引状态文本
  const indexStatusText = computed(() => {
    if (globalStatus.indexBuilding) return 'BUILDING'
    if (globalStatus.indexReady) return 'READY'
    return 'OFFLINE'
  })

  // 加载项目信息
  async function loadProjectInfo() {
    try {
      const result = await invoke<{ path: string; has_agents: boolean }>('detect_project_agents')
      globalStatus.projectPath = result.path
      globalStatus.hasAgents = result.has_agents
    } catch (e) {
      console.warn('Failed to detect project:', e)
    }
  }

  // 加载记忆统计
  async function loadMemoryStats() {
    if (!globalStatus.projectPath) return
    
    try {
      const memories = await invoke<any[]>('memory_list', {
        projectPath: globalStatus.projectPath,
        page: 1,
        pageSize: 1000,
      })
      
      const items = memories || []
      globalStatus.memoryCount = items.length
      globalStatus.memoryRules = items.filter((m: any) => m.category === 'rule').length
      globalStatus.memoryPreferences = items.filter((m: any) => m.category === 'preference').length
      globalStatus.memoryPatterns = items.filter((m: any) => m.category === 'pattern').length
    } catch {
      // 忽略错误
    }
  }

  // 加载索引状态
  async function loadIndexStatus() {
    try {
      const result = await invoke<{
        ready: boolean
        file_count: number
        building: boolean
        project_path: string | null
      }>('get_index_status')
      
      globalStatus.indexReady = result.ready
      globalStatus.indexFileCount = result.file_count
      globalStatus.indexBuilding = result.building
      
      if (result.ready) {
        globalStatus.lastIndexTime = new Date()
      }
    } catch (e) {
      console.warn('Failed to load index status:', e)
      globalStatus.indexReady = false
      globalStatus.indexFileCount = 0
    }
  }

  // 刷新所有状态
  async function refresh() {
    isLoading.value = true
    error.value = null
    
    try {
      await loadProjectInfo()
      await Promise.all([
        loadMemoryStats(),
        loadIndexStatus(),
      ])
    } catch (e) {
      error.value = String(e)
      console.error('Failed to load system status:', e)
    } finally {
      isLoading.value = false
    }
  }

  // 启动定时器
  function startTimers() {
    // 运行时间计时器
    if (!uptimeInterval) {
      uptimeInterval = window.setInterval(() => {
        globalStatus.uptime++
      }, 1000)
    }

    // 状态刷新定时器（每 30 秒）
    if (!refreshInterval) {
      refreshInterval = window.setInterval(() => {
        loadMemoryStats()
        loadIndexStatus()
      }, 30000)
    }
  }

  // 停止定时器
  function stopTimers() {
    if (uptimeInterval) {
      clearInterval(uptimeInterval)
      uptimeInterval = null
    }
    if (refreshInterval) {
      clearInterval(refreshInterval)
      refreshInterval = null
    }
  }

  // 生命周期
  onMounted(() => {
    refresh()
    startTimers()
  })

  onUnmounted(() => {
    // 不停止定时器，保持全局状态
  })

  return {
    // 状态
    status: toRefs(globalStatus),
    isLoading,
    error,
    
    // 计算属性
    formattedUptime,
    formattedLastIndex,
    indexStatusText,
    
    // 方法
    refresh,
    loadProjectInfo,
    loadMemoryStats,
    loadIndexStatus,
  }
}

/**
 * 格式化时间差
 */
export function formatTimeDiff(date: Date | null): string {
  if (!date) return '从未'
  const diff = Date.now() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}小时前`
  return `${Math.floor(hours / 24)}天前`
}
