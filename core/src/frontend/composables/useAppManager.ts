import { computed } from 'vue'
import { useAppInitialization } from './useAppInitialization'
import { useMcpHandler } from './useMcpHandler'
import { useSettings } from './useSettings'
import { useTheme } from './useTheme'

/**
 * 统一的应用管理器
 * 封装所有组合式函数，提供简洁的API
 */
export function useAppManager() {
  // 初始化各个模块
  const theme = useTheme()
  const settings = useSettings()
  const mcpHandler = useMcpHandler()
  const appInit = useAppInitialization(mcpHandler)

  // 创建统一的配置对象
  const appConfig = computed(() => {
    const config = {
      theme: theme.currentTheme.value,
      window: {
        alwaysOnTop: settings.alwaysOnTop.value,
        width: settings.windowWidth.value,
        height: settings.windowHeight.value,
        fixed: settings.fixedWindowSize.value,
      },
      reply: {
        enabled: settings.continueReplyEnabled.value,
        prompt: settings.continuePrompt.value,
      },
    }

    return config
  })

  // 包装 MCP 响应处理，添加记忆建议分析
  async function handleMcpResponseWithMemoryAnalysis(response: any) {
    // 收集用户输入到对话历史
    if (response?.user_input) {
      mcpHandler.collectMessage(response.user_input)
    }
    if (response?.selected_options?.length > 0) {
      mcpHandler.collectMessage(response.selected_options.join(', '))
    }

    // 分析记忆建议
    const suggestionCount = await mcpHandler.analyzeForMemorySuggestions()
    if (suggestionCount > 0) {
      // 通过 settings 的 message 实例显示提示
      const msg = settings.getMessageInstance()
      if (msg) {
        msg.info(`检测到 ${suggestionCount} 条可记忆内容，点击记忆管理查看`, { duration: 5000 })
      }
    }

    // 根据模式分流：daemon 模式直接回传 daemon，避免退出应用
    const requestId = mcpHandler.mcpRequest.value?.id
    if (requestId) {
      try {
        return await mcpHandler.handleDaemonPopupResponse(requestId, JSON.stringify(response))
      }
      catch (error) {
        console.error('[Daemon MCP] Response failed:', error)
        // 失败时也要清理弹窗状态，避免卡住
        mcpHandler.showMcpPopup.value = false
        mcpHandler.mcpRequest.value = null
      }
      return
    }

    // 旧模式：发送响应并退出应用
    return mcpHandler.handleMcpResponse(response)
  }

  // 包装 MCP 取消处理，区分 daemon 模式
  async function handleMcpCancelWithModeCheck() {
    const requestId = mcpHandler.mcpRequest.value?.id
    if (requestId) {
      // daemon 模式：发送取消响应，不退出应用
      const cancelResponse = {
        user_input: '用户取消了操作',
        selected_options: [],
        images: [],
        metadata: {
          timestamp: new Date().toISOString(),
          request_id: requestId,
          source: 'popup_cancel',
        },
      }
      try {
        return await mcpHandler.handleDaemonPopupResponse(requestId, JSON.stringify(cancelResponse))
      }
      catch (error) {
        console.error('[Daemon MCP] Cancel failed:', error)
        // 失败时也要清理弹窗状态
        mcpHandler.showMcpPopup.value = false
        mcpHandler.mcpRequest.value = null
      }
      return
    }

    // 旧模式：发送取消并退出应用
    return mcpHandler.handleMcpCancel()
  }

  // 创建统一的操作对象
  const actions = {
    // 设置操作
    settings: {
      toggleAlwaysOnTop: settings.toggleAlwaysOnTop,
      updateWindowSize: settings.updateWindowSize,
      updateReplyConfig: settings.updateReplyConfig,
      setMessageInstance: settings.setMessageInstance,
      reloadAllSettings: settings.reloadAllSettings,
    },
    // MCP操作
    mcp: {
      handleResponse: handleMcpResponseWithMemoryAnalysis,
      handleCancel: handleMcpCancelWithModeCheck,
      handleDaemonResponse: mcpHandler.handleDaemonPopupResponse,
    },
    // 应用操作
    app: {
      initialize: appInit.initializeApp,
      cleanup: () => {
        // 清理窗口焦点监听器
        settings.removeWindowFocusListener()
      },
    },
  }

  // 返回状态和操作 - 保持响应式
  return {
    // 直接解构状态，Vue模板会自动处理响应式
    naiveTheme: theme.naiveTheme,
    mcpRequest: mcpHandler.mcpRequest,
    showMcpPopup: mcpHandler.showMcpPopup,
    appConfig,
    isInitializing: appInit.isInitializing,

    // 操作
    actions,
  }
}
