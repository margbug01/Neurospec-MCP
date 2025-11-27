<script setup lang="ts">
import { ref } from 'vue'

interface Props {
  src: string
  alt?: string
  width?: number | string
  height?: number | string
  objectFit?: 'contain' | 'cover' | 'fill' | 'none' | 'scale-down'
  previewable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  alt: '',
  objectFit: 'cover',
  previewable: true,
})

const showPreview = ref(false)

function openPreview() {
  if (props.previewable) {
    showPreview.value = true
  }
}

function closePreview() {
  showPreview.value = false
}
</script>

<template>
  <div class="inline-block">
    <img
      :src="src"
      :alt="alt"
      :width="width"
      :height="height"
      :style="{ objectFit }"
      :class="{ 'cursor-pointer': previewable }"
      @click="openPreview"
    >

    <!-- Preview Modal -->
    <Teleport to="body">
      <Transition
        enter-active-class="transition-opacity duration-200"
        leave-active-class="transition-opacity duration-200"
        enter-from-class="opacity-0"
        leave-to-class="opacity-0"
      >
        <div
          v-if="showPreview"
          class="fixed inset-0 z-50 flex items-center justify-center bg-black/90"
          @click="closePreview"
        >
          <button
            class="absolute top-4 right-4 i-carbon-close w-8 h-8 text-white hover:text-gray-300 cursor-pointer z-10"
            @click="closePreview"
          />
          <img
            :src="src"
            :alt="alt"
            class="max-w-[90vw] max-h-[90vh] object-contain"
            @click.stop
          >
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
