# TrackLens System — Code-Level Analysis

> **Date:** 2026-04-12
> **Scope:** Full architectural and implementation analysis of TrackLens across all layers (Rust backend, TypeScript integration, React frontend).

---

## 1. Architecture Overview

TrackLens is Maestro's **local-first review system**. It launches a browser-based UI for reviewing specs, plans, walkthroughs, and code diffs. Unlike Ultraplan (which uses a remote CCR session), TrackLens runs a **local Axum HTTP server** and requires no authentication or remote connectivity.

### System Boundary Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         User's Browser                           │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  React SPA (packages/tracklens-editor)                     │  │
│  │  - parseMarkdownToBlocks() → typed Block[]                 │  │
│  │  - Viewer + AnnotationPanel + Settings                     │  │
│  │  - Mermaid diagram rendering                               │  │
│  │  - POST /api/decision  { approved, feedback, annotations } │  │
│  └───────────────┬────────────────────────────────────────────┘  │
│                  │ HTTP (localhost only)                          │
└──────────────────┼───────────────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────────────────┐
│  Axum Server (src/leindex/src/tracklens/server.rs)               │
│  - Routes: GET /, /api/content, /api/plan, /api/diff, ...        │
│  - POST /api/decision → watch::Sender<TrackLensDecision>         │
│  - POST /api/client-ready → watch::Sender<bool>                  │
│  - CORS restricted to localhost:{port}                           │
│  - HTML injection: client-ready bootstrap script into </body>    │
└──────────────────▲───────────────────────────────────────────────┘
                   │
┌──────────────────┴───────────────────────────────────────────────┐
│  CLI / Pi-Maestro Integration                                    │
│  ┌─────────────────────────┐  ┌────────────────────────────────┐ │
│  │ crates/cli/src/commands │  │ pi-maestro/src/tracklens/      │ │
│  │   /tracklens.rs         │  │   extension/tools.ts           │ │
│  │   - review              │  │   extension/command.ts         │ │
│  │   - walkthrough         │  │   walkthrough/generator.ts     │ │
│  │   - code-review         │  │   walkthrough/remediation.ts   │ │
│  └─────────────────────────┘  │   walkthrough/storage.ts       │ │
│                               └────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. Key Files

| File | Language | Role |
|------|----------|------|
| `src/leindex/src/tracklens/server.rs` | Rust | Axum HTTP server with review endpoints |
| `src/leindex/src/tracklens/types.rs` | Rust | Core types: `ReviewMode`, `TrackLensDecision`, `Annotation`, `AutonomyMode` |
| `src/leindex/src/tracklens/walkthrough.rs` | Rust | Walkthrough generation from track spec/plan + git history |
| `src/leindex/src/tracklens/mod.rs` | Rust | Module re-exports |
| `crates/cli/src/commands/tracklens.rs` | Rust | CLI subcommands: `review`, `walkthrough`, `code-review` |
| `pi-maestro/src/tracklens/extension/tools.ts` | TypeScript | Pi-Maestro tool registration (`tracklens_review`, `tracklens_walkthrough`) |
| `pi-maestro/src/tracklens/extension/command.ts` | TypeScript | `/tracklens` toggle command for Pi-Maestro |
| `pi-maestro/src/tracklens/walkthrough/generator.ts` | TypeScript | Walkthrough document generator (TS mirror of Rust impl) |
| `pi-maestro/src/tracklens/walkthrough/remediation.ts` | TypeScript | Annotation → remediation task conversion, validation, loop |
| `pi-maestro/src/tracklens/walkthrough/remediation-loop.ts` | TypeScript | Orchestrates the iterative review/remediation cycle |
| `pi-maestro/src/tracklens/walkthrough/storage.ts` | TypeScript | Compressed walkthrough persistence (`deflate + base64url`) |
| `pi-maestro/src/tracklens/walkthrough/types.ts` | TypeScript | Shared types: `WalkthroughOptions`, `ChangedFile`, `CompletedTask`, etc. |
| `packages/tracklens-editor/src/App.tsx` | React/TSX | Full React SPA — plan viewer, annotation panel, decision submission |
| `crates/cli/dist/tracklens-editor.html` | HTML | Bundled SPA served by the Axum server |

---

## 3. The Four Pillars — How TrackLens Works

### 3.1 Pillar 1: Content Loading

TrackLens supports three content pathways, each exposed as a CLI subcommand:

#### CLI Subcommands (`crates/cli/src/commands/tracklens.rs`)

```rust
pub enum TrackLensCommands {
    Review { file: PathBuf, mode: String, no_browser: bool },
    Walkthrough { track_id: String, full_diffs: bool, no_browser: bool },
    CodeReview { commit: String, no_browser: bool },
}
```

**`tracklens review`** — Reads a markdown file, parses the review mode (`review` | `code-review` | `annotate`). For `code-review` mode, runs `git diff -- <file>` (falling back to staged changes, then raw file content).

**`tracklens walkthrough`** — Reads `spec.md` and `plan.md` from `./maestro/tracks/<trackId>/`. Uses `WalkthroughGenerator` (Rust) to extract completed tasks (`- [x]` lines), get changed files via `git log --grep <trackId>`, and build a structured markdown walkthrough with file tables, snippets, and collapsible diffs.

**`tracklens code-review`** — Runs `git diff <commit>` and passes the raw patch as `ReviewContent` with `ReviewMode::CodeReview`.

All content is wrapped in the `ReviewContent` struct:

```rust
pub struct ReviewContent {
    pub mode: ReviewMode,        // Review | CodeReview | Annotate
    pub content: String,         // The markdown or diff text
    pub metadata: ReviewMetadata, // track_id, document_type, origin
}
```

#### Pi-Maestro Tools (`pi-maestro/src/tracklens/extension/tools.ts`)

Two tools are registered:
- **`tracklens_review`** — Accepts `markdown` content directly or via `filePath`, plus `documentType` and `mode` parameters.
- **`tracklens_walkthrough`** — Takes a `trackId`, generates the walkthrough via `generateWalkthrough()` (TS), and optionally starts an interactive TrackLens server with remediation loop support.

#### Path Traversal Protection

Track IDs are validated before filesystem access:

```rust
fn validate_track_id(track_id: &str) -> Result<()> {
    // Rejects: empty, contains '/' or '\\', contains '..', absolute paths
    // Allows only: alphanumeric, '-', '_'
}
```

On walkthrough approval, the output path is canonicalized and verified to stay within the tracks directory.

---

### 3.2 Pillar 2: Server Launch and Browser Opening

#### Server Construction (`server.rs`)

`TrackLensServer::new(config)` creates the Axum server with three `tokio::sync::watch` channels:
- **`decision_tx/rx`** — `Option<TrackLensDecision>` for approve/deny flow
- **`client_ready_tx/rx`** — `bool` for UI readiness detection
- **`deadline_tx/rx`** — `u64` (Unix timestamp) for timeout management

#### Port Binding Strategy (`start()`)

```rust
// Priority: config.port (if non-zero) → 3847 → 17579 → 3000 → OS-assigned (port 0)
let preferred_ports = [3847u16, 17579u16, 3000u16];
```

The actual port is read from `listener.local_addr()` after binding to eliminate race conditions.

#### Router Configuration

```rust
Router::new()
    .route("/",                  get(index))
    .route("/api/decision",      post(submit_decision))
    .route("/api/client-ready",  post(mark_client_ready))
    .route("/api/extend-timeout",post(extend_timeout))
    .route("/api/content",       get(get_content))
    .route("/api/plan",          get(get_plan))
    .route("/api/diff",          get(get_diff))
    .route("/api/status",        get(get_status))
    .route("/api/vaults",        get(get_vaults))
    .route("/api/agents",        get(get_agents))
```

#### Security Layers

- **CORS**: Restricted to `http://localhost:{port}` and `http://127.0.0.1:{port}` — only the dynamically determined port.
- **Compression**: `tower_http::CompressionLayer` for HTML/JSON responses.
- **Request body limit**: 100KB via `RequestBodyLimitLayer`.
- **Static assets**: Conditional `ServeDir`/`ServeFile` for `/assets/` and `/favicon.svg`.

#### Bundle Discovery (`find_bundle_dir()`)

Checks these paths in order for an `index.html`, `editor.html`, `tracklens-editor.html`, or `review.html`:

1. `~/.maestro/tracklens/` (installed location)
2. Next to the binary
3. `packages/tracklens-editor/dist` (development)
4. `apps/tracklens-hook/dist` (development)
5. `crates/cli/dist` (development)

#### HTML Injection and Browser Opening

The `index()` handler selects the appropriate HTML file based on `ReviewMode`, then injects a client-ready bootstrap script before `</body>`:

```javascript
// Injected into every served HTML page
function markClientReady() {
    fetch("/api/client-ready", {
        method: "POST",
        headers: { "Content-Type": "application/json" }
    });
}
if (document.readyState === 'complete') {
    setTimeout(markClientReady, 100);
} else {
    window.addEventListener('load', function() {
        setTimeout(markClientReady, 100);
    });
}
```

The browser is opened via `open::that(&url)` in a non-blocking `tokio::spawn`.

#### Client Readiness Detection

The CLI waits for the injected script to POST to `/api/client-ready`:

```rust
pub async fn wait_for_client_ready(&self, timeout_duration: Duration) -> Result<()> {
    // Uses tokio watch channel; timeout is configurable via
    // TRACKLENS_CLIENT_READY_TIMEOUT_MS env var (default: 20000ms)
}
```

---

### 3.3 Pillar 3: Content Rendering in the UI

#### React SPA (`packages/tracklens-editor/src/App.tsx`)

On mount, the app fetches content from `/api/plan`:

```typescript
fetch('/api/plan')
  .then(res => res.json())
  .then(data => {
    setMarkdown(data.plan);
    setIsApiMode(true);
    // Trigger permission/UI setup dialogs as needed
  })
  .catch(() => {
    setMarkdown(DEMO_PLAN); // Fallback demo content
  });
```

When markdown changes, it is parsed into typed blocks:

```typescript
useEffect(() => {
    const { frontmatter: fm } = extractFrontmatter(markdown);
    setFrontmatter(fm);
    setBlocks(parseMarkdownToBlocks(markdown));
}, [markdown]);
```

`parseMarkdownToBlocks()` (from `@maestro/tracklens-ui`) splits markdown into typed `Block[]` objects: `heading`, `code`, `paragraph`, `blockquote`, `list-item`, `hr`, `table`.

#### UI Layout

- **Left panel**: Resizable annotation list (240–480px, stored in `localStorage`)
- **Main area**: Rendered markdown via `Viewer` component with highlight/selection support
- **Right sidebar**: Table of contents + optional vault browser
- **Bottom toolbar**: Approve/Deny/Feedback buttons, timeout controls, export/settings

#### Features

- **Mermaid diagrams**: Rendered via `mermaid.initialize()` with dark theme (lazy-loaded by `MermaidBlock`)
- **Annotations**: Full CRUD — add (text selection), edit, delete. Each annotation carries `id`, `blockId`, `type` (comment/concern/suggestion), `text`, `author`, `timestamp`
- **Keyboard shortcuts**: `Cmd/Ctrl+Enter` (approve/deny), `Cmd/Ctrl+S` (save to notes)
- **Linked documents**: Cross-document navigation via `useLinkedDoc` hook
- **Vault browser**: Optional Obsidian vault integration
- **Agent switch**: Configurable per-review agent selection
- **Timeout management**: Visual countdown with extend button (`POST /api/extend-timeout`)
- **Export**: Annotations to Obsidian, Bear, or clipboard

---

### 3.4 Pillar 4: Decision Feedback

#### Client-Side Submission

The React app has three action paths, all posting to `/api/decision`:

**Approve:**
```typescript
body = {
    approved: true,
    permissionMode,        // (claude-code only)
    agentSwitch,           // (if agent selected)
    obsidian: { ... },     // (if Obsidian export enabled)
    bear: { ... },         // (if Bear export enabled)
    feedback,              // (if annotations present)
    annotations: JSON.stringify(annotations),
};
```

**Deny:**
```typescript
body = {
    approved: false,
    feedback: exportAnnotations(blocks, annotations, globalAttachments),
    annotations: JSON.stringify(annotations),
};
```

**Feedback:** Semantically identical to Deny but shown to the user as a softer action.

#### Server-Side Reception (`server.rs`)

```rust
async fn submit_decision(
    State(state): State<Arc<ServerState>>,
    Json(decision): Json<TrackLensDecision>,
) -> Result<impl IntoResponse, StatusCode> {
    state.decision_tx.send(Some(decision))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}
```

The CLI blocks on the watch channel:

```rust
pub async fn wait_for_decision(&self) -> Result<TrackLensDecision> {
    let mut rx = self.state.decision_rx.clone();
    loop {
        rx.changed().await?;
        if let Some(decision) = rx.borrow().as_ref() {
            return Ok(decision.clone());
        }
    }
}
```

#### Decision Types (`types.rs`)

```rust
pub struct TrackLensDecision {
    #[serde(alias = "approved")]
    pub behavior: DecisionBehavior,      // Allow | Deny
    pub annotations: Option<Vec<Annotation>>,
    #[serde(alias = "feedback")]
    pub feedback: Option<String>,
    pub autonomy_mode: Option<AutonomyMode>, // FullAuto | SemiAuto | Checkpoint
}
```

`DecisionBehavior` has a custom deserializer accepting both:
- String format: `"allow"` / `"deny"`
- Boolean format: `true` → Allow, `false` → Deny (legacy compatibility)

#### Annotation Model

```rust
pub struct Annotation {
    pub id: String,
    pub selection: TextSelection,   // start/end Position + selected text
    pub content: AnnotationContent, // comment + severity (Info|Warning|Error)
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct Position {
    pub line: usize,   // 1-indexed
    pub column: usize, // 1-indexed
}
```

#### CLI Exit Behavior

On `Deny`, the CLI returns a non-zero exit code so agents can detect rejection:

```rust
match decision.behavior {
    DecisionBehavior::Allow => Ok(()),
    DecisionBehavior::Deny => Err(anyhow!("Review denied. See annotations above.")),
}
```

#### Remediation Loop (Pi-Maestro)

The `tracklens_walkthrough` tool supports an iterative remediation cycle when a walkthrough is denied with annotations:

1. Annotations are converted to `RemediationTask[]` via `annotationToRemediationTask()` with priority classification (`high`/`medium`/`low`) and effort estimation.
2. Tasks are appended to `plan.md` as checklist items.
3. Walkthrough is regenerated via `generateWalkthrough()`.
4. Re-presented for review in a new TrackLens server instance.
5. Loop repeats up to `maxIterations` (default: 3 in tools, 5 in remediation-loop).

```typescript
// remediation-loop.ts
while (iteration < maxIterations) {
    iteration++;
    const walkthrough = generateWalkthrough({ trackId, root, trackDir, ... });
    const reviewResult = await onReview(walkthrough.markdown, iteration);
    if (reviewResult.approved) {
        saveFinalWalkthrough(trackDir, walkthrough.markdown);
        return { approved: true, totalIterations: iteration, ... };
    }
    const tasks = processWalkthroughReview(reviewResult, walkthrough);
    await executeRemediationTasks(tasks, trackDir);
}
```

---

## 4. Walkthrough Generation — Dual Implementation

### Rust Implementation (`walkthrough.rs`)

`WalkthroughGenerator` operates on the local filesystem and git history:

- **Task extraction**: Parses `- [x]` and `- [X]` lines from `plan.md`
- **Changed files**: `git log --all --oneline --grep <trackId> --name-status --diff-filter=ADMR`
- **Diff info**: Per-file `git log --all -p --grep <trackId> -- <file>`, counts `+`/`-` lines
- **Snippets**: Reads first N lines of each file (configurable via `max_snippet_lines`)
- **Metadata**: Extracts title from first `# ` heading, description from `## Description` section
- **Language detection**: Extension-based mapping (`.rs` → rust, `.ts`/`.tsx` → typescript, etc.)
- **Output**: Structured `Walkthrough` struct with `to_markdown()` renderer

### TypeScript Implementation (`generator.ts`)

`generateWalkthrough()` provides a parallel implementation for Pi-Maestro:

- **Task extraction**: Regex `^\s*-\s*\[x\]\s+(.+?)` with optional commit hash suffix, phase grouping via `## Phase N` headers
- **Changed files**: Uses batched git operations (`git diff --name-status`, `git diff --numstat`, `git diff` all in single calls) for better performance
- **Track start commit**: Three-tier lookup — `metadata.json` → structured commit prefix `[tracklens:trackName]` → fallback grep
- **Spec summary**: Extracts `## Overview` and `## Goals`/`## Objectives` sections (max 10 lines)
- **Storage**: Compressed via `deflate + base64url` to `.maestro/tracklens/walkthroughs/<trackId>.json`

### Key Differences Between Implementations

| Aspect | Rust | TypeScript |
|--------|------|------------|
| Task parsing | Simple `- [x]` prefix match | Regex with commit hash + phase grouping |
| Git strategy | Per-file `git log -p` calls | Batched `git diff --numstat` + single `git diff` |
| Track start commit | Searches by `track_id` in commit messages | Three-tier: metadata.json → structured prefix → grep |
| Output format | `Walkthrough` struct → `to_markdown()` | `GeneratedWalkthrough` interface → inline markdown |
| Persistence | None (CLI writes final file) | Compressed storage in `.maestro/tracklens/` |
| Remediation | CLI prints tasks, exits with error code | Full remediation loop with plan.md updates |

---

## 5. Strengths

1. **Local-first**: No remote session needed, no auth required, works fully offline.
2. **Rich annotation model**: Structured annotations with text selection (line/column positions), severity levels (`Info`/`Warning`/`Error`), and timestamped authorship.
3. **Multiple review modes**: `Review` (plans/specs), `CodeReview` (git diffs), `Annotate` (arbitrary markdown).
4. **Autonomy mode control**: Users can change agent autonomy (`FullAuto`/`SemiAuto`/`Checkpoint`) during review.
5. **Client readiness detection**: Server waits for UI to load via injected bootstrap script before proceeding, preventing race conditions.
6. **Remediation loop**: Denied walkthroughs trigger an iterative fix cycle — annotations become tasks, plan is updated, walkthrough is regenerated and re-presented (up to 3–5 iterations).
7. **Security hardening**: CORS locked to localhost with dynamic port, path traversal validation on track IDs, request body size limits (100KB), output path canonicalization.
8. **Timeout management**: Configurable via `TRACKLENS_CLIENT_READY_TIMEOUT_MS` env var, extendable from UI via `POST /api/extend-timeout`.
9. **Graceful degradation**: Pi-Maestro tools fall back to inline markdown review when the TrackLens server or HTML bundle is unavailable.
10. **Integration tests**: Server includes tests for creation, decision flow (HTML injection → JS bootstrap → decision POST → Rust state), and unique URL allocation.

---

## 6. Weaknesses and Risks

1. **No keyword auto-trigger**: Unlike Ultraplan, TrackLens is only invoked explicitly via tool call or CLI command. There is no automatic detection in user messages — if an agent doesn't explicitly call `tracklens_review` or `tracklens_walkthrough`, the review step is skipped entirely.

2. **Limited integration points**: Only wired into walkthrough and manual review. Not integrated into `maestro:setup`, `maestro:implement`, `maestro:orchestrate`, or other workflow phases. The `/tracklens` command state (`isTrackLensEnabled`) is checked in `implement.ts` and `orchestrate.ts` but there's no deeper integration.

3. **Fragile bundle discovery**: `find_bundle_dir()` checks five hardcoded paths sequentially. If none contain a recognized HTML file, the server starts but serves a "bundle missing" error page. There is no build step validation or startup-time check that logs a clear diagnostic.

4. **No plan editing in UI**: Unlike Ultraplan where the user edits the plan in the browser, TrackLens is primarily read-only with annotation overlay. Users cannot inline-edit the content — they can only annotate and approve/deny.

5. **No seed plan / iteration flow**: Ultraplan supports seed plans and multi-turn conversation for plan refinement. TrackLens is one-shot review per presentation — the remediation loop regenerates but doesn't allow back-and-forth editing.

6. **Server lifetime management**: The server is spawned via `tokio::spawn` with no graceful shutdown mechanism. It runs until the process exits. There is no explicit `server.stop()` on the Rust side (though the Pi-Maestro TS wrapper calls `server.stop()`).

7. **No progress/status polling**: Unlike Ultraplan's phase tracking (`running`/`needs_input`/`plan_ready`), TrackLens has no intermediate status communication to the agent. The CLI simply blocks on `wait_for_decision()`.

8. **Dual implementation drift risk**: Walkthrough generation exists in both Rust (`walkthrough.rs`) and TypeScript (`generator.ts`) with divergent behaviors (batched vs per-file git calls, phase grouping, start commit detection). Changes to one may not be propagated to the other.

9. **Legacy deserialization complexity**: `DecisionBehavior` accepts both `"allow"/"deny"` strings and `true/false` booleans, and `TrackLensDecision` has both `behavior` and `approved` as aliases. While backwards-compatible, this increases the surface area for deserialization bugs.

10. **Remediation loop max iterations inconsistency**: The `tracklens_walkthrough` tool in `tools.ts` uses `maxIterations: 3`, while `remediation-loop.ts` defaults to `maxIterations: 5`. This creates confusing behavior depending on the entry point.

---

## 7. Type Reference

### Rust Types (`types.rs`)

```rust
enum ReviewMode       { Review, CodeReview, Annotate }
enum DecisionBehavior { Allow, Deny }
enum AnnotationSeverity { Info, Warning, Error }
enum AutonomyMode     { FullAuto, SemiAuto, Checkpoint }

struct TrackLensDecision { behavior, annotations, feedback, autonomy_mode }
struct Annotation        { id, selection: TextSelection, content: AnnotationContent, timestamp }
struct TextSelection     { start: Position, end: Position, text: String }
struct Position          { line: usize, column: usize }
struct AnnotationContent { comment: String, severity: AnnotationSeverity }
```

### TypeScript Types (`walkthrough/types.ts`)

```typescript
interface WalkthroughOptions     { trackId, root, trackDir, isSubtrack?, includeDiffs?, includeSnippets?, maxSnippetLines? }
interface ChangedFile            { path, status: FileChangeStatus, language, diff?, snippet?, additions, deletions }
interface CompletedTask          { description, phase?, commit? }
interface WalkthroughMetadata    { trackId, description, type?, status, isSubtrack, parentTrackId?, generatedAt }
interface GeneratedWalkthrough   { markdown, metadata, completedTasks, changedFiles }
interface StoredWalkthrough      { metadata, compressed: string, version: 1 }
enum FileChangeStatus            { Added, Modified, Deleted, Renamed }
```

### Remediation Types (`remediation.ts`)

```typescript
interface WalkthroughAnnotation  { id, blockId, type: "comment"|"concern"|"suggestion", text?, originalText, created_a, author? }
interface RemediationTask        { description, annotation, priority: "high"|"medium"|"low", estimateHours }
interface WalkthroughReviewResult{ approved, feedback?, annotations?, savedPath? }
interface RemediationLoopResult  { approved, totalIterations, finalWalkthrough?, remediationTasks? }
```

---

## 8. API Endpoints Reference

| Method | Path | Handler | Purpose |
|--------|------|---------|---------|
| `GET` | `/` | `index()` | Serves HTML with injected client-ready script |
| `GET` | `/api/content` | `get_content()` | Returns full `ReviewContent` JSON |
| `GET` | `/api/plan` | `get_plan()` | Returns `{ plan: string }` for editor compat |
| `GET` | `/api/diff` | `get_diff()` | Returns `{ rawPatch, gitRef, origin, diffType, repoInfo }` |
| `GET` | `/api/status` | `get_status()` | Returns `{ status: "ok", version }` |
| `GET` | `/api/vaults` | `get_vaults()` | Returns `{ vaults: [] }` (placeholder) |
| `GET` | `/api/agents` | `get_agents()` | Returns hardcoded agent list for UI dropdown |
| `POST` | `/api/decision` | `submit_decision()` | Receives `TrackLensDecision`, sends via watch channel |
| `POST` | `/api/client-ready` | `mark_client_ready()` | Marks browser UI as loaded |
| `POST` | `/api/extend-timeout` | `extend_timeout()` | Extends deadline by `{ minutes }` |

---

## 9. Data Flow Summary

```
CLI invocation / Pi-Maestro tool call
  │
  ▼
Content loaded (file read / walkthrough generated / git diff)
  │
  ▼
ReviewContent { mode, content, metadata } created
  │
  ▼
TrackLensServer::new(config) → watch channels created
  │
  ▼
server.start() → bind port → build router → open browser
  │
  ▼
wait_for_client_ready() ← POST /api/client-ready (from injected JS)
  │
  ▼
React SPA fetches GET /api/plan → parseMarkdownToBlocks() → render
  │
  ▼
User reviews, annotates, clicks Approve/Deny/Feedback
  │
  ▼
POST /api/decision { approved/behavior, feedback, annotations, autonomy_mode }
  │
  ▼
wait_for_decision() unblocks → TrackLensDecision returned
  │
  ├─ Allow → CLI exits 0, walkthrough saved (if applicable)
  │
  └─ Deny  → CLI exits 1 with annotations
             Pi-Maestro: remediation loop (if walkthrough mode)
               → annotations → tasks → plan.md updated
               → regenerate walkthrough → re-present → repeat
```
