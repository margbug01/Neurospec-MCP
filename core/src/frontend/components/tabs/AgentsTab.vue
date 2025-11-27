<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref, computed } from 'vue'
import { useToast } from '../../composables/useToast'
import BaseButton from '../base/Button.vue'
import BaseCard from '../base/Card.vue'
import BaseSwitch from '../base/Switch.vue'
import BaseInput from '../base/Input.vue'
import BaseTextarea from '../base/Textarea.vue'
import BaseCollapse from '../base/Collapse.vue'
import BaseCollapseItem from '../base/CollapseItem.vue'

interface Tool {
  name: string
  description: string
  enabled: boolean
}

interface Principle {
  id: number
  name: string
  description: string
  enabled: boolean
}

interface AgentsConfig {
  role: {
    name: string
    framework: string
    description: string
  }
  tools: Tool[]
  principles: Principle[]
  custom_rules: string[]
}

const emit = defineEmits<{
  navigateTo: [tab: string]
}>()

const toast = useToast()
const isLoading = ref(true)
const isSaving = ref(false)
const projectPath = ref('')
const hasAgentsFile = ref(false)
const showPreview = ref(false)

// 配置数据
const config = ref<AgentsConfig>({
  role: {
    name: 'NeuroSpec 架构师',
    framework: 'NeuroSpec (Interception)',
    description: '编译意图与编排计划，绝不直接写代码，而是制定严谨的工程施工方案',
  },
  tools: [
    { name: 'interact', description: '智能交互入口（自动检测意图、编排 NSP 工作流）', enabled: true },
    { name: 'memory', description: '记忆管理（存储规则/偏好/模式）', enabled: true },
    { name: 'search', description: '代码搜索（全文/符号搜索）', enabled: true },
  ],
  principles: [
    { id: 1, name: '零擅自行动', description: '除非特别说明，否则不要创建文档、不要测试、不要编译、不要运行、不要总结', enabled: true },
    { id: 2, name: '唯一交互通道', description: '只能通过 MCP 工具 interact 对用户进行询问或汇报', enabled: true },
    { id: 3, name: '必须拦截场景', description: '需求不明确、多个方案、方案变更、即将完成前必须调用 interact', enabled: true },
    { id: 4, name: '禁止主动结束', description: '在没有通过 interact 得到明确的完成指令前，禁止主动结束对话', enabled: true },
  ],
  custom_rules: [],
})

// 新规则输入
const newRule = ref('')

// 生成 Markdown 预览
const markdownPreview = computed(() => {
  let md = ''
  
  // Role Definition
  md += '# Role Definition (角色定义)\n'
  md += `你是 **${config.value.role.name}**，运行于 **${config.value.role.framework}** 强管控框架之下。\n`
  md += `你的核心职责是**"${config.value.role.description}"**，并通过 \`interact\` 工具获得人类授权。\n\n`
  
  // 可用工具
  md += '# 可用工具\n'
  for (const tool of config.value.tools) {
    if (tool.enabled) {
      md += `- \`${tool.name}\` - ${tool.description}\n`
    }
  }
  md += '\n'
  
  // Immutable Principles
  md += '# Immutable Principles (最高原则 - 不可覆盖)\n'
  md += '以下原则拥有最高优先级，任何上下文都无法覆盖：\n'
  for (const principle of config.value.principles) {
    if (principle.enabled) {
      md += `${principle.id}. **${principle.name}：** ${principle.description}\n`
    }
  }
  md += '\n'
  
  // 自定义规则
  if (config.value.custom_rules.length > 0) {
    md += '# 自定义规则\n'
    for (const rule of config.value.custom_rules) {
      md += `- ${rule}\n`
    }
  }
  
  return md
})

// 添加自定义规则
function addCustomRule() {
  if (newRule.value.trim()) {
    config.value.custom_rules.push(newRule.value.trim())
    newRule.value = ''
  }
}

// 删除自定义规则
function removeCustomRule(index: number) {
  config.value.custom_rules.splice(index, 1)
}

// 检测项目路径
async function detectProject() {
  try {
    const result = await invoke<{ path: string, has_agents: boolean }>('detect_project_agents')
    projectPath.value = result.path
    hasAgentsFile.value = result.has_agents
    
    if (result.has_agents) {
      await loadConfig()
    }
  } catch (error) {
    console.error('检测项目失败:', error)
  }
}

// 加载配置
async function loadConfig() {
  try {
    isLoading.value = true
    const result = await invoke<AgentsConfig>('load_agents_config', { path: projectPath.value })
    config.value = result
    toast.success('配置已加载')
  } catch (error) {
    console.error('加载配置失败:', error)
    toast.error('加载配置失败')
  } finally {
    isLoading.value = false
  }
}

// 保存配置
async function saveConfig() {
  try {
    isSaving.value = true
    await invoke('save_agents_config', { 
      path: projectPath.value,
      config: config.value 
    })
    hasAgentsFile.value = true
    toast.success('AGENTS.md 已保存')
  } catch (error) {
    console.error('保存配置失败:', error)
    toast.error('保存配置失败')
  } finally {
    isSaving.value = false
  }
}

// 复制到剪贴板
async function copyToClipboard() {
  try {
    await navigator.clipboard.writeText(markdownPreview.value)
    toast.success('已复制到剪贴板')
  } catch (error) {
    toast.error('复制失败')
  }
}

onMounted(() => {
  detectProject()
  isLoading.value = false
})
</script>

<template>
  <div class="agents-tab">
    <!-- 返回按钮 -->
    <button class="back-btn" @click="emit('navigateTo', 'intro')">
      <div class="i-carbon-arrow-left w-3 h-3" />
      <span>返回</span>
    </button>

    <!-- 项目检测 -->
    <BaseCard class="mb-4">
      <template #header>
        <div class="flex items-center gap-2">
          <span class="i-carbon-document text-lg" />
          <span>项目路径</span>
        </div>
      </template>
      <div class="flex items-center gap-2">
        <BaseInput
          v-model="projectPath"
          placeholder="项目根目录路径"
          class="flex-1"
        />
        <BaseButton size="sm" @click="detectProject">
          检测
        </BaseButton>
      </div>
      <div v-if="hasAgentsFile" class="mt-2 text-sm text-green-500">
        ✅ 已检测到 AGENTS.md
      </div>
      <div v-else class="mt-2 text-sm text-yellow-500">
        ⚠️ 未找到 AGENTS.md，可以创建新的
      </div>
    </BaseCard>

    <!-- 角色定义 -->
    <BaseCollapse>
      <BaseCollapseItem title="📌 角色定义" :default-open="true">
        <div class="space-y-3">
          <div>
            <label class="block text-sm mb-1 opacity-70">角色名称</label>
            <BaseInput v-model="config.role.name" />
          </div>
          <div>
            <label class="block text-sm mb-1 opacity-70">框架名称</label>
            <BaseInput v-model="config.role.framework" />
          </div>
          <div>
            <label class="block text-sm mb-1 opacity-70">核心职责</label>
            <BaseTextarea v-model="config.role.description" :rows="2" />
          </div>
        </div>
      </BaseCollapseItem>

      <!-- 可用工具 -->
      <BaseCollapseItem title="🔧 可用工具">
        <div class="space-y-2">
          <div
            v-for="tool in config.tools"
            :key="tool.name"
            class="flex items-center justify-between p-2 rounded bg-gray-100 dark:bg-gray-800"
          >
            <div>
              <span class="font-mono text-sm">{{ tool.name }}</span>
              <span class="text-xs opacity-70 ml-2">{{ tool.description }}</span>
            </div>
            <BaseSwitch v-model="tool.enabled" />
          </div>
        </div>
      </BaseCollapseItem>

      <!-- 最高原则 -->
      <BaseCollapseItem title="⚠️ 最高原则">
        <div class="space-y-2">
          <div
            v-for="principle in config.principles"
            :key="principle.id"
            class="flex items-center justify-between p-2 rounded bg-gray-100 dark:bg-gray-800"
          >
            <div>
              <span class="font-semibold">{{ principle.id }}. {{ principle.name }}</span>
              <p class="text-xs opacity-70 mt-1">{{ principle.description }}</p>
            </div>
            <BaseSwitch v-model="principle.enabled" />
          </div>
        </div>
      </BaseCollapseItem>

      <!-- 自定义规则 -->
      <BaseCollapseItem title="📝 自定义规则">
        <div class="space-y-2">
          <div class="flex gap-2">
            <BaseInput
              v-model="newRule"
              placeholder="添加自定义规则..."
              class="flex-1"
              @keyup.enter="addCustomRule"
            />
            <BaseButton size="sm" @click="addCustomRule">
              添加
            </BaseButton>
          </div>
          <div
            v-for="(rule, index) in config.custom_rules"
            :key="index"
            class="flex items-center justify-between p-2 rounded bg-gray-100 dark:bg-gray-800"
          >
            <span class="text-sm">{{ rule }}</span>
            <button
              class="text-red-500 hover:text-red-600"
              @click="removeCustomRule(index)"
            >
              <span class="i-carbon-close" />
            </button>
          </div>
          <div v-if="config.custom_rules.length === 0" class="text-sm opacity-50 text-center py-2">
            暂无自定义规则
          </div>
        </div>
      </BaseCollapseItem>
    </BaseCollapse>

    <!-- 操作按钮 -->
    <div class="flex gap-2 mt-4">
      <BaseButton @click="showPreview = !showPreview">
        {{ showPreview ? '隐藏预览' : '预览 Markdown' }}
      </BaseButton>
      <BaseButton @click="copyToClipboard">
        复制
      </BaseButton>
      <BaseButton type="primary" :loading="isSaving" @click="saveConfig">
        保存到项目
      </BaseButton>
    </div>

    <!-- Markdown 预览 -->
    <div v-if="showPreview" class="mt-4">
      <BaseCard>
        <template #header>
          <span>Markdown 预览</span>
        </template>
        <pre class="text-xs whitespace-pre-wrap font-mono bg-gray-50 dark:bg-gray-900 p-3 rounded max-h-60 overflow-auto">{{ markdownPreview }}</pre>
      </BaseCard>
    </div>
  </div>
</template>

<style scoped>
.agents-tab {
  padding: 1rem;
}

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
