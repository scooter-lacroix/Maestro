# Maestro Claude Code Commands

This directory contains the Maestro slash commands for Claude Code and OpenCode.

## Available Commands

### Core Commands
- `maestro:setup.md` - Initialize Maestro environment for new or existing projects
- `maestro:newTrack.md` - Create new track with interactive specification generation
- `maestro:implement.md` - Implement track tasks with automatic agent selection
- `maestro:status.md` - View project progress across all tracks
- `maestro:revert.md` - Revert work at track/phase/task level
- `maestro:configure.md` - Configure Maestro settings and preferences

### Memory Commands
- `maestro:memory.md` - Interact with Maestro Memory System (serve dashboard, check status)

## Installation

These files are automatically installed by the Maestro plugin marketplace or by running the installer script:
- Marketplace: `/plugin marketplace add scooter-lacroix/maestro` then `/plugin install maestro`
- Manual: `./install.sh` (enable “Claude Code (by Anthropic)” in the Conductor Wizard)

## Usage

In Claude Code, use the commands with the `/maestro:` prefix:

```bash
/maestro:setup              # Initialize Maestro environment
/maestro:newTrack Add user authentication  # Create new track
/maestro:implement [track]  # Implement track tasks
/maestro:status             # View progress
/maestro:revert [track]     # Revert work
/maestro:configure          # Configure settings
/maestro:memory serve       # Launch memory dashboard
```

## Slash Commands vs CLI Tools

**Claude Code Slash Commands** (these files):
- Designed for use within Claude Code/OpenCode sessions
- Work with AI assistance and context
- Examples: `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`

**CLI Tools** (require installer, run from terminal):
- Standalone tools that run in your terminal
- Require the full Maestro installation
- Examples: `maestro tui`, `maestro memory serve`, `maestro memory status`

The Maestro TUI is a CLI tool and must be run from your terminal as `maestro tui`, not as a slash command.

## Documentation

- [../../docs/CLAUDE-CODE.md](../../docs/CLAUDE-CODE.md) - Complete Claude Code usage guide
- [../../docs/OPENCODE.md](../../docs/OPENCODE.md) - OpenCode specific documentation
- [../../README.md](../../README.md) - Main project documentation
