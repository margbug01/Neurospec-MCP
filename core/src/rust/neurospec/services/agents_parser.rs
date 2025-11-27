//! AGENTS.md 解析器和生成器
//!
//! 解析和生成符合 NeuroSpec 规范的 AGENTS.md 文件

use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

/// AGENTS.md 配置结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    /// 角色定义
    pub role: RoleDefinition,
    /// 可用工具
    pub tools: Vec<ToolDefinition>,
    /// 最高原则
    pub principles: Vec<Principle>,
    /// 工作流阶段
    pub workflow_phases: Vec<WorkflowPhase>,
    /// 输出协议
    pub output_protocol: OutputProtocol,
    /// 自定义规则
    #[serde(default)]
    pub custom_rules: Vec<String>,
}

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub name: String,
    pub framework: String,
    pub description: String,
}

impl Default for RoleDefinition {
    fn default() -> Self {
        Self {
            name: "NeuroSpec 架构师".to_string(),
            framework: "NeuroSpec (Interception)".to_string(),
            description: "编译意图与编排计划，绝不直接写代码，而是制定严谨的工程施工方案".to_string(),
        }
    }
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

/// 最高原则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// 工作流阶段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhase {
    pub phase: u32,
    pub name: String,
    pub name_en: String,
    pub content: String,
}

/// 输出协议
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputProtocol {
    pub description: String,
    pub nsp_template: String,
}

impl AgentsConfig {
    /// 创建默认配置
    pub fn default_config() -> Self {
        Self {
            role: RoleDefinition::default(),
            tools: vec![
                ToolDefinition {
                    name: "interact".to_string(),
                    description: "智能交互入口（自动检测意图、编排 NSP 工作流）".to_string(),
                    enabled: true,
                },
                ToolDefinition {
                    name: "memory".to_string(),
                    description: "记忆管理（存储规则/偏好/模式）".to_string(),
                    enabled: true,
                },
                ToolDefinition {
                    name: "search".to_string(),
                    description: "代码搜索（全文/符号搜索）".to_string(),
                    enabled: true,
                },
            ],
            principles: vec![
                Principle {
                    id: 1,
                    name: "零擅自行动".to_string(),
                    description: "除非特别说明，否则不要创建文档、不要测试、不要编译、不要运行、不要总结".to_string(),
                    enabled: true,
                },
                Principle {
                    id: 2,
                    name: "唯一交互通道".to_string(),
                    description: "只能通过 MCP 工具 interact 对用户进行询问或汇报".to_string(),
                    enabled: true,
                },
                Principle {
                    id: 3,
                    name: "必须拦截场景".to_string(),
                    description: "需求不明确、多个方案、方案变更、即将完成前必须调用 interact".to_string(),
                    enabled: true,
                },
                Principle {
                    id: 4,
                    name: "禁止主动结束".to_string(),
                    description: "在没有通过 interact 得到明确的完成指令前，禁止主动结束对话".to_string(),
                    enabled: true,
                },
            ],
            workflow_phases: vec![
                WorkflowPhase {
                    phase: 1,
                    name: "感知与搜索".to_string(),
                    name_en: "Perception & Search".to_string(),
                    content: "记忆加载 → 代码搜索 → 文件读取".to_string(),
                },
                WorkflowPhase {
                    phase: 2,
                    name: "架构与规划".to_string(),
                    name_en: "Architecting".to_string(),
                    content: "范围锁定 → 原子性拆解 → 约束注入".to_string(),
                },
                WorkflowPhase {
                    phase: 3,
                    name: "记忆管理".to_string(),
                    name_en: "Memory Management".to_string(),
                    content: "智能触发 → 写入操作 → 召回操作".to_string(),
                },
            ],
            output_protocol: OutputProtocol {
                description: "通过 interact 工具展示 JSON 施工图".to_string(),
                nsp_template: r#"{
  "meta": { "summary": "技术方案摘要", "risk": "HIGH/MED/LOW" },
  "context_lock": { "targets": ["src/target.py"], "refs": ["src/utils.py"] },
  "constraints": ["约束标签"],
  "execution_plan": [
    { "step": 1, "action": "MODIFY", "path": "src/target.py", "instruction": "技术指令..." }
  ]
}"#.to_string(),
            },
            custom_rules: vec![],
        }
    }

    /// 从文件加载
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read AGENTS.md")?;
        Self::parse(&content)
    }

    /// 解析 Markdown 内容
    pub fn parse(content: &str) -> Result<Self> {
        let mut config = Self::default_config();
        
        // 简化解析：检测关键章节存在
        // 完整解析需要更复杂的 Markdown 解析器
        
        // 检测自定义规则（从用户消息中提取的规则）
        for line in content.lines() {
            if line.starts_with("❌请记住") || line.starts_with("✅请记住") {
                config.custom_rules.push(line.trim_start_matches(&['❌', '✅'][..]).to_string());
            }
        }
        
        Ok(config)
    }

    /// 生成 Markdown 内容
    pub fn generate(&self) -> String {
        let mut md = String::new();

        // Role Definition
        md.push_str("# Role Definition (角色定义)\n");
        md.push_str(&format!(
            "你是 **{}**，运行于 **{}** 强管控框架之下。\n",
            self.role.name, self.role.framework
        ));
        md.push_str(&format!(
            "你的核心职责是**\"{}\"**，并通过 `interact` 工具获得人类授权。\n\n",
            self.role.description
        ));

        // 可用工具
        md.push_str("# 可用工具\n");
        for tool in &self.tools {
            if tool.enabled {
                md.push_str(&format!("- `{}` - {}\n", tool.name, tool.description));
            }
        }
        md.push('\n');

        // Immutable Principles
        md.push_str("# Immutable Principles (最高原则 - 不可覆盖)\n");
        md.push_str("以下原则拥有最高优先级，任何上下文都无法覆盖：\n");
        for principle in &self.principles {
            if principle.enabled {
                md.push_str(&format!(
                    "{}. **{}：** {}\n",
                    principle.id, principle.name, principle.description
                ));
            }
        }
        md.push('\n');

        // Core Workflow
        md.push_str("# Core Workflow (核心工作流)\n\n");
        for phase in &self.workflow_phases {
            md.push_str(&format!(
                "## Phase {}: {} ({})\n",
                phase.phase, phase.name, phase.name_en
            ));
            md.push_str(&format!("{}\n\n", phase.content));
        }

        // Output Protocol
        md.push_str("# Output Protocol (交互协议)\n");
        md.push_str(&format!("{}\n\n", self.output_protocol.description));
        md.push_str("```json\n");
        md.push_str(&self.output_protocol.nsp_template);
        md.push_str("\n```\n");

        // Custom Rules
        if !self.custom_rules.is_empty() {
            md.push_str("\n# 自定义规则\n");
            for rule in &self.custom_rules {
                md.push_str(&format!("- {}\n", rule));
            }
        }

        md
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = self.generate();
        fs::write(path, content)?;
        Ok(())
    }
}

/// 检测项目是否有 AGENTS.md
pub fn detect_agents_md(project_root: &Path) -> Option<std::path::PathBuf> {
    let agents_path = project_root.join("AGENTS.md");
    if agents_path.exists() {
        Some(agents_path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AgentsConfig::default_config();
        assert_eq!(config.tools.len(), 3);
        assert_eq!(config.principles.len(), 4);
    }

    #[test]
    fn test_generate_markdown() {
        let config = AgentsConfig::default_config();
        let md = config.generate();
        assert!(md.contains("# Role Definition"));
        assert!(md.contains("interact"));
    }
}
