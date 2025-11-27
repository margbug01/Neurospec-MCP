<script setup lang="ts">
interface Props {
  modelValue?: string
  value: string
  name?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  name: 'radio',
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function handleChange() {
  if (!props.disabled) {
    emit('update:modelValue', props.value)
  }
}
</script>

<template>
  <label class="inline-flex items-center cursor-pointer select-none" :class="{ 'opacity-50 cursor-not-allowed': disabled }">
    <input
      type="radio"
      :name="name"
      :value="value"
      :checked="modelValue === value"
      :disabled="disabled"
      class="hidden"
      @change="handleChange"
    >
    <span
      class="w-4 h-4 rounded-full border-2 mr-2 flex items-center justify-center transition-all duration-200"
      :class="modelValue === value ? 'border-primary-500' : 'border-surface-400'"
    >
      <span
        v-if="modelValue === value"
        class="w-2 h-2 bg-primary-500 rounded-full"
      />
    </span>
    <slot />
  </label>
</template>
