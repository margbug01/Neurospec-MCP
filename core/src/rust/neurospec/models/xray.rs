use serde::{Deserialize, Serialize};

/// X-Ray: 项目结构摘要模型
/// 
/// MVP阶段可以只使用 kind="file" + path + language
/// 后续可以在Symbol中新增更多信息（类/函数等）

/// 符号类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "module")]
    Module,
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "function")]
    Function,
}

/// 符号定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// 符号类型
    pub kind: SymbolKind,
    /// 符号名称，文件则为文件名
    pub name: String,
    /// 相对项目根目录的路径（POSIX风格）
    pub path: String,
    /// 编程语言，如 'python', 'typescript'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// 可选的函数/类签名信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// 可选的引用信息列表，例如 ['src/api.py:42']
    #[serde(default)]
    pub references: Vec<String>,
}

/// X-Ray快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XRaySnapshot {
    /// 项目根目录的绝对路径或唯一标识
    pub project_root: String,
    /// 项目中的符号列表（MVP阶段以文件为主）
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    /// 扫描置信度 (0.0 ~ 1.0)
    /// 1.0 = 完全扫描，无错误
    /// <1.0 = 部分文件跳过或解析失败
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// 扫描过程中的警告信息
    #[serde(default)]
    pub warnings: Vec<String>,
    /// 跳过的文件数量
    #[serde(default)]
    pub skipped_files: usize,
    /// 解析失败的文件数量
    #[serde(default)]
    pub failed_files: usize,
}

fn default_confidence() -> f32 {
    1.0
}
