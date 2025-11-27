<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  type?: 'primary' | 'secondary' | 'ghost' | 'text'
  variant?: 'primary' | 'secondary' | 'ghost' | 'text' | 'danger' | 'warning' | 'success' | 'ghost-danger'
  size?: 'small' | 'medium' | 'large'
  disabled?: boolean
  loading?: boolean
  block?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  type: 'secondary',
  size: 'medium',
  disabled: false,
  loading: false,
  block: false,
})

const buttonClasses = computed(() => {
  const base = 'inline-flex items-center justify-center font-medium transition-all duration-200 rounded-lg cursor-pointer select-none outline-none'

  // Support both type and variant props for backwards compatibility
  const effectiveType = props.variant || props.type

  const typeClasses = {
    'primary': 'bg-primary-500 text-white hover:bg-primary-600 active:bg-primary-700 disabled:bg-surface-300 disabled:text-surface-600',
    'secondary': 'bg-surface-100 text-surface-900 border border-surface-200 hover:bg-surface-200 active:bg-surface-300 disabled:bg-surface-100 disabled:text-surface-500 disabled:border-surface-300',
    'ghost': 'bg-transparent text-surface-900 hover:bg-surface-100 active:bg-surface-200 disabled:text-surface-500',
    'text': 'bg-transparent text-surface-900 hover:bg-surface-100 active:bg-surface-200 disabled:text-surface-500',
    'danger': 'bg-error text-white hover:bg-error/90 active:bg-error/80 disabled:bg-surface-300 disabled:text-surface-600',
    'warning': 'bg-warning text-white hover:bg-warning/90 active:bg-warning/80 disabled:bg-surface-300 disabled:text-surface-600',
    'success': 'bg-success text-white hover:bg-success/90 active:bg-success/80 disabled:bg-surface-300 disabled:text-surface-600',
    'ghost-danger': 'bg-transparent text-error hover:bg-error/10 active:bg-error/20 disabled:text-surface-500',
  }

  const sizeClasses = {
    small: 'px-3 py-1.5 text-sm gap-1.5',
    medium: 'px-4 py-2 text-base gap-2',
    large: 'px-5 py-2.5 text-lg gap-2.5',
  }

  const blockClass = props.block ? 'w-full' : ''
  const disabledClass = props.disabled || props.loading ? 'opacity-60 cursor-not-allowed' : ''

  return [base, typeClasses[effectiveType], sizeClasses[props.size], blockClass, disabledClass].filter(Boolean).join(' ')
})
</script>

<template>
  <button
    :class="buttonClasses"
    :disabled="disabled || loading"
  >
    <span v-if="loading" class="i-carbon-loading animate-spin" />
    <slot />
  </button>
</template>
