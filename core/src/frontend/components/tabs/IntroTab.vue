<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { useSystemStatus } from '../../composables/useSystemStatus'
import { useToast } from '../../composables/useToast'

const toast = useToast()
const {
  status,
  formattedUptime,
} = useSystemStatus()

// 测试弹窗
async function showTestPopup() {
  try {
    const testRequest = {
      id: `test-${Date.now()}`,
      message: `# 🧪 测试弹窗

这是一个**测试弹窗**，用于检查代码块渲染效果。

## 代码示例

\`\`\`typescript
interface SearchResult {
  path: string;
  score: number;
  snippet: string;
  line_number: number;
}

async function search(query: string): Promise<SearchResult[]> {
  const results = await invoke('search', { query });
  return results.filter(r => r.score > 0.5);
}
\`\`\`

\`\`\`rust
pub fn main() {
    println!("Hello, NeuroSpec!");
    let config = Config::load().unwrap();
    println!("Config: {:?}", config);
}
\`\`\`

## 功能测试

- 支持 Markdown 渲染
- 支持**代码高亮**
- 支持选项选择
- 支持图片上传`,
      predefined_options: ['✅ 样式正常', '⚠️ 需要调整', '❌ 有问题'],
      is_markdown: true,
    }
    await invoke('create_test_popup', { request: testRequest })
    toast.success('测试弹窗已创建')
  } catch (error) {
    toast.error(`创建弹窗失败: ${error}`)
  }
}

// 快速入口按钮
const actionButtons = [
  { key: 'agents', label: 'AGENTS.md', icon: 'i-carbon-document', bg: 'bg-blue-100' },
  { key: 'mcp-tools', label: '工具调试', icon: 'i-carbon-tools', bg: 'bg-green-100' },
  { key: 'memory', label: '记忆管理', icon: 'i-carbon-data-base', bg: 'bg-purple-100' },
  { key: 'settings', label: '设置', icon: 'i-carbon-settings', bg: 'bg-gray-100' },
]

// MCP 状态卡片
const mcpStatusCard = { 
  title: 'MCP STATUS', 
  getValue: () => status.mcpConnected.value ? 'CONNECTED' : 'OFFLINE',
  color: () => status.mcpConnected.value ? 'bg-emerald-500' : 'bg-red-500',
  subtext: 'SIGNAL OK',
  icon: 'i-carbon-activity',
}

const emit = defineEmits<{
  navigateTo: [tab: string]
}>()

function handleAction(key: string) {
  emit('navigateTo', key)
}
</script>

<template>
  <div class="intro-page">
    <!-- 右上角固定按钮 -->
    <div class="corner-actions">
      <button class="test-popup-btn" title="测试弹窗" @click="showTestPopup">
        <div class="i-carbon-test-tool w-4 h-4" />
      </button>
      <div class="side-label">SIDE A</div>
    </div>

    <!-- 头部区域 -->
    <header class="header-section">
      <div>
        <h1 class="main-title">
          NEUROSPEC
          <span class="version-badge">v0.4.0</span>
        </h1>
        <p class="subtitle">
          <span class="subtitle-line" />
          AI-POWERED DEVELOPMENT ASSISTANT
          <span class="subtitle-line" />
        </p>
      </div>
    </header>

    <!-- MCP 状态卡片 -->
    <div class="status-single">
      <div class="status-card-wrapper">
        <!-- 胶带装饰 -->
        <div class="tape-decoration" />
        
        <div class="status-card">
          <div class="card-header">
            <div class="status-dot" :class="mcpStatusCard.color()" />
            <span class="card-title">{{ mcpStatusCard.title }}</span>
          </div>
          <div class="card-value">
            {{ mcpStatusCard.getValue() }}
          </div>
          <div class="card-footer">
            <span>{{ mcpStatusCard.subtext }}</span>
            <div :class="mcpStatusCard.icon" class="card-icon" />
          </div>
        </div>
      </div>
    </div>

    <!-- 快速入口按钮 -->
    <div class="action-grid">
      <button
        v-for="btn in actionButtons"
        :key="btn.key"
        class="action-button"
        @click="handleAction(btn.key)"
      >
        <div class="action-icon-wrapper" :class="btn.bg">
          <div :class="btn.icon" class="action-icon" />
        </div>
        <span class="action-label">{{ btn.label }}</span>
        
        <!-- 角落装饰 -->
        <span class="corner-screw top-1 left-1">+</span>
        <span class="corner-screw top-1 right-1">+</span>
        <span class="corner-screw bottom-1 left-1">+</span>
        <span class="corner-screw bottom-1 right-1">+</span>
      </button>
    </div>

    <!-- 终端日志区域 -->
    <div class="terminal-section">
      <!-- CRT 扫描线 -->
      <div class="scanline-overlay" />
      
      <div class="terminal-content">
        <div class="terminal-header">
          <span class="terminal-title">
            <div class="i-carbon-terminal mr-2" />
            SYSTEM_LOG
          </span>
          <span class="terminal-path">{{ status.projectPath || 'E:\\PROJECT' }}</span>
        </div>
        
        <div class="terminal-body">
          <div class="log-row">
            <span class="log-label">PROJECT:</span>
            <span class="log-value">NeuroSpec_v2</span>
          </div>
          <div class="log-row">
            <span class="log-label">UPTIME:</span>
            <span class="log-value uptime">{{ formattedUptime }}</span>
          </div>
        </div>

        <div class="terminal-footer">
          <p>END OF TAPE</p>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
.intro-page {
  max-width: 100%;
  position: relative;
}

/* 右上角固定按钮 */
.corner-actions {
  position: absolute;
  top: 0;
  right: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.375rem;
  z-index: 10;
}

/* 头部区域 */
.header-section {
  margin-bottom: 1rem;
  padding-bottom: 0.75rem;
  border-bottom: 2px dashed #9ca3af;
  padding-right: 3rem;
}

.main-title {
  font-size: 1.75rem;
  font-weight: 900;
  text-transform: uppercase;
  letter-spacing: -0.025em;
  color: #111827;
  margin-bottom: 0.25rem;
}

.version-badge {
  display: inline-block;
  background: #2563eb;
  color: white;
  padding: 0.125rem 0.5rem;
  font-size: 0.75rem;
  margin-left: 0.5rem;
  border: 2px solid #111827;
  transform: rotate(-2deg);
  vertical-align: middle;
}

.subtitle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  color: #6b7280;
}

.subtitle-line {
  display: inline-block;
  width: 1rem;
  height: 2px;
  background: #6b7280;
}

.test-popup-btn {
  width: 1.75rem;
  height: 1.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: white;
  border: 2px solid #1f2937;
  color: #6b7280;
  cursor: pointer;
  transition: all 0.1s;
}

.test-popup-btn:hover {
  background: #f97316;
  color: white;
}

.side-label {
  border: 2px solid #1f2937;
  padding: 0.125rem 0.5rem;
  background: #f3f4f6;
  font-weight: 700;
  font-size: 0.625rem;
}

/* 单个状态卡片 */
.status-single {
  display: flex;
  justify-content: center;
  margin-bottom: 1.25rem;
}

.status-single .status-card-wrapper {
  width: 100%;
  max-width: 280px;
}

.status-card-wrapper {
  position: relative;
}

.tape-decoration {
  position: absolute;
  top: -0.5rem;
  left: 50%;
  transform: translateX(-50%) rotate(1deg);
  width: 3rem;
  height: 0.75rem;
  background: rgba(254, 249, 195, 0.5);
  border: 1px solid rgba(253, 224, 71, 0.5);
  z-index: 0;
}

.status-card {
  position: relative;
  z-index: 10;
  background: white;
  border: 2px solid #1f2937;
  padding: 0.75rem;
  box-shadow: 3px 3px 0px 0px rgba(200, 200, 200, 1);
  transition: all 0.15s;
}

.status-card:hover {
  box-shadow: 3px 3px 0px 0px rgba(31, 41, 55, 1);
  transform: translateY(-2px);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  padding-bottom: 0.375rem;
  border-bottom: 1px solid #f3f4f6;
}

.status-dot {
  width: 0.625rem;
  height: 0.625rem;
  border-radius: 50%;
  border: 1px solid #111827;
}

.card-title {
  font-size: 0.625rem;
  font-weight: 700;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.card-value {
  font-weight: 700;
  font-size: 1rem;
  color: #1f2937;
  line-height: 1.2;
}

.card-subvalue {
  font-weight: 700;
  font-size: 1rem;
  color: #1f2937;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 0.5rem;
  font-size: 0.5rem;
  font-weight: 700;
  color: #9ca3af;
}

.card-icon {
  width: 0.75rem;
  height: 0.75rem;
}

/* 快速入口按钮网格 */
.action-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.625rem;
  margin-bottom: 1rem;
}

.action-button {
  position: relative;
  height: 3.5rem;
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  background: white;
  border: 2px solid #1f2937;
  box-shadow: 2px 2px 0px 0px rgba(31, 41, 55, 1);
  transition: all 0.1s;
  padding: 0 0.5rem;
}

.action-button:active {
  box-shadow: none;
  transform: translate(2px, 2px);
}

.action-button:hover .action-icon-wrapper {
  transform: scale(1.05);
}

.action-icon-wrapper {
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 0.25rem;
  border: 1px solid #1f2937;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.15s;
  flex-shrink: 0;
}

.action-icon {
  width: 1rem;
  height: 1rem;
  color: #1f2937;
}

.action-label {
  font-weight: 700;
  font-size: 0.625rem;
  text-transform: uppercase;
  white-space: nowrap;
}

.corner-screw {
  display: none;
}

/* 终端区域 */
.terminal-section {
  border: 2px solid #1f2937;
  background: #1a1c20;
  color: #22c55e;
  padding: 0.75rem;
  font-size: 0.75rem;
  box-shadow: inset 0 0 10px rgba(0, 0, 0, 0.5);
  position: relative;
  overflow: hidden;
}

.scanline-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.25) 50%),
              linear-gradient(90deg, rgba(255, 0, 0, 0.06), rgba(0, 255, 0, 0.02), rgba(0, 0, 255, 0.06));
  background-size: 100% 2px, 3px 100%;
  pointer-events: none;
  z-index: 10;
}

.terminal-content {
  position: relative;
  z-index: 20;
  opacity: 0.9;
}

.terminal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
  padding-bottom: 0.375rem;
  border-bottom: 1px solid #374151;
  font-size: 0.625rem;
  color: #9ca3af;
}

.terminal-title {
  display: flex;
  align-items: center;
}

.terminal-path {
  font-size: 0.625rem;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.terminal-body {
  space-y: 0.25rem;
}

.log-row {
  display: flex;
  justify-content: space-between;
  padding: 0.25rem 0;
  font-size: 0.75rem;
}

.log-label {
  color: #6b7280;
}

.log-value {
  color: white;
}

.log-value.uptime {
  color: #4ade80;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

.terminal-footer {
  margin-top: 0.75rem;
  padding-top: 0.5rem;
  border-top: 1px dashed #374151;
  text-align: center;
}

.terminal-footer p {
  font-size: 0.625rem;
  color: #4b5563;
  letter-spacing: 0.15em;
}

/* 响应式 */
@media (max-width: 768px) {
  .action-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  
  .header-main {
    flex-direction: column;
    align-items: flex-start;
    gap: 1rem;
  }
}
</style>
