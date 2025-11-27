import { listen } from '@tauri-apps/api/event'

/**
 * Toast API interface compatible with useToast
 */
interface ToastApi {
  warning: (message: string, options?: { duration?: number, closable?: boolean }) => void
}

/**
 * 退出警告监听器管理
 */
class ExitWarningManager {
  private unlistenExitWarning: (() => void) | null = null

  /**
   * 设置退出警告监听器
   */
  async setupListener(toastInstance: ToastApi): Promise<void> {
    // 先移除已存在的监听器
    this.removeListener()

    if (typeof window !== 'undefined') {
      try {
        this.unlistenExitWarning = await listen('exit-warning', (event: any) => {
          const message = event.payload as string

          // 显示退出警告消息
          if (toastInstance) {
            toastInstance.warning(message, {
              duration: 3000,
              closable: true,
            })
          }
          else {
            console.warn('Toast实例未准备好')
          }
        })
      }
      catch (error) {
        console.error('设置退出警告监听器失败:', error)
      }
    }
  }

  /**
   * 移除退出警告监听器
   */
  removeListener(): void {
    if (this.unlistenExitWarning) {
      this.unlistenExitWarning()
      this.unlistenExitWarning = null
    }
  }
}

// 全局单例实例
const exitWarningManager = new ExitWarningManager()

/**
 * 设置退出警告监听器（独立函数，不依赖Vue）
 */
export async function setupExitWarningListener(toastInstance: ToastApi): Promise<void> {
  await exitWarningManager.setupListener(toastInstance)
}

/**
 * 移除退出警告监听器（独立函数，不依赖Vue）
 */
export function removeExitWarningListener(): void {
  exitWarningManager.removeListener()
}

/**
 * 退出警告处理组合式函数（保留向后兼容性）
 * @deprecated 推荐直接使用 setupExitWarningListener 和 removeExitWarningListener
 */
export function useExitWarning() {
  return {
    setupExitWarningListener,
    removeExitWarningListener,
  }
}
