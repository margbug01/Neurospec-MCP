<script setup>
import BaseButton from '../base/Button.vue'

defineProps({
  currentTheme: {
    type: String,
    required: true,
  },
})
</script>

<template>
  <!-- 设置内容 -->
  <div class="flex items-center justify-between">
    <div class="flex items-center">
      <div class="w-1.5 h-1.5 bg-primary-500 rounded-full mr-3 flex-shrink-0" />
      <div>
        <div class="text-sm font-medium leading-relaxed">
          界面主题
        </div>
        <div class="text-xs opacity-60">
          当前使用浅色主题
        </div>
      </div>
    </div>
    <div>
      <!-- 浅色主题 - 固定选中 -->
      <BaseButton
        type="primary"
        size="small"
        disabled
      >
        <div
          class="w-3 h-3 rounded-full border transition-all duration-200 mr-2"
          :style="{
            backgroundColor: '#ffffff',
            borderColor: '#14b8a6',
          }"
        />
        浅色
      </BaseButton>
    </div>
  </div>
</template>
