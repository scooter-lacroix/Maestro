# Maestro - Technology Stack

## Overview

Maestro is a multilayer system:

- A **Rust core** (LeIndex analyzers + MCP plumbing + TUI) built with Cargo
- A **workflow layer** expressed as Markdown command protocols, templates, and skill definitions
- A **single installer** (`install.sh`) that launches the Rust Conductor Wizard to configure integrations

## Core Technologies

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Core CLI/TUI/MCP** | Rust (Cargo) | High-performance core runtime and terminal UI |
| **Installer wrapper** | Bash | Bootstrap + launch the Conductor Wizard |
| **Command protocols** | Markdown | Tool-agnostic workflow protocols (installed to `~/.maestro/integrations/commands/`) |
| **Skills & templates** | Markdown + Shell | Reusable workflow and styleguide templates; OpenCode skill packaging |
| **Configuration** | JSON + TOML | First-class config integration for external tools |

## Platform Integration

### Claude Code
- **Command Location**: `~/.claude/commands/`
- **Template Location**: `~/.claude/maestro-templates/`
- **Command Prefix**: `/maestro:`
- **Installation Method**: `install.sh` (enable Claude Code in the Conductor Wizard)

### OpenCode
- **Skill Location**: `~/.config/opencode/skill/maestro/`
- **Command Location**: `~/.config/opencode/commands/`
- **Command Prefix**: `/maestro`
- **Installation Method**: `install.sh` (enable OpenCode in the Conductor Wizard)

## Dependencies

### Required MCP Servers
- **nexus-memory**: Project context and memory storage with agent-type categorization and LLM enhancement

### Optional Enhancements
- **prompt-enhancer**: Context-aware question generation

## Architecture Type

**Dual-Variant Distribution Framework**

Maestro provides parallel support for two AI coding platforms:
- Claude Code (via slash commands)
- OpenCode (via skill system)

Both variants share:
- Common template files
- Documentation structure
- Core functionality

## Project Structure

```
maestro/
├── README.md                    # Main repository documentation
├── VERSION                      # Semantic version (1.1.0)
├── install.sh                  # Single installer entrypoint (Conductor Wizard)
│
├── docs/                       # User documentation
│   ├── CLAUDE-CODE.md          # Claude Code guide
│   ├── OPENCODE.md             # OpenCode guide
│   ├── AGENTS.md               # Agent documentation
│   └── MIGRATION.md            # Migration guide
│
├── claude-code/                # Claude Code variant
│   ├── commands/               # Slash command definitions (.md)
│   └── templates/              # Project templates
│
└── opencode/                   # OpenCode variant
    └── skill/maestro/          # Skill definition
        ├── SKILL.md            # Main skill file
        ├── templates/          # Templates (bundled)
        └── scripts/            # Utility scripts
```

## Code Styleguides

Available language-specific style guides:
- General principles
- Go
- HTML/CSS
- JavaScript
- Python
- TypeScript

## Version Strategy

- **Format**: Semantic versioning (MAJOR.MINOR.PATCH)
- **Current**: 1.1.0
- **Storage**: `VERSION` file

## Philosophy

Maestro's technology choices reflect:
- **Simplicity**: Markdown and shell scripts are universally accessible
- **Portability**: No build process, no compilation required
- **Transparency**: All "code" is human-readable markdown
- **Flexibility**: Easy to modify and extend
