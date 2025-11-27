<script setup lang="ts">
import { ref } from 'vue'

interface Props {
  content?: string
  placement?: 'top' | 'bottom' | 'left' | 'right'
}

const props = withDefaults(defineProps<Props>(), {
  placement: 'top',
})

const showTooltip = ref(false)
</script>

<template>
  <div class="relative inline-block">
    <div
      @mouseenter="showTooltip = true"
      @mouseleave="showTooltip = false"
    >
      <slot />
    </div>
    <Transition
      enter-active-class="transition-opacity duration-150"
      leave-active-class="transition-opacity duration-150"
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="showTooltip && (content || $slots.content)"
        class="absolute z-50 px-2 py-1 text-xs text-white bg-gray-900 rounded shadow-lg whitespace-nowrap pointer-events-none"
        :class="{
          'bottom-full left-1/2 -translate-x-1/2 mb-2': placement === 'top',
          'top-full left-1/2 -translate-x-1/2 mt-2': placement === 'bottom',
          'right-full top-1/2 -translate-y-1/2 mr-2': placement === 'left',
          'left-full top-1/2 -translate-y-1/2 ml-2': placement === 'right',
        }"
      >
        <slot name="content">
          {{ content }}
        </slot>
      </div>
    </Transition>
  </div>
</template>
