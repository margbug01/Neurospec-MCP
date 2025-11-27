//! Ripgrep 回退搜索
//!
//! 当 Tantivy 索引未就绪时，使用 ripgrep 进行即时搜索

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::io::Read;

/// Ripgrep 搜索超时（秒）
const RIPGREP_TIMEOUT_SECS: u64 = 5;

use anyhow::{Result, Context};

use super::types::SearchResult;

/// Ripgrep 搜索器
pub struct RipgrepSearcher {
    /// 最大结果数
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

    /// 执行 ripgrep 搜索（带超时）
    pub fn search(&self, project_root: &Path, query: &str) -> Result<Vec<SearchResult>> {
        // 检查 ripgrep 是否可用
        let rg_cmd = if cfg!(windows) { "rg.exe" } else { "rg" };
        
        let mut child = Command::new(rg_cmd)
            .current_dir(project_root)
            .args([
                "--json",                           // JSON 输出
                "--max-count", &self.max_results.to_string(),
                "-C", &self.context_lines.to_string(),  // 上下文
                "--type-add", "code:*.{rs,ts,tsx,js,jsx,py,go,java,c,cpp,h,hpp,vue,svelte}",
                "--type", "code",                   // 仅搜索代码文件
                "--ignore-case",                    // 忽略大小写
                query,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn ripgrep. Is 'rg' installed?")?;

        // 带超时等待
        let timeout = Duration::from_secs(RIPGREP_TIMEOUT_SECS);
        let start = std::time::Instant::now();
        
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // 进程已结束，读取输出
                    let mut stdout = Vec::new();
                    if let Some(mut pipe) = child.stdout.take() {
                        let _ = pipe.read_to_end(&mut stdout);
                    }
                    return self.parse_rg_json(&stdout, project_root);
                }
                Ok(None) => {
                    // 进程仍在运行
                    if start.elapsed() > timeout {
                        // 超时，终止进程
                        let _ = child.kill();
                        crate::log_important!(warn, "Ripgrep search timed out after {}s", RIPGREP_TIMEOUT_SECS);
                        return Ok(Vec::new());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to wait for ripgrep: {}", e));
                }
            }
        }
    }

    /// 解析 ripgrep JSON 输出
    fn parse_rg_json(&self, output: &[u8], _project_root: &Path) -> Result<Vec<SearchResult>> {
        let output_str = String::from_utf8_lossy(output);
        let mut results: Vec<SearchResult> = Vec::new();
        let mut current_file: Option<String> = None;
        let mut current_lines: Vec<String> = Vec::new();
        let mut match_line: Option<usize> = None;

        for line in output_str.lines() {
            if line.is_empty() {
                continue;
            }

            // 解析 JSON 行
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                match json.get("type").and_then(|t| t.as_str()) {
                    Some("begin") => {
                        // 新文件开始
                        if let Some(path) = json.get("data")
                            .and_then(|d| d.get("path"))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            // 保存上一个文件的结果
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
                                }
                            }
                            current_file = Some(path.to_string());
                            current_lines.clear();
                            match_line = None;
                        }
                    }
                    Some("match") => {
                        // 匹配行
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
                        // 上下文行
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
                        // 文件结束
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
            if !current_lines.is_empty() {
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

        // 限制结果数
        results.truncate(self.max_results);
        
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripgrep_available() {
        // 仅检查 ripgrep 是否可用，不强制要求
        let available = RipgrepSearcher::is_available();
        println!("Ripgrep available: {}", available);
    }
}
