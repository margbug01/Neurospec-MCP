# 架构文档

本文档介绍 NeuroSpec 的系统架构设计。

## 目录

- [系统概览](#系统概览)
- [核心组件](#核心组件)
- [通信架构](#通信架构)
- [数据流](#数据流)
- [技术栈](#技术栈)

---

## 系统概览

NeuroSpec 采用**双进程架构**：

```
┌─────────────────────────────────────────────────────────────┐
│                    AI 编程工具 (Windsurf/Cursor)              │
│                                                              │
│   ┌──────────────────┐                                      │
│   │   MCP Client     │◄────── MCP Protocol (stdio) ───────┐ │
│   └──────────────────┘                                     │ │
└────────────────────────────────────────────────────────────┼─┘
                                                             │
┌────────────────────────────────────────────────────────────┼─┐
│                    NeuroSpec-MCP (进程1)                    │ │
│                                                             │ │
│   ┌──────────────────┐    ┌─────────────────────┐          │ │
│   │  MCP Server      │───►│  Tool Dispatcher    │          │ │
│   │  (rmcp)          │    └─────────────────────┘          │ │
│   └──────────────────┘              │                      │ │
│                                     │ HTTP (localhost:15177) │
└─────────────────────────────────────┼──────────────────────┼─┘
                                      │                      │
┌─────────────────────────────────────▼──────────────────────┼─┐
│                    NeuroSpec GUI (进程2)                    │ │
│                                                             │ │
│   ┌──────────────────┐    ┌─────────────────────┐          │ │
│   │  Daemon Server   │◄───│  Tauri Backend      │          │ │
│   │  (Axum HTTP)     │    │  (Rust)             │          │ │
│   └──────────────────┘    └─────────────────────┘          │ │
│            │                        │                      │ │
│   ┌────────▼────────┐    ┌─────────▼───────────┐          │ │
│   │  Popup Handler  │    │  Vue 3 Frontend     │          │ │
│   │  (弹窗管理)      │◄──►│  (WebView)          │          │ │
│   └─────────────────┘    └─────────────────────┘          │ │
└────────────────────────────────────────────────────────────┴─┘
```

### 为什么是双进程？

1. **MCP 协议限制**：MCP 使用 stdio 通信，需要独立进程
2. **GUI 独立性**：GUI 应用需要持续运行以显示弹窗
3. **解耦设计**：MCP 服务器无状态，GUI 有状态

---

## 核心组件

### 1. MCP Server (`core/src/rust/mcp/`)

负责实现 MCP 协议，处理工具调用。

```
mcp/
├── mod.rs              # 模块入口
├── server.rs           # MCP 服务器实现
├── dispatcher.rs       # 工具分发器
├── tool_registry.rs    # 工具注册表
├── handlers/           # 请求处理
│   ├── popup.rs        # 弹窗处理
│   └── response.rs     # 响应解析
├── tools/              # MCP 工具实现
│   ├── interaction/    # interact 工具
│   ├── memory/         # memory 工具
│   ├── acemcp/         # search 工具
│   └── unified_store/  # 统一存储
└── types.rs            # 类型定义
```

### 2. Daemon Server (`core/src/rust/daemon/`)

HTTP 服务，桥接 MCP 和 GUI。

```
daemon/
├── server.rs           # Axum HTTP 服务器
├── routes.rs           # 路由处理
├── client.rs           # HTTP 客户端
├── popup_handler.rs    # 弹窗响应处理
└── types.rs            # 请求/响应类型
```

### 3. Tauri Backend (`core/src/rust/ui/`)

Tauri 命令和 UI 逻辑。

```
ui/
├── commands.rs         # Tauri 命令
├── agents_commands.rs  # AGENTS.md 相关
├── tray.rs             # 系统托盘
└── exit_handler.rs     # 退出处理
```

### 4. Vue Frontend (`core/src/frontend/`)

用户界面实现。

```
frontend/
├── components/
│   ├── popup/          # 弹窗组件
│   │   ├── McpPopup.vue
│   │   ├── PopupContent.vue
│   │   └── PopupInput.vue
│   └── tabs/           # 标签页
├── composables/        # 组合式函数
│   ├── useMcpHandler.ts
│   ├── useMemory.ts
│   └── useSystemStatus.ts
├── views/
│   └── MainLayout.vue
└── types/
    └── popup.d.ts
```

---

## 通信架构

### MCP 通信

```
IDE ─── stdio ───► NeuroSpec-MCP
                        │
                        │ JSON-RPC 2.0
                        ▼
              ┌─────────────────┐
              │ Tool Dispatcher │
              └─────────────────┘
                   │  │  │
        ┌──────────┘  │  └──────────┐
        ▼             ▼             ▼
   [interact]    [memory]      [search]
```

### Daemon 通信

```
NeuroSpec-MCP ─── HTTP POST ───► Daemon Server
                                     │
                                     │ Tauri Event
                                     ▼
                              ┌─────────────┐
                              │ Vue Frontend │
                              └─────────────┘
                                     │
                                     │ invoke()
                                     ▼
                              ┌─────────────┐
                              │ Popup Handler│
                              └─────────────┘
                                     │
                                     │ oneshot channel
                                     ▼
                              [Response to MCP]
```

### 请求/响应流程

```
1. IDE 调用 interact 工具
   └── MCP Server 接收 JSON-RPC 请求

2. Dispatcher 路由到 InteractionTool
   └── 创建 PopupRequest

3. DaemonClient 发送 HTTP 请求
   └── POST http://127.0.0.1:15177/mcp/execute

4. Daemon 处理请求
   ├── 创建 oneshot channel
   ├── 存储到 PENDING_RESPONSES
   └── 发送 Tauri 事件 "mcp-popup-request"

5. Vue 前端显示弹窗
   └── 用户交互

6. 用户提交响应
   └── invoke('handle_mcp_popup_response')

7. Popup Handler 处理响应
   ├── 从 PENDING_RESPONSES 获取 sender
   └── 通过 oneshot channel 发送响应

8. Daemon 返回 HTTP 响应
   └── DaemonClient 接收

9. MCP 返回工具结果
   └── IDE 接收 JSON-RPC 响应
```

---

## 数据流

### 记忆系统

```
┌─────────────────────────────────────────────────┐
│                Memory System                     │
│                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  recall  │───►│  Store   │◄───│ remember │  │
│  └──────────┘    │          │    └──────────┘  │
│                  │  JSON    │                   │
│                  │  Files   │                   │
│                  └──────────┘                   │
│                       │                         │
│              ~/.neurospec-memory/               │
│              └── {project_hash}/                │
│                  └── memories.json              │
└─────────────────────────────────────────────────┘
```

### 搜索索引

```
┌─────────────────────────────────────────────────┐
│               Search System                      │
│                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  search  │───►│ Tantivy  │◄───│ Indexer  │  │
│  └──────────┘    │  Index   │    └──────────┘  │
│                  └──────────┘          ▲        │
│                                        │        │
│                              ┌─────────┴──────┐ │
│                              │ File Watcher   │ │
│                              │ (增量更新)      │ │
│                              └────────────────┘ │
│                                                  │
│           %LOCALAPPDATA%/neurospec/              │
│           └── search_index/                      │
└─────────────────────────────────────────────────┘
```

---

## 技术栈

### 后端

| 技术 | 用途 |
|------|------|
| Rust | 主开发语言 |
| Tauri | 桌面应用框架 |
| Axum | HTTP 服务框架 |
| rmcp | MCP 协议实现 |
| Tantivy | 全文搜索引擎 |
| Tree-sitter | 代码解析 |
| Tokio | 异步运行时 |

### 前端

| 技术 | 用途 |
|------|------|
| Vue 3 | UI 框架 |
| TypeScript | 类型安全 |
| Vite | 构建工具 |
| UnoCSS | 原子化 CSS |
| Iconify | 图标库 |

### 数据存储

| 数据类型 | 存储方式 |
|----------|----------|
| 配置 | JSON 文件 |
| 记忆 | JSON 文件 |
| 搜索索引 | Tantivy 索引 |
| 交互历史 | JSON 文件 |

---

## 扩展点

### 添加新 MCP 工具

1. 在 `mcp/tools/` 创建新模块
2. 实现工具逻辑
3. 在 `tool_registry.rs` 注册工具
4. 在 `dispatcher.rs` 添加路由

### 添加新 UI 功能

1. 在 `frontend/components/` 创建组件
2. 在 `ui/commands.rs` 添加 Tauri 命令
3. 在 `frontend/composables/` 创建组合式函数

---

## 性能考虑

- **MCP 响应延迟**：< 100ms（不含用户交互）
- **搜索延迟**：< 50ms（索引就绪后）
- **内存占用**：~100MB（取决于索引大小）
- **索引构建**：~1000 文件/秒

---

如有架构相关问题，欢迎提交 Issue 讨论！
