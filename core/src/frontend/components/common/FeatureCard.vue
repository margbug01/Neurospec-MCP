<script setup lang="ts">
import BaseCard from '../base/Card.vue'

interface Feature {
  icon: string
  title: string
  subtitle: string
  iconBg: string
  features: string[]
}

defineProps<{
  feature: Feature
}>()
</script>

<template>
  <BaseCard padding="small" :hoverable="true" class="bg-white">
    <!-- 卡片头部 -->
    <div class="mb-4">
      <div class="flex items-center gap-3">
        <!-- 图标 -->
        <div
          class="w-10 h-10 rounded-lg flex items-center justify-center"
          :class="feature.iconBg"
        >
          <!-- UnoCSS图标 -->
          <div
            :class="feature.icon"
          />
        </div>

        <!-- 标题和副标题 -->
        <div>
          <div class="text-lg font-medium mb-1 tracking-tight">
            {{ feature.title }}
          </div>
          <div class="text-sm opacity-60 font-normal">
            {{ feature.subtitle }}
          </div>
        </div>
      </div>
    </div>

    <!-- 功能列表 -->
    <div class="space-y-2">
      <div v-for="(item, index) in feature.features" :key="index" class="flex items-center text-sm leading-relaxed">
        <div class="w-1.5 h-1.5 bg-green-500 rounded-full mr-3 flex-shrink-0" />
        <span class="opacity-90">{{ item }}</span>
      </div>
    </div>
  </BaseCard>
</template>
