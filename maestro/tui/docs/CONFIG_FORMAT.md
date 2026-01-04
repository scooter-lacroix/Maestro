# Maestro TUI Configuration Format

**Config Location:** `~/.maestro/config.toml`

**Version:** Maestro 2.0 (agent-deck migrated)

## Overview

The Maestro TUI uses a TOML configuration file (`config.toml`) for user preferences. This file controls default tools, MCP server definitions, search settings, and more.

## Configuration Structure

```toml
# Default AI tool when creating new sessions
# Valid values: "claude", "gemini", "opencode", "codex", "shell", or custom tool names
# If empty or invalid, defaults to "shell" (no pre-selection)
default_tool = "claude"

# Custom AI tool definitions
[tools.claude]
command = "claude"
icon = ""
busy_patterns = ["esc_to_interrupt", "Connecting...", "Thinking..."]

[tools.gemini]
command = "gemini"
icon = ""
busy_patterns = []

# MCP server definitions for the MCP Manager
[mcps.nexus-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-nexus-memory"]
description = "Nexus Memory System for persistent context"

[mcps.exa]
command = "npx"
args = ["-y", "@mcpr/server-exa"]
description = "Exa semantic search API"

[mcps.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
description = "Filesystem access"

# HTTP/SSE MCP examples
[mcps.memory-http]
url = "http://localhost:3000/mcp"
transport = "http"
description = "HTTP-based memory server"

[mcps.pool-test]
url = "http://localhost:8010/sse"
transport = "sse"
description = "SSE-based pooled MCP"

# Claude Code integration settings
[claude]
config_dir = "~/.claude"           # Claude's config directory (default: ~/.claude)
dangerous_mode = false              # Enable --dangerously-skip-permissions flag

# Global conversation search settings
[global_search]
enabled = true                      # Enable/disable global search (default: true)
tier = "auto"                       # Strategy: "auto", "instant", "balanced", "disabled"
memory_limit_mb = 100               # Memory cap for balanced tier (default: 100)
recent_days = 90                    # Limit search to last N days (0 = all)
index_rate_limit = 20               # Files indexed per second (default: 20)

# Session log management settings
[logs]
max_size_mb = 10                    # Max log size before truncation (default: 10)
max_lines = 10000                   # Lines to keep when truncating (default: 10000)
remove_orphans = true               # Remove logs for deleted sessions (default: true)

# HTTP MCP pool settings (experimental)
[mcp_pool]
enabled = false                     # Enable HTTP pool mode (default: false)
auto_start = true                   # Start pool when TUI launches (default: true)
port_start = 8001                   # First port in pool range (default: 8001)
port_end = 8050                     # Last port in pool range (default: 8050)
start_on_demand = false             # Start MCPs lazily on first attach (default: false)
shutdown_on_exit = true             # Stop HTTP servers when TUI quits (default: true)
pool_mcps = []                      # MCPs to pool (empty = auto-detect)
fallback_to_stdio = true            # Use stdio for MCPs without socket support (default: true)
show_pool_status = true             # Show pool status in TUI (default: true)
pool_all = false                    # Pool all MCPs by default (default: false)
exclude_mcps = []                   # MCPs to exclude when pool_all = true

# Auto-update settings
[updates]
auto_update = false                 # Auto-install updates (default: false)
check_enabled = true                # Check for updates on startup (default: true)
check_interval_hours = 24           # Update check frequency in hours (default: 24)
notify_in_cli = true                # Show update notifications in CLI (default: true)
```

## Section Details

### `default_tool`

Pre-selected AI tool when creating new sessions via the TUI.

**Values:**
- `"claude"` - Anthropic Claude Code
- `"gemini"` - Google Gemini CLI
- `"opencode"` - OpenCode CLI
- `"codex"` - OpenAI Codex CLI
- `"shell"` - Generic shell (no AI tool)
- Any custom tool name defined in `[tools]`

### `[tools.*]`

Custom AI tool definitions.

**Fields:**
- `command` (required): Shell command to run
- `icon` (optional): Emoji/symbol to display in TUI
- `busy_patterns` (optional): Strings indicating the tool is working

### `[mcps.*]`

MCP server definitions for the MCP Manager (accessed via `M` key in TUI).

**Fields:**
- `command` (optional for HTTP): Executable to run (e.g., `npx`, `docker`, `node`)
- `args` (optional for HTTP): Command-line arguments
- `env` (optional): Environment variables as key/value pairs
- `description` (optional): Help text shown in MCP Manager
- `url` (optional): HTTP/SSE endpoint (e.g., `http://localhost:8000/mcp`)
- `transport` (optional): Transport type (`"stdio"`, `"http"`, `"sse"`)

**Transport Types:**
- `"stdio"` - Standard input/output (default, requires `command`)
- `"http"` - HTTP POST endpoint (requires `url`)
- `"sse"` - Server-Sent Events (requires `url`)

### `[claude]`

Claude Code integration settings.

**Fields:**
- `config_dir`: Path to Claude's config directory (default: `~/.claude` or `$CLAUDE_CONFIG_DIR`)
- `dangerous_mode`: Enable `--dangerously-skip-permissions` flag (security risk, use with caution)

### `[global_search]`

Global conversation search across all sessions.

**Fields:**
- `enabled`: Enable/disable feature
- `tier`: Search strategy
  - `"auto"` - Auto-detect based on data size (recommended)
  - `"instant"` - Full in-memory (fast, uses more RAM)
  - `"balanced"` - LRU cache mode (slower, capped RAM)
  - `"disabled"` - Disable entirely
- `memory_limit_mb`: Memory cap for balanced tier (default: 100MB)
- `recent_days`: Limit search to sessions from last N days (0 = all)
- `index_rate_limit`: Files indexed per second during background indexing (default: 20)

### `[logs]`

Session log file management.

**Fields:**
- `max_size_mb`: Max size before truncation (default: 10MB)
- `max_lines`: Lines to keep when truncating (default: 10,000)
- `remove_orphans`: Remove logs for deleted sessions (default: true)

### `[mcp_pool]`

HTTP MCP pool settings (experimental feature for sharing MCP connections).

**Fields:**
- `enabled`: Enable pool mode (default: false)
- `auto_start`: Start pool when TUI launches (default: true)
- `port_start`: First port in pool range (default: 8001)
- `port_end`: Last port in pool range (default: 8050)
- `start_on_demand`: Start MCPs lazily on first attach (default: false)
- `shutdown_on_exit`: Stop HTTP servers when TUI quits (default: true)
- `pool_mcps`: MCPs to pool (empty = auto-detect common MCPs)
- `fallback_to_stdio`: Use stdio for MCPs without socket support (default: true)
- `show_pool_status`: Show pool status in TUI (default: true)
- `pool_all`: Pool all MCPs by default (default: false)
- `exclude_mcps`: MCPs to exclude when `pool_all = true`

### `[updates]`

Auto-update settings.

**Fields:**
- `auto_update`: Auto-install updates without prompting (default: false)
- `check_enabled`: Check for updates on startup (default: true)
- `check_interval_hours`: Update check frequency in hours (default: 24)
- `notify_in_cli`: Show update notifications in CLI commands (default: true)

## Migration from agent-deck

If migrating from agent-deck, use:

```bash
maestro migrate:agent-deck
```

This will:
1. Copy `~/.agent-deck/config.toml` to `~/.maestro/config.toml`
2. Create a backup of the original
3. Preserve all settings and MCP definitions

## Environment Variables

- `MAESTRO_PROFILE` - Default profile to use (overrides `default_tool` per profile)
- `CLAUDE_CONFIG_DIR` - Claude config directory (overrides `[claude].config_dir`)
- `MAESTRO_COLOR` - Color mode: `truecolor`, `256`, `16`, `none`

## Profiles

Maestro TUI supports multiple configuration profiles via the `-p` flag:

```bash
maestro tui -p work    # Uses ~/.maestro/work/config.toml
maestro tui -p personal # Uses ~/.maestro/personal/config.toml
```

See `maestro profile --help` for profile management commands.

## See Also

- `maestro tui --help` - TUI command reference
- `maestro mcp --help` - MCP management commands
- `maestro profile --help` - Profile management commands
