<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'

interface Props {
  show?: boolean
  title?: string
  closable?: boolean
  maskClosable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  closable: true,
  maskClosable: true,
})

const emit = defineEmits<{
  'update:show': [value: boolean]
  'close': []
}>()

function handleClose() {
  emit('update:show', false)
  emit('close')
}

function handleMaskClick() {
  if (props.maskClosable) {
    handleClose()
  }
}

function handleEscape(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.show && props.closable) {
    handleClose()
  }
}

watch(() => props.show, (newVal) => {
  if (newVal) {
    document.body.style.overflow = 'hidden'
  }
  else {
    document.body.style.overflow = ''
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleEscape)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleEscape)
  document.body.style.overflow = ''
})
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-opacity duration-200"
      leave-active-class="transition-opacity duration-200"
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click.self="handleMaskClick"
      >
        <Transition
          enter-active-class="transition-all duration-200"
          leave-active-class="transition-all duration-200"
          enter-from-class="opacity-0 scale-95"
          leave-to-class="opacity-0 scale-95"
        >
          <div
            v-if="show"
            class="relative bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col"
            @click.stop
          >
            <!-- Header -->
            <div v-if="title || closable" class="flex items-center justify-between px-6 py-4 border-b border-gray-200">
              <h3 class="text-lg font-semibold text-gray-900">
                <slot name="header">
                  {{ title }}
                </slot>
              </h3>
              <button
                v-if="closable"
                class="i-carbon-close w-5 h-5 text-gray-500 hover:text-gray-700 cursor-pointer transition-colors"
                @click="handleClose"
              />
            </div>

            <!-- Content -->
            <div class="flex-1 overflow-y-auto px-6 py-4">
              <slot />
            </div>

            <!-- Footer -->
            <div v-if="$slots.footer" class="px-6 py-4 border-t border-gray-200">
              <slot name="footer" />
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>
