# MCP 配置指南

本文档详细介绍如何在各种 AI 编程工具中配置 NeuroSpec MCP 服务器。

## 目录

- [Windsurf](#windsurf)
- [Cursor](#cursor)
- [Claude Desktop](#claude-desktop)
- [通用配置选项](#通用配置选项)
- [故障排除](#故障排除)

---

## Windsurf

### 配置文件位置

- **Windows**: `%USERPROFILE%\.windsurf\mcp.json`
- **macOS**: `~/.windsurf/mcp.json`
- **Linux**: `~/.windsurf/mcp.json`

### 配置示例

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

### 注意事项

1. 路径使用正斜杠 `/` 或双反斜杠 `\\`
2. 确保 NeuroSpec GUI 应用正在运行（交互功能需要）
3. 重启 Windsurf 使配置生效

---

## Cursor

### 配置文件位置

- **Windows**: `%USERPROFILE%\.cursor\mcp.json`
- **macOS**: `~/.cursor/mcp.json`
- **Linux**: `~/.cursor/mcp.json`

### 配置示例

```json
{
  "mcpServers": {
    "neurospec": {
      "command": "/path/to/NeuroSpec-MCP",
      "args": []
    }
  }
}
```

---

## Claude Desktop

### 配置文件位置

- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

### 配置示例

```json
{
  "mcpServers": {
    "neurospec": {
      "command": "C:\\Users\\YourName\\NeuroSpec-MCP.exe",
      "args": []
    }
  }
}
```

### 验证配置

1. 打开 Claude Desktop
2. 在对话中输入：`请列出可用的 MCP 工具`
3. 应该能看到 `interact`、`memory`、`search` 等工具

---

## 通用配置选项

### 完整配置结构

```json
{
  "mcpServers": {
    "neurospec": {
      "command": "/path/to/NeuroSpec-MCP",
      "args": [],
      "env": {
        "NEUROSPEC_LOG_LEVEL": "info"
      }
    }
  }
}
```

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `NEUROSPEC_LOG_LEVEL` | 日志级别 | `info` |
| `NEUROSPEC_DAEMON_PORT` | Daemon 端口 | `15177` |

---

## 故障排除

### 问题：MCP 服务器无法启动

**症状**：IDE 提示 MCP 服务器连接失败

**解决方案**：
1. 确认 `NeuroSpec-MCP.exe` 路径正确
2. 尝试在命令行直接运行：
   ```bash
   /path/to/NeuroSpec-MCP --version
   ```
3. 检查是否有权限问题

### 问题：interact 工具无响应

**症状**：调用 `interact` 后无弹窗显示

**解决方案**：
1. 确保 NeuroSpec GUI 应用正在运行
2. 检查任务栏是否有 NeuroSpec 图标
3. 查看 NeuroSpec 日志（设置 → 日志）

### 问题：search 工具返回空结果

**症状**：搜索始终返回 "No relevant code context found"

**解决方案**：
1. 确保提供了正确的 `project_root_path`
2. 首次搜索需要 10-30 秒建立索引
3. 检查项目是否在 `.gitignore` 中被排除

### 问题：memory 工具报错

**症状**：记忆操作失败

**解决方案**：
1. 确保 `project_path` 指向有效的 Git 仓库
2. 检查 `~/.neurospec-memory/` 目录权限
3. 尝试清理缓存：删除 `~/.neurospec-memory/` 目录

---

## 多 MCP 服务器配置

可以同时配置多个 MCP 服务器：

```json
{
  "mcpServers": {
    "neurospec": {
      "command": "/path/to/NeuroSpec-MCP",
      "args": []
    },
    "other-server": {
      "command": "/path/to/other-mcp",
      "args": []
    }
  }
}
```

---

## 更新配置后

修改配置文件后，需要：

1. **Windsurf/Cursor**：重启 IDE
2. **Claude Desktop**：完全退出并重新打开

---

如有其他问题，请查看 [GitHub Issues](https://github.com/YOUR_USERNAME/neurospec/issues) 或提交新的 Issue。
