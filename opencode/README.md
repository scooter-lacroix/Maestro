# Maestro for OpenCode

This directory contains the OpenCode-specific integration files for Maestro.

## Structure

```
opencode/
├── skill/
│   └── maestro/
│       ├── SKILL.md              # Main skill definition
│       ├── README.md             # This file
│       ├── commands/             # Command file references
│       ├── templates/            # Template symlinks
│       └── scripts/              # Utility scripts
└── opencode-config.patch         # Config snippet for opencode.json
```

## Installation

Run the installer script:
```bash
./install-opencode.sh
```

This will:
1. Copy skill files to `~/.opencode/skill/maestro/`
2. Copy templates to `~/.claude/maestro-templates/`
3. Update `~/.config/opencode/opencode.json` with command entries

## Usage

In OpenCode, use the commands with the `/maestro` prefix:

```
/maestro setup
/maestro newTrack <description>
/maestro implement [track_name]
/maestro status
/maestro revert [track|phase|task]
```

## Agent Integration

Maestro automatically selects OpenCode agents based on task complexity:

| Task Type | Agent |
|-----------|-------|
| ETL/data pipelines | amp-code |
| Large codebase analysis | gemini-analyzer |
| Rapid prototyping | opencode-scaffolder |
| Refactoring/tests | qwen-coder |
| Security review | codex-reviewer |
| Complex orchestration | kilocode-orchestrator |

## Documentation

- [../../docs/OPENCODE.md](../../docs/OPENCODE.md) - Complete OpenCode guide
- [skill/maestro/SKILL.md](skill/maestro/SKILL.md) - Skill documentation
- [skill/maestro/README.md](skill/maestro/README.md) - Skill usage
