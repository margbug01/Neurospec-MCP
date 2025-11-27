# NeuroSpec

[中文](README.md) | **English**

> **AI-Powered Development Assistant - Interactive Dialogue, Memory Management & Code Search**

<p align="center">
  <img src="icon-new.png" alt="NeuroSpec Logo" width="128" height="128">
</p>

NeuroSpec is a next-generation AI development assistant that deeply integrates with AI coding tools (Windsurf, Cursor, Claude Desktop) via MCP (Model Context Protocol). It provides human-AI interaction interfaces, project-level memory management, and intelligent code search capabilities.

## ✨ Core Features

| Tool | Function | Description |
|------|----------|-------------|
| 🤝 `interact` | Smart Interaction | Rich text messages, predefined options, free input, image upload |
| 🧠 `memory` | Memory Management | Store and recall project rules, dev preferences, code patterns |
| 🔍 `search` | Code Search | Tantivy-based semantic search with auto incremental indexing |

### Why NeuroSpec?

Pain points of traditional AI coding assistants:
- ❌ AI cannot proactively ask users, only blindly guesses
- ❌ Need to repeat project specs every conversation
- ❌ AI cannot efficiently search large codebases

NeuroSpec's solutions:
- ✅ **Interaction Interception**: AI pops up dialog when uncertain
- ✅ **Persistent Memory**: Project rules auto-loaded, no repetition needed
- ✅ **Smart Search**: Millisecond-level full-text/symbol search

## 📦 Installation

### Requirements

- **Rust**: 1.70+
- **Node.js**: 18+ (with pnpm)
- **OS**: Windows 10+, macOS 11+, Linux

### Build from Source

```bash
# Clone repository
git clone https://github.com/margbug01/Neurospec-MCP.git
cd Neurospec-MCP/core

# Install frontend dependencies
pnpm install

# Build application
pnpm tauri build
```

Build outputs in `core/target/release/`:
- `NeuroSpec.exe` - GUI application
- `NeuroSpec-MCP.exe` - MCP server

## 🔧 MCP Configuration

Add NeuroSpec to your AI coding tool:

### Windsurf / Cursor

Edit `~/.cursor/mcp.json` or Windsurf's MCP config:

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

Edit `%APPDATA%/Claude/claude_desktop_config.json`:

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

> 📖 See [MCP Configuration Guide](docs/MCP_CONFIG.md) for details

## 🛠️ MCP Tools

### 1. `interact` - Smart Interaction

Enable AI to display information and collect user feedback.

```json
{
  "message": "## Two approaches detected\n\nPlease select implementation:",
  "predefined_options": ["Option A: Use existing library", "Option B: Custom implementation"],
  "is_markdown": true
}
```

**Features**:
- Markdown rendering
- Predefined options + free text input
- Drag & drop image upload
- Interaction history

### 2. `memory` - Memory Management

Store project-level development rules and preferences.

```json
{
  "action": "remember",
  "content": "Use 4-space indentation, no any type allowed",
  "category": "rule"
}
```

**Memory Categories**:
- `rule` - Development rules (e.g., coding standards)
- `preference` - User preferences (e.g., framework choices)
- `pattern` - Code patterns (e.g., common design patterns)
- `context` - Project context (e.g., tech stack info)

### 3. `search` - Code Search

High-performance code search engine.

```json
{
  "query": "user authentication logic",
  "mode": "text",
  "project_root_path": "/path/to/project"
}
```

**Search Modes**:
- `text` - Full-text semantic search
- `symbol` - Symbol definition search (functions, classes)
- `structure` - Project structure overview

## 📂 Project Structure

```
neurospec/
├── core/                    # Main project directory
│   ├── src/
│   │   ├── frontend/        # Vue 3 frontend
│   │   └── rust/           # Rust backend
│   │       ├── mcp/        # MCP protocol implementation
│   │       ├── daemon/     # HTTP service
│   │       └── ui/         # Tauri commands
│   └── src-tauri/          # Tauri configuration
├── docs/                    # Documentation
├── AGENTS.md               # AI behavior specification
└── README.md
```

## 🔐 Configuration

NeuroSpec config locations:
- **Windows**: `%APPDATA%/neurospec/config.json`
- **macOS**: `~/Library/Application Support/neurospec/config.json`
- **Linux**: `~/.config/neurospec/config.json`

## 📚 Documentation

- [MCP Configuration Guide](docs/MCP_CONFIG.md) - IDE-specific configurations
- [Development Guide](docs/DEVELOPMENT.md) - Local dev environment setup
- [Tools Reference](docs/TOOLS.md) - Complete MCP tool parameters
- [Architecture](docs/ARCHITECTURE.md) - System design

## 🤝 Contributing

Contributions welcome! Please read [Development Guide](docs/DEVELOPMENT.md) for guidelines.

## 📄 License

[MIT License](LICENSE)

---

<p align="center">
  Built with Rust + Tauri + Vue 3 ❤️
</p>
