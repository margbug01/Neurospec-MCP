<script setup lang="ts">
import { computed, ref, watch } from 'vue'

interface Props {
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  readonly?: boolean
  rows?: number
  autosize?: boolean | { minRows?: number, maxRows?: number }
  size?: 'small' | 'medium' | 'large'
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  placeholder: '',
  disabled: false,
  readonly: false,
  rows: 3,
  autosize: false,
  size: 'medium',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'focus': [e: FocusEvent]
  'blur': [e: FocusEvent]
  'paste': [e: ClipboardEvent]
}>()

const textareaRef = ref<HTMLTextAreaElement | null>(null)

const textareaClasses = computed(() => {
  const base = 'w-full rounded-lg border transition-all duration-200 outline-none bg-surface-100 text-surface-900 placeholder:text-surface-500 resize-none'

  const sizeClasses = {
    small: 'px-3 py-1.5 text-sm',
    medium: 'px-4 py-2 text-base',
    large: 'px-5 py-2.5 text-lg',
  }

  const stateClasses = 'border-surface-400 focus:border-primary-500 focus:ring-2 focus:ring-primary-500/20'

  const disabledClass = props.disabled || props.readonly
    ? 'opacity-60 cursor-not-allowed bg-surface-200'
    : 'hover:border-surface-500'

  return [base, sizeClasses[props.size], stateClasses, disabledClass].join(' ')
})

function handleInput(e: Event) {
  const target = e.target as HTMLTextAreaElement
  emit('update:modelValue', target.value)
  if (props.autosize) {
    adjustHeight()
  }
}

function adjustHeight() {
  if (!textareaRef.value || !props.autosize)
    return

  const textarea = textareaRef.value
  textarea.style.height = 'auto'

  let minRows = 3
  let maxRows = 6

  if (typeof props.autosize === 'object') {
    minRows = props.autosize.minRows || 3
    maxRows = props.autosize.maxRows || 6
  }

  const lineHeight = Number.parseInt(getComputedStyle(textarea).lineHeight)
  const minHeight = lineHeight * minRows
  const maxHeight = lineHeight * maxRows

  const scrollHeight = textarea.scrollHeight
  const newHeight = Math.min(Math.max(scrollHeight, minHeight), maxHeight)

  textarea.style.height = `${newHeight}px`
}

watch(() => props.modelValue, () => {
  if (props.autosize) {
    setTimeout(adjustHeight, 0)
  }
})

defineExpose({
  focus: () => textareaRef.value?.focus(),
  blur: () => textareaRef.value?.blur(),
})
</script>

<template>
  <textarea
    ref="textareaRef"
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    :readonly="readonly"
    :rows="autosize ? undefined : rows"
    :class="textareaClasses"
    @input="handleInput"
    @focus="emit('focus', $event as FocusEvent)"
    @blur="emit('blur', $event as FocusEvent)"
    @paste="emit('paste', $event as ClipboardEvent)"
  />
</template>
