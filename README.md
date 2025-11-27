# NeuroSpec

**中文** | [English](README_EN.md)

> **AI 驱动的开发助手 - 智能交互、记忆管理与代码搜索**

[![Release](https://img.shields.io/github/v/release/margbug01/Neurospec-MCP)](https://github.com/margbug01/Neurospec-MCP/releases/latest)
[![Download](https://img.shields.io/github/downloads/margbug01/Neurospec-MCP/total)](https://github.com/margbug01/Neurospec-MCP/releases/latest)

**[⬇️ 下载最新版本](https://github.com/margbug01/Neurospec-MCP/releases/latest)**

<p align="center">
  <img src="icon-new.png" alt="NeuroSpec Logo" width="128" height="128">
</p>

NeuroSpec 是一个新一代的 AI 开发助手，通过 MCP（Model Context Protocol）协议与 AI 编程工具（如 Windsurf、Cursor、Claude Desktop）深度集成。它为 AI 提供了人机交互界面、项目级记忆管理和智能代码搜索能力。

## ✨ 核心特性

| 工具 | 功能 | 描述 |
|------|------|------|
| 🤝 `interact` | 智能交互 | 富文本消息展示、预定义选项、自由输入、图片上传 |
| 🧠 `memory` | 记忆管理 | 存储和召回项目规则、开发偏好、代码模式 |
| 🔍 `search` | 代码搜索 | 基于 Tantivy 的语义搜索，自动增量索引 |

### 为什么需要 NeuroSpec？

传统 AI 编程助手的痛点：
- ❌ AI 无法主动询问用户，只能盲目猜测
- ❌ 每次对话都要重复说明项目规范
- ❌ AI 无法高效搜索大型代码库

NeuroSpec 的解决方案：
- ✅ **交互拦截**：AI 不确定时主动弹窗询问
- ✅ **持久记忆**：项目规则自动加载，无需重复说明
- ✅ **智能搜索**：毫秒级全文/符号搜索

## 📦 安装

### 系统要求

- **Rust**: 1.70+
- **Node.js**: 18+ (使用 pnpm)
- **操作系统**: Windows 10+、macOS 11+、Linux

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/YOUR_USERNAME/neurospec.git
cd neurospec/core

# 安装前端依赖
pnpm install

# 构建应用
pnpm tauri build
```

构建产物位于 `core/target/release/`：
- `NeuroSpec.exe` - GUI 主程序
- `NeuroSpec-MCP.exe` - MCP 服务端

## 🔧 配置 MCP

将 NeuroSpec 添加到你的 AI 编程工具中：

### Windsurf / Cursor

编辑 `~/.cursor/mcp.json` 或 Windsurf 的 MCP 配置文件：

```json
{
  "mcpServers": {
    "neurospec": {
      "command": "C:/path/to/NeuroSpec-MCP.exe",
      "args": []
    }
  }
}
```

### Claude Desktop

编辑 `%APPDATA%/Claude/claude_desktop_config.json`：

```json
{
  "mcpServers": {
    "neurospec": {
      "command": "C:/path/to/NeuroSpec-MCP.exe",
      "args": []
    }
  }
}
```

> 📖 详细配置请参考 [MCP 配置指南](docs/MCP_CONFIG.md)

## 🛠️ MCP 工具详解

### 1. `interact` - 智能交互

让 AI 能够向用户展示信息并收集反馈。

```json
{
  "message": "## 检测到两种方案\n\n请选择实现方式：",
  "predefined_options": ["方案A: 使用现有库", "方案B: 自己实现"],
  "is_markdown": true
}
```

**特性**：
- Markdown 渲染
- 预定义选项 + 自由输入
- 支持图片拖拽上传
- 交互历史记录

### 2. `memory` - 记忆管理

存储项目级的开发规则和偏好。

```json
{
  "action": "remember",
  "content": "本项目使用 4 空格缩进，禁止使用 any 类型",
  "category": "rule"
}
```

**记忆类型**：
- `rule` - 开发规则（如编码规范）
- `preference` - 用户偏好（如框架选择）
- `pattern` - 代码模式（如常用设计模式）
- `context` - 项目上下文（如技术栈信息）

### 3. `search` - 代码搜索

高效的代码搜索引擎。

```json
{
  "query": "用户认证逻辑",
  "mode": "text",
  "project_root_path": "/path/to/project"
}
```

**搜索模式**：
- `text` - 全文语义搜索
- `symbol` - 符号定义搜索（函数、类名）
- `structure` - 项目结构概览

## 📂 项目结构

```
neurospec/
├── core/                    # 主项目目录
│   ├── src/
│   │   ├── frontend/        # Vue 3 前端
│   │   └── rust/           # Rust 后端
│   │       ├── mcp/        # MCP 协议实现
│   │       ├── daemon/     # HTTP 服务
│   │       └── ui/         # Tauri 命令
│   └── src-tauri/          # Tauri 配置
├── docs/                    # 文档
├── AGENTS.md               # AI 行为规范
└── README.md
```

## 🔐 配置文件

NeuroSpec 配置存储位置：
- **Windows**: `%APPDATA%/neurospec/config.json`
- **macOS**: `~/Library/Application Support/neurospec/config.json`
- **Linux**: `~/.config/neurospec/config.json`

## 📚 文档

- [MCP 配置指南](docs/MCP_CONFIG.md) - 各 IDE 的详细配置
- [开发指南](docs/DEVELOPMENT.md) - 本地开发环境搭建
- [工具详解](docs/TOOLS.md) - MCP 工具完整参数说明
- [架构文档](docs/ARCHITECTURE.md) - 系统架构设计

## 🤝 贡献

欢迎贡献代码！请阅读 [开发指南](docs/DEVELOPMENT.md) 了解开发流程。

## 📄 许可证

[MIT License](LICENSE)

---

<p align="center">
  使用 Rust + Tauri + Vue 3 构建 ❤️
</p>
