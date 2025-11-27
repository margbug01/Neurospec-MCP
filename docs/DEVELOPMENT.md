# 开发指南

本文档介绍如何搭建 NeuroSpec 本地开发环境。

## 目录

- [环境准备](#环境准备)
- [项目结构](#项目结构)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [调试技巧](#调试技巧)
- [发布流程](#发布流程)

---

## 环境准备

### 必需工具

| 工具 | 版本 | 用途 |
|------|------|------|
| Rust | 1.70+ | 后端开发 |
| Node.js | 18+ | 前端开发 |
| pnpm | 8+ | 包管理 |
| Git | 2.0+ | 版本控制 |

### 安装步骤

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装 pnpm
npm install -g pnpm

# 3. 克隆项目
git clone https://github.com/YOUR_USERNAME/neurospec.git
cd neurospec/core

# 4. 安装依赖
pnpm install
```

### Tauri 依赖

根据操作系统安装 Tauri 所需的系统依赖：

**Windows**：
```powershell
# 安装 WebView2 (Windows 10/11 通常已预装)
# 如未安装，从 https://developer.microsoft.com/en-us/microsoft-edge/webview2/ 下载
```

**macOS**：
```bash
xcode-select --install
```

**Linux (Ubuntu/Debian)**：
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## 项目结构

```
neurospec/
├── core/                       # 主项目
│   ├── src/
│   │   ├── frontend/          # Vue 3 前端
│   │   │   ├── components/    # Vue 组件
│   │   │   ├── composables/   # 组合式函数
│   │   │   ├── views/         # 页面视图
│   │   │   └── types/         # TypeScript 类型
│   │   │
│   │   └── rust/              # Rust 后端
│   │       ├── app/           # Tauri 应用入口
│   │       ├── mcp/           # MCP 协议实现
│   │       │   ├── tools/     # MCP 工具 (interact, memory, search)
│   │       │   ├── handlers/  # 请求处理器
│   │       │   └── types.rs   # 类型定义
│   │       ├── daemon/        # HTTP Daemon 服务
│   │       └── ui/            # Tauri 命令
│   │
│   ├── src-tauri/             # Tauri 配置
│   │   ├── Cargo.toml         # Rust 依赖
│   │   ├── tauri.conf.json    # Tauri 配置
│   │   └── icons/             # 应用图标
│   │
│   ├── package.json           # 前端依赖
│   └── vite.config.ts         # Vite 配置
│
├── docs/                       # 文档
├── AGENTS.md                   # AI 行为规范
└── README.md
```

---

## 开发流程

### 启动开发服务器

```bash
cd core

# 启动开发模式（前端热重载 + Rust 实时编译）
pnpm tauri:dev
```

### 常用命令

| 命令 | 说明 |
|------|------|
| `pnpm tauri:dev` | 启动开发服务器 |
| `pnpm tauri build` | 构建生产版本 |
| `pnpm lint` | 类型检查 |
| `pnpm test:ui` | 启动 UI 测试环境 |
| `cargo test` | 运行 Rust 测试 |
| `cargo clippy` | Rust 代码检查 |

### 开发工作流

1. **创建功能分支**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **开发与测试**
   ```bash
   pnpm tauri:dev
   ```

3. **代码检查**
   ```bash
   pnpm lint
   cargo clippy
   ```

4. **提交代码**
   ```bash
   git add .
   git commit -m "feat: 添加新功能"
   ```

---

## 代码规范

### Rust 代码规范

- 使用 `rustfmt` 格式化代码
- 遵循 Clippy 建议
- 重要函数添加文档注释 (`///`)
- 使用 `log_important!` 宏记录关键日志

```rust
/// 处理用户交互请求
/// 
/// # Arguments
/// * `request` - 交互请求参数
/// 
/// # Returns
/// * `Ok(CallToolResult)` - 成功响应
/// * `Err(McpError)` - 错误信息
pub async fn interact(request: InteractRequest) -> Result<CallToolResult, McpError> {
    log_important!(info, "Processing interact request");
    // ...
}
```

### TypeScript/Vue 代码规范

- 使用 Composition API
- 组件使用 `<script setup>` 语法
- 类型定义放在 `types/` 目录
- 使用 `useXxx` 命名组合式函数

```vue
<script setup lang="ts">
import { ref, computed } from 'vue'
import type { McpRequest } from '../types/popup'

const props = defineProps<{
  request: McpRequest
}>()

const isLoading = ref(false)
</script>
```

---

## 调试技巧

### Rust 后端调试

1. **查看日志**
   ```bash
   # 设置日志级别
   RUST_LOG=debug pnpm tauri:dev
   ```

2. **使用断点**
   - VS Code: 安装 `rust-analyzer` 和 `CodeLLDB` 扩展
   - 在 `.vscode/launch.json` 配置调试

3. **关键日志点**
   - `daemon/popup_handler.rs` - 弹窗处理
   - `mcp/dispatcher.rs` - 工具分发
   - `mcp/tools/*/mcp.rs` - 各工具实现

### 前端调试

1. **Vue DevTools**
   - 安装 Vue DevTools 浏览器扩展
   - 在开发模式下按 F12 打开

2. **Console 日志**
   - `[Daemon MCP]` 前缀：MCP 相关日志
   - `[Toast]` 前缀：消息提示日志

### MCP 调试

1. **测试 MCP 服务器**
   ```bash
   # 直接运行 MCP 服务器（stdio 模式）
   ./NeuroSpec-MCP

   # 发送测试请求
   echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | ./NeuroSpec-MCP
   ```

2. **查看 MCP 日志**
   - 日志位置：`%LOCALAPPDATA%/neurospec/logs/` (Windows)

---

## 发布流程

### 1. 更新版本号

编辑以下文件：
- `core/src-tauri/Cargo.toml` - `version`
- `core/src-tauri/tauri.conf.json` - `version`
- `core/package.json` - `version`

### 2. 构建发布版本

```bash
cd core
pnpm tauri build
```

### 3. 产物位置

- **Windows**: `core/target/release/NeuroSpec.exe`
- **macOS**: `core/target/release/bundle/macos/NeuroSpec.app`
- **Linux**: `core/target/release/neurospec`

### 4. 创建 Release

```bash
git tag v1.0.0
git push origin v1.0.0
```

---

## 常见问题

### Q: 前端热重载不生效？

确保 Vite 配置正确，尝试：
```bash
rm -rf node_modules/.vite
pnpm tauri:dev
```

### Q: Rust 编译很慢？

使用增量编译：
```bash
cargo build --profile dev
```

### Q: WebView2 找不到？

Windows 用户需要安装 WebView2 Runtime：
https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

如有其他问题，欢迎提交 Issue！
