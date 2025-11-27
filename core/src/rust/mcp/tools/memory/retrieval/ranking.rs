//! 记忆排序算法
//!
//! 综合考虑相关性、时效性、使用频率和分类权重

use chrono::{DateTime, Utc};

use super::tfidf::TfIdfEngine;
use crate::mcp::tools::memory::types::{MemoryEntry, MemoryCategory};
use crate::mcp::tools::memory::storage::traits::MemoryUsageStat;

/// 带分数的记忆条目
#[derive(Debug, Clone)]
pub struct ScoredMemory {
    pub memory: MemoryEntry,
    pub score: f64,
    pub relevance_score: f64,
    pub recency_score: f64,
    pub frequency_score: f64,
    pub category_score: f64,
}

/// 排序配置
#[derive(Debug, Clone)]
pub struct RankingConfig {
    /// 相关性权重
    pub relevance_weight: f64,
    /// 时效性权重
    pub recency_weight: f64,
    /// 使用频率权重
    pub frequency_weight: f64,
    /// 分类权重
    pub category_weight: f64,
    /// 最小相关性阈值（低于此值的记忆将被过滤）
    pub min_relevance: f64,
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            relevance_weight: 0.4,
            recency_weight: 0.3,
            frequency_weight: 0.2,
            category_weight: 0.1,
            min_relevance: 0.1,
        }
    }
}

/// 记忆排序器
pub struct MemoryRanker {
    tfidf: TfIdfEngine,
    config: RankingConfig,
}

impl MemoryRanker {
    pub fn new() -> Self {
        Self {
            tfidf: TfIdfEngine::new(),
            config: RankingConfig::default(),
        }
    }

    pub fn with_config(config: RankingConfig) -> Self {
        Self {
            tfidf: TfIdfEngine::new(),
            config,
        }
    }

    /// 从记忆列表构建索引
    pub fn build_index(&mut self, memories: &[MemoryEntry]) {
        let documents: Vec<String> = memories.iter()
            .map(|m| m.content.clone())
            .collect();
        self.tfidf.build_from_documents(&documents);
    }

    /// 对记忆进行排序
    pub fn rank(
        &self,
        query: &str,
        memories: &[MemoryEntry],
        usage_stats: &[(String, MemoryUsageStat)],
        limit: usize,
    ) -> Vec<ScoredMemory> {
        let now = Utc::now();
        let max_usage = usage_stats.iter()
            .map(|(_, s)| s.usage_count)
            .max()
            .unwrap_or(1) as f64;

        let stats_map: std::collections::HashMap<_, _> = usage_stats.iter()
            .map(|(id, stat)| (id.clone(), stat.clone()))
            .collect();

        let mut scored: Vec<ScoredMemory> = memories.iter()
            .map(|memory| {
                let relevance_score = self.compute_relevance(query, &memory.content);
                let recency_score = self.compute_recency(&memory.updated_at, &now);
                let frequency_score = self.compute_frequency(
                    stats_map.get(&memory.id),
                    max_usage,
                );
                let category_score = self.compute_category_weight(&memory.category);

                let score = 
                    self.config.relevance_weight * relevance_score +
                    self.config.recency_weight * recency_score +
                    self.config.frequency_weight * frequency_score +
                    self.config.category_weight * category_score;

                ScoredMemory {
                    memory: memory.clone(),
                    score,
                    relevance_score,
                    recency_score,
                    frequency_score,
                    category_score,
                }
            })
            .filter(|sm| sm.relevance_score >= self.config.min_relevance || query.is_empty())
            .collect();

        // 按分数降序排序
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // 限制返回数量
        scored.truncate(limit);
        scored
    }

    /// 计算相关性分数 (TF-IDF 余弦相似度)
    fn compute_relevance(&self, query: &str, content: &str) -> f64 {
        if query.is_empty() {
            return 1.0; // 无查询时，所有记忆相关性相同
        }
        self.tfidf.similarity(query, content)
    }

    /// 计算时效性分数
    fn compute_recency(&self, updated_at: &DateTime<Utc>, now: &DateTime<Utc>) -> f64 {
        let days = (*now - *updated_at).num_days() as f64;
        1.0 / (1.0 + days.max(0.0) / 30.0) // 30天衰减
    }

    /// 计算使用频率分数
    fn compute_frequency(&self, stat: Option<&MemoryUsageStat>, max_usage: f64) -> f64 {
        match stat {
            Some(s) => {
                let usage: f64 = s.usage_count as f64;
                if max_usage > 0.0 {
                    (1.0_f64 + usage).ln() / (1.0_f64 + max_usage).ln()
                } else {
                    0.0
                }
            }
            None => 0.0,
        }
    }

    /// 计算分类权重
    fn compute_category_weight(&self, category: &MemoryCategory) -> f64 {
        match category {
            MemoryCategory::Rule => 1.0,
            MemoryCategory::Pattern => 0.8,
            MemoryCategory::Preference => 0.6,
            MemoryCategory::Context => 0.4,
        }
    }
}

impl Default for MemoryRanker {
    fn default() -> Self {
        Self::new()
    }
}
