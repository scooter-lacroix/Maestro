# @stanford-cooper/pi-maestro

Maestro workflow commands for pi-mono - control Maestro from within pi-mono.

This extension provides spec-driven development workflows within pi-mono by implementing the same commands available in Claude Code, but adapted for pi-mono's extension system.

## Installation

```bash
pi install npm:@stanford-cooper/pi-maestro
```

## Usage

Once installed, the following commands are available within pi-mono:

### Core Workflow Commands

#### `/maestro:setup`
Initialize or refresh a maestro project structure.

```bash
/maestro:setup
```

Creates:
- `maestro/product.md` - Product definition
- `maestro/tech-stack.md` - Technology stack
- `maestro/workflow.md` - Development workflow
- `maestro/tracks.md` - Track registry
- `maestro/critical_think/templates/` - Critical Think templates

#### `/maestro:newTrack <description>` (CRITICAL)
Create a new maestro track with spec.md and plan.md.

```bash
/maestro:newTrack Add user authentication
```

This is the **most critical command** - it:
1. Asks 3-5 questions interactively to gather requirements
2. Generates `spec.md` with functional/non-functional requirements
3. Generates `plan.md` with phased implementation tasks
4. Creates track directory: `maestro/tracks/<track_id>/`
5. Updates `maestro/tracks.md` registry

#### `/maestro:implement <track_id>`
Execute a track's implementation plan.

```bash
/maestro:implement auth_20260127
```

Executes tasks from the track's `plan.md`, delegating to pi-mono subagents for complex work.

#### `/maestro:orchestrate <master_track_id>`
Orchestrate a master track (manage sub-tracks).

```bash
/maestro:orchestrate master_20260127
```

For master tracks that contain multiple sub-tracks, this command coordinates their execution.

#### `/maestro:status`
Show project progress and status.

```bash
/maestro:status
```

Displays summary of all tracks with progress percentages and current phases.

#### `/maestro:revert <track_id> [phase] [task]`
Revert track work (git-aware rollback).

```bash
/maestro:revert auth_20260127
/maestro:revert auth_20260127 "Implementation"
/maestro:revert auth_20260127 "Implementation" "Add login form"
```

#### `/maestro:configure [key=value]`
Configure maestro settings.

```bash
/maestro:configure                    # Interactive wizard
/maestro:configure model=opus         # Quick set
```

Settings:
- `model` - Default AI model (sonnet, opus, haiku)
- `workflow` - Workflow mode (sequential, parallel)
- `claude-hud` - Enable Claude-Hud integration
- `critical-think` - Enable Critical Think analysis

### Supporting Commands

#### `/maestro:tui`
Launch the Rust Cockpit TUI.

```bash
/maestro:tui
```

Opens the terminal-based Cockpit interface for visual project management.

#### `/maestro:leindex [args]`
Run Maestro LeIndex code analysis.

```bash
/maestro:leindex analyze ./src
/maestro:leindex search "authentication"
```

## Maestro Workflow

1. **Planning Phase** - Use `/maestro:newTrack` to create a track with spec.md and plan.md
2. **Implementation Phase** - Use `/maestro:implement` to execute the plan
3. **Review Phase** - Use `/maestro:status` to check progress
4. **Completion Phase** - Track is marked complete when all tasks are done

## Critical Think Integration

This extension integrates Maestro's Critical Think framework for metacognitive analysis:

- Before asking user questions
- Before generating documentation (spec/plan)
- Before implementation
- Before agent delegation
- After completing actions

Critical Think templates are automatically copied to `maestro/critical_think/templates/` during setup.

## File Compatibility

The extension creates the same file formats as Claude Code maestro commands:
- `spec.md` - Track specification
- `plan.md` - Implementation plan with tasks
- `metadata.json` - Track metadata
- `tracks.md` - Track registry

This means tracks can be worked on from either pi-mono or Claude Code.

## Architecture

```
pi-maestro/
├── src/
│   ├── index.ts              # Entry point - registers commands
│   ├── lib/
│   │   ├── project.ts        # Maestro project file I/O
│   │   ├── tracks.ts         # Track creation and management
│   │   ├── templates.ts      # Spec/plan generation
│   │   ├── criticalThink.ts  # Critical Think integration
│   │   └── cli.ts            # Maestro CLI wrapper
│   └── commands/
│       ├── setup.ts
│       ├── newTrack.ts       # CRITICAL COMMAND
│       ├── implement.ts
│       ├── orchestrate.ts
│       ├── status.ts
│       ├── revert.ts
│       ├── configure.ts
│       ├── tui.ts
│       └── leindex.ts
├── templates/                # Critical Think templates
└── dist/                     # Compiled output
```

## Requirements

- pi-mono CLI with extension support
- Node.js for running the TypeScript extension
- Maestro CLI (for TUI and LeIndex commands)

## License

MIT

## Contributing

This extension implements the same workflows as the Claude Code maestro commands found in `~/.claude/commands/maestro:*.md`.

For issues or questions, please refer to the main Maestro project.
