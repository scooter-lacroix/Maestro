# TrackLens Hook for Claude Code

Claude Code integration for TrackLens - visual review, annotation, and walkthrough system.

## Overview

This is the Claude Code hook plugin that provides:
- PermissionRequest/ExitPlanMode hook binding
- Slash commands: `/tracklens-review`, `/tracklens-annotate`
- Browser-based UI for plan review, code review, and annotation

## Modes

1. **Plan Review (default)**: Triggered by ExitPlanMode hook
2. **Code Review**: `tracklens review` subcommand
3. **Annotate**: `tracklens annotate <file.md>` subcommand

## Development

```bash
# Build the hook bundle
npm run build

# Test hook locally
claude-hook-test
```

## Integration

- Plugin manifest: `.claude-plugin/plugin.json`
- Hook binding: `hooks/hooks.json`
- Slash commands: `commands/`

## Status

🚧 **Under Construction** - Porting from Plannotator to TrackLens
