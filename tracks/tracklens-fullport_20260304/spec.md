# Track: TrackLens Full Port and Remediation

**Track ID:** tracklens-fullport_20260304  
**Created:** 2026-03-04  
**Status:** IN_PROGRESS  
**Priority:** P0-P2  
**Estimated Complexity:** Very High (40+ files, 4 major subsystems)

---

## Goal

Complete the port and remediation of Plannotator to TrackLens, achieving full web UI reconstruction with comprehensive Maestro integration. This track finalizes the rebranding, ports all remaining components, and integrates TrackLens into the Maestro workflow (newTrack, implement, orchestrate).

## Scope

### Phase 1: Component Inventory and Analysis (DONE)
- Comprehensive gap analysis between Plannotator and TrackLens
- Identification of 14 fully ported, 5 partially ported, and 21 missing UI components
- Server API endpoint comparison and gap identification
- Editor and review editor feature gap analysis

### Phase 2: UI Component Port (DONE)
- Core annotation components (AnnotationPanel, AnnotationSidebar, AnnotationToolbar)
- Attachment and completion components
- Modal dialogs (ConfirmDialog, ExportModal)
- Theme and layout components (ThemeProvider, ModeSwitcher, ResizeHandle)
- Table of contents and navigation

### Phase 3: Review Editor Port (DONE)
- Basic diff viewer implementation
- File tree and review panel
- Inline annotation display
- Syntax highlighting via HighlightedCode

### Phase 4: Server Completion (DONE)
- Core HTTP server with Axum
- Plan serving endpoints
- Image upload and serving
- Obsidian/Bear integration endpoints
- Git integration and VS Code diff

### Phase 5: Editor Enhancement (DONE)
- Mermaid diagram rendering
- Basic annotation management
- Settings UI
- Image attachment handling

### Phase 6: Maestro Integration (IN PROGRESS)
- newTrack integration: spec.md and plan.md review checkpoints
- implement integration: walkthrough generation and review
- orchestrate integration: TrackLens decision routing
- CLI commands: `maestro tracklens review|walkthrough|code-review`

### Phase 7: Build System (IN PROGRESS)
- HTML bundle generation and embedding
- Package scripts and build pipeline
- Asset bundling for distribution

### Phase 8: Testing and Verification (PENDING)
- Component unit tests
- Integration tests for server endpoints
- End-to-end workflow validation
- Cross-platform testing (Claude Code, OpenCode, Pi-mono)

## Requirements

Based on `plannotator-port-plan.md`:

### Functional Requirements
1. **Multi-Platform Support**: Claude Code (hook + slash commands), OpenCode (plugin), Pi-mono (extension)
2. **Review Modes**: Plan/spec review, code review (git diff), markdown annotation, walkthrough
3. **Annotation Types**: Comment, deletion, insertion, replacement, global comment
4. **Integrations**: Obsidian vault, Bear notes, VS Code diff
5. **Autonomy Modes**: FullAuto, SemiAuto, Checkpoint (permission mode mapping)

### Missing Components to Port

#### Critical (P0)
- `useSharing.ts` - URL-based state sharing (381 lines)
- `sharing.ts` - URL compression/decompression utilities
- `annotate.ts` server - Standalone file annotation server

#### High Value (P1)
- Plan diff visualization system (7 components):
  - PlanCleanDiffView.tsx
  - PlanDiffBadge.tsx
  - PlanDiffMarketing.tsx
  - PlanDiffModeSwitcher.tsx
  - PlanDiffViewer.tsx
  - PlanRawDiffView.tsx
  - VSCodeIcon.tsx
- Sidebar multi-tab system (4 components):
  - SidebarContainer.tsx
  - SidebarTabs.tsx
  - VaultBrowser.tsx
  - VersionBrowser.tsx
- Suggestion workflow (review editor):
  - SuggestionModal.tsx
  - SuggestionBlock.tsx
  - SuggestionDiff.tsx

#### Medium Priority (P2)
- Image annotator sub-components:
  - Toolbar.tsx
  - types.ts
  - utils.ts
- Advanced keyboard shortcuts (Cmd+S, Cmd+Enter, etc.)
- Viewed files tracking in review editor

#### Low Priority (P3)
- Tater mode animations and sprites
- Import modal for share URLs
- Update banner
- Landing/marketing components

## Integration Points

### newTrack Workflow
```
Step 3.6: TrackLens spec.md review checkpoint
  → Present drafted spec.md for visual review
  → User annotates, approves, or requests changes
  → If denied: parse annotations → LLM revises → re-present

Step 4.5: TrackLens plan.md review checkpoint
  → Same flow for plan.md

Step 5.7: Final consolidated review
  → Review all artifacts before finalization
```

### implement Workflow
```
Step 4.0: Finalize track with walkthrough
  → Generate walkthrough from completed tasks and git history
  → Present via TrackLens for review
  → If denied: remediate → regenerate → re-present
  → On approval: save walkthrough-final.md
```

### orchestrate Integration
- TrackLens decision routing to appropriate agents
- Agent switching for OpenCode integration
- Permission mode updates from review decisions

## Dependencies
- §6 Maestro Integration → §7 Build System
- §7 Build System → §8 Testing
- HTML bundle assets must be built before integration testing

## Success Criteria
- All P0 and P1 missing components ported
- `cargo check --workspace` — zero errors
- `cargo clippy --workspace --all-targets` — zero new warnings
- `cargo test --workspace` — all tests pass
- `maestro tracklens review <file.md>` functional
- `maestro tracklens walkthrough <track-id>` functional
- `maestro tracklens code-review` functional
- Multi-platform integration verified (Claude Code, OpenCode, Pi-mono)
