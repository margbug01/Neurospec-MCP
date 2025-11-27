<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useFontManager } from '../../composables/useFontManager'
import { useToast } from '../../composables/useToast'
import BaseButton from '../base/Button.vue'
import BaseInput from '../base/Input.vue'

const { showSuccess, showError } = useToast()
const {
  fontConfig,
  fontFamilyOptions,
  fontSizeOptions,
  currentFontFamily,
  currentFontScale,
  loadFontConfig,
  loadFontOptions,
  setFontFamily,
  setFontSize,
  setCustomFontFamily,
  resetFontConfig,
} = useFontManager()

// 本地状态
const customFontInput = ref('')
const fontNameInput = ref('')
const isLoading = ref(false)

// 计算当前字体系列的显示名称
const currentFontFamilyName = computed(() => {
  const option = fontFamilyOptions.value.find(opt => opt.id === fontConfig.value.font_family)
  return option?.name || '未知'
})

// 计算当前字体大小的显示名称
const currentFontSizeName = computed(() => {
  const option = fontSizeOptions.value.find(opt => opt.id === fontConfig.value.font_size)
  return option?.name || '未知'
})

// 处理字体系列变更
async function handleFontFamilyChange(e: Event) {
  const value = (e.target as HTMLSelectElement).value
  if (isLoading.value)
    return

  try {
    isLoading.value = true
    await setFontFamily(value)

    // 如果切换到非自定义字体，清空自定义字体输入
    if (value !== 'custom') {
      customFontInput.value = ''
    }
    else {
      // 如果切换到自定义字体，使用当前的自定义字体值
      customFontInput.value = fontConfig.value.custom_font_family
    }

    showSuccess('字体系列已更新')
  }
  catch (error) {
    showError('更新字体系列失败')
    console.error(error)
  }
  finally {
    isLoading.value = false
  }
}

// 处理字体大小变更
async function handleFontSizeChange(value: string) {
  if (isLoading.value)
    return

  try {
    isLoading.value = true
    await setFontSize(value)
    showSuccess('字体大小已更新')
  }
  catch (error) {
    showError('更新字体大小失败')
    console.error(error)
  }
  finally {
    isLoading.value = false
  }
}

// 处理自定义字体系列变更
async function handleCustomFontFamilyChange() {
  if (isLoading.value || !customFontInput.value.trim())
    return

  try {
    isLoading.value = true
    await setCustomFontFamily(customFontInput.value.trim())
    showSuccess('自定义字体系列已更新')
  }
  catch (error) {
    showError('更新自定义字体系列失败')
    console.error(error)
  }
  finally {
    isLoading.value = false
  }
}

// 处理字体名称输入应用
async function handleFontNameApply() {
  if (isLoading.value || !fontNameInput.value.trim())
    return

  const fontName = fontNameInput.value.trim()
  // 构建字体CSS值，包含降级字体
  const fontCssValue = `"${fontName}", -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`

  try {
    isLoading.value = true
    // 先设置为自定义字体模式
    await setFontFamily('custom')
    // 然后设置自定义字体值
    await setCustomFontFamily(fontCssValue)
    customFontInput.value = fontCssValue
    showSuccess(`字体 "${fontName}" 已应用`)
  }
  catch (error) {
    showError('应用字体失败')
    console.error(error)
  }
  finally {
    isLoading.value = false
  }
}

// 处理重置配置
async function handleResetConfig() {
  if (isLoading.value)
    return

  try {
    isLoading.value = true
    await resetFontConfig()
    customFontInput.value = fontConfig.value.custom_font_family
    showSuccess('字体配置已重置')
  }
  catch (error) {
    showError('重置字体配置失败')
    console.error(error)
  }
  finally {
    isLoading.value = false
  }
}

// 组件挂载时加载数据
onMounted(async () => {
  try {
    await Promise.all([
      loadFontConfig(),
      loadFontOptions(),
    ])
    customFontInput.value = fontConfig.value.custom_font_family
  }
  catch (error) {
    console.error('加载字体设置失败:', error)
  }
})
</script>

<template>
  <!-- 设置内容 -->
  <div class="space-y-6">
    <!-- 字体系列设置 -->
    <div>
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center">
          <div class="w-1.5 h-1.5 bg-orange-500 rounded-full mr-3 flex-shrink-0" />
          <div class="flex-1">
            <div class="text-sm font-medium leading-relaxed">
              字体系列
            </div>
            <div class="text-xs opacity-60">
              选择或自定义应用使用的字体系列
            </div>
          </div>
        </div>
        <!-- 重置按钮 -->
        <BaseButton
          size="small"
          type="secondary"
          :loading="isLoading"
          @click="handleResetConfig"
        >
          重置
        </BaseButton>
      </div>

      <!-- 字体选择器 -->
      <select
        :value="fontConfig.font_family"
        :disabled="isLoading"
        class="w-full rounded-lg border border-surface-400 bg-surface-100 text-surface-900 px-3 py-2 text-sm transition-all duration-200 outline-none focus:border-primary-500 focus:ring-2 focus:ring-primary-500/20 hover:border-surface-500 disabled:opacity-60 disabled:cursor-not-allowed"
        @change="handleFontFamilyChange"
      >
        <option
          v-for="opt in fontFamilyOptions"
          :key="opt.id"
          :value="opt.id"
        >
          {{ opt.name }}
        </option>
      </select>

      <!-- 自定义字体输入（当选择自定义时显示） -->
      <div v-if="fontConfig.font_family === 'custom'" class="mt-3 space-y-2">
        <div class="text-xs opacity-60">
          自定义字体CSS值
        </div>
        <div class="flex gap-2">
          <BaseInput
            v-model="customFontInput"
            placeholder="例如: 'MyFont', Arial, sans-serif"
            size="small"
            :disabled="isLoading"
            @keyup.enter="handleCustomFontFamilyChange"
          />
          <BaseButton
            type="primary"
            size="small"
            :loading="isLoading"
            @click="handleCustomFontFamilyChange"
          >
            应用
          </BaseButton>
        </div>
      </div>

      <div class="text-xs opacity-50 mt-2">
        当前: {{ currentFontFamilyName }}
      </div>
    </div>

    <!-- 快速应用字体 -->
    <div>
      <div class="flex items-center mb-3">
        <div class="w-1.5 h-1.5 bg-orange-500 rounded-full mr-3 flex-shrink-0" />
        <div class="flex-1">
          <div class="text-sm font-medium leading-relaxed">
            快速应用字体
          </div>
          <div class="text-xs opacity-60">
            输入系统中已安装的字体名称
          </div>
        </div>
      </div>

      <div class="flex gap-2">
        <BaseInput
          v-model="fontNameInput"
          placeholder="例如: Microsoft YaHei, 微软雅黑, PingFang SC"
          size="small"
          :disabled="isLoading"
          @keyup.enter="handleFontNameApply"
        />
        <BaseButton
          type="primary"
          size="small"
          :loading="isLoading"
          @click="handleFontNameApply"
        >
          应用
        </BaseButton>
      </div>

      <div class="text-xs opacity-50 mt-2">
        常用字体：微软雅黑、苹方、思源黑体、JetBrains Mono 等
      </div>
    </div>

    <!-- 字体大小设置 -->
    <div>
      <div class="flex items-center mb-3">
        <div class="w-1.5 h-1.5 bg-orange-500 rounded-full mr-3 flex-shrink-0" />
        <div class="flex-1">
          <div class="text-sm font-medium leading-relaxed">
            字体大小
          </div>
          <div class="text-xs opacity-60">
            调整应用界面的字体大小
          </div>
        </div>
      </div>
      <div class="flex flex-wrap gap-2">
        <BaseButton
          v-for="option in fontSizeOptions"
          :key="option.id"
          :type="fontConfig.font_size === option.id ? 'primary' : 'secondary'"
          size="small"
          :loading="isLoading"
          @click="handleFontSizeChange(option.id)"
        >
          {{ option.name }}
        </BaseButton>
      </div>
      <div class="text-xs opacity-50 mt-1">
        当前: {{ currentFontSizeName }} ({{ (currentFontScale * 100).toFixed(0) }}%)
      </div>
    </div>

    <!-- 字体预览 -->
    <div>
      <div class="flex items-center justify-between mb-3">
        <div class="flex items-center">
          <div class="w-1.5 h-1.5 bg-orange-500 rounded-full mr-3 flex-shrink-0" />
          <div>
            <div class="text-sm font-medium leading-relaxed">
              字体预览
            </div>
            <div class="text-xs opacity-60">
              实时预览当前字体效果
            </div>
          </div>
        </div>
      </div>
      <div
        class="bg-gray-100 rounded p-4 border border-gray-300 transition-all duration-200"
        :style="{
          fontFamily: currentFontFamily,
          fontSize: `${currentFontScale}rem`,
        }"
      >
        <div class="mb-3 font-medium text-lg">
          NeuroSpec - AI 对话持续工具
        </div>
        <div class="mb-3 opacity-80">
          The quick brown fox jumps over the lazy dog.
        </div>
        <div class="mb-3 opacity-80">
          微软雅黑 苹方 思源黑体 Source Han Sans
        </div>
        <div class="text-sm opacity-60 mb-2">
          ABCDEFGHIJKLMNOPQRSTUVWXYZ
        </div>
        <div class="text-sm opacity-60 mb-2">
          abcdefghijklmnopqrstuvwxyz
        </div>
        <div class="text-sm opacity-60">
          0123456789 !@#$%^&amp;*()_+-=[]{}|;:&apos;&quot;,./?
        </div>
      </div>
      <div class="text-xs opacity-50 mt-2 space-y-1">
        <div>当前字体: {{ currentFontFamily }}</div>
      </div>
    </div>
  </div>
</template>
