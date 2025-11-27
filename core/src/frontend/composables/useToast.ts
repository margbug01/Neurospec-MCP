import { reactive } from 'vue'

export interface Toast {
  id: string
  type: 'success' | 'error' | 'warning' | 'info' | 'loading'
  message: string
  duration: number
}

const toasts = reactive<Toast[]>([])
let toastIdCounter = 0

function addToast(type: Toast['type'], message: string, duration = 3000) {
  const id = `toast-${++toastIdCounter}`
  const toast: Toast = { id, type, message, duration }

  toasts.push(toast)

  if (duration > 0) {
    setTimeout(() => {
      removeToast(id)
    }, duration)
  }

  return {
    destroy: () => removeToast(id),
  }
}

function removeToast(id: string) {
  const index = toasts.findIndex(t => t.id === id)
  if (index !== -1) {
    toasts.splice(index, 1)
  }
}

export function useToast() {
  return {
    toasts,
    removeToast,
    success: (message: string, options?: { duration?: number }) =>
      addToast('success', message, options?.duration ?? 3000),
    error: (message: string, options?: { duration?: number }) =>
      addToast('error', message, options?.duration ?? 3000),
    warning: (message: string, options?: { duration?: number }) =>
      addToast('warning', message, options?.duration ?? 3000),
    info: (message: string, options?: { duration?: number }) =>
      addToast('info', message, options?.duration ?? 3000),
    loading: (message: string, options?: { duration?: number }) =>
      addToast('loading', message, options?.duration ?? 0),
  }
}
