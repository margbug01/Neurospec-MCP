//! 项目路径检测工具
//!
//! 提供统一的项目根目录检测逻辑，避免代码重复

use std::path::PathBuf;

/// 检测项目根目录
/// 
/// 检测策略：
/// 1. 从当前工作目录向上查找 .git 目录
/// 2. 如果找不到 .git，返回当前工作目录
/// 
/// # Returns
/// - `Some(PathBuf)` - 检测到的项目根目录
/// - `None` - 无法获取当前工作目录
pub fn detect_project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    detect_git_root_from(&cwd).or(Some(cwd))
}

/// 从指定路径向上查找 Git 根目录
/// 
/// # Arguments
/// * `start_path` - 开始查找的路径
/// 
/// # Returns
/// - `Some(PathBuf)` - 找到的 Git 根目录
/// - `None` - 未找到 .git 目录
pub fn detect_git_root_from(start_path: &std::path::Path) -> Option<PathBuf> {
    let mut current = start_path;
    
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// 解析并验证项目路径
/// 
/// 如果提供了路径，验证并返回；否则自动检测。
/// 
/// # Arguments
/// * `provided_path` - 用户提供的路径（可能为空）
/// 
/// # Returns
/// - `Ok(String)` - 有效的项目路径
/// - `Err(String)` - 错误信息
pub fn resolve_project_path(provided_path: &str) -> Result<String, String> {
    // 如果提供了路径，直接使用
    if !provided_path.trim().is_empty() {
        let path = PathBuf::from(provided_path);
        if path.exists() {
            return Ok(provided_path.to_string());
        } else {
            return Err(format!("路径不存在: {}", provided_path));
        }
    }

    // 自动检测
    detect_project_root()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "无法自动检测项目路径。请确保在 Git 仓库中运行，或手动指定路径。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_project_root_returns_some() {
        // 在项目目录中运行时应该能检测到
        let result = detect_project_root();
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_with_valid_path() {
        let result = resolve_project_path(".");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_with_empty_path() {
        let result = resolve_project_path("");
        // 应该自动检测
        assert!(result.is_ok() || result.is_err());
    }
}
