<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue?: boolean
  disabled?: boolean
  size?: 'small' | 'medium' | 'large'
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: false,
  disabled: false,
  size: 'medium',
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const switchClasses = computed(() => {
  const base = 'relative inline-flex items-center rounded-full transition-all duration-200 cursor-pointer select-none'

  const sizeClasses = {
    small: 'h-5 w-9',
    medium: 'h-6 w-11',
    large: 'h-7 w-13',
  }

  const stateClasses = props.modelValue
    ? 'bg-primary-500'
    : 'bg-surface-400'

  const disabledClass = props.disabled
    ? 'opacity-50 cursor-not-allowed'
    : 'hover:opacity-90'

  return [base, sizeClasses[props.size], stateClasses, disabledClass].join(' ')
})

const thumbClasses = computed(() => {
  const base = 'inline-block rounded-full bg-white transition-transform duration-200 shadow-sm'

  const sizeClasses = {
    small: 'h-4 w-4',
    medium: 'h-5 w-5',
    large: 'h-6 w-6',
  }

  const positionClasses = {
    small: props.modelValue ? 'translate-x-4' : 'translate-x-0.5',
    medium: props.modelValue ? 'translate-x-5' : 'translate-x-0.5',
    large: props.modelValue ? 'translate-x-6' : 'translate-x-0.5',
  }

  return [base, sizeClasses[props.size], positionClasses[props.size]].join(' ')
})

function handleToggle() {
  if (!props.disabled) {
    emit('update:modelValue', !props.modelValue)
  }
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    :class="switchClasses"
    @click="handleToggle"
  >
    <span :class="thumbClasses" />
  </button>
</template>
