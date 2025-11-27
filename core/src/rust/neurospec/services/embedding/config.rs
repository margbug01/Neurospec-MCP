//! 嵌入服务配置

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 嵌入服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider 类型: "jina" | "siliconflow" | "openai" | "dashscope"
    pub provider: String,
    
    /// API Key
    pub api_key: String,
    
    /// 模型名称
    pub model: String,
    
    /// 自定义 Base URL（可选）
    pub base_url: Option<String>,
    
    /// 是否启用缓存
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
    
    /// 缓存路径
    #[serde(default = "default_cache_path")]
    pub cache_path: PathBuf,
    
    /// 请求超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_cache_enabled() -> bool { true }
fn default_timeout() -> u64 { 30 }
fn default_max_retries() -> u32 { 3 }

fn default_cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".neurospec")
        .join("embedding_cache")
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "jina".to_string(),
            api_key: String::new(),
            model: "jina-embeddings-v3".to_string(),
            base_url: None,
            cache_enabled: true,
            cache_path: default_cache_path(),
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

impl EmbeddingConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        let provider = std::env::var("NEUROSPEC_EMBEDDING_PROVIDER")
            .unwrap_or_else(|_| "jina".to_string());
        
        let api_key = std::env::var("NEUROSPEC_EMBEDDING_API_KEY")
            .or_else(|_| std::env::var("JINA_API_KEY"))
            .or_else(|_| std::env::var("SILICONFLOW_API_KEY"))
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .unwrap_or_default();
        
        let model = std::env::var("NEUROSPEC_EMBEDDING_MODEL")
            .unwrap_or_else(|_| Self::default_model_for_provider(&provider));
        
        Self {
            provider,
            api_key,
            model,
            ..Default::default()
        }
    }

    /// 根据 Provider 返回默认模型
    fn default_model_for_provider(provider: &str) -> String {
        match provider {
            "jina" => "jina-embeddings-v3".to_string(),
            "siliconflow" => "Qwen/Qwen3-Embedding-8B".to_string(),
            "openai" => "text-embedding-3-small".to_string(),
            "dashscope" => "text-embedding-v2".to_string(),
            "deepseek" => "deepseek-chat".to_string(),
            _ => "default".to_string(),
        }
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("API key is required".to_string());
        }
        
        let valid_providers = ["jina", "siliconflow", "openai", "dashscope", "deepseek"];
        if !valid_providers.contains(&self.provider.as_str()) {
            return Err(format!(
                "Invalid provider '{}'. Valid options: {:?}",
                self.provider, valid_providers
            ));
        }
        
        Ok(())
    }

    /// 创建 Jina 配置
    pub fn jina(api_key: &str) -> Self {
        Self {
            provider: "jina".to_string(),
            api_key: api_key.to_string(),
            model: "jina-embeddings-v3".to_string(),
            base_url: Some("https://api.jina.ai/v1".to_string()),
            ..Default::default()
        }
    }

    /// 创建 SiliconFlow 配置
    pub fn siliconflow(api_key: &str) -> Self {
        Self {
            provider: "siliconflow".to_string(),
            api_key: api_key.to_string(),
            model: "Qwen/Qwen3-Embedding-8B".to_string(),
            base_url: Some("https://api.siliconflow.cn/v1".to_string()),
            ..Default::default()
        }
    }

    /// 创建 OpenAI 配置
    pub fn openai(api_key: &str) -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: api_key.to_string(),
            model: "text-embedding-3-small".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            ..Default::default()
        }
    }

    /// 创建 DeepSeek 配置
    pub fn deepseek(api_key: &str) -> Self {
        Self {
            provider: "deepseek".to_string(),
            api_key: api_key.to_string(),
            model: "deepseek-chat".to_string(),
            base_url: Some("https://api.deepseek.com".to_string()),
            ..Default::default()
        }
    }
}
