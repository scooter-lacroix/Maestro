# Maestro Commands

This directory contains the Maestro slash commands for Claude Code.

## Files

- `maestro:setup.md` - Initialize Maestro environment
- `maestro:newTrack.md` - Create new track with spec generation
- `maestro:implement.md` - Implement track with automatic agent selection
- `maestro:status.md` - View project progress
- `maestro:revert.md` - Revert work at track/phase/task level

## Installation

These files are copied to `~/.claude/commands/` by the installer script.

## Usage

In Claude Code, use the commands with the `/maestro:` prefix:

```
/maestro:setup
/maestro:newTrack <description>
/maestro:implement [track_name]
/maestro:status
/maestro:revert [track|phase|task]
```

## Documentation

See [../../docs/CLAUDE-CODE.md](../../docs/CLAUDE-CODE.md) for complete usage guide.
