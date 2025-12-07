import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref } from 'vue'
import type { MemorySuggestion } from './useMemory'

/**
 * MCP处理组合式函数
 */
// MCP 请求类型（包含 daemon 模式的 id 字段）
interface McpRequestData {
  id?: string
  message?: string
  predefined_options?: string[]
  is_markdown?: boolean
  [key: string]: unknown
}

export function useMcpHandler() {
  const mcpRequest = ref<McpRequestData | null>(null)
  const showMcpPopup = ref(false)
  
  // 记忆建议相关状态
  const memorySuggestions = ref<MemorySuggestion[]>([])
  const showMemorySuggestionModal = ref(false)
  const conversationMessages = ref<string[]>([])

  /**
   * 统一的MCP响应处理
   */
  async function handleMcpResponse(response: any) {
    try {
      // 通过Tauri命令发送响应并退出应用
      await invoke('send_mcp_response', { response })
      await invoke('exit_app')
    }
    catch (error) {
      console.error('MCP响应处理失败:', error)
    }
  }

  /**
   * 统一的MCP取消处理
   */
  async function handleMcpCancel() {
    try {
      // 发送取消信息并退出应用
      await invoke('send_mcp_response', { response: 'CANCELLED' })
      await invoke('exit_app')
    }
    catch (error) {
      // 静默处理MCP取消错误
      console.error('MCP取消处理失败:', error)
    }
  }

  /**
   * 显示MCP弹窗
   */
  async function showMcpDialog(request: any) {
    // 设置请求数据和显示状态
    mcpRequest.value = request
    showMcpPopup.value = true
  }

  /**
   * 检查MCP模式
   */
  async function checkMcpMode() {
    try {
      const args = await invoke('get_cli_args')

      if (args && (args as any).mcp_request) {
        // 读取MCP请求文件
        const content = await invoke('read_mcp_request', { filePath: (args as any).mcp_request })

        if (content) {
          await showMcpDialog(content)
        }
        return { isMcp: true, mcpContent: content }
      }
    }
    catch (error) {
      console.error('检查MCP模式失败:', error)
    }
    return { isMcp: false, mcpContent: null }
  }

  /**
   * 设置MCP事件监听器
   */
  async function setupMcpEventListener() {
    try {
      await listen('mcp-request', (event) => {
        showMcpDialog(event.payload)
      })
    }
    catch (error) {
      console.error('设置MCP事件监听器失败:', error)
    }
  }

  /**
   * 设置 Daemon 模式的 MCP popup 监听器
   * 用于新的 HTTP daemon 架构
   */
  async function setupDaemonPopupListener() {
    try {
      await listen('mcp-popup-request', async (event) => {
        const request = event.payload as any
        console.log('[Daemon MCP] Received popup request:', request)

        // 显示弹窗
        mcpRequest.value = request
        showMcpPopup.value = true

        // 强制显示并激活窗口
        await invoke('show_window')

        // 注意：响应通过 handleDaemonPopupResponse 发送
      })
      console.log('[Daemon MCP] Popup listener initialized')
    }
    catch (error) {
      console.error('[Daemon MCP] Failed to setup popup listener:', error)
    }
  }

  /**
   * 处理 Daemon 模式的 popup 响应
   * 发送响应到 daemon server
   */
  async function handleDaemonPopupResponse(requestId: string, response: string) {
    try {
      await invoke('handle_mcp_popup_response', {
        requestId,
        response,
      })
      console.log('[Daemon MCP] Response sent successfully')

      // 关闭弹窗
      showMcpPopup.value = false
      mcpRequest.value = null
    }
    catch (error) {
      console.error('[Daemon MCP] Failed to send response:', error)
      throw error
    }
  }

  /**
   * 关闭 Daemon 模式的弹窗
   * 用于响应已通过 invoke 发送后的清理
   */
  function closeDaemonPopup() {
    console.log('[Daemon MCP] closeDaemonPopup called - closing popup without exit')
    showMcpPopup.value = false
    mcpRequest.value = null
    console.log('[Daemon MCP] Popup closed successfully')
  }

  /**
   * 收集对话消息用于记忆分析
   */
  function collectMessage(message: string) {
    conversationMessages.value.push(message)
    // 保留最近 20 条消息
    if (conversationMessages.value.length > 20) {
      conversationMessages.value.shift()
    }
  }

  /**
   * 分析对话并检测记忆建议
   * 返回高置信度建议数量
   */
  async function analyzeForMemorySuggestions(): Promise<number> {
    if (conversationMessages.value.length === 0) {
      return 0
    }

    try {
      const suggestions = await invoke<MemorySuggestion[]>('analyze_memory_suggestions', {
        messages: conversationMessages.value,
        projectPath: null,
      })

      // 过滤高置信度建议 (>= 0.8)
      const highConfidence = (suggestions || []).filter(s => s.confidence >= 0.8)
      
      if (highConfidence.length > 0) {
        memorySuggestions.value = highConfidence
        return highConfidence.length
      }
      
      return 0
    }
    catch (error) {
      console.error('分析记忆建议失败:', error)
      return 0
    }
  }

  /**
   * 显示记忆建议弹窗
   */
  function showMemorySuggestions() {
    if (memorySuggestions.value.length > 0) {
      showMemorySuggestionModal.value = true
    }
  }

  /**
   * 关闭记忆建议弹窗
   */
  function hideMemorySuggestions() {
    showMemorySuggestionModal.value = false
  }

  /**
   * 清空记忆建议
   */
  function clearMemorySuggestions() {
    memorySuggestions.value = []
    conversationMessages.value = []
  }

  return {
    mcpRequest,
    showMcpPopup,
    handleMcpResponse,
    handleMcpCancel,
    showMcpDialog,
    checkMcpMode,
    setupMcpEventListener,
    setupDaemonPopupListener,
    handleDaemonPopupResponse,
    closeDaemonPopup,
    // 记忆建议相关
    memorySuggestions,
    showMemorySuggestionModal,
    collectMessage,
    analyzeForMemorySuggestions,
    showMemorySuggestions,
    hideMemorySuggestions,
    clearMemorySuggestions,
  }
}
