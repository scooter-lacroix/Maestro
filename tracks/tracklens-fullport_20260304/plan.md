# Implementation Plan: TrackLens Full Port and Remediation

**Track:** tracklens-fullport_20260304  
**Last Updated:** 2026-03-05

---

## Phase 1: Component Inventory and Analysis (DONE)

### Task 1.1: Gap Analysis Document
- [x] Analyze UI components (14 fully ported, 5 partial, 21 missing)
- [x] Analyze hooks (7 fully ported, 2 partial, 3 missing)
- [x] Analyze utilities (10 fully ported, 3 partial, 2 missing)
- [x] Analyze editor features (6 fully ported, 5 partial, 8 missing)
- [x] Analyze review editor features (5 fully ported, 7 partial, 10 missing)
- [x] Analyze server modules (8 fully ported, 4 partial, 3 missing)
- [x] Document in `tracklens-remediation-analysis.md`

---

## Phase 2: UI Component Port (DONE)

### Task 2.1: Core Annotation Components
- [x] AnnotationPanel.tsx - Core annotation list and management
- [x] AnnotationSidebar.tsx - Sidebar wrapper for annotations
- [x] AnnotationToolbar.tsx - Text selection toolbar

### Task 2.2: Supporting Components
- [x] AttachmentsButton.tsx - Image attachment management
- [x] CompletionOverlay.tsx - Post-action completion screen
- [x] ConfirmDialog.tsx - Modal confirmation dialogs
- [x] ExportModal.tsx - Export functionality (partial - missing share tab)
- [x] ImageThumbnail.tsx - Thumbnail display for images
- [x] MermaidBlock.tsx - Mermaid diagram rendering
- [x] ModeSwitcher.tsx - Editor mode selection
- [x] ModeToggle.tsx - Light/dark theme toggle
- [x] ResizeHandle.tsx - Panel resize handle
- [x] Settings.tsx - Settings UI (simplified)
- [x] TableOfContents.tsx - Document navigation
- [x] ThemeProvider.tsx - Theme context provider
- [x] UIFeaturesSetup.tsx - Initial setup dialog
- [x] Viewer.tsx - Main document viewer (simplified, 416 vs 1243 lines)

---

## Phase 3: Review Editor Port (DONE)

### Task 3.1: Core Review Components
- [x] App.tsx - Review editor main application (simplified, 207 vs 931 lines)
- [x] DiffViewer.tsx - Simplified diff rendering
- [x] FileTree.tsx - Basic file tree
- [x] ReviewPanel.tsx - Simplified annotation panel
- [x] InlineAnnotation.tsx - Inline comment display
- [x] FileHeader.tsx - File path header
- [x] HighlightedCode.tsx - Syntax highlighting

### Task 3.2: Review Utils
- [x] detectLanguage.ts - Language detection
- [x] formatLineRange.ts - Line range formatting
- [x] patchParser.ts - Diff parsing (partial)
- [x] renderInlineMarkdown.tsx - Inline markdown rendering

---

## Phase 4: Server Completion (DONE)

### Task 4.1: Main Server (index.ts)
- [x] startTrackLensServer renamed from startPlannotatorServer
- [x] Port configuration (remote/local detection)
- [x] API routes (core endpoints)
- [x] Plan serving (/api/plan)
- [x] Decision endpoint (/api/decision with auth token)
- [x] Obsidian integration (vault detection and saving)
- [x] Bear integration (notes saving)
- [x] Image upload (/api/upload-image)
- [x] Image serving (/api/images/)
- [x] Vault tree generation
- [x] Project detection
- [x] VS Code diff integration
- [x] Git integration (repo info detection)

### Task 4.2: Review Server (review.ts)
- [x] startReviewServer implementation
- [x] Diff serving (/api/diff)
- [x] Diff switching (/api/switch-diff)
- [x] Image handling
- [x] Decision endpoint

### Task 4.3: Server Utilities
- [x] integrations.ts - Obsidian/Bear integration
- [x] git.ts - Git operations
- [x] image.ts - Image handling with sanitization
- [x] browser.ts - Browser opening
- [x] remote.ts - Remote detection
- [x] repo.ts - Repo info detection
- [x] project.ts - Project name detection
- [x] ide.ts - VS Code integration
- [x] storage.ts - Partial (version history removed)

---

## Phase 5: Editor Enhancement (DONE)

### Task 5.1: Editor Features
- [x] App.tsx - Editor main application (partial, 364 vs 1400+ lines)
- [x] Main layout (Header, panels, viewer)
- [x] Annotation management (add/edit/delete)
- [x] Export modal (partial - missing share tab)
- [x] UI features setup (initial configuration)
- [x] Keyboard shortcuts (basic only)
- [x] Mermaid rendering

### Task 5.2: Editor Utils
- [x] agentSwitch.ts - Agent switching logic
- [x] annotationHelpers.ts - Annotation formatting
- [x] bear.ts - Bear notes integration
- [x] defaultNotesApp.ts - Default notes app selection
- [x] editorMode.ts - Editor mode persistence
- [x] identity.ts - User identity management
- [x] obsidian.ts - Obsidian integration
- [x] parser.ts - Markdown parsing (simplified)
- [x] autonomyMode.ts - Permission mode renamed
- [x] docSave.ts - Plan save renamed
- [x] uiPreferences.ts - UI preference management

### Task 5.3: Editor Hooks
- [x] useActiveSection.ts - TOC section tracking
- [x] useAgents.ts - OpenCode agent querying
- [x] useAutoClose.ts - Auto-close timer hook
- [x] useDismissOnOutsideAndEscape.ts - Click/escape dismissal
- [x] useLinkedDoc.ts - Linked document navigation
- [x] usePlanDiff.ts - Plan diff (simplified)
- [x] useResizablePanel.ts - Panel resizing logic
- [x] useVaultBrowser.ts - Obsidian vault browsing (partial)

---

## Phase 6: Maestro Integration (IN PROGRESS)

### Task 6.1: Rust Types Module
- [ ] Create `src/leindex/src/tracklens/mod.rs` - Module declaration
- [ ] Create `src/leindex/src/tracklens/types.rs` - Core types:
  - TrackLensMode enum (Review, CodeReview, Annotate, Walkthrough)
  - TrackLensOrigin enum (ClaudeCode, OpenCode, PiMono, Maestro)
  - TrackLensServerOptions struct
  - TrackLensDecision struct
  - Annotation and CodeAnnotation structs
  - AutonomyMode enum (FullAuto, SemiAuto, Checkpoint)
  - WalkthroughConfig and ChangedFile structs

### Task 6.2: Axum HTTP Server
- [ ] Create `src/leindex/src/tracklens/server.rs`:
  - TrackLensServer struct with port, shutdown, decision channel
  - start() method with random port selection
  - wait_for_decision() blocking method
  - stop() graceful shutdown
  - Route handlers: serve_ui, get_state, handle_approve, handle_deny
  - Browser opening with MAESTRO_BROWSER env var

### Task 6.3: Walkthrough Generator
- [ ] Create `src/leindex/src/tracklens/walkthrough.rs`:
  - generate_walkthrough() function
  - extract_completed_tasks() helper
  - get_track_changed_files() using git log
  - get_file_diff_info() for stats and snippets
  - status_icon() and detect_language() helpers

### Task 6.4: Cockpit TUI Pane
- [ ] Create `crates/cockpit/src/tracklens/pane.rs`:
  - TrackLensPane struct with active status and history
  - ReviewStatus and ReviewHistoryEntry structs
  - render() method with status and history sections

### Task 6.5: CLI Commands
- [ ] Create `crates/cli/src/commands/tracklens.rs`:
  - TrackLensArgs and TrackLensCommand enums
  - Review, Walkthrough, CodeReview subcommands
  - run() implementation with server lifecycle
  - load_html_bundle() helper

### Task 6.6: Integration Wiring
- [ ] Wire TrackLens into newTrack workflow:
  - spec.md review checkpoint after Q&A phase
  - plan.md review checkpoint after generation
  - Final artifact review step
- [ ] Wire TrackLens into implement workflow:
  - Walkthrough generation on track completion
  - Remediation loop if denied
  - Save walkthrough-final.md on approval

---

## Phase 7: Build System (IN PROGRESS)

### Task 7.1: HTML Bundle Generation
- [ ] Configure build script for tracklens-editor HTML bundle
- [ ] Configure build script for tracklens-review-editor HTML bundle
- [ ] Set up embedded asset loading in Rust

### Task 7.2: Package Scripts
- [ ] Add build:tracklens script to package.json
- [ ] Add build:tracklens-review script
- [ ] Add watch mode for development

### Task 7.3: Distribution
- [ ] Ensure HTML bundles included in cargo package
- [ ] Set up static asset embedding (rust-embed or include_str!)

---

## Phase 8: Testing and Verification (PENDING)

### Task 8.1: Unit Tests
- [ ] Test TrackLensServer start/stop lifecycle
- [ ] Test decision channel communication
- [ ] Test walkthrough generation helpers
- [ ] Test type serialization/deserialization

### Task 8.2: Integration Tests
- [ ] Test /api/state endpoint
- [ ] Test /api/approve and /api/deny endpoints
- [ ] Test browser opening
- [ ] Test HTML bundle injection

### Task 8.3: End-to-End Tests
- [ ] Test maestro tracklens review <file>
- [ ] Test maestro tracklens walkthrough <track>
- [ ] Test maestro tracklens code-review
- [ ] Test full newTrack workflow with TrackLens

### Task 8.4: Cross-Platform Verification
- [ ] Verify Claude Code hook integration
- [ ] Verify OpenCode plugin integration
- [ ] Verify Pi-mono extension integration

### Task 8.5: Remaining Component Ports (Post-MVP)
- [ ] Plan diff visualization suite (7 components) - P1
- [ ] Sidebar multi-tab system (4 components) - P1
- [ ] Suggestion workflow (3 components) - P1
- [ ] useSharing.ts and sharing.ts - P0 (if sharing needed)
- [ ] annotate.ts server - P2

---

## Verification Checklist

After each phase:
1. `cargo check --workspace`
2. `cargo clippy --workspace --all-targets`
3. `cargo test --workspace`
4. `cargo build --workspace --release`

Final verification:
- [ ] All 12+ missing critical components addressed
- [ ] Multi-platform integration functional
- [ ] CLI commands working
- [ ] Build system generating correct bundles
