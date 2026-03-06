# TrackLens Remediation Analysis

## Executive Summary

This document provides a comprehensive gap analysis between the original **Plannotator** project and the current **Maestro TrackLens** implementation. The analysis identifies what has been fully ported, partially ported, or completely missing across all major components.

**Last Updated:** 2025-03-05

---

## 1. UI Components Analysis

### Source (Plannotator): `packages/ui/components/`
### Target (TrackLens): `packages/tracklens-ui/src/components/`

| Component | Status | Notes |
|-----------|--------|-------|
| **AnnotationPanel.tsx** | FULLY PORTED | Core annotation list and management |
| **AnnotationSidebar.tsx** | FULLY PORTED | Sidebar wrapper for annotations |
| **AnnotationToolbar.tsx** | FULLY PORTED | Text selection toolbar |
| **AttachmentsButton.tsx** | FULLY PORTED | Image attachment management |
| **CompletionOverlay.tsx** | FULLY PORTED | Post-action completion screen |
| **ConfirmDialog.tsx** | FULLY PORTED | Modal confirmation dialogs |
| **ExportModal.tsx** | PARTIALLY PORTED | Missing "share" tab functionality |
| **ImageAnnotator/** | PARTIALLY PORTED | Canvas.tsx exists but simplified |
| **ImageThumbnail.tsx** | FULLY PORTED | Thumbnail display for images |
| **ImportModal.tsx** | MISSING | URL import functionality not ported |
| **Landing.tsx** | MISSING | Marketing landing page excluded |
| **MermaidBlock.tsx** | FULLY PORTED | Mermaid diagram rendering |
| **ModeSwitcher.tsx** | FULLY PORTED | Editor mode selection |
| **ModeToggle.tsx** | FULLY PORTED | Light/dark theme toggle |
| **ResizeHandle.tsx** | FULLY PORTED | Panel resize handle |
| **Settings.tsx** | PARTIALLY PORTED | Simplified settings UI |
| **TableOfContents.tsx** | FULLY PORTED | Document navigation |
| **ThemeProvider.tsx** | FULLY PORTED | Theme context provider |
| **UIFeaturesSetup.tsx** | FULLY PORTED | Initial setup dialog |
| **UpdateBanner.tsx** | MISSING | Auto-update check banner not needed |
| **Viewer.tsx** | PARTIALLY PORTED | Simplified 416 lines vs 1243 lines |

### Plan-Diff Components (plannotator/packages/ui/components/plan-diff/)

| Component | Status | Notes |
|-----------|--------|-------|
| **PlanCleanDiffView.tsx** | MISSING | Clean diff visualization |
| **PlanDiffBadge.tsx** | MISSING | Version comparison badge |
| **PlanDiffMarketing.tsx** | MISSING | Feature marketing dialog |
| **PlanDiffModeSwitcher.tsx** | MISSING | Diff mode selector |
| **PlanDiffViewer.tsx** | MISSING | Full diff viewer component |
| **PlanRawDiffView.tsx** | MISSING | Raw diff display |
| **VSCodeIcon.tsx** | MISSING | VS Code integration icon |

### Sidebar Components (plannotator/packages/ui/components/sidebar/)

| Component | Status | Notes |
|-----------|--------|-------|
| **SidebarContainer.tsx** | MISSING | Multi-tab sidebar container |
| **SidebarTabs.tsx** | MISSING | Tab navigation for sidebar |
| **VaultBrowser.tsx** | MISSING | Obsidian vault file browser |
| **VersionBrowser.tsx** | MISSING | Plan version history browser |

### Image Annotator Sub-components

| Component | Status | Notes |
|-----------|--------|-------|
| **Canvas.tsx** | PARTIALLY PORTED | Simplified canvas implementation |
| **index.tsx** | MISSING | Main ImageAnnotator component |
| **Toolbar.tsx** | MISSING | Drawing toolbar |
| **types.ts** | MISSING | Type definitions |
| **utils.ts** | MISSING | Canvas utilities |

### Summary: UI Components

- **Fully Ported:** 14 components
- **Partially Ported:** 5 components
- **Missing:** 21 components

**Critical Gaps:**
1. Plan diff visualization system (7 components)
2. Sidebar multi-tab system (4 components)
3. Image annotation toolbar and utilities
4. URL import modal
5. Landing/marketing page

---

## 2. Hooks Analysis

### Source (Plannotator): `packages/ui/hooks/`
### Target (TrackLens): `packages/tracklens-ui/src/hooks/`

| Hook | Status | Notes |
|------|--------|-------|
| **useActiveSection.ts** | FULLY PORTED | TOC section tracking |
| **useAgents.ts** | FULLY PORTED | OpenCode agent querying |
| **useAutoClose.ts** | FULLY PORTED | Auto-close timer hook |
| **useDismissOnOutsideAndEscape.ts** | FULLY PORTED | Click/escape dismissal |
| **useLinkedDoc.ts** | FULLY PORTED | Linked document navigation |
| **usePlanDiff.ts** | PARTIALLY PORTED | Simplified version |
| **useResizablePanel.ts** | FULLY PORTED | Panel resizing logic |
| **useSharing.ts** | MISSING | URL sharing functionality |
| **useSidebar.ts** | MISSING | Sidebar state management |
| **useUpdateCheck.ts** | MISSING | Version checking |
| **useVaultBrowser.ts** | PARTIALLY PORTED | Obsidian vault browsing |

### Summary: Hooks

- **Fully Ported:** 7 hooks
- **Partially Ported:** 2 hooks
- **Missing:** 3 hooks

**Critical Gaps:**
1. `useSharing.ts` - URL-based state sharing (381 lines)
2. `useSidebar.ts` - Multi-tab sidebar state management
3. `useUpdateCheck.ts` - Auto-update checking

---

## 3. Utils Analysis

### Source (Plannotator): `packages/ui/utils/`
### Target (TrackLens): `packages/tracklens-ui/src/utils/`

| Utility | Status | Notes |
|---------|--------|-------|
| **agentSwitch.ts** | FULLY PORTED | Agent switching logic |
| **annotationHelpers.ts** | FULLY PORTED | Annotation formatting |
| **bear.ts** | FULLY PORTED | Bear notes integration |
| **defaultNotesApp.ts** | FULLY PORTED | Default notes app selection |
| **editorMode.ts** | FULLY PORTED | Editor mode persistence |
| **identity.ts** | FULLY PORTED | User identity management |
| **obsidian.ts** | FULLY PORTED | Obsidian integration |
| **parser.ts** | PARTIALLY PORTED | Markdown parsing (simplified) |
| **permissionMode.ts** | RENAMED | Now `autonomyMode.ts` |
| **planDiffEngine.ts** | PARTIALLY PORTED | Diff computation |
| **planDiffMarketing.ts** | MISSING | Marketing dialog logic |
| **planSave.ts** | RENAMED | Now `docSave.ts` |
| **sharing.ts** | MISSING | URL sharing compression |
| **storage.ts** | PARTIALLY PORTED | Local storage wrapper |
| **uiPreferences.ts** | FULLY PORTED | UI preference management |

### Summary: Utils

- **Fully Ported:** 10 utilities
- **Partially Ported:** 3 utilities
- **Missing:** 2 utilities
- **Renamed:** 2 utilities

**Critical Gaps:**
1. `sharing.ts` - URL compression/decompression for sharing (10,154 bytes in original)
2. `planDiffMarketing.ts` - Feature marketing logic

---

## 4. Editor Package Analysis

### Source (Plannotator): `packages/editor/`
### Target (TrackLens): `packages/tracklens-editor/`

| Feature | Status | Notes |
|---------|--------|-------|
| **App.tsx** | PARTIALLY PORTED | 364 lines vs 1400+ lines |
| **Main layout** | FULLY PORTED | Header, panels, viewer |
| **Annotation management** | FULLY PORTED | Add/edit/delete annotations |
| **Export modal** | PARTIALLY PORTED | Missing share tab |
| **Permission mode setup** | RENAMED | Now "AutonomyModeSetup" |
| **UI features setup** | FULLY PORTED | Initial configuration |
| **Keyboard shortcuts** | PARTIALLY PORTED | Basic shortcuts only |
| **Linked documents** | MISSING | Wiki-link navigation |
| **Tater mode** | MISSING | Easter egg animation mode |
| **Plan diff** | MISSING | Version comparison |
| **Vault browser** | MISSING | Obsidian vault integration |
| **Import modal** | MISSING | Import from share URL |
| **Global attachments** | PARTIALLY PORTED | Simplified image handling |
| **Copy/paste handling** | MISSING | Clipboard image paste |
| **Mermaid rendering** | FULLY PORTED | Diagram support |

### Summary: Editor

- **Fully Ported:** 6 features
- **Partially Ported:** 5 features
- **Missing:** 8 features

**Critical Gaps:**
1. Linked document navigation system
2. Tater mode animations and sprites
3. Plan diff/version comparison
4. Vault browser integration
5. Import from share URL
6. Advanced keyboard shortcuts (Cmd+S, etc.)

---

## 5. Review Editor Package Analysis

### Source (Plannotator): `packages/review-editor/`
### Target (TrackLens): `packages/tracklens-review-editor/`

| Feature | Status | Notes |
|---------|--------|-------|
| **App.tsx** | PARTIALLY PORTED | 207 lines vs 931 lines |
| **DiffViewer.tsx** | PARTIALLY PORTED | Simplified diff rendering |
| **FileTree.tsx** | PARTIALLY PORTED | Basic file tree only |
| **ReviewPanel.tsx** | PARTIALLY PORTED | Simplified annotation panel |
| **AnnotationToolbar.tsx** | MISSING | Line selection toolbar |
| **Diff switching** | PARTIALLY PORTED | Basic implementation |
| **InlineAnnotation.tsx** | FULLY PORTED | Inline comment display |
| **FileHeader.tsx** | FULLY PORTED | File path header |
| **HighlightedCode.tsx** | FULLY PORTED | Syntax highlighting |
| **SuggestionModal.tsx** | MISSING | Suggestion creation modal |
| **SuggestionBlock.tsx** | MISSING | Suggestion display |
| **SuggestionDiff.tsx** | MISSING | Suggestion diff view |
| **Export feedback** | PARTIALLY PORTED | Basic markdown export |
| **Keyboard shortcuts** | MISSING | Cmd/Ctrl+Enter, etc. |
| **Viewed files tracking** | MISSING | File review status |
| **Hide viewed files** | MISSING | Filter option |
| **Approve warning** | MISSING | Annotation loss warning |
| **Agent switch** | PARTIALLY PORTED | Simplified |
| **Settings integration** | PARTIALLY PORTED | Basic settings only |

### Utils Comparison

| Utility | Status | Notes |
|---------|--------|-------|
| **detectLanguage.ts** | FULLY PORTED | Language detection |
| **formatLineRange.ts** | FULLY PORTED | Line range formatting |
| **patchParser.ts** | PARTIALLY PORTED | Diff parsing |
| **renderInlineMarkdown.tsx** | FULLY PORTED | Inline markdown rendering |

### Hooks Comparison

| Hook | Status | Notes |
|------|--------|-------|
| **useAnnotationToolbar.ts** | MISSING | Toolbar positioning |
| **useTabIndent.ts** | MISSING | Tab indentation handling |

### Summary: Review Editor

- **Fully Ported:** 5 features
- **Partially Ported:** 7 features
- **Missing:** 10 features

**Critical Gaps:**
1. Suggestion modal and display system
2. Advanced keyboard shortcuts
3. Viewed files tracking
4. Annotation toolbar positioning
5. Complete diff switching UI

---

## 6. Server Package Analysis

### Source (Plannotator): `packages/server/` + `apps/hook/server/`
### Target (TrackLens): `packages/tracklens-server/`

### Main Server (index.ts)

| Feature | Status | Notes |
|---------|--------|-------|
| **startPlannotatorServer** | RENAMED | Now `startTrackLensServer` |
| **Port configuration** | FULLY PORTED | Remote/local detection |
| **API routes** | PARTIALLY PORTED | Core routes implemented |
| **Plan serving** | FULLY PORTED | `/api/plan` endpoint |
| **Decision endpoints** | MODIFIED | `/api/decision` with auth token |
| **Obsidian integration** | FULLY PORTED | Vault detection and saving |
| **Bear integration** | FULLY PORTED | Bear notes saving |
| **Image upload** | FULLY PORTED | `/api/upload-image` endpoint |
| **Image serving** | FULLY PORTED | `/api/images/` endpoint |
| **Vault tree** | FULLY PORTED | File tree generation |
| **Project detection** | FULLY PORTED | Project name detection |
| **VS Code diff** | FULLY PORTED | Editor integration |
| **Git integration** | FULLY PORTED | Repo info detection |
| **Version history** | MISSING | Plan versioning system |
| **Share URL generation** | MISSING | Not needed for TrackLens |
| **Paste service** | MISSING | Short URL service |
| **Agent querying** | MISSING | OpenCode agent list |
| **Document serving** | MISSING | Linked doc endpoint |
| **Approval with notes** | MODIFIED | Different endpoint structure |

### Review Server (review.ts)

| Feature | Status | Notes |
|---------|--------|-------|
| **startReviewServer** | FULLY PORTED | Review server implementation |
| **Diff serving** | FULLY PORTED | `/api/diff` endpoint |
| **Diff switching** | MODIFIED | `/api/switch-diff` endpoint |
| **Image handling** | FULLY PORTED | Upload and serve images |
| **Decision endpoint** | MODIFIED | Uses auth token |
| **Agent querying** | MISSING | `/api/agents` endpoint |
| **Feedback endpoint** | MODIFIED | Consolidated into `/api/decision` |

### Annotate Server (annotate.ts)

| Feature | Status | Notes |
|---------|--------|-------|
| **startAnnotateServer** | MISSING | Annotate mode server |
| **File watching** | MISSING | File change detection |
| **Markdown serving** | MISSING | Single file annotation |

### Server Files Comparison

| File | Status | Notes |
|------|--------|-------|
| **index.ts** | PARTIALLY PORTED | 592 lines vs 706 lines |
| **review.ts** | PARTIALLY PORTED | 310 lines vs 323 lines |
| **annotate.ts** | MISSING | 7,608 bytes not ported |
| **storage.ts** | PARTIALLY PORTED | Version history removed |
| **integrations.ts** | FULLY PORTED | Obsidian/Bear integration |
| **git.ts** | FULLY PORTED | Git operations |
| **image.ts** | MODIFIED | Added sanitization |
| **browser.ts** | FULLY PORTED | Browser opening |
| **remote.ts** | FULLY PORTED | Remote detection |
| **repo.ts** | FULLY PORTED | Repo info detection |
| **project.ts** | FULLY PORTED | Project name detection |
| **ide.ts** | FULLY PORTED | VS Code integration |
| **share-url.ts** | MISSING | URL sharing utilities |

### Summary: Server

- **Fully Ported:** 8 modules
- **Partially Ported:** 4 modules
- **Missing:** 3 modules
- **Renamed/Modified:** 3 modules

**Critical Gaps:**
1. `annotate.ts` - Standalone file annotation server
2. `share-url.ts` - URL sharing for remote sessions
3. Version history system
4. Paste service integration

---

## 7. Package.json Dependencies

### Plannotator UI Dependencies

```json
{
  "name": "@plannotator/ui",
  "dependencies": {
    "@plannotator/web-highlighter": "^1.x",
    "highlight.js": "^11.x",
    "mermaid": "^10.x"
  }
}
```

### TrackLens UI Dependencies

```json
{
  "name": "@maestro/tracklens-ui",
  "dependencies": {
    "@maestro/tracklens-web-highlighter": "^1.x",
    "highlight.js": "^11.x",
    "mermaid": "^10.x"
  }
}
```

**Note:** Dependencies are equivalent with rebranded package names.

---

## 8. Type Definitions

### Plannotator Types (packages/ui/types.ts)

| Type | Status | Notes |
|------|--------|-------|
| **Block** | FULLY PORTED | Document block structure |
| **Annotation** | FULLY PORTED | Annotation data model |
| **AnnotationType** | FULLY PORTED | Type enum |
| **EditorMode** | FULLY PORTED | Mode enum |
| **CodeAnnotation** | FULLY PORTED | Code review annotation |
| **ImageAttachment** | FULLY PORTED | Image attachment model |
| **SelectedLineRange** | FULLY PORTED | Line selection for diffs |
| **Frontmatter** | FULLY PORTED | YAML frontmatter |

---

## 9. Test Coverage

| Test File | Status | Notes |
|-----------|--------|-------|
| **Viewer.test.tsx** | MISSING | Component tests not ported |
| **project.test.ts** | MISSING | Server tests not ported |
| **security.test.ts** | PRESENT | New in TrackLens |
| **crypto.test.ts** | MISSING | Shared crypto tests |

---

## 10. Critical Remediation Priorities

### Priority 1 (Blocking)

1. **Sharing System** - Implement URL-based sharing if needed
2. **Version History** - Restore plan versioning for iterative development
3. **Linked Documents** - Restore wiki-link navigation

### Priority 2 (High Value)

4. **Plan Diff** - Restore version comparison visualization
5. **Vault Browser** - Restore Obsidian vault file browser
6. **Review Editor Features** - Complete suggestion modal and toolbar

### Priority 3 (Polish)

7. **Tater Mode** - Easter egg animations
8. **Import Modal** - Share URL import
9. **Update Banner** - Version checking

### Priority 4 (Nice to Have)

10. **Marketing Components** - Landing page, marketing dialogs
11. **Advanced Keyboard Shortcuts** - Power user features
12. **File Watching** - Annotate mode file monitoring

---

## 11. Lines of Code Comparison

| Component | Plannotator | TrackLens | Ratio |
|-----------|-------------|-----------|-------|
| Viewer.tsx | 1,243 lines | 416 lines | 33% |
| Editor App.tsx | ~1,400 lines | 364 lines | 26% |
| Review App.tsx | 931 lines | 207 lines | 22% |
| Server index.ts | 706 lines | 592 lines | 84% |
| Server review.ts | 323 lines | 310 lines | 96% |
| useSharing.ts | 381 lines | 0 lines | 0% |
| sharing.ts | ~300 lines | 0 lines | 0% |

**Overall Code Reduction:** ~60-70% of original functionality

---

## 12. API Endpoint Comparison

### Plannotator Endpoints

```
GET  /api/plan
POST /api/approve
POST /api/deny
POST /api/feedback
POST /api/save-notes
GET  /api/plan/version
GET  /api/plan/versions
GET  /api/plan/history
GET  /api/doc
GET  /api/image
POST /api/upload
POST /api/plan/vscode-diff
GET  /api/obsidian/vaults
GET  /api/reference/obsidian/files
GET  /api/reference/obsidian/doc
GET  /api/agents
POST /api/save-notes

GET  /api/diff
POST /api/diff/switch
POST /api/feedback
```

### TrackLens Endpoints

```
GET  /api/plan
POST /api/decision
POST /api/save
POST /api/obsidian
POST /api/bear
GET  /api/vaults
GET  /api/project
POST /api/validate-image
POST /api/upload-image
GET  /api/images/:filename
POST /api/vault-tree
POST /api/open-diff

GET  /api/diff
POST /api/switch-diff
POST /api/decision
```

**Key Changes:**
- Consolidated approve/deny/feedback into `/api/decision`
- Added authentication token to decision endpoint
- Removed paste service endpoints
- Simplified vault endpoints

---

## Appendix A: File Mapping Reference

### Component Mappings

| Plannotator Path | TrackLens Path |
|------------------|----------------|
| `packages/ui/components/Viewer.tsx` | `packages/tracklens-ui/src/components/Viewer.tsx` |
| `packages/ui/components/AnnotationPanel.tsx` | `packages/tracklens-ui/src/components/AnnotationPanel.tsx` |
| `packages/ui/components/Settings.tsx` | `packages/tracklens-ui/src/components/Settings.tsx` |
| `packages/editor/App.tsx` | `packages/tracklens-editor/src/App.tsx` |
| `packages/review-editor/App.tsx` | `packages/tracklens-review-editor/src/App.tsx` |
| `packages/server/index.ts` | `packages/tracklens-server/src/index.ts` |
| `packages/server/review.ts` | `packages/tracklens-server/src/review.ts` |

### Hook Mappings

| Plannotator Path | TrackLens Path |
|------------------|----------------|
| `packages/ui/hooks/useResizablePanel.ts` | `packages/tracklens-ui/src/hooks/useResizablePanel.ts` |
| `packages/ui/hooks/useLinkedDoc.ts` | `packages/tracklens-ui/src/hooks/useLinkedDoc.ts` |

### Util Mappings

| Plannotator Path | TrackLens Path |
|------------------|----------------|
| `packages/ui/utils/parser.ts` | `packages/tracklens-ui/src/utils/parser.ts` |
| `packages/ui/utils/obsidian.ts` | `packages/tracklens-ui/src/utils/obsidian.ts` |
| `packages/ui/utils/permissionMode.ts` | `packages/tracklens-ui/src/utils/autonomyMode.ts` |

---

## Appendix B: Rebrand Mapping

| Plannotator | TrackLens |
|-------------|-----------|
| `@plannotator/ui` | `@maestro/tracklens-ui` |
| `@plannotator/server` | `@maestro/tracklens-server` |
| `@plannotator/web-highlighter` | `@maestro/tracklens-web-highlighter` |
| `startPlannotatorServer` | `startTrackLensServer` |
| `PLANNOTATOR_PORT` | `TRACKLENS_PORT` |
| `PLANNOTATOR_REMOTE` | `TRACKLENS_REMOTE` |
| `permissionMode` | `autonomyMode` |
| `plannotator-tater-mode` | `tracklens-tater-mode` |

---

## Conclusion

The TrackLens implementation represents a **streamlined subset** of Plannotator's functionality, with approximately **60-70% of features ported**. The core annotation, review, and server systems are functional, but several advanced features are missing:

### What's Working:
- Basic plan annotation and review
- Code diff visualization
- Obsidian/Bear integration
- Image attachments
- Theme switching
- Settings management

### What's Missing:
- URL-based sharing system
- Plan version history and diff
- Linked document navigation
- Vault browser
- Suggestion workflow
- Tater mode

### Recommendation:
For Maestro's use case, the current implementation is **sufficient** for track-based plan review. The missing features (sharing, version history) may not be required for the integrated Maestro workflow. However, if users need:

1. **Collaborative sharing** - Port the `useSharing.ts` and `sharing.ts` modules
2. **Version comparison** - Port the plan-diff component suite
3. **Knowledge base integration** - Port the vault browser and linked documents

