//! TF-IDF 文本相似度计算
//!
//! 轻量级本地实现，无需外部依赖

use std::collections::{HashMap, HashSet};

/// TF-IDF 引擎
pub struct TfIdfEngine {
    /// 文档频率 (DF): 每个词出现在多少文档中
    document_freq: HashMap<String, usize>,
    /// 总文档数
    total_docs: usize,
    /// 停用词
    stop_words: HashSet<String>,
}

impl TfIdfEngine {
    pub fn new() -> Self {
        let stop_words = Self::default_stop_words();
        Self {
            document_freq: HashMap::new(),
            total_docs: 0,
            stop_words,
        }
    }

    /// 默认停用词（中英文混合）
    fn default_stop_words() -> HashSet<String> {
        let words = [
            // 英文停用词
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "shall", "can", "need", "dare",
            "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
            "into", "through", "during", "before", "after", "above", "below",
            "between", "under", "again", "further", "then", "once", "here",
            "there", "when", "where", "why", "how", "all", "each", "few",
            "more", "most", "other", "some", "such", "no", "nor", "not",
            "only", "own", "same", "so", "than", "too", "very", "just",
            "and", "but", "if", "or", "because", "until", "while", "this",
            "that", "these", "those", "it", "its", "i", "you", "he", "she",
            "we", "they", "what", "which", "who", "whom",
            // 中文停用词
            "的", "了", "和", "是", "就", "都", "而", "及", "与", "着",
            "或", "一个", "没有", "我们", "你们", "他们", "它们", "这个",
            "那个", "这些", "那些", "什么", "怎么", "如何", "为什么",
            "可以", "需要", "使用", "进行", "通过", "根据", "按照",
        ];
        words.iter().map(|s| s.to_string()).collect()
    }

    /// 分词（简单实现：按空格和标点分割）
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let text_lower = text.to_lowercase();
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for ch in text_lower.chars() {
            if ch.is_alphanumeric() || ch > '\u{4E00}' && ch < '\u{9FFF}' {
                // 字母数字或中文字符
                current_token.push(ch);
            } else {
                if !current_token.is_empty() {
                    if !self.stop_words.contains(&current_token) && current_token.len() > 1 {
                        tokens.push(current_token.clone());
                    }
                    current_token.clear();
                }
            }
        }

        if !current_token.is_empty() 
            && !self.stop_words.contains(&current_token) 
            && current_token.len() > 1 
        {
            tokens.push(current_token);
        }

        tokens
    }

    /// 计算词频 (TF)
    fn term_frequency(tokens: &[String]) -> HashMap<String, f64> {
        let mut tf = HashMap::new();
        let total = tokens.len() as f64;

        for token in tokens {
            *tf.entry(token.clone()).or_insert(0.0) += 1.0;
        }

        // 归一化
        for value in tf.values_mut() {
            *value /= total.max(1.0);
        }

        tf
    }

    /// 从文档集合构建 IDF
    pub fn build_from_documents(&mut self, documents: &[String]) {
        self.document_freq.clear();
        self.total_docs = documents.len();

        for doc in documents {
            let tokens: HashSet<_> = self.tokenize(doc).into_iter().collect();
            for token in tokens {
                *self.document_freq.entry(token).or_insert(0) += 1;
            }
        }
    }

    /// 计算逆文档频率 (IDF)
    fn inverse_document_frequency(&self, term: &str) -> f64 {
        let df = self.document_freq.get(term).copied().unwrap_or(0) as f64;
        let n = self.total_docs as f64;
        
        if df == 0.0 {
            0.0
        } else {
            (n / df).ln() + 1.0
        }
    }

    /// 计算文档的 TF-IDF 向量
    pub fn compute_tfidf(&self, text: &str) -> HashMap<String, f64> {
        let tokens = self.tokenize(text);
        let tf = Self::term_frequency(&tokens);
        
        let mut tfidf = HashMap::new();
        for (term, tf_value) in tf {
            let idf = self.inverse_document_frequency(&term);
            tfidf.insert(term, tf_value * idf);
        }

        tfidf
    }

    /// 计算两个 TF-IDF 向量的余弦相似度
    pub fn cosine_similarity(
        vec1: &HashMap<String, f64>,
        vec2: &HashMap<String, f64>,
    ) -> f64 {
        let mut dot_product = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;

        // 计算点积和 vec1 的范数
        for (term, value1) in vec1 {
            norm1 += value1 * value1;
            if let Some(value2) = vec2.get(term) {
                dot_product += value1 * value2;
            }
        }

        // 计算 vec2 的范数
        for value2 in vec2.values() {
            norm2 += value2 * value2;
        }

        let norm1 = norm1.sqrt();
        let norm2 = norm2.sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            0.0
        } else {
            dot_product / (norm1 * norm2)
        }
    }

    /// 计算查询与文档的相似度
    pub fn similarity(&self, query: &str, document: &str) -> f64 {
        let query_vec = self.compute_tfidf(query);
        let doc_vec = self.compute_tfidf(document);
        Self::cosine_similarity(&query_vec, &doc_vec)
    }
}

impl Default for TfIdfEngine {
    fn default() -> Self {
        Self::new()
    }
}
