<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, ref } from 'vue'
import { useToast } from '../../composables/useToast'
import BaseButton from '../base/Button.vue'
import BaseCollapse from '../base/Collapse.vue'
import BaseCollapseItem from '../base/CollapseItem.vue'
import CustomPromptSettings from '../settings/CustomPromptSettings.vue'
import FontSettings from '../settings/FontSettings.vue'
import ReplySettings from '../settings/ReplySettings.vue'
import ShortcutSettings from '../settings/ShortcutSettings.vue'
import ThemeSettings from '../settings/ThemeSettings.vue'
import VersionChecker from '../settings/VersionChecker.vue'
import WindowSettings from '../settings/WindowSettings.vue'

interface Props {
  currentTheme: string
  alwaysOnTop: boolean
  windowWidth: number
  windowHeight: number
  fixedWindowSize: boolean
}

defineProps<Props>()
const emit = defineEmits<Emits>()
const toast = useToast()
const isReloading = ref(false)
const configFilePath = ref('config.json')
let unlistenConfigReloaded: (() => void) | null = null

// 重新加载配置（通过重新加载设置实现）
async function reloadConfig() {
  if (isReloading.value)
    return

  isReloading.value = true
  try {
    // 触发重新加载设置的事件
    emit('configReloaded')
    toast.success('配置已重新加载')
  }
  catch (error) {
    console.error('重新加载配置失败:', error)
    toast.error('重新加载配置失败')
  }
  finally {
    isReloading.value = false
  }
}

// 获取配置文件路径
async function loadConfigFilePath() {
  try {
    const path = await invoke('get_config_file_path')
    configFilePath.value = path as string
    console.log('配置文件路径:', configFilePath.value)
  }
  catch (error) {
    console.error('获取配置文件路径失败:', error)
    configFilePath.value = 'config.json' // 使用默认值
  }
}

// 监听配置重载事件
onMounted(async () => {
  try {
    // 获取配置文件路径
    await loadConfigFilePath()

    unlistenConfigReloaded = await listen('config_reloaded', () => {
      // 配置重载后，重新加载设置而不是刷新整个页面
      console.log('收到配置重载事件，重新加载设置')
      // 触发重新加载设置的事件
      emit('configReloaded')
    })
  }
  catch (error) {
    console.error('设置配置重载监听器失败:', error)
  }
})

onUnmounted(() => {
  if (unlistenConfigReloaded) {
    unlistenConfigReloaded()
  }
})

interface Emits {
  toggleAlwaysOnTop: []
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
  configReloaded: []
  navigateTo: [tab: string]
}

// 处理窗口尺寸更新
function handleWindowSizeUpdate(size: { width: number, height: number, fixed: boolean }) {
  emit('updateWindowSize', size)
}
</script>

<template>
  <div class="max-w-3xl mx-auto tab-content">
    <!-- 返回按钮 -->
    <button class="back-btn" @click="emit('navigateTo', 'intro')">
      <div class="i-carbon-arrow-left w-3 h-3" />
      <span>返回</span>
    </button>

    <BaseCollapse :default-expanded-names="[]">
      <!-- 主题设置 -->
      <BaseCollapseItem name="theme">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900 flex items-center justify-center mr-4">
                <div class="i-carbon-color-palette text-lg text-primary-600 dark:text-primary-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  主题设置
                </div>
                <div class="text-sm opacity-60 font-normal">
                  选择您喜欢的界面主题
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <ThemeSettings :current-theme="currentTheme" />
        </div>
      </BaseCollapseItem>

      <!-- 字体设置 -->
      <BaseCollapseItem name="font">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-orange-100 dark:bg-orange-900 flex items-center justify-center mr-4">
                <div class="i-carbon-text-font text-lg text-orange-600 dark:text-orange-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  字体设置
                </div>
                <div class="text-sm opacity-60 font-normal">
                  自定义应用字体系列和大小
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <FontSettings />
        </div>
      </BaseCollapseItem>

      <!-- 继续回复设置 -->
      <BaseCollapseItem name="reply">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center mr-4">
                <div class="i-carbon-continue text-lg text-blue-600 dark:text-blue-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  继续回复设置
                </div>
                <div class="text-sm opacity-60 font-normal">
                  配置AI继续回复的行为
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <ReplySettings />
        </div>
      </BaseCollapseItem>

      <!-- 窗口设置 -->
      <BaseCollapseItem name="window">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-green-100 dark:bg-green-900 flex items-center justify-center mr-4">
                <div class="i-carbon-application text-lg text-green-600 dark:text-green-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  窗口设置
                </div>
                <div class="text-sm opacity-60 font-normal">
                  调整窗口显示和行为
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <WindowSettings
            :always-on-top="alwaysOnTop"
            :window-width="windowWidth"
            :window-height="windowHeight"
            :fixed-window-size="fixedWindowSize"
            @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
            @update-window-size="handleWindowSizeUpdate"
          />
        </div>
      </BaseCollapseItem>

      <!-- 快捷模板设置 -->
      <BaseCollapseItem name="custom-prompt">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-orange-100 dark:bg-orange-900 flex items-center justify-center mr-4">
                <div class="i-carbon-text-creation text-lg text-orange-600 dark:text-orange-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  提示词模板
                </div>
                <div class="text-sm opacity-60 font-normal">
                  管理快捷模板和上下文追加
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <CustomPromptSettings />
        </div>
      </BaseCollapseItem>

      <!-- 快捷键设置 -->
      <BaseCollapseItem name="shortcuts">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-indigo-100 dark:bg-indigo-900 flex items-center justify-center mr-4">
                <div class="i-carbon-keyboard text-lg text-indigo-600 dark:text-indigo-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  快捷键设置
                </div>
                <div class="text-sm opacity-60 font-normal">
                  自定义应用快捷键绑定
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <ShortcutSettings />
        </div>
      </BaseCollapseItem>

      <!-- 配置管理 -->
      <BaseCollapseItem name="config">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-blue-100 dark:bg-blue-900 flex items-center justify-center mr-4">
                <div class="i-carbon-settings-adjust text-lg text-blue-600 dark:text-blue-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  配置管理
                </div>
                <div class="text-sm opacity-60 font-normal">
                  重新加载配置文件和管理设置
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content space-y-6">
          <!-- 重新加载配置 -->
          <div class="flex items-center justify-between">
            <div class="flex items-center">
              <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 flex-shrink-0" />
              <div>
                <div class="text-sm font-medium leading-relaxed">
                  重新加载配置文件
                </div>
                <div class="text-xs opacity-60">
                  从 config.json 重新加载所有配置设置
                </div>
              </div>
            </div>
            <BaseButton
              size="small"
              type="primary"
              :loading="isReloading"
              @click="reloadConfig"
            >
              <span class="i-carbon-restart w-4 h-4" />
              重新加载
            </BaseButton>
          </div>

          <!-- 配置文件位置说明 -->
          <div class="flex items-start">
            <div class="w-1.5 h-1.5 bg-warning rounded-full mr-3 flex-shrink-0 mt-2" />
            <div>
              <div class="text-sm font-medium leading-relaxed mb-1">
                配置文件位置
              </div>
              <div class="text-xs opacity-60 leading-relaxed">
                配置文件路径：<br>
                <code class="bg-gray-100 px-1 rounded text-xs break-all">{{ configFilePath }}</code><br>
                您可以直接编辑该文件，然后点击"重新加载"按钮使更改生效
              </div>
            </div>
          </div>
        </div>
      </BaseCollapseItem>

      <!-- 版本检查 -->
      <BaseCollapseItem name="version">
        <template #header>
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <div class="w-10 h-10 rounded-lg bg-purple-100 dark:bg-purple-900 flex items-center justify-center mr-4">
                <div class="i-carbon-update-now text-lg text-purple-600 dark:text-purple-400" />
              </div>
              <div>
                <div class="text-lg font-medium tracking-tight mb-1">
                  版本检查
                </div>
                <div class="text-sm opacity-60 font-normal">
                  检查应用更新和版本信息
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="setting-content">
          <VersionChecker />
        </div>
      </BaseCollapseItem>
    </BaseCollapse>
  </div>
</template>

<style scoped>
.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  margin-bottom: 0.75rem;
  background: white;
  border: 2px solid #1f2937;
  font-weight: 700;
  font-size: 0.625rem;
  letter-spacing: 0.05em;
  cursor: pointer;
  transition: all 0.1s;
  font-family: ui-monospace, monospace;
}

.back-btn:hover {
  background: #1f2937;
  color: white;
}
</style>
