# TrackLens Review Editor

React app for code review with git diff visualization and annotation.

## Overview

This package provides the React application for:
- Code review with git diff display
- Inline annotation and feedback
- File tree with changed files
- Diff viewer with syntax highlighting
- Review panel with approve/deny

## Development

```bash
# Start dev server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Usage

The review editor can be launched via:
- Claude Code: `/tracklens-review` command
- OpenCode: `tracklens_review` tool
- CLI: `maestro tracklens review --diff-type unified`

## Status

✅ **Operational** - Ported from Plannotator to TrackLens. Supports multiple diff types (unified, split, side-by-side).

## Known Limitations

- Backend wiring pending (Phase 10 - currently uses demo data)
- Requires git diff to be pre-generated and served via `/api/content`
- Syntax highlighting limited to languages supported by bundled highlighter
