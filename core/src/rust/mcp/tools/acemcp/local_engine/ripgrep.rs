//! Ripgrep 回退搜索
//!
//! 当 Tantivy 索引未就绪时，使用 ripgrep 进行即时搜索

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::io::{BufRead, BufReader};

/// Ripgrep 搜索超时（秒）
const RIPGREP_TIMEOUT_SECS: u64 = 5;

use anyhow::{Result, Context};

use super::types::SearchResult;

/// Ripgrep 搜索器
pub struct RipgrepSearcher {
    /// 最大结果数（文件数）
    max_results: usize,
    /// 上下文行数
    context_lines: usize,
}

impl RipgrepSearcher {
    pub fn new(max_results: usize, context_lines: usize) -> Self {
        Self {
            max_results,
            context_lines,
        }
    }

    /// 执行 ripgrep 搜索（带超时和流式结果限制）
    pub fn search(&self, project_root: &Path, query: &str) -> Result<Vec<SearchResult>> {
        let rg_cmd = if cfg!(windows) { "rg.exe" } else { "rg" };
        
        let mut child = Command::new(rg_cmd)
            .current_dir(project_root)
            .args([
                "--json",
                "-C", &self.context_lines.to_string(),
                "--type-add", "code:*.{rs,ts,tsx,js,jsx,py,go,java,c,cpp,h,hpp,vue,svelte}",
                "--type", "code",
                "--ignore-case",
                query,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn ripgrep. Is 'rg' installed?")?;

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
        
        let reader = BufReader::new(stdout);
        let timeout = Duration::from_secs(RIPGREP_TIMEOUT_SECS);
        let start = std::time::Instant::now();
        
        let mut results: Vec<SearchResult> = Vec::new();
        let mut current_file: Option<String> = None;
        let mut current_lines: Vec<String> = Vec::new();
        let mut match_line: Option<usize> = None;
        let mut file_count = 0;
        
        for line_result in reader.lines() {
            // 检查超时
            if start.elapsed() > timeout {
                crate::log_important!(warn, "Ripgrep search timed out after {}s", RIPGREP_TIMEOUT_SECS);
                let _ = child.kill();
                break;
            }
            
            // 检查是否已达到最大结果数
            if file_count >= self.max_results {
                let _ = child.kill();
                break;
            }
            
            let line = match line_result {
                Ok(l) => l,
                Err(_) => continue,
            };
            
            if line.is_empty() {
                continue;
            }
            
            // 解析 JSON 行
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                match json.get("type").and_then(|t| t.as_str()) {
                    Some("begin") => {
                        // 新文件开始 - 保存上一个文件的结果
                        if let Some(file) = current_file.take() {
                            if !current_lines.is_empty() {
                                results.push(SearchResult {
                                    path: file,
                                    score: 1.0,
                                    snippet: current_lines.join("\n"),
                                    line_number: match_line.unwrap_or(1),
                                    context: None,
                                    match_info: None,
                                });
                                file_count += 1;
                            }
                        }
                        
                        if let Some(path) = json.get("data")
                            .and_then(|d| d.get("path"))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            current_file = Some(path.to_string());
                            current_lines.clear();
                            match_line = None;
                        }
                    }
                    Some("match") => {
                        if let Some(data) = json.get("data") {
                            if let Some(line_num) = data.get("line_number").and_then(|n| n.as_u64()) {
                                if match_line.is_none() {
                                    match_line = Some(line_num as usize);
                                }
                            }
                            if let Some(text) = data.get("lines")
                                .and_then(|l| l.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                let line_num = data.get("line_number")
                                    .and_then(|n| n.as_u64())
                                    .unwrap_or(0);
                                current_lines.push(format!("> {:4} | {}", line_num, text.trim_end()));
                            }
                        }
                    }
                    Some("context") => {
                        if let Some(data) = json.get("data") {
                            if let Some(text) = data.get("lines")
                                .and_then(|l| l.get("text"))
                                .and_then(|t| t.as_str())
                            {
                                let line_num = data.get("line_number")
                                    .and_then(|n| n.as_u64())
                                    .unwrap_or(0);
                                current_lines.push(format!("  {:4} | {}", line_num, text.trim_end()));
                            }
                        }
                    }
                    Some("end") => {
                        if let Some(file) = current_file.take() {
                            if !current_lines.is_empty() {
                                results.push(SearchResult {
                                    path: file,
                                    score: 1.0,
                                    snippet: current_lines.join("\n"),
                                    line_number: match_line.unwrap_or(1),
                                    context: None,
                                    match_info: None,
                                });
                                file_count += 1;
                            }
                        }
                        current_lines.clear();
                        match_line = None;
                    }
                    _ => {}
                }
            }
        }

        // 处理最后一个文件
        if let Some(file) = current_file {
            if !current_lines.is_empty() && file_count < self.max_results {
                results.push(SearchResult {
                    path: file,
                    score: 1.0,
                    snippet: current_lines.join("\n"),
                    line_number: match_line.unwrap_or(1),
                    context: None,
                    match_info: None,
                });
            }
        }

        // 等待子进程结束（已经被 kill 或自然结束）
        let _ = child.wait();
        
        Ok(results)
    }

    /// 检查 ripgrep 是否可用
    pub fn is_available() -> bool {
        let rg_cmd = if cfg!(windows) { "rg.exe" } else { "rg" };
        Command::new(rg_cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
