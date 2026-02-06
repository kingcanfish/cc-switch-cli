<div align="center">

# CC-Switch CLI

[![Version](https://img.shields.io/badge/version-0.1.3-blue.svg)](https://github.com/kingcanfish/cc-switch-cli/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/kingcanfish/cc-switch-cli/releases)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**Command-Line Management Tool for Claude Code, Codex, Gemini & OpenCode CLI**

Unified management for Claude Code, Codex, Gemini & OpenCode CLI provider configurations, MCP servers, Skills extensions, and system prompts.

[English](README.md) | [中文](README_ZH.md)

</div>

---

## 📖 About

This project is a **CLI fork** of [CC-Switch](https://github.com/farion1231/cc-switch).


**Credits:** Original architecture and core functionality from [farion1231/cc-switch](https://github.com/farion1231/cc-switch)

## 📝 Changelog

See [`docs/CHANGELOG.md`](docs/CHANGELOG.md) for recent updates and migration notes.

---

## 🚀 Quick Start

**Interactive Mode (Recommended)**
```bash
cc-switch-cli
```
🤩 Follow on-screen menus to explore features.

- Full interactive mode is now TUI-based end-to-end.
- Long-running actions use a bottom lightweight status spinner (no full-page loading switch).

**Command-Line Mode**
```bash
cc-switch-cli provider list              # List providers
cc-switch-cli provider switch <id>       # Switch provider
cc-switch-cli mcp sync                   # Sync MCP servers

# Use the global `--app` flag to target specific applications:
cc-switch-cli --app claude provider list    # Manage Claude providers
cc-switch-cli --app codex mcp sync          # Sync Codex MCP servers
cc-switch-cli --app gemini prompts list     # List Gemini prompts
cc-switch-cli --app opencode provider list  # Manage OpenCode providers

# Supported apps: `claude` (default), `codex`, `gemini`, `opencode`
```

See the "Features" section below for full command list.

---

## ✨ Features

### 🔌 Provider Management

Manage API configurations for **Claude Code**, **Codex**, **Gemini**, and **OpenCode**.

**Features:** One-click switching, multi-endpoint support, API key management, speed testing, provider duplication.

```bash
cc-switch-cli provider list              # List all providers
cc-switch-cli provider current           # Show current provider
cc-switch-cli provider switch <id>       # Switch provider
cc-switch-cli provider add               # Add new provider
cc-switch-cli provider edit <id>         # Edit existing provider
cc-switch-cli provider duplicate <id>    # Duplicate a provider
cc-switch-cli provider delete <id>       # Delete provider
cc-switch-cli provider speedtest <id>    # Test API latency
```

**OpenCode note:** OpenCode uses additive providers (no “current” switch). Use add/edit/delete to manage entries.

### 🛠️ MCP Server Management

Manage Model Context Protocol servers across Claude/Codex/Gemini/OpenCode.

**Features:** Unified management, multi-app support, three transport types (stdio/http/sse), automatic sync, smart TOML parser.

```bash
cc-switch-cli mcp list                   # List all MCP servers
cc-switch-cli mcp add                    # Add new MCP server (interactive)
cc-switch-cli mcp edit <id>              # Edit MCP server
cc-switch-cli mcp delete <id>            # Delete MCP server
cc-switch-cli mcp enable <id> --app claude   # Enable for specific app
cc-switch-cli mcp disable <id> --app claude  # Disable for specific app
cc-switch-cli mcp validate <command>     # Validate command in PATH
cc-switch-cli mcp sync                   # Sync to live files
cc-switch-cli mcp import --app claude    # Import from live config
```

### 💬 Prompts Management

Manage system prompt presets for AI coding assistants.

**Cross-app support:** Claude (`CLAUDE.md`), Codex (`AGENTS.md`), Gemini (`GEMINI.md`), OpenCode (`AGENTS.md`).

```bash
cc-switch-cli prompts list               # List prompt presets
cc-switch-cli prompts current            # Show current active prompt
cc-switch-cli prompts activate <id>      # Activate prompt
cc-switch-cli prompts deactivate         # Deactivate current active prompt
cc-switch-cli prompts create             # Create new prompt preset
cc-switch-cli prompts edit <id>          # Edit prompt preset
cc-switch-cli prompts show <id>          # Display full content
cc-switch-cli prompts delete <id>        # Delete prompt
```

### 🎯 Skills Management

Manage and extend Claude Code/Codex/Gemini/OpenCode capabilities with community skills.

**Features:** Search skill marketplace, install/uninstall, repository management, skill information.

```bash
cc-switch-cli skills list                # List installed skills
cc-switch-cli skills search <query>      # Search available skills
cc-switch-cli skills install <name>      # Install a skill
cc-switch-cli skills uninstall <name>    # Uninstall a skill
cc-switch-cli skills info <name>         # Show skill information
cc-switch-cli skills repos               # Manage skill repositories
```

### ⚙️ Configuration Management

Manage configuration backups, imports, and exports.

**Features:** Custom backup naming, interactive backup selection, automatic rotation (keep 10), import/export.

```bash
cc-switch-cli config show                # Display configuration
cc-switch-cli config path                # Show config file paths
cc-switch-cli config validate            # Validate config file

# Common snippet (shared settings across providers)
cc-switch-cli --app claude config common show
cc-switch-cli --app claude config common set --json '{"env":{"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":1},"includeCoAuthoredBy":false}' --apply
cc-switch-cli --app claude config common clear --apply

# Backup
cc-switch-cli config backup              # Create backup (auto-named)
cc-switch-cli config backup --name my-backup  # Create backup with custom name

# Restore
cc-switch-cli config restore             # Interactive: select from backup list
cc-switch-cli config restore --backup <id>    # Restore specific backup by ID
cc-switch-cli config restore --file <path>    # Restore from external file

# Import/Export
cc-switch-cli config export <path>       # Export to external file
cc-switch-cli config import <path>       # Import from external file

cc-switch-cli config reset               # Reset to default configuration
```

### 🌐 Multi-language Support

Interactive mode supports English and Chinese, language settings are automatically saved.

- Default language: English
- Go to `⚙️ Settings` menu to switch language

### 🔧 Utilities

Shell completions, environment management, and other utilities.

```bash
# Shell completions
cc-switch-cli completions <shell>        # Generate shell completions (bash/zsh/fish/powershell)

# Environment management
cc-switch-cli env check                  # Check for environment conflicts
cc-switch-cli env list                   # List environment variables

# Self update (macOS/Linux)
cc-switch-cli update                     # Update to the latest release
```

---

## 📥 Installation

### Method 1: Homebrew (macOS & Linux)

```bash
brew tap kingcanfish/tap
brew install cc-switch-cli
brew upgrade cc-switch-cli
```

### Method 2: Download Pre-built Binaries (Recommended)

Download the latest release from [GitHub Releases](https://github.com/kingcanfish/cc-switch-cli/releases).

#### macOS

```bash
# Download Universal Binary (recommended, supports Apple Silicon + Intel)
curl -LO https://github.com/kingcanfish/cc-switch-cli/releases/latest/download/cc-switch-cli-v0.1.3-darwin-universal.tar.gz

# Extract
tar -xzf cc-switch-cli-v0.1.3-darwin-universal.tar.gz

# Add execute permission
chmod +x cc-switch-cli

# Move to PATH
sudo mv cc-switch-cli /usr/local/bin/

# If you encounter "cannot be verified" warning
xattr -cr /usr/local/bin/cc-switch-cli
```

#### Linux (x64)

```bash
# Download
curl -LO https://github.com/kingcanfish/cc-switch-cli/releases/latest/download/cc-switch-cli-v0.1.3-linux-x64-musl.tar.gz

# Extract
tar -xzf cc-switch-cli-v0.1.3-linux-x64-musl.tar.gz

# Add execute permission
chmod +x cc-switch-cli

# Move to PATH
sudo mv cc-switch-cli /usr/local/bin/
```

#### Linux (ARM64)

```bash
# For Raspberry Pi or ARM servers
curl -LO https://github.com/kingcanfish/cc-switch-cli/releases/latest/download/cc-switch-cli-v0.1.3-linux-arm64-musl.tar.gz
tar -xzf cc-switch-cli-v0.1.3-linux-arm64-musl.tar.gz
chmod +x cc-switch-cli
sudo mv cc-switch-cli /usr/local/bin/
```

#### Windows

```powershell
# Download the zip file
# https://github.com/kingcanfish/cc-switch-cli/releases/latest/download/cc-switch-cli-v0.1.3-windows-x64.zip

# After extracting, move cc-switch-cli.exe to a PATH directory, e.g.:
move cc-switch-cli.exe C:\Windows\System32\

# Or run directly
.\cc-switch-cli.exe
```

### Method 3: Build from Source

**Prerequisites:**
- Rust 1.85+ ([install via rustup](https://rustup.rs/))

**Build:**
```bash
git clone https://github.com/kingcanfish/cc-switch-cli.git
cd cc-switch-cli
cargo build --release

# Binary location: ./target/release/cc-switch-cli
```

**Install to System:**
```bash
# macOS/Linux
sudo cp target/release/cc-switch-cli /usr/local/bin/

# Windows
copy target\release\cc-switch-cli.exe C:\Windows\System32\
```

---

## 🏗️ Architecture

### Core Design

- **SSOT**: All config in `~/.cc-switch-cli/config.json`, live configs are generated artifacts
- **Atomic Writes**: Temp file + rename pattern prevents corruption
- **Service Layer Reuse**: 100% reused from original GUI version
- **Concurrency Safe**: RwLock with scoped guards

### Configuration Files

**CC-Switch Storage:**
- `~/.cc-switch-cli/config.json` - Main configuration (SSOT)
- `~/.cc-switch-cli/settings.json` - Settings
- `~/.cc-switch-cli/backups/` - Auto-rotation (keep 10)

**Live Configs:**
- Claude: `~/.claude/settings.json`, `~/.claude.json` (MCP), `~/.claude/CLAUDE.md` (prompts)
- Codex: `~/.codex/auth.json`, `~/.codex/config.toml` (MCP), `~/.codex/AGENTS.md` (prompts)
- Gemini: `~/.gemini/.env`, `~/.gemini/settings.json` (MCP), `~/.gemini/GEMINI.md` (prompts)
- OpenCode: `~/.config/opencode/opencode.json` (providers & MCP), `~/.config/opencode/AGENTS.md` (prompts)

---

## ❓ FAQ (Frequently Asked Questions)

<details>
<summary><b>Why doesn't my configuration take effect after switching providers?</b></summary>

<br>

This is usually caused by **environment variable conflicts**. If you have API keys set in system environment variables (like `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`), they will override CC-Switch's configuration.

**Solution:**

1. Check for conflicts:
   ```bash
   cc-switch-cli env check --app claude
   ```

2. List all related environment variables:
   ```bash
   cc-switch-cli env list --app claude
   ```

3. If conflicts are found, manually remove them:
   - **macOS/Linux**: Edit your shell config file (`~/.bashrc`, `~/.zshrc`, etc.)
     ```bash
     # Find and delete the line with the environment variable
     nano ~/.zshrc
     # Or use your preferred text editor: vim, code, etc.
     ```
   - **Windows**: Open System Properties → Environment Variables and delete the conflicting variables

4. Restart your terminal for changes to take effect.

</details>

<details>
<summary><b>Which apps are supported?</b></summary>

<br>

CC-Switch currently supports four AI coding assistants:
- **Claude Code** (`--app claude`, default)
- **Codex** (`--app codex`)
- **Gemini** (`--app gemini`)
- **OpenCode** (`--app opencode`)

Use the global `--app` flag to specify which app to manage:
```bash
cc-switch-cli --app codex provider list
```

</details>

<details>
<summary><b>How do I report bugs or request features?</b></summary>

<br>

Please open an issue on our [GitHub Issues](https://github.com/kingcanfish/cc-switch-cli/issues) page with:
- Detailed description of the problem or feature request
- Steps to reproduce (for bugs)
- Your system information (OS, version)
- Relevant logs or error messages

</details>

---

## 🛠️ Development

### Requirements

- **Rust**: 1.85+ ([rustup](https://rustup.rs/))
- **Cargo**: Bundled with Rust

### Commands

```bash
cargo run                            # Development mode
cargo run -- provider list           # Run specific command
cargo build --release                # Build release

cargo fmt                            # Format code
cargo clippy                         # Lint code
cargo test                           # Run tests
```

### Code Structure

```
src/
├── cli/
│   ├── commands/          # CLI subcommands (provider, mcp, prompts, config)
│   ├── interactive/       # Interactive TUI mode
│   └── ui.rs              # UI utilities (tables, colors)
├── services/              # Business logic
├── main.rs                # CLI entry point
└── ...
```


## 🤝 Contributing

Contributions welcome! This fork focuses on CLI functionality.

**Before submitting PRs:**
- ✅ Pass format check: `cargo fmt --check`
- ✅ Pass linter: `cargo clippy`
- ✅ Pass tests: `cargo test`
- 💡 Open an issue for discussion first

---

## 📜 License

- MIT © Original Author: Jason Young
- CLI Fork Maintainer: kingcanfish
