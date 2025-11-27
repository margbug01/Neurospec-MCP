<script setup lang="ts">
import { useToast } from '../../composables/useToast'

const { toasts, removeToast } = useToast()

function getToastClasses(type: string) {
  const base = 'flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg border min-w-[300px] max-w-md'

  const typeClasses = {
    info: 'bg-blue-50 border-blue-200 text-blue-800',
    success: 'bg-green-50 border-green-200 text-green-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    error: 'bg-red-50 border-red-200 text-red-800',
    loading: 'bg-blue-50 border-blue-200 text-blue-800',
  }

  return [base, typeClasses[type as keyof typeof typeClasses]].join(' ')
}

function getIconClass(type: string) {
  const icons = {
    info: 'i-carbon-information',
    success: 'i-carbon-checkmark-filled',
    warning: 'i-carbon-warning',
    error: 'i-carbon-error-filled',
    loading: 'i-carbon-loading animate-spin',
  }

  return [icons[type as keyof typeof icons], 'w-5 h-5 flex-shrink-0'].join(' ')
}
</script>

<template>
  <Teleport to="body">
    <div class="fixed top-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
      <TransitionGroup
        enter-active-class="transition-all duration-200"
        leave-active-class="transition-all duration-200"
        enter-from-class="opacity-0 translate-x-full"
        leave-to-class="opacity-0 translate-x-full"
      >
        <div
          v-for="toast in toasts"
          :key="toast.id"
          :class="getToastClasses(toast.type)"
          class="pointer-events-auto"
        >
          <div :class="getIconClass(toast.type)" />
          <span class="flex-1 text-sm font-medium">{{ toast.message }}</span>
          <button
            v-if="toast.type !== 'loading'"
            class="i-carbon-close w-4 h-4 opacity-60 hover:opacity-100 cursor-pointer flex-shrink-0"
            @click="removeToast(toast.id)"
          />
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>
