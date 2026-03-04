# TrackLens OpenCode Plugin

OpenCode integration for TrackLens - visual review, annotation, and walkthrough system.

## Overview

This is the OpenCode plugin that provides:
- Tool registration: `tracklens`, `tracklens-review`, `tracklens-annotate`
- Agent switching support via `agentSwitch` response
- Integration with TrackLens server for review workflows

## Tools

1. **tracklens**: Main review tool (plan/spec/markdown review)
2. **tracklens-review**: Code review mode (git diff)
3. **tracklens-annotate**: Markdown annotation mode

## Development

```bash
# Build the plugin
npm run build

# Test with OpenCode
opencode-dev-test
```

## Integration

- Plugin entry point: `index.ts`
- Tool registration follows OpenCode plugin API
- Returns `agentSwitch` when needed for model switching

## Status

🚧 **Under Construction** - Porting from Plannotator to TrackLens
