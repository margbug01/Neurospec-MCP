<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  type?: 'default' | 'primary' | 'success' | 'warning' | 'error' | 'info'
  variant?: 'default' | 'primary' | 'success' | 'warning' | 'error' | 'info'
  size?: 'small' | 'medium'
  bordered?: boolean
  closable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  type: 'default',
  size: 'medium',
  bordered: true,
  closable: false,
})

const emit = defineEmits<{
  close: []
}>()

const tagClasses = computed(() => {
  const base = 'inline-flex items-center gap-1 rounded font-medium transition-all duration-200'

  // Support both type and variant props for backwards compatibility
  const effectiveType = props.variant || props.type

  const typeClasses = {
    default: 'bg-surface-200 text-surface-900 border-surface-400',
    primary: 'bg-primary-100 text-primary-700 border-primary-300',
    success: 'bg-green-100 text-green-700 border-green-300',
    warning: 'bg-orange-100 text-orange-700 border-orange-300',
    error: 'bg-red-100 text-red-700 border-red-300',
    info: 'bg-blue-100 text-blue-700 border-blue-300',
  }

  const sizeClasses = {
    small: 'px-2 py-0.5 text-xs',
    medium: 'px-2.5 py-1 text-sm',
  }

  const borderClass = props.bordered ? 'border' : ''

  return [base, typeClasses[effectiveType], sizeClasses[props.size], borderClass].filter(Boolean).join(' ')
})
</script>

<template>
  <span :class="tagClasses">
    <slot name="icon" />
    <slot />
    <button
      v-if="closable"
      type="button"
      class="i-carbon-close w-3 h-3 hover:opacity-70 cursor-pointer"
      @click="emit('close')"
    />
  </span>
</template>
