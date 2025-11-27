<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useToast } from '../../composables/useToast'
import { useVersionCheck } from '../../composables/useVersionCheck'
import BaseButton from '../base/Button.vue'
import BaseCard from '../base/Card.vue'

const { showSuccess, showError, showWarning, showInfo } = useToast()
const { versionInfo, manualCheckUpdate, safeOpenUrl, lastCheckTime, isChecking, getVersionInfo } = useVersionCheck()

// 格式化最后检查时间
const formattedLastCheckTime = computed(() => {
  return lastCheckTime.value ? lastCheckTime.value.toLocaleString('zh-CN') : ''
})

// 安全打开GitHub链接
async function openGitHub() {
  try {
    await safeOpenUrl('https://github.com/neurospec/neurospec')
    showSuccess('正在打开GitHub页面...')
  }
  catch (error) {
    const errorMsg = error instanceof Error ? error.message : '打开GitHub失败，请手动访问'
    if (errorMsg.includes('已复制到剪贴板')) {
      showWarning(errorMsg)
    }
    else {
      showError(errorMsg)
    }
  }
}

// 安全打开GitHub Star页面
async function openGitHubStars() {
  try {
    await safeOpenUrl('https://github.com/neurospec/neurospec/stargazers')
    showSuccess('正在打开Star页面...')
  }
  catch (error) {
    const errorMsg = error instanceof Error ? error.message : '打开Star页面失败，请手动访问'
    if (errorMsg.includes('已复制到剪贴板')) {
      showWarning(errorMsg)
    }
    else {
      showError(errorMsg)
    }
  }
}

// 检查版本更新
async function checkVersion() {
  try {
    const info = await manualCheckUpdate()
    if (info?.hasUpdate) {
      showInfo(`发现新版本 v${info.latest}！`)
    }
    else {
      showSuccess('当前已是最新版本')
    }
  }
  catch (error) {
    console.error('检查版本失败:', error)
    showError('检查版本失败，请稍后重试')
  }
}

// 组件挂载时初始化版本信息
onMounted(async () => {
  try {
    await getVersionInfo()
  }
  catch (error) {
    console.error('初始化版本信息失败:', error)
  }
})
</script>

<template>
  <BaseCard
    padding="small"
    :hoverable="true"
  >
    <!-- 主要内容区域 -->
    <div class="flex items-center justify-between mb-2">
      <!-- 左侧：项目信息 -->
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center">
          <div class="i-carbon-logo-github text-blue-600 dark:text-blue-400" />
        </div>
        <div>
          <h3 class="font-semibold text-gray-900 dark:text-white text-sm">
            NeuroSpec {{ versionInfo ? `v${versionInfo.current}` : 'v0.2.0' }}
          </h3>
          <p class="text-xs text-gray-500 dark:text-gray-400">
            AI-powered development assistant with MCP integration
          </p>
        </div>
      </div>

      <!-- 右侧：版本检查区域 -->
      <div class="flex flex-col items-end gap-1">
        <BaseButton
          size="medium"
          type="secondary"
          :loading="isChecking"
          @click="checkVersion"
        >
          <div class="i-carbon-renew text-green-600 dark:text-green-400" />
          检查更新
        </BaseButton>

        <!-- 最后检查时间 -->
        <div
          v-if="formattedLastCheckTime"
          class="text-xs text-gray-400 dark:text-gray-500"
        >
          最后检查: {{ formattedLastCheckTime }}
        </div>
      </div>
    </div>

    <!-- 底部：GitHub区域 -->
    <div class="flex items-center justify-between border-t border-gray-100 dark:border-gray-700 pt-2">
      <div class="flex items-center gap-1">
        <BaseButton
          size="medium"
          type="primary"
          @click="openGitHub"
        >
          <div class="i-carbon-logo-github" />
          GitHub
        </BaseButton>

        <BaseButton
          size="medium"
          type="secondary"
          @click="openGitHubStars"
        >
          <div class="i-carbon-star text-yellow-500" />
          Star
        </BaseButton>
      </div>

      <!-- 弱化的提示文字 -->
      <p class="text-xs text-gray-400 dark:text-gray-500">
        如果对您有帮助，请给我们一个 Star ⭐
      </p>
    </div>
  </BaseCard>
</template>
