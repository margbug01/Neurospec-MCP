<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  size?: 'small' | 'medium' | 'large'
}

const props = withDefaults(defineProps<Props>(), {
  size: 'medium',
})

const spinnerClasses = computed(() => {
  const sizeClasses = {
    small: 'w-4 h-4',
    medium: 'w-6 h-6',
    large: 'w-8 h-8',
  }

  return ['i-carbon-loading animate-spin text-primary-500', sizeClasses[props.size]].join(' ')
})
</script>

<template>
  <div :class="spinnerClasses" />
</template>
