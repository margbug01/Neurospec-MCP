//! Ctags 集成
//!
//! 使用 Universal Ctags 进行符号提取和搜索

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

use anyhow::{Result, Context};

/// Ctags 符号
#[derive(Debug, Clone)]
pub struct CtagsSymbol {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub kind: String,
    pub signature: Option<String>,
}

/// Ctags 索引器
pub struct CtagsIndexer {
    /// 项目根目录
    project_root: PathBuf,
    /// tags 文件路径
    tags_file: PathBuf,
    /// 符号缓存
    symbols: HashMap<String, Vec<CtagsSymbol>>,
}

impl CtagsIndexer {
    /// 创建新的 ctags 索引器
    pub fn new(project_root: &Path) -> Self {
        let tags_file = project_root.join(".neurospec").join("tags");
        Self {
            project_root: project_root.to_path_buf(),
            tags_file,
            symbols: HashMap::new(),
        }
    }

    /// 检查 ctags 是否可用
    pub fn is_available() -> bool {
        // 尝试 universal-ctags 和普通 ctags
        for cmd in &["ctags", "universal-ctags", "uctags"] {
            if Command::new(cmd)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    /// 获取 ctags 命令
    fn get_ctags_cmd() -> Option<&'static str> {
        for cmd in &["ctags", "universal-ctags", "uctags"] {
            if Command::new(cmd)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(cmd);
            }
        }
        None
    }

    /// 生成 tags 文件
    pub fn generate_tags(&self) -> Result<()> {
        let cmd = Self::get_ctags_cmd()
            .context("ctags not found")?;

        // 确保目录存在
        if let Some(parent) = self.tags_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let output = Command::new(cmd)
            .current_dir(&self.project_root)
            .args([
                "-R",                           // 递归
                "--fields=+n",                  // 包含行号
                "--excmd=number",               // 使用行号而非搜索模式
                "-f", &self.tags_file.to_string_lossy(),
                "--exclude=.git",
                "--exclude=node_modules",
                "--exclude=target",
                "--exclude=dist",
                "--exclude=build",
                ".",
            ])
            .output()
            .context("Failed to run ctags")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ctags failed: {}", stderr);
        }

        Ok(())
    }

    /// 加载并解析 tags 文件
    pub fn load_tags(&mut self) -> Result<usize> {
        if !self.tags_file.exists() {
            self.generate_tags()?;
        }

        let content = fs::read_to_string(&self.tags_file)
            .context("Failed to read tags file")?;

        self.symbols.clear();
        let mut count = 0;

        for line in content.lines() {
            // 跳过注释
            if line.starts_with("!_TAG_") {
                continue;
            }

            if let Some(symbol) = self.parse_tag_line(line) {
                let name_lower = symbol.name.to_lowercase();
                self.symbols.entry(name_lower).or_default().push(symbol);
                count += 1;
            }
        }

        Ok(count)
    }

    /// 解析单行 tag
    fn parse_tag_line(&self, line: &str) -> Option<CtagsSymbol> {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() < 3 {
            return None;
        }

        let name = parts[0].to_string();
        let file = parts[1].to_string();
        
        // 解析行号（格式: "123;" 或搜索模式）
        let line_str = parts[2];
        let line_num = if let Ok(num) = line_str.trim_end_matches(';').parse::<usize>() {
            num
        } else {
            1 // 默认第一行
        };

        // 解析扩展字段（如果有）
        let mut kind = String::new();
        let mut signature = None;

        if parts.len() > 3 {
            for field in parts[3].split('\t') {
                if let Some(k) = field.strip_prefix("kind:") {
                    kind = k.to_string();
                } else if let Some(sig) = field.strip_prefix("signature:") {
                    signature = Some(sig.to_string());
                } else if field.len() == 1 {
                    // 单字符是类型缩写 (f=function, c=class, etc.)
                    kind = match field {
                        "f" => "function".to_string(),
                        "c" => "class".to_string(),
                        "m" => "method".to_string(),
                        "v" => "variable".to_string(),
                        "s" => "struct".to_string(),
                        "e" => "enum".to_string(),
                        "t" => "typedef".to_string(),
                        _ => field.to_string(),
                    };
                }
            }
        }

        Some(CtagsSymbol {
            name,
            file,
            line: line_num,
            kind,
            signature,
        })
    }

    /// 搜索符号
    pub fn search_symbol(&self, query: &str) -> Vec<&CtagsSymbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // 精确匹配优先
        if let Some(symbols) = self.symbols.get(&query_lower) {
            results.extend(symbols.iter());
        }

        // 前缀匹配
        for (name, symbols) in &self.symbols {
            if name.starts_with(&query_lower) && name != &query_lower {
                results.extend(symbols.iter());
            }
        }

        // 包含匹配
        for (name, symbols) in &self.symbols {
            if name.contains(&query_lower) && !name.starts_with(&query_lower) {
                results.extend(symbols.iter());
            }
        }

        // 限制结果数
        results.truncate(20);
        results
    }

    /// 获取符号数量
    pub fn symbol_count(&self) -> usize {
        self.symbols.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctags_available() {
        let available = CtagsIndexer::is_available();
        println!("Ctags available: {}", available);
    }

    #[test]
    fn test_parse_tag_line() {
        let indexer = CtagsIndexer::new(Path::new("/tmp"));
        
        // 标准格式
        let line = "main\tsrc/main.rs\t10;\"\tf";
        let symbol = indexer.parse_tag_line(line);
        assert!(symbol.is_some());
        let s = symbol.unwrap();
        assert_eq!(s.name, "main");
        assert_eq!(s.line, 10);
    }
}
