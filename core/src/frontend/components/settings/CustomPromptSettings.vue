<script setup lang="ts">
import type { CustomPrompt, CustomPromptConfig } from '../../types/popup'
import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { onMounted, ref } from 'vue'
import { useToast } from '../../composables/useToast'
import BaseButton from '../base/Button.vue'
import BaseInput from '../base/Input.vue'
import BaseModal from '../base/Modal.vue'
import BaseRadio from '../base/Radio.vue'
import BaseSpinner from '../base/Spinner.vue'
import BaseSwitch from '../base/Switch.vue'
import BaseTag from '../base/Tag.vue'
import BaseTextarea from '../base/Textarea.vue'

const { showSuccess, showError, showWarning } = useToast()

// 配置状态
const config = ref<CustomPromptConfig>({
  prompts: [],
  enabled: true,
  maxPrompts: 50,
})

// UI状态
const loading = ref(false)
const showAddDialog = ref(false)
const showEditDialog = ref(false)
const showDeleteDialog = ref(false)
const editingPrompt = ref<CustomPrompt | null>(null)
const deletingPromptId = ref<string>('')

// 新prompt表单
const newPrompt = ref({
  name: '',
  content: '',
  description: '',
  type: 'normal' as 'normal' | 'conditional',
  condition_text: '',
  template_true: '',
  template_false: '',
  current_state: false,
})

// 加载配置
async function loadConfig() {
  try {
    loading.value = true
    const result = await invoke('get_custom_prompt_config')
    config.value = result as CustomPromptConfig
    // 按sort_order排序
    config.value.prompts.sort((a, b) => a.sort_order - b.sort_order)
  }
  catch (error) {
    console.error('加载自定义prompt配置失败:', error)
    showError('加载配置失败')
  }
  finally {
    loading.value = false
  }
}

// 切换启用状态
async function toggleEnabled() {
  try {
    await invoke('set_custom_prompt_enabled', { enabled: config.value.enabled })

    // 发送事件通知其他组件更新
    await emit('custom-prompt-updated')

    showSuccess(config.value.enabled ? '已启用快捷模板功能' : '已禁用快捷模板功能')
  }
  catch (error) {
    console.error('更新启用状态失败:', error)
    showError('更新失败')
    // 回滚状态
    config.value.enabled = !config.value.enabled
  }
}

// 添加prompt
async function addPrompt() {
  if (!newPrompt.value.name.trim()) {
    showWarning('请填写名称')
    return
  }

  // 上下文追加的验证
  if (newPrompt.value.type === 'conditional') {
    if (!newPrompt.value.condition_text.trim()) {
      showWarning('请填写条件描述')
      return
    }
    if (!newPrompt.value.template_true.trim() && !newPrompt.value.template_false.trim()) {
      showWarning('请至少填写一个模板内容')
      return
    }
  }

  try {
    const prompt: CustomPrompt = {
      id: `custom_${Date.now()}`,
      name: newPrompt.value.name.trim(),
      content: newPrompt.value.content, // 允许空内容
      description: newPrompt.value.description.trim() || undefined,
      sort_order: config.value.prompts.length + 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      type: newPrompt.value.type,
      condition_text: newPrompt.value.type === 'conditional' ? newPrompt.value.condition_text.trim() || undefined : undefined,
      template_true: newPrompt.value.type === 'conditional' ? newPrompt.value.template_true.trim() || undefined : undefined,
      template_false: newPrompt.value.type === 'conditional' ? newPrompt.value.template_false.trim() || undefined : undefined,
      current_state: newPrompt.value.type === 'conditional' ? newPrompt.value.current_state : undefined,
    }

    await invoke('add_custom_prompt', { prompt })
    config.value.prompts.push(prompt)

    // 发送事件通知其他组件更新
    await emit('custom-prompt-updated')

    // 重置表单
    newPrompt.value = {
      name: '',
      content: '',
      description: '',
      type: 'normal',
      condition_text: '',
      template_true: '',
      template_false: '',
      current_state: false,
    }
    showAddDialog.value = false
    showSuccess('添加成功')
  }
  catch (error) {
    console.error('添加prompt失败:', error)
    showError(`添加失败: ${error}`)
  }
}

// 编辑prompt
function editPrompt(prompt: CustomPrompt) {
  editingPrompt.value = { ...prompt }
  showEditDialog.value = true
}

// 更新prompt
async function updatePrompt() {
  if (!editingPrompt.value)
    return

  // 上下文追加的验证
  if (editingPrompt.value.type === 'conditional') {
    if (!editingPrompt.value.condition_text?.trim()) {
      showWarning('请填写条件描述')
      return
    }
    if (!editingPrompt.value.template_true?.trim() && !editingPrompt.value.template_false?.trim()) {
      showWarning('请至少填写一个模板内容')
      return
    }
  }

  try {
    editingPrompt.value.updated_at = new Date().toISOString()
    await invoke('update_custom_prompt', { prompt: editingPrompt.value })

    // 更新本地状态
    const index = config.value.prompts.findIndex(p => p.id === editingPrompt.value!.id)
    if (index !== -1) {
      config.value.prompts[index] = { ...editingPrompt.value }
    }

    // 发送事件通知其他组件更新
    await emit('custom-prompt-updated')

    showEditDialog.value = false
    editingPrompt.value = null
    showSuccess('更新成功')
  }
  catch (error) {
    console.error('更新prompt失败:', error)
    showError(`更新失败: ${error}`)
  }
}

// 显示删除确认对话框
function showDeleteConfirm(promptId: string) {
  deletingPromptId.value = promptId
  showDeleteDialog.value = true
}

// 删除prompt
async function deletePrompt() {
  if (!deletingPromptId.value)
    return

  try {
    await invoke('delete_custom_prompt', { promptId: deletingPromptId.value })
    config.value.prompts = config.value.prompts.filter(p => p.id !== deletingPromptId.value)

    // 发送事件通知其他组件更新
    await emit('custom-prompt-updated')

    showSuccess('删除成功')
  }
  catch (error) {
    console.error('删除prompt失败:', error)
    showError(`删除失败: ${error}`)
  }
  finally {
    showDeleteDialog.value = false
    deletingPromptId.value = ''
  }
}

// 取消编辑
function cancelEdit() {
  showEditDialog.value = false
  editingPrompt.value = null
}

// 组件挂载时加载配置
onMounted(() => {
  loadConfig()
})
</script>

<template>
  <div class="p-4">
    <!-- 启用开关 -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <div class="text-sm opacity-60">
          是否开启快捷模板功能
        </div>
      </div>
      <BaseSwitch
        v-model="config.enabled"
        @update:model-value="toggleEnabled"
      />
    </div>

    <div v-if="config.enabled" data-guide="custom-prompt-settings">
      <!-- 添加按钮 -->
      <div class="flex justify-between items-center mb-4">
        <div class="text-sm opacity-60">
          已创建 {{ config.prompts.length }} 个模板
        </div>
        <BaseButton
          variant="primary"
          size="small"
          :disabled="config.prompts.length >= config.maxPrompts"
          data-guide="add-prompt-button"
          @click="showAddDialog = true"
        >
          <div class="i-carbon-add w-4 h-4 mr-2" />
          添加模板
        </BaseButton>
      </div>

      <!-- Prompt列表 -->
      <div v-if="loading" class="text-center py-8">
        <BaseSpinner size="medium" />
      </div>

      <div v-else-if="config.prompts.length === 0" class="text-center py-8 opacity-60">
        <div class="i-carbon-document-blank text-4xl mb-2" />
        <div>暂无快捷模板</div>
      </div>

      <div v-else class="space-y-3">
        <div class="space-y-3">
          <div
            v-for="prompt in config.prompts"
            :key="prompt.id"
            class="bg-white rounded-lg p-4 border border-gray-200 shadow-sm hover:border-gray-300 transition-colors"
          >
            <div class="flex justify-between items-start mb-2">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <span class="font-medium text-primary">{{ prompt.name }}</span>
                  <!-- 类型标识 -->
                  <BaseTag v-if="prompt.type === 'conditional'" size="small" variant="info">
                    上下文追加
                  </BaseTag>
                  <BaseTag v-else size="small" variant="default">
                    快捷模板
                  </BaseTag>
                </div>
                <div v-if="prompt.description" class="text-sm opacity-60 mb-2">
                  {{ prompt.description }}
                </div>

                <!-- 快捷模板内容显示 -->
                <div v-if="prompt.type !== 'conditional'" class="text-sm bg-gray-50 p-2 rounded border border-gray-200">
                  <span v-if="prompt.content.trim()">{{ prompt.content }}</span>
                  <span v-else class="italic opacity-60">（空内容 - 清空输入框）</span>
                </div>

                <!-- 上下文追加内容显示 -->
                <div v-else class="space-y-2">
                  <div class="text-sm bg-gray-50 p-2 rounded border border-gray-200">
                    <div class="font-medium mb-1">
                      条件：{{ prompt.condition_text }}
                    </div>
                    <div class="space-y-1 text-xs">
                      <div v-if="prompt.template_true">
                        <span class="text-green-400">✓ 开启：</span>{{ prompt.template_true }}
                      </div>
                      <div v-if="prompt.template_false">
                        <span class="text-red-400">✗ 关闭：</span>{{ prompt.template_false }}
                      </div>
                      <div class="text-gray-700 dark:text-gray-600">
                        当前状态：{{ prompt.current_state ? '开启' : '关闭' }}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <div class="flex gap-1 ml-4">
                <BaseButton variant="ghost" size="small" @click="editPrompt(prompt)">
                  <div class="i-carbon-edit w-4 h-4" />
                </BaseButton>
                <BaseButton variant="ghost-danger" size="small" @click="showDeleteConfirm(prompt.id)">
                  <div class="i-carbon-trash-can w-4 h-4" />
                </BaseButton>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 添加对话框 -->
    <BaseModal v-model="showAddDialog" title="添加快捷模板" max-width="600px">
      <div class="space-y-4">
        <div>
          <label class="block text-sm font-medium mb-2">名称 <span class="text-error">*</span></label>
          <BaseInput v-model="newPrompt.name" placeholder="输入模板名称" />
        </div>
        <div>
          <label class="block text-sm font-medium mb-2">描述</label>
          <BaseInput v-model="newPrompt.description" placeholder="简短描述这个模板的用途" />
        </div>

        <!-- 模板类型选择 -->
        <div>
          <label class="block text-sm font-medium mb-2">类型</label>
          <div class="flex gap-4">
            <BaseRadio v-model="newPrompt.type" value="normal" name="add-type">
              快捷模板
            </BaseRadio>
            <BaseRadio v-model="newPrompt.type" value="conditional" name="add-type">
              上下文追加
            </BaseRadio>
          </div>
        </div>

        <!-- 快捷模板内容 -->
        <div v-if="newPrompt.type === 'normal'">
          <label class="block text-sm font-medium mb-2">内容</label>
          <BaseTextarea
            v-model="newPrompt.content"
            placeholder="输入模板内容（留空可实现清空输入框效果）"
            :autosize="{ minRows: 4, maxRows: 8 }"
          />
        </div>

        <!-- 上下文追加字段 -->
        <template v-if="newPrompt.type === 'conditional'">
          <div>
            <label class="block text-sm font-medium mb-2">条件描述 <span class="text-error">*</span></label>
            <BaseInput v-model="newPrompt.condition_text" placeholder="例如：是否使用TypeScript" />
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">开启时的内容</label>
            <BaseTextarea
              v-model="newPrompt.template_true"
              placeholder="例如：✔️需要使用TypeScript"
              :autosize="{ minRows: 2, maxRows: 4 }"
            />
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">关闭时的内容</label>
            <BaseTextarea
              v-model="newPrompt.template_false"
              placeholder="例如：❌切记，不要使用TypeScript"
              :autosize="{ minRows: 2, maxRows: 4 }"
            />
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">当前状态</label>
            <div class="flex items-center gap-2">
              <BaseSwitch v-model="newPrompt.current_state" />
              <span class="text-sm">{{ newPrompt.current_state ? '开启' : '关闭' }}</span>
            </div>
          </div>
        </template>
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <BaseButton @click="showAddDialog = false">
            取消
          </BaseButton>
          <BaseButton variant="primary" @click="addPrompt">
            添加
          </BaseButton>
        </div>
      </template>
    </BaseModal>

    <!-- 编辑对话框 -->
    <BaseModal v-model="showEditDialog" title="编辑快捷模板" max-width="600px">
      <div v-if="editingPrompt" class="space-y-4">
        <div>
          <label class="block text-sm font-medium mb-2">名称 <span class="text-error">*</span></label>
          <BaseInput v-model="editingPrompt.name" placeholder="输入模板名称" />
        </div>
        <div>
          <label class="block text-sm font-medium mb-2">描述</label>
          <BaseInput v-model="editingPrompt.description" placeholder="简短描述这个模板的用途" />
        </div>

        <!-- 模板类型选择 -->
        <div>
          <label class="block text-sm font-medium mb-2">类型</label>
          <div class="flex gap-4">
            <BaseRadio v-model="editingPrompt.type" value="normal" name="edit-type">
              快捷模板
            </BaseRadio>
            <BaseRadio v-model="editingPrompt.type" value="conditional" name="edit-type">
              上下文追加
            </BaseRadio>
          </div>
        </div>

        <!-- 快捷模板内容 -->
        <div v-if="editingPrompt.type === 'normal' || !editingPrompt.type">
          <label class="block text-sm font-medium mb-2">内容</label>
          <BaseTextarea
            v-model="editingPrompt.content"
            placeholder="输入模板内容（留空可实现清空输入框效果）"
            :autosize="{ minRows: 4, maxRows: 8 }"
          />
        </div>

        <!-- 上下文追加字段 -->
        <template v-if="editingPrompt.type === 'conditional'">
          <div>
            <label class="block text-sm font-medium mb-2">条件描述 <span class="text-error">*</span></label>
            <BaseInput v-model="editingPrompt.condition_text" placeholder="例如：是否使用TypeScript" />
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">开启时的内容</label>
            <BaseTextarea
              v-model="editingPrompt.template_true"
              placeholder="例如：✔️需要使用TypeScript"
              :autosize="{ minRows: 2, maxRows: 4 }"
            />
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">关闭时的内容</label>
            <BaseTextarea
              v-model="editingPrompt.template_false"
              placeholder="例如：❌切记，不要使用TypeScript"
              :autosize="{ minRows: 2, maxRows: 4 }"
            />
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">当前状态</label>
            <div class="flex items-center gap-2">
              <BaseSwitch v-model="editingPrompt.current_state" />
              <span class="text-sm">{{ editingPrompt.current_state ? '开启' : '关闭' }}</span>
            </div>
          </div>
        </template>
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <BaseButton @click="cancelEdit">
            取消
          </BaseButton>
          <BaseButton variant="primary" @click="updatePrompt">
            保存
          </BaseButton>
        </div>
      </template>
    </BaseModal>

    <!-- 删除确认对话框 -->
    <BaseModal v-model="showDeleteDialog" title="确认删除" max-width="400px">
      <div>确定要删除这个模板吗？此操作无法撤销。</div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <BaseButton @click="showDeleteDialog = false">
            取消
          </BaseButton>
          <BaseButton variant="danger" @click="deletePrompt">
            确定删除
          </BaseButton>
        </div>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
/* 拖拽排序样式 */
.sortable-item {
  cursor: default;
  transition: all 0.2s ease;
}

.sortable-ghost {
  opacity: 0.5;
  transform: scale(0.95);
  background: rgba(59, 130, 246, 0.1) !important;
  border: 2px dashed rgba(59, 130, 246, 0.3) !important;
}

.sortable-chosen {
  cursor: grabbing !important;
  transform: scale(1.02);
}

.sortable-drag {
  opacity: 0.8;
  transform: rotate(2deg);
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.15);
  z-index: 1000;
}
</style>
