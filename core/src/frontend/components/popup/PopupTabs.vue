<script setup lang="ts">
interface Props {
  activeTab: 'interact' | 'history' | 'memory'
}

interface Emits {
  (e: 'update:activeTab', tab: 'interact' | 'history' | 'memory'): void
}

defineProps<Props>()
const emit = defineEmits<Emits>()

const tabs = [
  { id: 'interact', label: '交互', icon: 'i-carbon-chat' },
  { id: 'history', label: '历史', icon: 'i-carbon-time' },
  { id: 'memory', label: '记忆', icon: 'i-carbon-brain' },
] as const

function selectTab(tabId: 'interact' | 'history' | 'memory') {
  emit('update:activeTab', tabId)
}
</script>

<template>
  <div class="popup-tabs">
    <button
      v-for="tab in tabs"
      :key="tab.id"
      class="tab-btn"
      :class="{ active: activeTab === tab.id }"
      @click="selectTab(tab.id)"
    >
      <div :class="[tab.icon, 'w-3.5 h-3.5']" />
      <span>{{ tab.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.popup-tabs {
  display: flex;
  gap: 0.25rem;
  padding: 0.5rem;
  background: #f9fafb;
  border-bottom: 2px solid #1f2937;
}

.tab-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.375rem;
  padding: 0.5rem 0.75rem;
  background: white;
  border: 2px solid #1f2937;
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
  font-weight: 700;
  color: #1f2937;
  cursor: pointer;
  transition: all 0.1s;
  text-transform: uppercase;
  letter-spacing: 0.025em;
}

.tab-btn:hover {
  background: #f3f4f6;
}

.tab-btn.active {
  background: #1f2937;
  color: white;
  box-shadow: 2px 2px 0px 0px rgba(31, 41, 55, 0.3);
}

.tab-btn:active {
  transform: translateY(1px);
}
</style>
