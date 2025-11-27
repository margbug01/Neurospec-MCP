<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  type?: 'info' | 'success' | 'warning' | 'error'
  title?: string
  closable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  type: 'info',
  closable: false,
})

const emit = defineEmits<{
  close: []
}>()

const alertClasses = computed(() => {
  const base = 'rounded-lg p-4 border'

  const typeClasses = {
    info: 'bg-blue-50 border-blue-200 text-blue-800',
    success: 'bg-green-50 border-green-200 text-green-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    error: 'bg-red-50 border-red-200 text-red-800',
  }

  return [base, typeClasses[props.type]].join(' ')
})

const iconClasses = computed(() => {
  const icons = {
    info: 'i-carbon-information',
    success: 'i-carbon-checkmark-filled',
    warning: 'i-carbon-warning',
    error: 'i-carbon-error-filled',
  }

  return [icons[props.type], 'w-5 h-5'].join(' ')
})
</script>

<template>
  <div :class="alertClasses">
    <div class="flex items-start gap-3">
      <div :class="iconClasses" />
      <div class="flex-1">
        <div v-if="title" class="font-semibold mb-1">
          {{ title }}
        </div>
        <slot />
      </div>
      <button
        v-if="closable"
        class="i-carbon-close w-4 h-4 opacity-60 hover:opacity-100 cursor-pointer"
        @click="emit('close')"
      />
    </div>
  </div>
</template>
