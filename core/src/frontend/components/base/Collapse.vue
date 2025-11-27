<script setup lang="ts">
import { provide, ref } from 'vue'

interface Props {
  defaultExpandedNames?: string[]
}

const props = withDefaults(defineProps<Props>(), {
  defaultExpandedNames: () => [],
})

const expandedNames = ref<string[]>(props.defaultExpandedNames)

function toggleItem(name: string) {
  const index = expandedNames.value.indexOf(name)
  if (index === -1) {
    expandedNames.value.push(name)
  }
  else {
    expandedNames.value.splice(index, 1)
  }
}

function isExpanded(name: string) {
  return expandedNames.value.includes(name)
}

provide('collapse', {
  expandedNames,
  toggleItem,
  isExpanded,
})
</script>

<template>
  <div class="space-y-2">
    <slot />
  </div>
</template>
