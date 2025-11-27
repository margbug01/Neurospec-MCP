<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useToast } from '../../composables/useToast'
import { useVersionCheck } from '../../composables/useVersionCheck'
import BaseButton from '../base/Button.vue'
import BaseSpinner from '../base/Spinner.vue'
import BaseTag from '../base/Tag.vue'

const loading = ref(false)
const { showSuccess, showError, showInfo, showWarning } = useToast()

const {
  versionInfo,
  isChecking,
  lastCheckTime,
  isUpdating,
  updateProgress,
  updateStatus,
  manualCheckUpdate,
  getVersionInfo,
  openDownloadPage,
  openReleasePage,
  performOneClickUpdate,
  restartApp,
} = useVersionCheck()

// 格式化最后检查时间
const formattedLastCheckTime = computed(() => {
  return lastCheckTime.value ? lastCheckTime.value.toLocaleString('zh-CN') : ''
})

// 手动检查更新
async function handleCheckUpdate() {
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
    console.error('检查版本更新失败:', error)
    showError(`检查版本更新失败: ${error}`)
  }
}

// 安全下载更新
async function handleDownloadUpdate() {
  try {
    await openDownloadPage()
    showSuccess('正在打开下载页面...')
  }
  catch (error) {
    const errorMsg = error instanceof Error ? error.message : '打开下载页面失败，请手动访问GitHub'
    if (errorMsg.includes('已复制到剪贴板')) {
      showWarning(errorMsg)
    }
    else {
      showError(errorMsg)
    }
  }
}

// 查看更新日志
async function handleViewReleaseNotes() {
  try {
    await openReleasePage()
    showSuccess('正在打开更新日志...')
  }
  catch (error) {
    const errorMsg = error instanceof Error ? error.message : '打开更新日志失败，请手动访问GitHub'
    if (errorMsg.includes('已复制到剪贴板')) {
      showWarning(errorMsg)
    }
    else {
      showError(errorMsg)
    }
  }
}

// 一键更新
async function handleOneClickUpdate() {
  try {
    showInfo('开始下载更新...')
    await performOneClickUpdate()

    if (updateStatus.value === 'completed') {
      showSuccess('更新完成！点击重启按钮应用更新')
    }
  }
  catch (error) {
    console.error('一键更新失败:', error)
    const errorMsg = error instanceof Error ? error.message : '更新失败，请稍后重试或手动下载'
    showError(errorMsg)
  }
}

// 重启应用
async function handleRestartApp() {
  try {
    await restartApp()
  }
  catch (error) {
    console.error('重启失败:', error)
    showError('重启失败，请手动重启应用')
  }
}

// 组件挂载时初始化版本信息
onMounted(async () => {
  loading.value = true
  try {
    await getVersionInfo()
  }
  catch (error) {
    console.error('初始化版本信息失败:', error)
  }
  finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="space-y-4">
    <!-- 版本信息显示 -->
    <div
      v-if="!loading && versionInfo"
      class="space-y-3"
    >
      <div class="flex items-center justify-between">
        <span class="text-sm text-on-surface-secondary">当前版本:</span>
        <BaseTag
          size="small"
          variant="info"
        >
          v{{ versionInfo.current }}
        </BaseTag>
      </div>

      <div
        v-if="versionInfo.latest !== versionInfo.current"
        class="flex items-center justify-between"
      >
        <span class="text-sm text-on-surface-secondary">最新版本:</span>
        <BaseTag
          size="small"
          :variant="versionInfo.hasUpdate ? 'warning' : 'success'"
        >
          v{{ versionInfo.latest }}
        </BaseTag>
      </div>

      <!-- 更新提示 -->
      <div
        v-if="versionInfo.hasUpdate"
        class="p-3 bg-warning/10 dark:bg-warning/20 rounded-lg border border-warning/20 dark:border-warning/30"
      >
        <div class="flex items-start gap-2">
          <div class="i-carbon-warning text-warning mt-0.5" />
          <div class="flex-1">
            <p class="text-sm font-medium text-on-surface dark:text-on-surface">
              发现新版本 v{{ versionInfo.latest }}
            </p>
            <p class="text-xs text-on-surface-secondary dark:text-on-surface-secondary mt-1">
              建议更新到最新版本以获得更好的体验
            </p>
          </div>
        </div>
      </div>

      <!-- 更新进度显示 -->
      <div
        v-if="isUpdating"
        class="p-3 bg-surface-100 dark:bg-surface-800 rounded-lg border border-surface-200 dark:border-surface-700"
      >
        <div class="space-y-2">
          <div class="flex items-center gap-2">
            <BaseSpinner size="small" />
            <span class="text-sm font-medium text-on-surface dark:text-on-surface">
              {{ updateStatus === 'checking' ? '检查更新中...'
                : updateStatus === 'downloading' ? '下载更新中...'
                  : updateStatus === 'installing' ? '安装更新中...'
                    : updateStatus === 'completed' ? '更新完成' : '更新中...' }}
            </span>
          </div>

          <!-- 下载进度条 -->
          <div
            v-if="updateProgress && updateStatus === 'downloading'"
            class="space-y-1"
          >
            <!-- CSS Progress Bar -->
            <div class="w-full bg-surface-200 dark:bg-surface-700 rounded-full h-1.5 overflow-hidden">
              <div
                class="bg-primary-500 h-full transition-all duration-300 rounded-full"
                :style="{ width: `${Math.round(updateProgress.percentage)}%` }"
              />
            </div>
            <div class="flex justify-between text-xs text-on-surface-secondary dark:text-on-surface-secondary">
              <span>{{ Math.round(updateProgress.downloaded / 1024 / 1024 * 100) / 100 }}MB</span>
              <span v-if="updateProgress.content_length">
                / {{ Math.round(updateProgress.content_length / 1024 / 1024 * 100) / 100 }}MB
              </span>
              <span>{{ Math.round(updateProgress.percentage) }}%</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 最后检查时间 -->
      <div
        v-if="formattedLastCheckTime"
        class="text-xs text-on-surface-muted dark:text-on-surface-muted"
      >
        最后检查: {{ formattedLastCheckTime }}
      </div>
    </div>

    <!-- 加载状态 -->
    <div
      v-else-if="loading"
      class="flex items-center justify-center py-4"
    >
      <BaseSpinner size="small" />
      <span class="ml-2 text-sm text-on-surface-secondary">加载版本信息...</span>
    </div>

    <!-- 操作按钮 -->
    <div class="flex items-center gap-2 pt-2 border-t border-surface-200 dark:border-surface-700 flex-wrap">
      <BaseButton
        size="small"
        :loading="isChecking"
        :disabled="isUpdating"
        @click="handleCheckUpdate"
      >
        <div class="i-carbon-renew w-4 h-4 mr-1" />
        检查更新
      </BaseButton>

      <!-- 立即更新按钮 -->
      <BaseButton
        v-if="versionInfo?.hasUpdate && updateStatus !== 'completed'"
        variant="primary"
        size="small"
        :loading="isUpdating"
        @click="handleOneClickUpdate"
      >
        <div class="i-carbon-upgrade w-4 h-4 mr-1" />
        立即更新
      </BaseButton>

      <!-- 重启按钮 -->
      <BaseButton
        v-if="updateStatus === 'completed'"
        variant="success"
        size="small"
        @click="handleRestartApp"
      >
        <div class="i-carbon-restart w-4 h-4 mr-1" />
        重启应用
      </BaseButton>

      <!-- 手动下载按钮（备选方案） -->
      <BaseButton
        v-if="versionInfo?.hasUpdate"
        variant="secondary"
        size="small"
        :disabled="isUpdating"
        @click="handleDownloadUpdate"
      >
        <div class="i-carbon-download w-4 h-4 mr-1" />
        手动下载
      </BaseButton>

      <BaseButton
        v-if="versionInfo?.releaseUrl"
        variant="secondary"
        size="small"
        :disabled="isUpdating"
        @click="handleViewReleaseNotes"
      >
        <div class="i-carbon-document w-4 h-4 mr-1" />
        更新日志
      </BaseButton>
    </div>
  </div>
</template>
