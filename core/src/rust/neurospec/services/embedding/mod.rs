//! 统一嵌入服务
//!
//! 提供文本向量化能力，支持多个外部 API Provider

pub mod provider;
pub mod cache;
pub mod config;

pub use provider::{EmbeddingProvider, EmbeddingResult};
pub use cache::EmbeddingCache;
pub use config::EmbeddingConfig;

use std::sync::Arc;
use anyhow::Result;

/// 统一嵌入服务
/// 
/// 封装 Provider 和 Cache，提供简单的接口
pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    cache: Option<EmbeddingCache>,
}

impl EmbeddingService {
    /// 从配置创建服务
    pub fn from_config(config: &EmbeddingConfig) -> Result<Self> {
        let provider = provider::create_provider(config)?;
        
        let cache = if config.cache_enabled {
            Some(EmbeddingCache::new(&config.cache_path)?)
        } else {
            None
        };
        
        Ok(Self { provider, cache })
    }

    /// 获取文本的嵌入向量
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // 检查缓存
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get(text)? {
                return Ok(cached);
            }
        }

        // 调用 Provider
        let vector = self.provider.embed(text).await?;

        // 存入缓存
        if let Some(ref cache) = self.cache {
            let _ = cache.set(text, &vector);
        }

        Ok(vector)
    }

    /// 批量获取嵌入向量
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // 检查缓存，找出未缓存的
        let mut results: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        if let Some(ref cache) = self.cache {
            for (i, text) in texts.iter().enumerate() {
                if let Ok(Some(cached)) = cache.get(text) {
                    results[i] = Some(cached);
                } else {
                    uncached_indices.push(i);
                    uncached_texts.push(text.clone());
                }
            }
        } else {
            uncached_indices = (0..texts.len()).collect();
            uncached_texts = texts.to_vec();
        }

        // 批量调用 Provider
        if !uncached_texts.is_empty() {
            let vectors = self.provider.embed_batch(&uncached_texts).await?;
            
            for (idx, vector) in uncached_indices.iter().zip(vectors.iter()) {
                results[*idx] = Some(vector.clone());
                
                // 存入缓存
                if let Some(ref cache) = self.cache {
                    let _ = cache.set(&texts[*idx], vector);
                }
            }
        }

        // 确保所有结果都有值
        Ok(results.into_iter().map(|r| r.unwrap_or_default()).collect())
    }

    /// 计算两个文本的相似度
    pub async fn similarity(&self, text1: &str, text2: &str) -> Result<f32> {
        let v1 = self.embed(text1).await?;
        let v2 = self.embed(text2).await?;
        Ok(cosine_similarity(&v1, &v2))
    }

    /// 在候选列表中找到最相似的
    pub async fn find_most_similar(
        &self,
        query: &str,
        candidates: &[String],
        top_k: usize,
    ) -> Result<Vec<(usize, f32)>> {
        let query_vec = self.embed(query).await?;
        let candidate_vecs = self.embed_batch(candidates).await?;

        let mut scores: Vec<(usize, f32)> = candidate_vecs
            .iter()
            .enumerate()
            .map(|(i, v)| (i, cosine_similarity(&query_vec, v)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);

        Ok(scores)
    }

    /// 获取向量维度
    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }
}

/// 计算余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

// ============================================================================
// 全局单例管理
// ============================================================================

use std::sync::OnceLock;
use tokio::sync::RwLock;
use std::path::PathBuf;

static GLOBAL_EMBEDDING_SERVICE: OnceLock<RwLock<Option<EmbeddingService>>> = OnceLock::new();

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".neurospec")
        .join("embedding_config.json")
}

/// 从配置文件加载配置
fn load_config_from_file() -> Option<EmbeddingConfig> {
    let path = get_config_path();
    if !path.exists() {
        return None;
    }
    
    let content = std::fs::read_to_string(&path).ok()?;
    
    #[derive(serde::Deserialize)]
    struct ConfigFile {
        provider: String,
        api_key: String,
        model: String,
        base_url: String,
        cache_enabled: bool,
    }
    
    let file_config: ConfigFile = serde_json::from_str(&content).ok()?;
    
    Some(EmbeddingConfig {
        provider: file_config.provider,
        api_key: file_config.api_key,
        model: file_config.model,
        base_url: Some(file_config.base_url),
        cache_enabled: file_config.cache_enabled,
        ..Default::default()
    })
}

/// 初始化全局嵌入服务
pub async fn init_global_embedding_service() -> Result<bool> {
    let lock = GLOBAL_EMBEDDING_SERVICE.get_or_init(|| RwLock::new(None));
    
    // 尝试从配置文件加载
    if let Some(config) = load_config_from_file() {
        if config.api_key.is_empty() {
            log::warn!("嵌入服务配置缺少 API Key，跳过初始化");
            return Ok(false);
        }
        
        match EmbeddingService::from_config(&config) {
            Ok(service) => {
                // 自动清理 7 天前的缓存
                if let Some(ref cache) = service.cache {
                    match cache.cleanup(7) {
                        Ok(deleted) if deleted > 0 => {
                            log::info!("自动清理了 {} 条过期缓存", deleted);
                        }
                        Err(e) => {
                            log::warn!("缓存清理失败: {}", e);
                        }
                        _ => {}
                    }
                }
                
                let mut guard = lock.write().await;
                *guard = Some(service);
                log::info!("嵌入服务初始化成功 (Provider: {})", config.provider);
                return Ok(true);
            }
            Err(e) => {
                log::warn!("嵌入服务初始化失败: {}", e);
                return Ok(false);
            }
        }
    }
    
    log::info!("未找到嵌入服务配置，跳过初始化");
    Ok(false)
}

/// 检查嵌入服务是否已初始化
pub async fn has_embedding_service() -> bool {
    if let Some(lock) = GLOBAL_EMBEDDING_SERVICE.get() {
        let guard = lock.read().await;
        guard.is_some()
    } else {
        false
    }
}

/// 使用嵌入服务执行操作
pub async fn with_embedding_service<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&EmbeddingService) -> std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send + '_>>,
    R: Send,
{
    let lock = GLOBAL_EMBEDDING_SERVICE.get()?;
    let guard = lock.read().await;
    let service = guard.as_ref()?;
    Some(f(service).await)
}

/// 获取全局嵌入服务的引用（简化版，返回克隆的 Arc）
pub fn get_global_embedding_service() -> Option<&'static RwLock<Option<EmbeddingService>>> {
    GLOBAL_EMBEDDING_SERVICE.get()
}

/// 检查嵌入服务是否可用
pub fn is_embedding_available() -> bool {
    GLOBAL_EMBEDDING_SERVICE.get()
        .map(|lock| {
            // 尝试非阻塞读取
            lock.try_read().map(|guard| guard.is_some()).unwrap_or(false)
        })
        .unwrap_or(false)
}

/// 重新加载嵌入服务配置
pub async fn reload_embedding_service() -> Result<bool> {
    init_global_embedding_service().await
}

/// 使用嵌入服务计算相似度（便捷函数）
pub async fn compute_similarity(text1: &str, text2: &str) -> Option<f32> {
    let lock = match get_global_embedding_service() {
        Some(l) => l,
        None => return None,
    };
    let guard = lock.read().await;
    let service = guard.as_ref()?;
    service.similarity(text1, text2).await.ok()
}

/// 使用嵌入服务找最相似的（便捷函数）
pub async fn find_similar(query: &str, candidates: &[String], top_k: usize) -> Option<Vec<(usize, f32)>> {
    let lock = match get_global_embedding_service() {
        Some(l) => l,
        None => return None,
    };
    let guard = lock.read().await;
    let service = guard.as_ref()?;
    service.find_most_similar(query, candidates, top_k).await.ok()
}
