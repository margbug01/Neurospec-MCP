<script setup lang="ts">
import { ref, withDefaults } from 'vue'
import AgentsTab from '../tabs/AgentsTab.vue'
import HistoryTab from '../tabs/HistoryTab.vue'
import IntroTab from '../tabs/IntroTab.vue'
import McpToolsTab from '../tabs/McpToolsTab.vue'
import MemoryTab from '../tabs/MemoryTab.vue'
import PromptsTab from '../tabs/PromptsTab.vue'
import SettingsTab from '../tabs/SettingsTab.vue'

interface Props {
  currentTheme: string
  alwaysOnTop: boolean
  windowWidth: number
  windowHeight: number
  fixedWindowSize: boolean
  initialTab?: string
  fromPopup?: boolean
}

interface Emits {
  toggleAlwaysOnTop: []
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
  configReloaded: []
  closeToPopup: []
}

const props = withDefaults(defineProps<Props>(), {
  fromPopup: false
})
const emit = defineEmits<Emits>()

function handleConfigReloaded() {
  emit('configReloaded')
}

const activeTab = ref(props.initialTab || 'intro')
</script>

<template>
  <div class="retro-container">
    <!-- 噪点纹理层 -->
    <div class="noise-overlay" />

    <!-- 磁带卡带主容器 -->
    <div class="cassette-card">
      <!-- 顶部三色条 -->
      <div class="top-stripe">
        <div class="stripe-orange" />
        <div class="stripe-teal" />
        <div class="stripe-dark" />
      </div>

      <!-- 主内容区 -->
      <div class="main-content custom-scrollbar">
        <component
          :is="activeTab === 'intro' ? IntroTab
            : activeTab === 'mcp-tools' ? McpToolsTab
              : activeTab === 'memory' ? MemoryTab
                : activeTab === 'history' ? HistoryTab
                  : activeTab === 'prompts' ? PromptsTab
                    : activeTab === 'agents' ? AgentsTab
                      : SettingsTab"
          :current-theme="props.currentTheme"
          :always-on-top="props.alwaysOnTop"
          :window-width="props.windowWidth"
          :window-height="props.windowHeight"
          :fixed-window-size="props.fixedWindowSize"
          :from-popup="props.fromPopup"
          @toggle-always-on-top="emit('toggleAlwaysOnTop')"
          @update-window-size="emit('updateWindowSize', $event)"
          @config-reloaded="handleConfigReloaded"
          @navigate-to="activeTab = $event"
          @close-to-popup="emit('closeToPopup')"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.retro-container {
  min-height: 100vh;
  background-color: #e8e4d9;
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace;
  color: #1f2937;
  padding: 0.5rem;
  position: relative;
}

/* 噪点纹理 */
.noise-overlay {
  position: fixed;
  inset: 0;
  pointer-events: none;
  opacity: 0.02;
  z-index: 50;
  mix-blend-mode: multiply;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
}

/* 磁带卡片主容器 */
.cassette-card {
  max-width: 680px;
  margin: 0 auto;
  background-color: #fbfaf8;
  border: 3px solid #1f2937;
  box-shadow: 6px 6px 0px 0px rgba(31, 41, 55, 1);
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 1rem);
  position: relative;
  overflow: hidden;
}

/* 顶部三色条 */
.top-stripe {
  height: 8px;
  width: 100%;
  display: flex;
  border-bottom: 2px solid #1f2937;
  flex-shrink: 0;
}

.stripe-orange { flex: 1; background-color: #f97316; }
.stripe-teal { flex: 1; background-color: #0d9488; }
.stripe-dark { flex: 1; background-color: #1f2937; }

/* 主内容区 */
.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

/* 自定义滚动条 */
.custom-scrollbar::-webkit-scrollbar {
  width: 8px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: #e8e4d9;
  border-left: 1px solid #1f2937;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #1f2937;
  border: 1px solid #e8e4d9;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #ea580c;
}
</style>
