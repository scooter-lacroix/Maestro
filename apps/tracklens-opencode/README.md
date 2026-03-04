# TrackLens OpenCode Plugin

OpenCode integration for TrackLens - visual review, annotation, and walkthrough system.

## Overview

This is the OpenCode plugin that provides:
- **submit_plan**: Submit plan/spec for visual review with annotation
- **tracklens_review**: Code review mode (git diff with visual feedback)
- **tracklens_annotate**: Markdown annotation mode for documents
- Agent switching support via `agentSwitch` response
- Integration with TrackLens server for review workflows

## Tools

1. **submit_plan**: Plan/spec review tool
   - Opens TrackLens UI for plan review and annotation
   - Supports agent switching after approval
   - Returns formatted feedback on denial

2. **tracklens_review**: Code review mode
   - Launches git diff review with visual interface
   - Supports multiple diff types (uncommitted, staged, unstaged, last-commit, branch)
   - Captures annotations and feedback

3. **tracklens_annotate**: Document annotation mode
   - Opens markdown files for annotation
   - Supports any markdown document (specs, plans, walkthroughs)
   - Returns annotation feedback

## Usage

Agents can call these tools to request user review:
- Submit plans for approval before implementation
- Request code review with git diff visualization
- Annotate documents with visual feedback

## Development

```bash
# Build the plugin
bun run build

# Run tests
bun test
```

## Integration Points

- Plugin entry point: `src/index.ts`
- Tool registration follows OpenCode plugin API
- Returns `agentSwitch` when needed for model switching
- Uses TrackLens server packages for UI rendering

## Features

- Visual review interface with annotation support
- Agent switching on approval/denial
- Multiple git diff types for code review
- Document annotation for any markdown file
- Full TrackLens branding (no legacy Plannotator references)

## Status

✅ **Complete** - All three tools implemented and functional
