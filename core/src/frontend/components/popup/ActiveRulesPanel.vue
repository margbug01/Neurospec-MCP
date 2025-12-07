<script setup lang="ts">
import { computed, ref } from 'vue'

interface Props {
  rules: string[]
}

const props = defineProps<Props>()
const collapsed = ref(false)

const hasRules = computed(() => props.rules && props.rules.length > 0)
</script>

<template>
  <div v-if="hasRules" class="active-rules-panel">
    <button class="panel-header" @click="collapsed = !collapsed">
      <div class="header-left">
        <div class="i-carbon-rule w-3.5 h-3.5" />
        <span class="header-title">生效规则 ({{ rules.length }})</span>
      </div>
      <div
        class="collapse-icon"
        :class="{ 'rotate-180': !collapsed }"
      >
        <div class="i-carbon-chevron-down w-3 h-3" />
      </div>
    </button>

    <div v-if="!collapsed" class="rules-list">
      <div v-for="(rule, index) in rules" :key="index" class="rule-item">
        <div class="i-carbon-checkmark w-3 h-3 text-emerald-600" />
        <span>{{ rule }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.active-rules-panel {
  background: #fefce8;
  border: 2px solid #ca8a04;
  margin-bottom: 0.75rem;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: background 0.1s;
}

.panel-header:hover {
  background: rgba(202, 138, 4, 0.1);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: #92400e;
}

.header-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.collapse-icon {
  color: #92400e;
  transition: transform 0.2s;
}

.rules-list {
  padding: 0 0.75rem 0.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.rule-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.25rem 0;
  font-size: 0.75rem;
  color: #1f2937;
}
</style>
