import { invoke } from '@tauri-apps/api/core'
import { ref, reactive } from 'vue'

// 嵌入配置接口
export interface EmbeddingConfig {
  provider: string
  api_key: string
  model: string
  base_url: string
  cache_enabled: boolean
}

// Provider 选项
export const providerOptions = [
  { value: 'siliconflow', label: 'SiliconFlow (推荐)', defaultModel: 'Qwen/Qwen3-Embedding-8B', defaultUrl: 'https://api.siliconflow.cn/v1' },
  { value: 'jina', label: 'Jina AI', defaultModel: 'jina-embeddings-v3', defaultUrl: 'https://api.jina.ai/v1' },
  { value: 'openai', label: 'OpenAI', defaultModel: 'text-embedding-3-small', defaultUrl: 'https://api.openai.com/v1' },
  { value: 'dashscope', label: 'DashScope', defaultModel: 'text-embedding-v2', defaultUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1' },
  { value: 'deepseek', label: 'DeepSeek (不推荐)', defaultModel: 'deepseek-chat', defaultUrl: 'https://api.deepseek.com' },
]

// 单例实例
let embeddingInstance: ReturnType<typeof createEmbedding> | null = null

function createEmbedding() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  const config = reactive<EmbeddingConfig>({
    provider: 'siliconflow',
    api_key: '',
    model: 'Qwen/Qwen3-Embedding-8B',
    base_url: 'https://api.siliconflow.cn/v1',
    cache_enabled: true,
  })

  // 加载配置
  async function loadConfig() {
    loading.value = true
    error.value = null
    try {
      const result = await invoke<EmbeddingConfig | null>('get_embedding_config_cmd')
      if (result) {
        Object.assign(config, result)
      }
    } catch (e: any) {
      console.warn('Failed to load embedding config:', e)
      // 使用默认配置，不报错
    } finally {
      loading.value = false
    }
  }

  // 保存配置
  async function saveConfig() {
    loading.value = true
    error.value = null
    try {
      await invoke('save_embedding_config_cmd', { config })
      return true
    } catch (e: any) {
      error.value = `保存失败: ${e}`
      return false
    } finally {
      loading.value = false
    }
  }

  // 切换 Provider 时自动更新默认值
  function onProviderChange(provider: string) {
    const option = providerOptions.find(p => p.value === provider)
    if (option) {
      config.provider = provider
      config.model = option.defaultModel
      config.base_url = option.defaultUrl
    }
  }

  // 测试连接
  async function testConnection(): Promise<{ success: boolean; message: string }> {
    loading.value = true
    try {
      const result = await invoke<{ success: boolean; message: string }>('test_embedding_connection_cmd', { config })
      return result
    } catch (e: any) {
      return { success: false, message: `连接失败: ${e}` }
    } finally {
      loading.value = false
    }
  }

  // 掩码显示 API Key
  function getMaskedApiKey(): string {
    if (!config.api_key) return ''
    if (config.api_key.length <= 8) return '****'
    return config.api_key.slice(0, 4) + '****' + config.api_key.slice(-4)
  }

  return {
    config,
    loading,
    error,
    loadConfig,
    saveConfig,
    onProviderChange,
    testConnection,
    getMaskedApiKey,
  }
}

export function useEmbedding() {
  if (!embeddingInstance) {
    embeddingInstance = createEmbedding()
  }
  return embeddingInstance
}
