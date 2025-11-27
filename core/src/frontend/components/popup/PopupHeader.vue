<script setup lang="ts">
interface Props {
  currentTheme?: string
  loading?: boolean
  showMainLayout?: boolean
  alwaysOnTop?: boolean
}

interface Emits {
  openMainLayout: []
  toggleAlwaysOnTop: []
}

const props = withDefaults(defineProps<Props>(), {
  currentTheme: 'light',
  loading: false,
  showMainLayout: false,
  alwaysOnTop: false,
})

const emit = defineEmits<Emits>()

function handleOpenMainLayout() {
  emit('openMainLayout')
}

function handleToggleAlwaysOnTop() {
  emit('toggleAlwaysOnTop')
}
</script>

<template>
  <div class="px-4 py-2 select-none">
    <div class="flex items-center justify-between">
      <!-- 左侧：标题 -->
      <div class="flex items-center gap-2">
        <div class="w-2.5 h-2.5 rounded-full bg-gray-900" />
        <h1 class="text-[13px] font-semibold text-gray-900 tracking-wide">
          NeuroSpec
        </h1>
      </div>

      <!-- 右侧：操作按钮 -->
      <div class="flex items-center gap-1.5">
        <!-- 置顶按钮 -->
        <button
          class="w-7 h-7 rounded-lg flex items-center justify-center text-gray-500 hover:text-gray-900 hover:bg-black/5 transition-all duration-200 outline-none focus:outline-none"
          :title="props.alwaysOnTop ? '取消置顶' : '窗口置顶'"
          @click="handleToggleAlwaysOnTop"
        >
          <div
            :class="props.alwaysOnTop ? 'i-carbon-pin-filled' : 'i-carbon-pin'"
            class="w-3.5 h-3.5"
          />
        </button>

        <!-- 聊天/设置按钮 -->
        <button
          class="w-7 h-7 rounded-lg flex items-center justify-center text-gray-500 hover:text-gray-900 hover:bg-black/5 transition-all duration-200 outline-none focus:outline-none"
          :title="props.showMainLayout ? '返回聊天' : '打开设置'"
          @click="handleOpenMainLayout"
        >
          <div
            :class="props.showMainLayout ? 'i-carbon-chat' : 'i-carbon-settings'"
            class="w-3.5 h-3.5"
          />
        </button>
      </div>
    </div>
  </div>
</template>
