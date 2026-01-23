---
description: Launch Maestro Cockpit TUI (Rust Terminal UI) for session management
argument-hint: [no arguments]
allowed-tools:
  - Bash
model: haiku
---

## Maestro Cockpit TUI Command

You are the Maestro Cockpit TUI command handler. Your role is to launch the Rust-based Maestro Terminal UI.

## What is Maestro Cockpit?

Maestro Cockpit v2 is a powerful terminal interface built with Rust and ratatui. It provides:

- **Session Management**: Create, fork, rename, delete Claude Code sessions
- **Visual Status**: See running/waiting/idle/error states at a glance
- **MCP Manager**: Toggle Model Context Protocol servers per session
- **LSP Integration**: Manage Language Server Protocol servers per session
- **Fuzzy Search**: Find sessions instantly with `/` key
- **Project Grouping**: Organize sessions by project hierarchy
- **Socket Pooling**: Share MCP processes (85% memory reduction)

## When to Use

User runs: `maestro tui` or `/maestro:tui`

## Protocol

1. Check if `maestro` binary is available:
   ```bash
   which maestro
   ```

2. If not found, offer to build:
   ```
   Maestro binary not found.

   To build and install from source:
     make build

   Or use cargo:
     cargo build --workspace --release
     cargo install --path crates/cli
   ```

3. If available, launch Cockpit TUI:
   ```bash
   maestro tui
   ```

4. The Cockpit TUI will take over the terminal. Inform user:
   ```
   Maestro Cockpit v2 is now running.

   Keyboard shortcuts:
     q / Ctrl+C  - Quit
     Tab         - Switch tabs (Dashboard/Sessions/Projects/Analysis/LSP/Settings)
     /           - Search sessions
     n           - New session
     Enter       - Open selected session
     ?           - Help
   ```

## Cockpit v2 Features

### Tabs
- **Dashboard**: Overview of all sessions and quick actions
- **Sessions**: Detailed session management with MCP/LSP controls
- **Projects**: Project browser and workspace management
- **Analysis**: Code analysis using LeIndex 5-phase framework
- **LSP**: Language Server Protocol management per session
- **Settings**: Theme, editor, and installation path configuration

### Session Operations
- `n` - Create new session
- `Enter` - Open selected session
- `f` - Fork/duplicate session
- `r` - Rename session
- `d` - Delete session
- `Ctrl+j` / `Ctrl+k` - Move up/down
- `g` - Move session to group

### MCP Manager
- `m` - Open MCP manager within Sessions tab
- Toggle MCP servers per session
- View MCP logs and connection status
- Socket pooling for memory efficiency

### LSP Manager
- `s` - Toggle LSP for selected session
- `R` - Restart LSP
- `l` - View LSP logs
- Installation guidance for missing LSPs

### Search
- `/` - Fuzzy search all sessions
- `!` - Filter running sessions
- `@` - Filter waiting sessions
- `#` - Filter idle sessions
- `$` - Filter error sessions

## Architecture

- **Language**: Rust (not Go)
- **Framework**: ratatui for TUI
- **Binary**: `maestro` (from `crates/cli`)
- **Library**: `maestro-cockpit` (from `crates/cockpit`)
- **Location**: `crates/cockpit/src/app.rs`
