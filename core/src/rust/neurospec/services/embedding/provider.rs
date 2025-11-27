//! 嵌入服务 Provider 实现

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::future::Future;
use std::pin::Pin;

use super::config::EmbeddingConfig;

/// 嵌入结果
pub type EmbeddingResult = Vec<f32>;

/// 嵌入服务 Provider trait
pub trait EmbeddingProvider: Send + Sync {
    /// 获取单个文本的嵌入向量
    fn embed(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult>> + Send + '_>>;
    
    /// 批量获取嵌入向量
    fn embed_batch(&self, texts: &[String]) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>>> + Send + '_>>;
    
    /// 获取向量维度
    fn dimension(&self) -> usize;
}

/// 创建 Provider
pub fn create_provider(config: &EmbeddingConfig) -> Result<Arc<dyn EmbeddingProvider>> {
    match config.provider.as_str() {
        "jina" | "siliconflow" | "openai" | "dashscope" | "deepseek" => {
            Ok(Arc::new(OpenAICompatibleProvider::new(config)?))
        }
        _ => Err(anyhow!("Unknown provider: {}", config.provider)),
    }
}

/// OpenAI 兼容的 Provider
/// 
/// 支持所有使用 OpenAI API 格式的服务：
/// - OpenAI
/// - Jina
/// - SiliconFlow
/// - DashScope (阿里云)
pub struct OpenAICompatibleProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    dimension: usize,
}

impl OpenAICompatibleProvider {
    pub fn new(config: &EmbeddingConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        
        let base_url = config.base_url.clone().unwrap_or_else(|| {
            match config.provider.as_str() {
                "jina" => "https://api.jina.ai/v1".to_string(),
                "siliconflow" => "https://api.siliconflow.cn/v1".to_string(),
                "openai" => "https://api.openai.com/v1".to_string(),
                "dashscope" => "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                "deepseek" => "https://api.deepseek.com".to_string(),
                _ => "https://api.openai.com/v1".to_string(),
            }
        });
        
        // 根据模型确定维度
        let dimension = Self::infer_dimension(&config.model);
        
        Ok(Self {
            client,
            base_url,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            dimension,
        })
    }

    /// 根据模型名称推断维度
    fn infer_dimension(model: &str) -> usize {
        match model {
            // OpenAI
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            // Jina
            "jina-embeddings-v3" => 1024,
            "jina-embeddings-v2-base-en" => 768,
            // BGE
            "BAAI/bge-m3" => 1024,
            "BAAI/bge-large-zh-v1.5" => 1024,
            "BAAI/bge-small-zh-v1.5" => 512,
            // Qwen Embedding
            "Qwen/Qwen3-Embedding-8B" => 4096,
            "Qwen/Qwen3-Embedding-0.6B" => 1024,
            // 默认
            _ => 768,
        }
    }
}

impl EmbeddingProvider for OpenAICompatibleProvider {
    fn embed(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<EmbeddingResult>> + Send + '_>> {
        let text = text.to_string();
        Box::pin(async move {
            let results = self.embed_batch_impl(&[text]).await?;
            results.into_iter().next().ok_or_else(|| anyhow!("Empty response"))
        })
    }

    fn embed_batch(&self, texts: &[String]) -> Pin<Box<dyn Future<Output = Result<Vec<EmbeddingResult>>> + Send + '_>> {
        let texts = texts.to_vec();
        Box::pin(async move {
            self.embed_batch_impl(&texts).await
        })
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

impl OpenAICompatibleProvider {
    /// 内部实现批量嵌入
    async fn embed_batch_impl(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let url = format!("{}/embeddings", self.base_url);
        
        let request_body = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
            encoding_format: Some("float".to_string()),
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("API error {}: {}", status, error_text));
        }

        let result: EmbeddingResponse = response.json().await?;
        
        // 按 index 排序并提取向量
        let mut embeddings: Vec<(usize, Vec<f32>)> = result.data
            .into_iter()
            .map(|e| (e.index, e.embedding))
            .collect();
        embeddings.sort_by_key(|(idx, _)| *idx);
        
        Ok(embeddings.into_iter().map(|(_, v)| v).collect())
    }
}

// API 请求/响应结构

#[derive(Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    #[allow(dead_code)]
    prompt_tokens: u32,
    #[allow(dead_code)]
    total_tokens: u32,
}
