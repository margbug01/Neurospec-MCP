<script setup lang="ts">
import { computed } from 'vue'

interface CapsuleProps {
  type?: 'primary' | 'success' | 'warning' | 'error' | 'neutral'
  variant?: 'filled' | 'outlined' | 'glass'
  size?: 'sm' | 'md'
  clickable?: boolean
}

const props = withDefaults(defineProps<CapsuleProps>(), {
  type: 'neutral',
  variant: 'filled',
  size: 'md',
  clickable: false,
})

const emit = defineEmits<{
  click: [event: MouseEvent]
}>()

function handleClick(event: MouseEvent) {
  if (props.clickable) {
    emit('click', event)
  }
}

const capsuleClasses = computed(() => {
  const classes = ['capsule']

  // Size classes
  const sizeClasses = {
    sm: 'px-2 py-0.5 text-xs',
    md: 'px-3 py-1 text-sm',
  }
  classes.push(sizeClasses[props.size])

  // Type and variant classes
  const typeVariantClasses = {
    primary: {
      filled: 'bg-primary-500 text-white',
      outlined: 'border-2 border-primary-500 text-primary-500 bg-transparent',
      glass: 'bg-primary-500/20 text-primary-600 backdrop-blur-sm',
    },
    success: {
      filled: 'bg-[#34C759] text-white',
      outlined: 'border-2 border-[#34C759] text-[#34C759] bg-transparent',
      glass: 'bg-[#34C759]/20 text-[#34C759] backdrop-blur-sm',
    },
    warning: {
      filled: 'bg-[#FF9500] text-white',
      outlined: 'border-2 border-[#FF9500] text-[#FF9500] bg-transparent',
      glass: 'bg-[#FF9500]/20 text-[#FF9500] backdrop-blur-sm',
    },
    error: {
      filled: 'bg-[#FF3B30] text-white',
      outlined: 'border-2 border-[#FF3B30] text-[#FF3B30] bg-transparent',
      glass: 'bg-[#FF3B30]/20 text-[#FF3B30] backdrop-blur-sm',
    },
    neutral: {
      filled: 'bg-gray-500 text-white',
      outlined: 'border-2 border-gray-500 text-gray-700 bg-transparent',
      glass: 'bg-gray-500/20 text-gray-800 backdrop-blur-sm',
    },
  }
  classes.push(typeVariantClasses[props.type][props.variant])

  // Clickable classes
  if (props.clickable) {
    classes.push('cursor-pointer hover:opacity-80 active:opacity-60 transition-opacity')
  }

  return classes.join(' ')
})
</script>

<template>
  <span
    :class="capsuleClasses"
    class="rounded-full inline-flex items-center justify-center font-medium select-none"
    @click="handleClick"
  >
    <slot />
  </span>
</template>

<style scoped>
.capsule {
  transition: opacity 0.2s ease;
}
</style>
