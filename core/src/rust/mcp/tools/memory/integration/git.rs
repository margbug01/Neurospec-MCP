//! Git 集成
//!
//! 从 Git commit message 提取项目规范

use std::path::Path;
use std::process::Command;
use anyhow::Result;

use serde::{Deserialize, Serialize};

use crate::mcp::tools::memory::types::MemoryCategory;

/// Git 集成
pub struct GitIntegration {
    project_path: String,
}

/// Commit 类型
#[derive(Debug, Clone)]
pub enum CommitType {
    Feature,
    Fix,
    Docs,
    Style,
    Refactor,
    Test,
    Chore,
    Unknown,
}

/// Git 提取的建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSuggestion {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub confidence: f32,
    pub reason: String,
}

impl GitIntegration {
    pub fn new(project_path: &str) -> Self {
        Self {
            project_path: project_path.to_string(),
        }
    }

    /// 获取最近的 commit messages
    pub fn get_recent_commits(&self, limit: usize) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["log", "--oneline", "-n", &limit.to_string()])
            .current_dir(&self.project_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Git command failed"));
        }

        let commits = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(commits)
    }

    /// 解析 commit 类型
    fn parse_commit_type(message: &str) -> CommitType {
        let lower = message.to_lowercase();
        if lower.starts_with("feat") { CommitType::Feature }
        else if lower.starts_with("fix") { CommitType::Fix }
        else if lower.starts_with("docs") { CommitType::Docs }
        else if lower.starts_with("style") { CommitType::Style }
        else if lower.starts_with("refactor") { CommitType::Refactor }
        else if lower.starts_with("test") { CommitType::Test }
        else if lower.starts_with("chore") { CommitType::Chore }
        else { CommitType::Unknown }
    }

    /// 从 commit messages 提取记忆建议
    pub fn extract_suggestions(&self, limit: usize) -> Result<Vec<GitSuggestion>> {
        let commits = self.get_recent_commits(limit)?;
        let mut suggestions = Vec::new();
        let mut patterns: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

        for commit in &commits {
            let commit_type = Self::parse_commit_type(commit);
            let type_key = format!("{:?}", commit_type);
            *patterns.entry(type_key).or_insert(0) += 1;

            if let Some(suggestion) = self.extract_from_commit(commit, &commit_type) {
                suggestions.push(suggestion);
            }
        }

        for (pattern, count) in patterns {
            if count >= 5 && pattern != "Unknown" {
                suggestions.push(GitSuggestion {
                    id: format!("git_pattern_{}", pattern.to_lowercase()),
                    content: format!("项目使用 {} 类型的 commit 规范", pattern),
                    category: MemoryCategory::Rule,
                    confidence: 0.6 + (count as f32 / 20.0).min(0.3),
                    reason: format!("检测到 {} 条 {} 类型的 commit", count, pattern),
                });
            }
        }

        Ok(suggestions)
    }

    fn extract_from_commit(&self, commit: &str, commit_type: &CommitType) -> Option<GitSuggestion> {
        if let Some(start) = commit.find('(') {
            if let Some(end) = commit.find(')') {
                let scope = &commit[start + 1..end];
                if !scope.is_empty() && scope.len() < 30 {
                    return Some(GitSuggestion {
                        id: format!("git_scope_{}", scope.to_lowercase().replace(' ', "_")),
                        content: format!("项目模块: {}", scope),
                        category: MemoryCategory::Context,
                        confidence: 0.5,
                        reason: format!("从 commit scope 提取: {:?}", commit_type),
                    });
                }
            }
        }
        None
    }

    pub fn is_git_repo(path: &str) -> bool {
        Path::new(path).join(".git").exists()
    }
}
