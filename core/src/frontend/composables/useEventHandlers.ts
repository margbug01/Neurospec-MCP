/**
 * 事件处理器封装
 * 将复杂的事件传递简化为可复用的处理器
 */
export function useEventHandlers(actions: any) {
  return {
    // MCP 事件
    onMcpResponse: actions.mcp.handleResponse,
    onMcpCancel: actions.mcp.handleCancel,

    // 设置事件
    onToggleAlwaysOnTop: actions.settings.toggleAlwaysOnTop,
    onUpdateWindowSize: actions.settings.updateWindowSize,
    onUpdateReplyConfig: actions.settings.updateReplyConfig,
    onMessageReady: actions.settings.setMessageInstance,

    // 配置事件
    onConfigReloaded: actions.settings.reloadAllSettings,
  }
}
