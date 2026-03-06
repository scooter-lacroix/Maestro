# TrackLens Editor

React app for plan/spec/walkthrough review and annotation - the main TrackLens editor UI.

## Overview

This package provides the React application for:
- Plan/spec review with annotation
- Walkthrough review with remediation loop
- Markdown rendering with Mermaid diagrams
- File tree and diff viewer
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

## Integration

The editor is served by TrackLens servers (Rust Axum server in `leindex-core` and Node/Bun server in `tracklens-server`). Authentication tokens are injected via `window.TRACKLENS_AUTH_TOKEN`.

## Status

✅ **Operational** - Ported from Plannotator to TrackLens. Full feature parity with rebranded identity.

## Known Limitations

- Requires server-side HTML injection for auth token
- Markdown rendering uses client-side libraries (Mermaid, ReactMarkdown)
- Diff viewer expects unified diff format from server
