# Maestro - Technology Stack

## Overview

Maestro is a documentation and command framework rather than a traditional compiled application. Its "code" consists primarily of markdown files that define commands, skills, and templates, along with shell scripts for installation.

## Core Technologies

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Shell Scripting** | Bash | Installation scripts and utilities |
| **Commands** | Markdown | Claude Code slash command definitions |
| **Skills** | Markdown + Shell | OpenCode skill definitions and integration |
| **Documentation** | Markdown | All project documentation |
| **Templates** | Markdown | Reusable project scaffolding templates |

## Platform Integration

### Claude Code
- **Command Location**: `~/.claude/commands/`
- **Template Location**: `~/.claude/maestro-templates/`
- **Command Prefix**: `/maestro:`
- **Installation Method**: `install-claude-code.sh`

### OpenCode
- **Skill Location**: `~/.opencode/skill/maestro/`
- **Template Location**: `~/.claude/maestro-templates/`
- **Command Prefix**: `/maestro`
- **Installation Method**: `install-opencode.sh`

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
├── install-claude-code.sh      # Claude Code installer
├── install-opencode.sh         # OpenCode installer
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
        ├── commands/           # Command references
        ├── templates/          # Template symlinks
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
