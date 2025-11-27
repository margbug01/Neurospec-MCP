<script setup lang="ts">
import { ref } from 'vue'
import FeatureCard from '../../components/common/FeatureCard.vue'
import ReplySettings from '../../components/settings/ReplySettings.vue'
import ThemeSettings from '../../components/settings/ThemeSettings.vue'
import WindowSettings from '../../components/settings/WindowSettings.vue'

// 模拟状态
const currentTheme = ref('dark')
const alwaysOnTop = ref(false)
const audioNotificationEnabled = ref(true)
const audioUrl = ref('')

// 测试用的组件状态
const inputValue = ref('')
const switchValue = ref(false)
const checkboxValue = ref(false)
const selectedTags = ref<string[]>(['标签1'])
const buttonLoading = ref(false)

// 事件处理
function handleThemeChange(theme: string) {
  currentTheme.value = theme
  console.log('主题切换:', theme)
}

function handleToggleAlwaysOnTop() {
  alwaysOnTop.value = !alwaysOnTop.value
  console.log('置顶切换:', alwaysOnTop.value)
}

function handleToggleAudioNotification() {
  audioNotificationEnabled.value = !audioNotificationEnabled.value
  console.log('音频通知切换:', audioNotificationEnabled.value)
}

function handleUpdateAudioUrl(url: string) {
  audioUrl.value = url
  console.log('音频URL更新:', url)
}

function handleTestAudio() {
  console.log('测试音频播放')
}

function handleButtonClick() {
  buttonLoading.value = true
  setTimeout(() => {
    buttonLoading.value = false
  }, 2000)
}

// 功能卡片数据
const features = [
  {
    icon: 'i-carbon-brain',
    title: '智能交互',
    description: 'MCP 标准兼容的智能助手',
    bgClass: 'bg-blue-100 dark:bg-blue-900',
  },
  {
    icon: 'i-carbon-color-palette',
    title: '主题切换',
    description: '支持浅色/深色主题',
    bgClass: 'bg-purple-100 dark:bg-purple-900',
  },
  {
    icon: 'i-carbon-settings',
    title: '设置管理',
    description: '完整的设置管理功能',
    bgClass: 'bg-green-100 dark:bg-green-900',
  },
]
</script>

<template>
  <div class="components-test">
    <n-card title="组件库测试 - 真实组件">
      <template #header-extra>
        <n-tag size="small" type="info">
          引用真实组件
        </n-tag>
      </template>

      <n-space vertical size="large">
        <!-- 设置组件测试 -->
        <div class="component-section">
          <h3 class="section-title">
            设置组件
          </h3>

          <n-space vertical size="large">
            <!-- 主题设置 -->
            <div class="component-demo">
              <h4>ThemeSettings 组件</h4>
              <div class="demo-container">
                <ThemeSettings
                  :current-theme="currentTheme"
                  @theme-change="handleThemeChange"
                />
              </div>
              <div class="demo-info">
                <n-tag size="small">
                  src/frontend/components/settings/ThemeSettings.vue
                </n-tag>
              </div>
            </div>

            <!-- 窗口设置 -->
            <div class="component-demo">
              <h4>WindowSettings 组件</h4>
              <div class="demo-container">
                <WindowSettings
                  :always-on-top="alwaysOnTop"
                  :audio-notification-enabled="audioNotificationEnabled"
                  :audio-url="audioUrl"
                  @toggle-always-on-top="handleToggleAlwaysOnTop"
                  @toggle-audio-notification="handleToggleAudioNotification"
                  @update-audio-url="handleUpdateAudioUrl"
                  @test-audio="handleTestAudio"
                />
              </div>
              <div class="demo-info">
                <n-tag size="small">
                  src/frontend/components/settings/WindowSettings.vue
                </n-tag>
              </div>
            </div>

            <!-- 回复设置 -->
            <div class="component-demo">
              <h4>ReplySettings 组件</h4>
              <div class="demo-container">
                <ReplySettings />
              </div>
              <div class="demo-info">
                <n-tag size="small">
                  src/frontend/components/settings/ReplySettings.vue
                </n-tag>
              </div>
            </div>
          </n-space>
        </div>

        <!-- 通用组件测试 -->
        <div class="component-section">
          <h3 class="section-title">
            通用组件
          </h3>

          <div class="component-demo">
            <h4>FeatureCard 组件</h4>
            <div class="demo-container">
              <div class="feature-cards-grid">
                <FeatureCard
                  v-for="feature in features"
                  :key="feature.title"
                  :icon="feature.icon"
                  :title="feature.title"
                  :description="feature.description"
                  :bg-class="feature.bgClass"
                />
              </div>
            </div>
            <div class="demo-info">
              <n-tag size="small">
                src/frontend/components/common/FeatureCard.vue
              </n-tag>
            </div>
          </div>
        </div>

        <!-- 基础组件测试 -->
        <div class="component-section">
          <h3 class="section-title">
            基础 Naive UI 组件
          </h3>

          <div class="component-demo">
            <h4>统一 size="small" 测试</h4>
            <div class="demo-container">
              <n-space vertical>
                <div class="form-row">
                  <label>输入框:</label>
                  <n-input v-model:value="inputValue" size="small" placeholder="测试输入" />
                </div>

                <div class="form-row">
                  <label>开关:</label>
                  <n-switch v-model:value="switchValue" size="small" />
                </div>

                <div class="form-row">
                  <label>复选框:</label>
                  <n-checkbox v-model:checked="checkboxValue" size="small">
                    测试选项
                  </n-checkbox>
                </div>

                <div class="form-row">
                  <label>按钮:</label>
                  <n-space>
                    <n-button size="small">
                      默认
                    </n-button>
                    <n-button type="primary" size="small" :loading="buttonLoading" @click="handleButtonClick">
                      主要按钮
                    </n-button>
                  </n-space>
                </div>

                <div class="form-row">
                  <label>标签:</label>
                  <n-space>
                    <n-tag
                      v-for="tag in selectedTags"
                      :key="tag"
                      size="small"
                      closable
                      @close="selectedTags.splice(selectedTags.indexOf(tag), 1)"
                    >
                      {{ tag }}
                    </n-tag>
                    <n-button size="small" @click="selectedTags.push(`标签${selectedTags.length + 1}`)">
                      添加
                    </n-button>
                  </n-space>
                </div>
              </n-space>
            </div>
          </div>
        </div>

        <!-- 状态监控 -->
        <div class="component-section">
          <h3 class="section-title">
            状态监控
          </h3>

          <n-card size="small">
            <n-space vertical>
              <div><strong>当前主题:</strong> {{ currentTheme }}</div>
              <div><strong>置顶状态:</strong> {{ alwaysOnTop ? '启用' : '禁用' }}</div>
              <div><strong>音频通知:</strong> {{ audioNotificationEnabled ? '启用' : '禁用' }}</div>
              <div><strong>音频URL:</strong> {{ audioUrl || '(默认)' }}</div>
              <div><strong>输入值:</strong> {{ inputValue || '(空)' }}</div>
              <div><strong>开关状态:</strong> {{ switchValue ? '开启' : '关闭' }}</div>
              <div><strong>复选框:</strong> {{ checkboxValue ? '选中' : '未选中' }}</div>
              <div><strong>标签列表:</strong> {{ selectedTags.join(', ') || '(无)' }}</div>
            </n-space>
          </n-card>
        </div>
      </n-space>
    </n-card>
  </div>
</template>

<style scoped>
.components-test {
  max-width: 1000px;
  margin: 0 auto;
}

.component-section {
  margin-bottom: 30px;
}

.section-title {
  margin: 0 0 20px 0;
  color: var(--text-color);
  font-size: 1.2rem;
  font-weight: 600;
  border-bottom: 2px solid var(--primary-color);
  padding-bottom: 8px;
  display: inline-block;
}

.component-demo {
  margin-bottom: 25px;
}

.component-demo h4 {
  margin: 0 0 10px 0;
  color: var(--text-color);
  font-size: 1rem;
  font-weight: 500;
}

.demo-container {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 20px;
  background: var(--card-color);
  margin-bottom: 10px;
}

.demo-info {
  text-align: right;
}

.feature-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
}

.form-row {
  display: flex;
  align-items: center;
  gap: 15px;
}

.form-row label {
  min-width: 80px;
  font-weight: 500;
  color: var(--text-color);
}
</style>
