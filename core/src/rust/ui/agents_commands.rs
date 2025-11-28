//! AGENTS.md 相关的 Tauri 命令

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

#[cfg(feature = "experimental-neurospec")]
use crate::neurospec::services::agents_parser::AgentsConfig;

/// 全局项目路径缓存
static PROJECT_PATH_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn get_cache() -> &'static Mutex<Option<String>> {
    PROJECT_PATH_CACHE.get_or_init(|| Mutex::new(None))
}

/// 更新项目路径缓存（供 MCP 调用时使用）
pub fn update_project_path_cache(path: &str) {
    let path_buf = PathBuf::from(path);
    if path_buf.exists() {
        if let Some(root) = find_git_root(&path_buf) {
            let root_str = root.to_string_lossy().to_string();
            if let Ok(mut cache) = get_cache().lock() {
                *cache = Some(root_str.clone());
            }
            // 同时保存到配置文件
            let _ = save_project_path_config(&root_str);
            log::info!("项目路径缓存已更新: {}", root_str);
        }
    }
}

/// 检测项目响应
#[derive(Serialize)]
pub struct DetectProjectResponse {
    pub path: String,
    pub has_agents: bool,
}

/// 索引状态响应
#[derive(Serialize)]
pub struct IndexStatusResponse {
    pub ready: bool,
    pub file_count: usize,
    pub building: bool,
    pub project_path: Option<String>,
}

/// 设置项目路径
#[tauri::command]
pub async fn set_project_path(path: String) -> Result<DetectProjectResponse, String> {
    let path_buf = PathBuf::from(&path);
    
    // 验证路径存在
    if !path_buf.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    
    // 查找 Git 根目录
    let root = find_git_root(&path_buf).ok_or_else(|| {
        format!("路径不是 Git 仓库: {}", path)
    })?;
    
    let root_str = root.to_string_lossy().to_string();
    
    // 缓存路径
    {
        let mut cache = get_cache().lock().unwrap();
        *cache = Some(root_str.clone());
    }
    
    // 保存到配置文件
    save_project_path_config(&root_str)?;
    
    let has_agents = root.join("AGENTS.md").exists();
    
    log::info!("项目路径已设置: {}", root_str);
    
    Ok(DetectProjectResponse {
        path: root_str,
        has_agents,
    })
}

/// 检测项目是否有 AGENTS.md
#[tauri::command]
pub async fn detect_project_agents() -> Result<DetectProjectResponse, String> {
    // 优先使用缓存的路径
    let cached = {
        let cache = get_cache().lock().unwrap();
        cache.clone()
    };
    
    if let Some(path) = cached {
        let root = PathBuf::from(&path);
        if root.exists() {
            let has_agents = root.join("AGENTS.md").exists();
            return Ok(DetectProjectResponse {
                path,
                has_agents,
            });
        }
    }
    
    // 尝试从配置文件加载
    if let Some(saved_path) = load_project_path_config() {
        let root = PathBuf::from(&saved_path);
        if root.exists() {
            // 更新缓存
            {
                let mut cache = get_cache().lock().unwrap();
                *cache = Some(saved_path.clone());
            }
            
            let has_agents = root.join("AGENTS.md").exists();
            return Ok(DetectProjectResponse {
                path: saved_path,
                has_agents,
            });
        }
    }
    
    // 回退：尝试从当前工作目录检测
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(root) = find_git_root(&cwd) {
            let path = root.to_string_lossy().to_string();
            let has_agents = root.join("AGENTS.md").exists();
            return Ok(DetectProjectResponse {
                path,
                has_agents,
            });
        }
    }
    
    // 无法检测到项目
    Ok(DetectProjectResponse {
        path: String::new(),
        has_agents: false,
    })
}

/// 获取索引状态
#[tauri::command]
pub async fn get_index_status() -> Result<IndexStatusResponse, String> {
    use crate::mcp::tools::unified_store::{is_search_initialized, is_project_indexed, is_project_indexing, get_indexed_file_count};
    
    // 获取当前项目路径
    let project_path = {
        let cache = get_cache().lock().unwrap();
        cache.clone()
    }.or_else(|| load_project_path_config());
    
    if let Some(ref path) = project_path {
        let path_buf = PathBuf::from(path);
        
        if is_search_initialized() {
            let ready = is_project_indexed(&path_buf);
            let building = is_project_indexing(&path_buf);
            let file_count = get_indexed_file_count(&path_buf).unwrap_or(0);
            
            return Ok(IndexStatusResponse {
                ready,
                file_count,
                building,
                project_path: Some(path.clone()),
            });
        }
    }
    
    Ok(IndexStatusResponse {
        ready: false,
        file_count: 0,
        building: false,
        project_path,
    })
}

/// 配置文件路径
fn get_config_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("neurospec").join("project_config.json"))
}

/// 项目配置
#[derive(Serialize, Deserialize, Default)]
struct ProjectConfig {
    project_path: Option<String>,
}

/// 保存项目路径到配置
fn save_project_path_config(path: &str) -> Result<(), String> {
    let config_path = get_config_path().ok_or("无法获取配置目录")?;
    
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
    }
    
    let config = ProjectConfig {
        project_path: Some(path.to_string()),
    };
    
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    
    std::fs::write(&config_path, content)
        .map_err(|e| format!("保存配置失败: {}", e))?;
    
    Ok(())
}

/// 从配置加载项目路径
fn load_project_path_config() -> Option<String> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        return None;
    }
    
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: ProjectConfig = serde_json::from_str(&content).ok()?;
    
    config.project_path
}

/// 加载 AGENTS.md 配置
#[tauri::command]
#[cfg(feature = "experimental-neurospec")]
pub async fn load_agents_config(path: String) -> Result<AgentsConfig, String> {
    let project_path = PathBuf::from(&path);
    let agents_path = project_path.join("AGENTS.md");
    
    if !agents_path.exists() {
        // 返回默认配置
        return Ok(AgentsConfig::default_config());
    }
    
    AgentsConfig::load_from_file(&agents_path)
        .map_err(|e| format!("加载配置失败: {}", e))
}

/// 加载 AGENTS.md 配置（非 neurospec 版本）
#[tauri::command]
#[cfg(not(feature = "experimental-neurospec"))]
pub async fn load_agents_config(_path: String) -> Result<serde_json::Value, String> {
    Err("需要启用 experimental-neurospec 特性".to_string())
}

/// 保存 AGENTS.md 配置
#[tauri::command]
#[cfg(feature = "experimental-neurospec")]
pub async fn save_agents_config(path: String, config: AgentsConfig) -> Result<(), String> {
    let project_path = PathBuf::from(&path);
    let agents_path = project_path.join("AGENTS.md");
    
    config.save_to_file(&agents_path)
        .map_err(|e| format!("保存配置失败: {}", e))
}

/// 保存 AGENTS.md 配置（非 neurospec 版本）
#[tauri::command]
#[cfg(not(feature = "experimental-neurospec"))]
pub async fn save_agents_config(_path: String, _config: serde_json::Value) -> Result<(), String> {
    Err("需要启用 experimental-neurospec 特性".to_string())
}

/// 查找 Git 根目录
fn find_git_root(start: &PathBuf) -> Option<PathBuf> {
    let mut current = start.as_path();
    
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        
        current = current.parent()?;
    }
}
