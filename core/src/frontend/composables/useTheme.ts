import { computed, ref } from 'vue'
import { applyThemeVariables, getTheme } from '../theme'

export function useTheme() {
  // 始终使用浅色主题
  const currentTheme = ref('light')

  // 计算 Naive UI 主题 - 始终返回浅色主题
  const naiveTheme = computed(() => {
    return getTheme()
  })

  // 应用主题 - 始终应用浅色主题
  function applyTheme() {
    applyThemeVariables()
    currentTheme.value = 'light'
  }

  // 加载主题设置 - 始终应用浅色主题
  async function loadTheme() {
    applyTheme()
  }

  // 立即应用浅色主题
  applyTheme()

  return {
    currentTheme,
    naiveTheme,
    loadTheme,
  }
}
