<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue?: boolean
  checked?: boolean
  disabled?: boolean
  size?: 'small' | 'medium' | 'large'
  label?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: false,
  checked: false,
  disabled: false,
  size: 'medium',
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'update:checked': [value: boolean]
}>()

const isChecked = computed(() => props.checked || props.modelValue)

const checkboxClasses = computed(() => {
  const base = 'inline-flex items-center justify-center rounded border-2 transition-all duration-200 cursor-pointer select-none'

  const sizeClasses = {
    small: 'w-4 h-4',
    medium: 'w-5 h-5',
    large: 'w-6 h-6',
  }

  const stateClasses = isChecked.value
    ? 'bg-primary-500 border-primary-500'
    : 'bg-surface-100 border-surface-400 hover:border-primary-500'

  const disabledClass = props.disabled
    ? 'opacity-50 cursor-not-allowed'
    : ''

  return [base, sizeClasses[props.size], stateClasses, disabledClass].join(' ')
})

function handleToggle() {
  if (!props.disabled) {
    const newValue = !isChecked.value
    emit('update:modelValue', newValue)
    emit('update:checked', newValue)
  }
}
</script>

<template>
  <label class="inline-flex items-center gap-2 cursor-pointer select-none">
    <div :class="checkboxClasses" @click="handleToggle">
      <div v-if="isChecked" class="i-carbon-checkmark text-white w-3 h-3" />
    </div>
    <span v-if="label || $slots.default" class="text-surface-900">
      <slot>{{ label }}</slot>
    </span>
  </label>
</template>
