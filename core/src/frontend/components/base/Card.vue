<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  padding?: 'none' | 'small' | 'medium' | 'large'
  bordered?: boolean
  hoverable?: boolean
  shadow?: 'none' | 'sm' | 'md' | 'lg'
}

const props = withDefaults(defineProps<Props>(), {
  padding: 'medium',
  bordered: true,
  hoverable: false,
  shadow: 'sm',
})

const cardClasses = computed(() => {
  const base = 'bg-surface-100 rounded-lg transition-all duration-200'

  const paddingClasses = {
    none: '',
    small: 'p-3',
    medium: 'p-4',
    large: 'p-6',
  }

  const borderClass = props.bordered ? 'border border-surface-400' : ''

  const shadowClasses = {
    none: '',
    sm: 'shadow-sm',
    md: 'shadow-md',
    lg: 'shadow-lg',
  }

  const hoverClass = props.hoverable
    ? 'hover:shadow-md hover:border-surface-500 cursor-pointer'
    : ''

  return [
    base,
    paddingClasses[props.padding],
    borderClass,
    shadowClasses[props.shadow],
    hoverClass,
  ].filter(Boolean).join(' ')
})
</script>

<template>
  <div :class="cardClasses">
    <slot />
  </div>
</template>
