//! Interact 历史记录存储
//!
//! 存储 interact 工具的调用历史，支持查询

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 历史记录文件名
const HISTORY_FILE: &str = "interact_history.json";
/// 最大历史记录数
const MAX_HISTORY_SIZE: usize = 100;

/// 全局历史记录路径缓存
static HISTORY_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 单条交互记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractRecord {
    /// 唯一 ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 请求消息
    pub request_message: String,
    /// 预定义选项
    pub predefined_options: Vec<String>,
    /// 用户响应
    pub user_response: Option<String>,
    /// 选中的选项
    pub selected_options: Vec<String>,
    /// 项目路径
    pub project_path: Option<String>,
}

/// 历史记录存储
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InteractHistory {
    /// 记录列表（最新在前）
    pub records: Vec<InteractRecord>,
}

impl InteractHistory {
    /// 获取历史记录文件路径
    /// 使用应用数据目录确保路径稳定
    fn get_history_path() -> Result<PathBuf> {
        // 使用缓存的路径
        if let Some(path) = HISTORY_PATH.get() {
            return Ok(path.clone());
        }
        
        // 使用应用数据目录 (跨平台)
        let app_data = dirs::data_dir()
            .or_else(|| dirs::home_dir())
            .ok_or_else(|| anyhow::anyhow!("Cannot find data directory"))?;
        
        let history_dir = app_data.join("neurospec");
        let path = history_dir.join(HISTORY_FILE);
        
        // 缓存路径
        let _ = HISTORY_PATH.set(path.clone());
        
        log::info!("History file path: {}", path.display());
        Ok(path)
    }
    
    /// 初始化历史记录路径（应用启动时调用）
    pub fn init() -> Result<PathBuf> {
        let path = Self::get_history_path()?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // 如果文件不存在，创建空历史
        if !path.exists() {
            let empty = Self::default();
            let content = serde_json::to_string_pretty(&empty)?;
            fs::write(&path, content)?;
            log::info!("Created new history file: {}", path.display());
        } else {
            log::info!("Found existing history file: {}", path.display());
        }
        
        Ok(path)
    }

    /// 加载历史记录
    pub fn load() -> Result<Self> {
        let path = Self::get_history_path()?;
        
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&path)?;
        let history: Self = serde_json::from_str(&content)?;
        Ok(history)
    }

    /// 保存历史记录
    pub fn save(&self) -> Result<()> {
        let path = Self::get_history_path()?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// 添加新记录
    pub fn add_record(&mut self, record: InteractRecord) {
        // 插入到最前面
        self.records.insert(0, record);
        
        // 限制大小
        if self.records.len() > MAX_HISTORY_SIZE {
            self.records.truncate(MAX_HISTORY_SIZE);
        }
    }

    /// 获取最近 N 条记录
    pub fn get_recent(&self, count: usize) -> Vec<&InteractRecord> {
        self.records.iter().take(count).collect()
    }

    /// 搜索记录
    pub fn search(&self, query: &str) -> Vec<&InteractRecord> {
        let query_lower = query.to_lowercase();
        self.records
            .iter()
            .filter(|r| {
                r.request_message.to_lowercase().contains(&query_lower)
                    || r.user_response
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

/// 保存一条交互记录
pub fn save_interact_record(
    request_id: &str,
    request_message: &str,
    predefined_options: &[String],
    user_response: Option<&str>,
    selected_options: &[String],
    project_path: Option<&str>,
) -> Result<()> {
    log::debug!("Saving interact record: {}", request_id);
    
    let mut history = InteractHistory::load().unwrap_or_default();
    
    let record = InteractRecord {
        id: request_id.to_string(),
        timestamp: Utc::now(),
        request_message: request_message.to_string(),
        predefined_options: predefined_options.to_vec(),
        user_response: user_response.map(|s| s.to_string()),
        selected_options: selected_options.to_vec(),
        project_path: project_path.map(|s| s.to_string()),
    };
    
    history.add_record(record);
    
    match history.save() {
        Ok(_) => {
            log::info!("Interact record saved successfully: {}", request_id);
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to save interact record: {}", e);
            Err(e)
        }
    }
}

/// 初始化历史记录系统（应用启动时调用）
pub fn init_interact_history() -> Result<()> {
    match InteractHistory::init() {
        Ok(path) => {
            log::info!("Interact history initialized at: {}", path.display());
            
            // 加载并显示记录数
            if let Ok(history) = InteractHistory::load() {
                log::info!("Loaded {} interact records", history.records.len());
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to initialize interact history: {}", e);
            Err(e)
        }
    }
}

/// 获取交互历史记录
pub fn get_interact_history(count: Option<usize>) -> Result<Vec<InteractRecord>> {
    let history = InteractHistory::load()?;
    let limit = count.unwrap_or(20);
    Ok(history.records.into_iter().take(limit).collect())
}

/// 搜索交互历史
pub fn search_interact_history(query: &str) -> Result<Vec<InteractRecord>> {
    let history = InteractHistory::load()?;
    Ok(history.search(query).into_iter().cloned().collect())
}

/// 清空交互历史
pub fn clear_interact_history() -> Result<()> {
    let mut history = InteractHistory::load().unwrap_or_default();
    history.clear();
    history.save()
}
