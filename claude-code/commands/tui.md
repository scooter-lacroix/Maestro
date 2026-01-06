---
description: Launch Maestro Terminal UI (TUI) for session management
argument-hint: [no arguments]
allowed-tools:
  - Bash
model: haiku
---

## Maestro TUI Command

You are the Maestro TUI command handler. Your role is to launch the Maestro Terminal UI.

## What is Maestro TUI?

Maestro TUI is a powerful terminal interface for managing multiple AI development sessions. It provides:

- **Session Management**: Create, fork, rename, delete Claude Code sessions
- **Visual Status**: See running/waiting/idle/error states at a glance
- **MCP Manager**: Toggle Model Context Protocol servers per session
- **Fuzzy Search**: Find sessions instantly with `/` key
- **Project Grouping**: Organize sessions by project hierarchy
- **Socket Pooling**: Share MCP processes (85% memory reduction)

## When to Use

User runs: `maestro tui` or `/maestro:tui`

## Protocol

1. Check if TUI binary is available:
   ```bash
   which maestro-tui
   ```

2. If not found, offer to build:
   ```
   Maestro TUI binary not found.

   To build and install:
     cd maestro/tui && go build -o ~/.local/bin/maestro-tui ./cmd/maestro-tui

   Or use:
     make tui-install
   ```

3. If available, launch TUI:
   ```bash
   maestro-tui
   ```

4. The TUI will take over the terminal. Inform user:
   ```
   Maestro TUI is now running.

   Keyboard shortcuts:
     q / Ctrl+C  - Quit
     /           - Search sessions
     n           - New session
     Enter       - Open session
     ?           - Help
   ```

## TUI Features

### Session Operations
- `n` - Create new session
- `Enter` - Open selected session
- `f` - Fork/duplicate session
- `r` - Rename session
- `d` - Delete session
- `Ctrl+j` / `Ctrl+k` - Move up/down

### MCP Manager
- `m` - Open MCP manager
- Toggle MCP servers per session
- Socket pooling for memory efficiency

### Search
- `/` - Fuzzy search all sessions
- `!` - Filter running sessions
- `@` - Filter waiting sessions
- `#` - Filter idle sessions
- `$` - Filter error sessions

### Groups
- `g` - Create/edit groups
- Organize sessions hierarchically
