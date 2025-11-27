<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue?: string | number
  type?: 'text' | 'password' | 'number' | 'email' | 'search'
  placeholder?: string
  disabled?: boolean
  readonly?: boolean
  size?: 'small' | 'medium' | 'large'
  error?: boolean
  clearable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  type: 'text',
  placeholder: '',
  disabled: false,
  readonly: false,
  size: 'medium',
  error: false,
  clearable: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  'focus': [e: FocusEvent]
  'blur': [e: FocusEvent]
  'clear': []
}>()

const inputClasses = computed(() => {
  const base = 'w-full rounded-lg border transition-all duration-200 outline-none bg-surface-100 text-surface-900 placeholder:text-surface-500'

  const sizeClasses = {
    small: 'px-3 py-1.5 text-sm',
    medium: 'px-4 py-2 text-base',
    large: 'px-5 py-2.5 text-lg',
  }

  const stateClasses = props.error
    ? 'border-error focus:border-error focus:ring-2 focus:ring-error/20'
    : 'border-surface-400 focus:border-primary-500 focus:ring-2 focus:ring-primary-500/20'

  const disabledClass = props.disabled || props.readonly
    ? 'opacity-60 cursor-not-allowed bg-surface-200'
    : 'hover:border-surface-500'

  return [base, sizeClasses[props.size], stateClasses, disabledClass].join(' ')
})

function handleInput(e: Event) {
  const target = e.target as HTMLInputElement
  emit('update:modelValue', props.type === 'number' ? Number(target.value) : target.value)
}

function handleClear() {
  emit('update:modelValue', '')
  emit('clear')
}
</script>

<template>
  <div class="relative w-full">
    <input
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :class="inputClasses"
      @input="handleInput"
      @focus="emit('focus', $event as FocusEvent)"
      @blur="emit('blur', $event as FocusEvent)"
    >
    <button
      v-if="clearable && modelValue && !disabled && !readonly"
      type="button"
      class="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded-full hover:bg-surface-200 text-surface-600 transition-colors"
      @click="handleClear"
    >
      <span class="i-carbon-close w-4 h-4" />
    </button>
  </div>
</template>
