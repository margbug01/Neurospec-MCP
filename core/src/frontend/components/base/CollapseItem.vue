<script setup lang="ts">
import { computed, inject } from 'vue'

interface Props {
  name: string
  title?: string
}

const props = defineProps<Props>()

const collapse = inject<{
  expandedNames: { value: string[] }
  toggleItem: (name: string) => void
  isExpanded: (name: string) => boolean
}>('collapse')

const isExpanded = computed(() => collapse?.isExpanded(props.name) ?? false)

function toggle() {
  collapse?.toggleItem(props.name)
}
</script>

<template>
  <div class="border border-surface-400 rounded-lg overflow-hidden bg-surface-100">
    <div
      class="flex items-center justify-between p-4 cursor-pointer hover:bg-surface-200 transition-colors"
      @click="toggle"
    >
      <slot name="header">
        <span class="font-medium">{{ title }}</span>
      </slot>
      <span
        class="i-carbon-chevron-down w-5 h-5 transition-transform duration-200"
        :class="{ 'rotate-180': isExpanded }"
      />
    </div>
    <Transition
      enter-active-class="transition-all duration-200 ease-out"
      leave-active-class="transition-all duration-200 ease-in"
      enter-from-class="opacity-0 max-h-0"
      enter-to-class="opacity-100 max-h-screen"
      leave-from-class="opacity-100 max-h-screen"
      leave-to-class="opacity-0 max-h-0"
    >
      <div v-if="isExpanded" class="px-4 pb-4 pt-2">
        <slot />
      </div>
    </Transition>
  </div>
</template>
