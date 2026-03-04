# Maestro TrackLens Integration Plan (Revised v2)

## Porting & Rebranding of Plannotator → TrackLens

> **TrackLens** — Maestro's integrated visual review, annotation, and walkthrough system for track creation and completion workflows, operating across **Claude Code**, **OpenCode**, and **Pi-mono**.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Plannotator Architecture Analysis](#2-plannotator-architecture-analysis)
3. [Maestro Integration Points](#3-maestro-integration-points)
4. [Rebranding Strategy](#4-rebranding-strategy)
5. [Integration Area 1: Track Creation (newTrack)](#5-integration-area-1-track-creation-newtrack)
6. [Integration Area 2: Track Completion Walkthroughs](#6-integration-area-2-track-completion-walkthroughs)
7. [Multi-Platform Integration (Claude Code + OpenCode + Pi)](#7-multi-platform-integration)
8. [Rust Port Plan](#8-rust-port-plan)
9. [File-by-File Porting Manifest](#9-file-by-file-porting-manifest)
10. [Implementation Phases](#10-implementation-phases)
11. [Testing Strategy](#11-testing-strategy)

---

## 1. Executive Summary

Plannotator is ported into Maestro as **TrackLens** with full multi-platform support:

- **Claude Code** — via hooks (`PermissionRequest`/`ExitPlanMode`) and slash commands (`/tracklens-review`, `/tracklens-annotate`)
- **OpenCode** — via plugin (`@maestro/tracklens-opencode`) with tool registration
- **Pi-mono** — via pi-maestro extension with tool and command registration

Two core integration points:

1. **Track Creation (`maestro:newTrack`)** — Visual review UI at each approval checkpoint (spec.md, plan.md, tracks.md). Does NOT affect the Q&A question-gathering phase — only activates after a document has been drafted for user approval.

2. **Track/Subtrack Completion** — Auto-generated walkthroughs with code snippets and file references, presented for annotation-based remediation loops.

---

## 2. Plannotator Architecture Analysis

### 2.1 Monorepo Structure

```
plannotator/
├── apps/
│   ├── hook/                 # Claude Code integration ← PORTED
│   │   ├── .claude-plugin/   #   plugin.json manifest
│   │   ├── commands/         #   /plannotator-review, /plannotator-annotate slash commands
│   │   ├── hooks/            #   hooks.json (PermissionRequest → ExitPlanMode)
│   │   ├── server/           #   CLI entry: plan review, code review, annotate modes
│   │   └── index.tsx         #   React entry (renders editor App)
│   ├── opencode-plugin/      # OpenCode integration ← PORTED
│   │   └── index.ts          #   Plugin with tool registration, git diff, review/annotate
│   ├── pi-extension/         # Pi-mono integration ← PORTED
│   │   ├── index.ts          #   Extension: phases, flags, commands, tools, hooks
│   │   ├── server.ts         #   Node HTTP servers for plan/review/annotate
│   │   └── utils.ts          #   Checklist parsing, bash safety, progress tracking
│   ├── paste-service/        # E2E encrypted paste service ← REMOVED (external service)
│   ├── portal/               # Web portal for shared plans ← REMOVED
│   └── marketing/            # Marketing site ← REMOVED
├── packages/
│   ├── editor/               # Plan editor React app ← PORTED
│   ├── review-editor/        # Code review React app ← PORTED
│   ├── server/               # Server utilities ← PORTED
│   ├── ui/                   # Shared React UI components ← PORTED (full)
│   ├── shared/               # Compression ← PORTED (compress.ts)
│   └── web-highlighter/      # Text selection/highlight ← PORTED
```

### 2.2 Claude Code Hook Architecture (`apps/hook/`)

The hook operates as a **Claude Code plugin** with three modes:

1. **Plan Review (default)** — Triggered by `PermissionRequest`/`ExitPlanMode` hook
   - Reads hook event JSON from stdin (`event.tool_input.plan`)
   - Starts `startPlannotatorServer()` with the plan content
   - Outputs JSON decision to stdout: `{ hookSpecificOutput: { decision: { behavior: "allow"|"deny" } } }`
   - Supports `updatedPermissions` for permission mode changes

2. **Code Review** — `plannotator review` subcommand, triggered by `/plannotator-review` slash command
   - Runs `git diff`, opens review UI, outputs feedback to stdout

3. **Annotate** — `plannotator annotate <file.md>` subcommand, triggered by `/plannotator-annotate`
   - Opens any markdown in annotation UI, outputs feedback to stdout

**Plugin manifest** (`plugin.json`):
```json
{
  "name": "plannotator",
  "description": "Interactive Plan Review...",
  "version": "0.10.0"
}
```

**Hook binding** (`hooks.json`):
```json
{
  "hooks": {
    "PermissionRequest": [{
      "matcher": "ExitPlanMode",
      "hooks": [{ "type": "command", "command": "plannotator", "timeout": 345600 }]
    }]
  }
}
```

### 2.3 OpenCode Plugin Architecture (`apps/opencode-plugin/`)

Registers as an `@opencode-ai/plugin` with tools:
- `plannotator` tool — reviews plans via `startPlannotatorServer()`
- `plannotator-review` tool — reviews git diffs via `startReviewServer()`  
- `plannotator-annotate` tool — annotates markdown via `startAnnotateServer()`
- Respects `sharingEnabled`, `PLANNOTATOR_SHARE_URL`, config from `ctx.client.config`
- Agent switching: returns `agentSwitch` field for OpenCode to route to different agents

### 2.4 Core Data Flow

```
User triggers review → Server starts on random port →
Browser opens UI → User annotates → POST /api/approve or /api/deny →
Server resolves promise → Feedback returned to agent → Server stops
```

### 2.5 Key Types (from `packages/ui/types.ts`)

```typescript
type EditorMode = 'selection' | 'comment' | 'redline';

interface Annotation {
  id: string;
  blockId: string;
  type: AnnotationType; // 'COMMENT' | 'DELETION' | 'INSERTION' | 'REPLACEMENT' | 'GLOBAL_COMMENT'
  text?: string;
  originalText: string;
  createdAt: number;
  author?: string;
  images?: ImageAttachment[];
  startMeta?: object;  // web-highlighter cross-element metadata
  endMeta?: object;
}

interface Block {
  id: string;
  type: 'paragraph' | 'heading' | 'blockquote' | 'list-item' | 'code' | 'hr' | 'table';
  content: string;
  level?: number;
  language?: string;
  checked?: boolean;
  order: number;
  startLine: number;
}

interface CodeAnnotation {
  id: string;
  type: CodeAnnotationType; // 'comment' | 'suggestion' | 'concern'
  filePath: string;
  lineStart: number;
  lineEnd: number;
  side: 'old' | 'new';
  text?: string;
  suggestedCode?: string;
  originalCode?: string;
}
```

---

## 3. Maestro Integration Points

### 3.1 Track Creation Flow (`newTrack.ts`)

```
Step 3.0: INTERACTIVE SPEC GENERATION
  3.1-3.5: Q&A Phase (AskUserQuestion tool) ← UNCHANGED
  3.6: User Confirmation ← TRACKLENS REPLACES THIS STEP ONLY

Step 4.0: INTERACTIVE PLAN GENERATION
  4.1-4.4: LLM generates plan.md ← UNCHANGED
  4.5: User Confirmation ← TRACKLENS REPLACES THIS STEP ONLY

Step 5.0: CREATE TRACK ARTIFACTS
  5.7: Final review of all artifacts ← TRACKLENS NEW STEP
```

**CRITICAL:** The Q&A question-gathering phase (LLM asking questions via `AskUserQuestion` / `ctx.ui.select` / text fallback) is **completely untouched**. TrackLens only activates at document review checkpoints AFTER the LLM has drafted a document.

### 3.2 Track Completion Flow (`implement.ts`)

```
Step 4.0: FINALIZE TRACK
  → Generate walkthrough document
  → Present via TrackLens for review
  → If denied: remediate → new walkthrough → re-present
  → If approved: finalize track
```

---

## 4. Rebranding Strategy

### 4.1 Name Mapping

| Plannotator Term | TrackLens Term | Rationale |
|---|---|---|
| `plannotator` | `tracklens` | Core brand |
| `Plannotator` | `TrackLens` | PascalCase |
| `@plannotator/*` | `@maestro/tracklens-*` | Package scope |
| `/plannotator-review` | `/tracklens-review` | Slash commands |
| `/plannotator-annotate` | `/tracklens-annotate` | Slash commands |
| `~/.plannotator/` | `~/.maestro/tracklens/` | Storage |
| `PLANNOTATOR_BROWSER` | `MAESTRO_BROWSER` | Env var |
| `PLANNOTATOR_REMOTE` | `TRACKLENS_REMOTE` | Env var |
| `PLANNOTATOR_PORT` | `TRACKLENS_PORT` | Env var |
| `PLANNOTATOR_SHARE` | Removed | No external sharing |
| `PLANNOTATOR_SHARE_URL` | Removed | No external sharing |
| `PLANNOTATOR_PASTE_URL` | Removed | No external sharing |
| `plannotator.ai` | Removed | No external service |
| `share.plannotator.ai` | Removed | No external sharing |
| `tater` / `TaterSprite*` | Removed | Plannotator mascot |
| `backnotprop` | Removed | Author references |

### 4.2 Internal Code Renaming

```
startPlannotatorServer  → startTrackLensServer
startReviewServer       → startTrackLensReviewServer
startAnnotateServer     → startTrackLensAnnotateServer
plannotator()           → tracklens()  (pi extension entry)
getPlanDir()            → getTrackLensDir()
savePlan()              → saveDocument()
generateSlug()          → generateDocSlug()
```

---

## 5. Integration Area 1: Track Creation (`newTrack`)

### 5.1 Modified Workflow (Q&A Phase Unchanged)

```
┌──────────────────────────────────────────────────────┐
│                  maestro:newTrack                      │
├──────────────────────────────────────────────────────┤
│  1. Setup Check                                       │
│  2. Get Track Description (AskUserQuestion — NO CHANGE)│
│  3. Interactive Spec Generation                       │
│     3a. Q&A Phase — LLM asks 3-5 questions            │
│         Uses AskUserQuestion / ctx.ui.select / text   │
│         ★ COMPLETELY UNCHANGED                        │
│     3b. Draft spec.md (LLM generates — NO CHANGE)     │
│     3c. ★ TrackLens: Present spec.md for visual review│
│         → Browser opens, user annotates               │
│         → Approve → proceed to plan                   │
│         → Deny + annotations → LLM revises, re-present│
│  4. Interactive Plan Generation                       │
│     4a. Generate plan.md (LLM — NO CHANGE)            │
│     4b. ★ TrackLens: Present plan.md for visual review│
│  5. Create Track Artifacts                            │
│     5a. Generate metadata.json, tracks.md entry       │
│     5b. ★ TrackLens: Final consolidated review        │
│  6. Master Track Integration                          │
│     6a. Each subtrack spec/plan reviewed individually │
└──────────────────────────────────────────────────────┘
```

### 5.2 `tracklens_review` Tool (Pi-mono)

```typescript
// pi-maestro/src/tracklens/extension/tools.ts

export function registerTrackLensTools(pi: ExtensionAPI) {
  pi.registerTool("tracklens_review", {
    description: "Present a document for visual review and annotation via TrackLens",
    parameters: {
      type: "object",
      properties: {
        markdown: { type: "string", description: "Markdown content to review" },
        documentType: { type: "string", description: "Document type label" },
        trackId: { type: "string", description: "Track ID for context" },
        mode: { type: "string", enum: ["review", "walkthrough"] },
      },
      required: ["markdown", "documentType", "mode"],
    },
    async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
      const { markdown, documentType, trackId, mode } = params;
      const htmlBundle = readFileSync(join(__dirname, "../../dist/tracklens-editor.html"), "utf-8");

      const server = startTrackLensServer({
        markdown, documentType, trackId, mode,
        origin: "maestro",
        htmlContent: htmlBundle,
      });

      openBrowser(server.url);
      ctx.ui.notify(`TrackLens: Reviewing ${documentType} at ${server.url}`);

      const decision = await server.waitForDecision();
      server.stop();

      if (decision.approved) {
        return {
          content: [{ type: "text", text: `${documentType} approved via TrackLens.` }],
          details: { approved: true },
        };
      } else {
        return {
          content: [{ type: "text", text: `${documentType} requires changes:\n\n${decision.feedback}` }],
          details: { approved: false, feedback: decision.feedback },
        };
      }
    },
  });
}
```

### 5.3 Modified Workflow Instructions (`newTrack.ts`)

The `buildNewTrackWorkflow()` string adds after step 3.5 (spec drafted):

```
## 3.6 USER REVIEW VIA TRACKLENS

After drafting spec.md, present it for visual review:

1. Call the `tracklens_review` tool with:
   - `markdown`: the full spec.md content
   - `documentType`: "spec.md"
   - `trackId`: the track ID (if known)
   - `mode`: "review"

2. The tool opens a browser-based review UI where the user can:
   - Read with syntax highlighting and code block rendering
   - Add inline comments, deletions, replacements
   - Add global comments
   - Approve or Deny

3. **If approved:** Proceed to Step 4 (plan generation)
4. **If denied with feedback:**
   - Parse the structured annotations
   - Revise spec.md according to each annotation
   - Re-present via `tracklens_review`
   - Repeat until approved

NOTE: This does NOT replace the Q&A questions in steps 3.1-3.2.
Those questions still use AskUserQuestion / ctx.ui.select / text fallback.
```

---

## 6. Integration Area 2: Track Completion Walkthroughs

### 6.1 Walkthrough Generation

```typescript
// pi-maestro/src/tracklens/walkthrough/generator.ts

export function generateWalkthrough(options: WalkthroughOptions): string {
  const { trackId, root, trackDir, isSubtrack, parentTrackId } = options;
  const metadata = JSON.parse(readFileSync(join(trackDir, "metadata.json"), "utf-8"));
  const specContent = readFileSync(join(trackDir, "spec.md"), "utf-8");
  const planContent = readFileSync(join(trackDir, "plan.md"), "utf-8");
  const changedFiles = getTrackChangedFiles(root, trackId);
  const completedTasks = extractCompletedTasks(planContent);

  let doc = `# Track Walkthrough: ${metadata.description}\n\n`;
  doc += `**Track ID:** \`${trackId}\`\n`;
  doc += `**Type:** ${metadata.type}\n**Status:** Completed\n`;
  if (isSubtrack && parentTrackId) doc += `**Parent:** \`${parentTrackId}\`\n`;
  doc += `\n---\n\n## Completed Tasks\n\n`;
  for (const task of completedTasks) doc += `- [x] ${task}\n`;

  doc += `\n## Files Changed\n\n| Status | File | Lines |\n|--------|------|-------|\n`;
  for (const file of changedFiles) {
    doc += `| ${statusIcon(file.status)} | [\`${file.path}\`](${file.path}) | ${countDiffLines(file.diff)} |\n`;
  }

  doc += `\n## Detailed Changes\n\n`;
  for (const file of changedFiles) {
    doc += `### ${file.path}\n\n`;
    if (file.snippet) doc += `\`\`\`${file.language}\n${file.snippet}\n\`\`\`\n\n`;
    if (file.diff) doc += `<details><summary>Full diff</summary>\n\n\`\`\`diff\n${file.diff}\n\`\`\`\n</details>\n\n`;
  }
  doc += `---\n> Review this walkthrough. Annotate issues for remediation.\n`;
  return doc;
}
```

### 6.2 Remediation Loop (in `implement.ts` workflow)

```
## 4.0 FINALIZE TRACK WITH WALKTHROUGH

1. Call `tracklens_walkthrough` with the track ID
2. TrackLens generates walkthrough and presents for review
3. If denied:
   a. Parse annotations → each becomes a remediation task
   b. Execute remediations
   c. Generate NEW walkthrough reflecting changes
   d. Re-present via TrackLens
   e. Repeat until approved
4. On approval: update status, bank memory, save walkthrough-final.md
```

---

## 7. Multi-Platform Integration

### 7.1 Claude Code — Hook + Slash Commands

**Ported from `apps/hook/`** → `apps/tracklens-hook/`

**Plugin manifest** (`plugin.json`):
```json
{
  "name": "tracklens",
  "description": "TrackLens: Visual review, annotation, and walkthrough for Maestro tracks",
  "version": "1.0.0",
  "author": { "name": "maestro" },
  "repository": "https://github.com/scooter-lacroix/maestro"
}
```

**Hook binding** (`hooks.json`):
```json
{
  "hooks": {
    "PermissionRequest": [{
      "matcher": "ExitPlanMode",
      "hooks": [{ "type": "command", "command": "tracklens", "timeout": 345600 }]
    }]
  }
}
```

**CLI entry** (`server/index.ts`):
```typescript
// Three modes (same architecture as plannotator hook):
// 1. Plan Review (default) — reads hook event from stdin, outputs decision JSON
// 2. Code Review — `tracklens review` — git diff review
// 3. Annotate — `tracklens annotate <file.md>` — markdown annotation

import { startTrackLensServer, handleServerReady } from "@maestro/tracklens-server";
import { startTrackLensReviewServer, handleReviewServerReady } from "@maestro/tracklens-server/review";
import { startTrackLensAnnotateServer, handleAnnotateServerReady } from "@maestro/tracklens-server/annotate";
import { getGitContext, runGitDiff } from "@maestro/tracklens-server/git";

if (args[0] === "review") {
  // CODE REVIEW MODE — identical flow, rebranded
  const gitContext = await getGitContext();
  const { patch, label, error } = await runGitDiff("uncommitted", gitContext.defaultBranch);
  const server = await startTrackLensReviewServer({
    rawPatch: patch, gitRef: label, error,
    origin: "claude-code", diffType: "uncommitted", gitContext,
    htmlContent: reviewHtmlContent,
    onReady: (url, isRemote, port) => handleReviewServerReady(url, isRemote, port),
  });
  const result = await server.waitForDecision();
  await Bun.sleep(1500);
  server.stop();
  console.log(result.feedback || "No feedback provided.");

} else if (args[0] === "annotate") {
  // ANNOTATE MODE — identical flow, rebranded
  // ... (same as plannotator but with TrackLens naming)

} else {
  // PLAN REVIEW MODE — reads stdin hook event
  const event = JSON.parse(await Bun.stdin.text());
  const planContent = event.tool_input?.plan || "";
  const permissionMode = event.permission_mode || "default";

  const server = await startTrackLensServer({
    plan: planContent, origin: "claude-code",
    permissionMode, htmlContent: planHtmlContent,
    onReady: (url, isRemote, port) => handleServerReady(url, isRemote, port),
  });

  const result = await server.waitForDecision();
  server.stop();

  // Output PermissionRequest decision JSON
  console.log(JSON.stringify({
    hookSpecificOutput: {
      hookEventName: "PermissionRequest",
      decision: result.approved
        ? { behavior: "allow", ...(result.permissionMode && {
            updatedPermissions: [{ type: "setMode", mode: result.permissionMode, destination: "session" }]
          })}
        : { behavior: "deny", message: result.feedback || "Changes requested" },
    },
  }));
}
```

**Slash commands** (`.md` files):

`commands/tracklens-review.md`:
```markdown
---
description: Open interactive code review for current changes
allowed-tools: Bash(tracklens:*)
---
## Code Review Feedback
!`tracklens review`
## Your task
Address the code review feedback above.
```

`commands/tracklens-annotate.md`:
```markdown
---
description: Open interactive annotation UI for a markdown file
allowed-tools: Bash(tracklens:*)
---
## Markdown Annotations
!`tracklens annotate $ARGUMENTS`
## Your task
Address the annotation feedback above.
```

### 7.2 OpenCode — Plugin

**Ported from `apps/opencode-plugin/`** → `apps/tracklens-opencode/`

```typescript
// apps/tracklens-opencode/index.ts

import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin";
import { startTrackLensServer, handleServerReady } from "@maestro/tracklens-server";
import { startTrackLensReviewServer, handleReviewServerReady } from "@maestro/tracklens-server/review";
import { startTrackLensAnnotateServer, handleAnnotateServerReady } from "@maestro/tracklens-server/annotate";
import { getGitContext, runGitDiff } from "@maestro/tracklens-server/git";

export default {
  name: "tracklens",
  tools: [
    tool({ name: "tracklens", ... }),          // Plan review
    tool({ name: "tracklens-review", ... }),     // Code review
    tool({ name: "tracklens-annotate", ... }),   // Markdown annotation
  ],
  commands: [
    { name: "tracklens-review", description: "Review code changes" },
    { name: "tracklens-annotate", description: "Annotate a markdown file" },
  ],
} satisfies Plugin;
```

**Agent switching preserved:** The OpenCode plugin continues to return `agentSwitch` in decisions, allowing users to route feedback to different OpenCode agents. For Claude Code, maestro agents (sonnet-specialist, amp-code, etc.) are invoked by the orchestrating agent as applicable.

### 7.3 Pi-mono — Extension

**Ported from `apps/pi-extension/`** → integrated into `pi-maestro/src/tracklens/`

The pi-mono extension registers `tracklens_review` and `tracklens_walkthrough` tools plus the `/tracklens` toggle command and plan-mode phases, as detailed in Sections 5-6.

---

## 8. Rust Port Plan

### 8.1 Components Ported to Rust

For Cockpit TUI and CLI integration:

#### 8.1.1 Module Declaration — `src/leindex/src/tracklens/mod.rs`

```rust
// src/leindex/src/tracklens/mod.rs

pub mod server;
pub mod walkthrough;
pub mod types;

pub use server::TrackLensServer;
pub use walkthrough::generate_walkthrough;
pub use types::*;
```

#### 8.1.2 Core Types — `src/leindex/src/tracklens/types.rs`

```rust
// src/leindex/src/tracklens/types.rs

use serde::{Deserialize, Serialize};

/// Modes the TrackLens server can operate in
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackLensMode {
    Review,       // Plan/spec review (annotation UI)
    CodeReview,   // Git diff review (code review UI)
    Annotate,     // Generic markdown annotation
    Walkthrough,  // Track completion walkthrough
}

/// Originating platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackLensOrigin {
    ClaudeCode,
    OpenCode,
    PiMono,
    Maestro,     // CLI / Cockpit TUI
}

/// Server startup options
#[derive(Debug, Clone)]
pub struct TrackLensServerOptions {
    pub markdown: String,
    pub document_type: String,
    pub track_id: Option<String>,
    pub mode: TrackLensMode,
    pub origin: TrackLensOrigin,
    pub html_content: String,
    pub port: Option<u16>,         // None = random available port
    pub open_browser: bool,        // defaults to true
}

/// User decision returned after review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackLensDecision {
    pub approved: bool,
    pub feedback: Option<String>,
    pub annotations: Vec<Annotation>,
    pub autonomy_mode: Option<AutonomyMode>,
    pub agent_switch: Option<String>,   // OpenCode agent routing
}

/// Individual annotation from the review UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub block_id: String,
    #[serde(rename = "type")]
    pub annotation_type: AnnotationType,
    pub text: Option<String>,
    pub original_text: String,
    pub created_at: u64,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnnotationType {
    Comment,
    Deletion,
    Insertion,
    Replacement,
    GlobalComment,
}

/// Maestro autonomy levels (merged from plannotator permission modes)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutonomyMode {
    FullAuto,    // was: bypassPermissions — auto-approve all tool calls
    SemiAuto,    // was: acceptEdits — auto-approve file edits, confirm others
    Checkpoint,  // was: default — confirm everything
}

/// Code review annotation (for diff review mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnnotation {
    pub id: String,
    #[serde(rename = "type")]
    pub annotation_type: CodeAnnotationType,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub side: DiffSide,
    pub text: Option<String>,
    pub suggested_code: Option<String>,
    pub original_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeAnnotationType {
    Comment,
    Suggestion,
    Concern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffSide {
    Old,
    New,
}

/// Walkthrough generation configuration
#[derive(Debug, Clone)]
pub struct WalkthroughConfig {
    pub track_id: String,
    pub root: std::path::PathBuf,
    pub track_dir: std::path::PathBuf,
    pub is_subtrack: bool,
    pub parent_track_id: Option<String>,
    pub include_diffs: bool,         // include full git diffs
    pub include_snippets: bool,      // include key code snippets
    pub max_snippet_lines: usize,    // default 30
}

/// A changed file entry for walkthrough
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileChangeStatus,
    pub language: String,
    pub diff: Option<String>,
    pub snippet: Option<String>,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}
```

#### 8.1.3 Axum HTTP Server — `src/leindex/src/tracklens/server.rs`

```rust
// src/leindex/src/tracklens/server.rs

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{info, warn};

use super::types::*;

/// TrackLens HTTP server — serves the review UI and waits for a user decision
pub struct TrackLensServer {
    port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
    decision_rx: Option<mpsc::Receiver<TrackLensDecision>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

struct AppState {
    options: TrackLensServerOptions,
    decision_tx: Mutex<Option<mpsc::Sender<TrackLensDecision>>>,
}

impl TrackLensServer {
    /// Start the server on the specified or a random available port.
    pub async fn start(options: TrackLensServerOptions) -> anyhow::Result<Self> {
        let (decision_tx, decision_rx) = mpsc::channel::<TrackLensDecision>(1);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let state = Arc::new(AppState {
            options: options.clone(),
            decision_tx: Mutex::new(Some(decision_tx)),
        });

        let app = Router::new()
            .route("/", get(serve_ui))
            .route("/api/state", get(get_state))
            .route("/api/approve", post(handle_approve))
            .route("/api/deny", post(handle_deny))
            .with_state(state);

        let port = options.port.unwrap_or(0);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let actual_port = listener.local_addr()?.port();

        info!("TrackLens server listening on http://127.0.0.1:{actual_port}");

        let join_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        if options.open_browser {
            let url = format!("http://127.0.0.1:{actual_port}");
            if let Err(e) = open_browser(&url).await {
                warn!("Failed to open browser: {e}. Visit {url} manually.");
            }
        }

        Ok(Self {
            port: actual_port,
            shutdown_tx: Some(shutdown_tx),
            decision_rx: Some(decision_rx),
            join_handle: Some(join_handle),
        })
    }

    /// URL the server is listening on.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Block until the user approves or denies in the UI.
    pub async fn wait_for_decision(&mut self) -> anyhow::Result<TrackLensDecision> {
        let rx = self.decision_rx.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Decision already consumed"))?;
        rx.recv().await
            .ok_or_else(|| anyhow::anyhow!("Server closed without decision"))
    }

    /// Gracefully shut down the server.
    pub fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
    }
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// Serve the pre-built HTML bundle with injected state
async fn serve_ui(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut html = state.options.html_content.clone();
    // Inject initial state as a script tag so React can hydrate
    let init_json = serde_json::json!({
        "markdown": state.options.markdown,
        "documentType": state.options.document_type,
        "trackId": state.options.track_id,
        "mode": state.options.mode,
        "origin": state.options.origin,
    });
    let script = format!(
        r#"<script>window.__TRACKLENS_INIT__={};</script>"#,
        serde_json::to_string(&init_json).unwrap_or_default()
    );
    html = html.replace("</head>", &format!("{script}</head>"));
    Html(html)
}

/// Return current server state as JSON
async fn get_state(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "markdown": state.options.markdown,
        "documentType": state.options.document_type,
        "trackId": state.options.track_id,
        "mode": state.options.mode,
    }))
}

/// Handle POST /api/approve
async fn handle_approve(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let decision = TrackLensDecision {
        approved: true,
        feedback: body.get("feedback").and_then(|v| v.as_str()).map(String::from),
        annotations: serde_json::from_value(
            body.get("annotations").cloned().unwrap_or(serde_json::json!([]))
        ).unwrap_or_default(),
        autonomy_mode: body.get("autonomyMode")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        agent_switch: body.get("agentSwitch").and_then(|v| v.as_str()).map(String::from),
    };
    send_decision(&state, decision).await;
    StatusCode::OK
}

/// Handle POST /api/deny
async fn handle_deny(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let feedback = body.get("feedback").and_then(|v| v.as_str()).map(String::from);
    let annotations: Vec<Annotation> = serde_json::from_value(
        body.get("annotations").cloned().unwrap_or(serde_json::json!([]))
    ).unwrap_or_default();

    let merged_feedback = build_structured_feedback(&feedback, &annotations);

    let decision = TrackLensDecision {
        approved: false,
        feedback: Some(merged_feedback),
        annotations,
        autonomy_mode: None,
        agent_switch: body.get("agentSwitch").and_then(|v| v.as_str()).map(String::from),
    };
    send_decision(&state, decision).await;
    StatusCode::OK
}

async fn send_decision(state: &AppState, decision: TrackLensDecision) {
    let mut guard = state.decision_tx.lock().await;
    if let Some(tx) = guard.take() {
        let _ = tx.send(decision).await;
    }
}

/// Build structured feedback string from annotations for LLM consumption
fn build_structured_feedback(feedback: &Option<String>, annotations: &[Annotation]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(fb) = feedback {
        if !fb.is_empty() { parts.push(format!("**Global Feedback:**\n{fb}")); }
    }
    for ann in annotations {
        let label = match ann.annotation_type {
            AnnotationType::Comment => "COMMENT",
            AnnotationType::Deletion => "DELETE",
            AnnotationType::Insertion => "INSERT",
            AnnotationType::Replacement => "REPLACE",
            AnnotationType::GlobalComment => "GLOBAL",
        };
        let text = ann.text.as_deref().unwrap_or("");
        parts.push(format!(
            "- [{label}] block `{}`: original=\"{}\" → {}",
            ann.block_id, ann.original_text, text
        ));
    }
    parts.join("\n\n")
}

/// Open browser using MAESTRO_BROWSER env var or system default
async fn open_browser(url: &str) -> anyhow::Result<()> {
    if let Ok(browser) = std::env::var("MAESTRO_BROWSER") {
        tokio::process::Command::new(&browser)
            .arg(url)
            .spawn()?;
    } else {
        opener::open(url)?;
    }
    Ok(())
}
```

#### 8.1.4 Walkthrough Generator — `src/leindex/src/tracklens/walkthrough.rs`

```rust
// src/leindex/src/tracklens/walkthrough.rs

use std::fs;
use std::path::Path;
use std::process::Command;

use super::types::*;

/// Generate a markdown walkthrough document for a completed track.
///
/// Reads track metadata, spec, plan, and git history to produce a
/// comprehensive review document for TrackLens annotation.
pub fn generate_walkthrough(config: &WalkthroughConfig) -> anyhow::Result<String> {
    let metadata_path = config.track_dir.join("metadata.json");
    let metadata: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&metadata_path)?
    )?;

    let spec_content = fs::read_to_string(config.track_dir.join("spec.md"))
        .unwrap_or_else(|_| String::from("(spec not found)"));
    let plan_content = fs::read_to_string(config.track_dir.join("plan.md"))
        .unwrap_or_else(|_| String::from("(plan not found)"));

    let description = metadata["description"].as_str().unwrap_or("Untitled");
    let track_type = metadata["type"].as_str().unwrap_or("feature");

    let completed_tasks = extract_completed_tasks(&plan_content);
    let changed_files = get_track_changed_files(&config.root, &config.track_id)?;

    let mut doc = String::with_capacity(8192);

    // ── Header ─────────────────────────────────────────
    doc.push_str(&format!("# Track Walkthrough: {description}\n\n"));
    doc.push_str(&format!("**Track ID:** `{}`\n", config.track_id));
    doc.push_str(&format!("**Type:** {track_type}\n"));
    doc.push_str("**Status:** Completed\n");
    if config.is_subtrack {
        if let Some(parent) = &config.parent_track_id {
            doc.push_str(&format!("**Parent:** `{parent}`\n"));
        }
    }

    // ── Spec Summary ───────────────────────────────────
    doc.push_str("\n---\n\n## Specification Summary\n\n");
    let spec_summary = extract_first_section(&spec_content, 20);
    doc.push_str(&spec_summary);

    // ── Completed Tasks ────────────────────────────────
    doc.push_str("\n\n---\n\n## Completed Tasks\n\n");
    for task in &completed_tasks {
        doc.push_str(&format!("- [x] {task}\n"));
    }

    // ── Files Changed ──────────────────────────────────
    doc.push_str("\n## Files Changed\n\n");
    doc.push_str("| Status | File | +/- |\n|--------|------|-----|\n");
    for file in &changed_files {
        let icon = status_icon(&file.status);
        doc.push_str(&format!(
            "| {icon} | [`{}`]({}) | +{} / -{} |\n",
            file.path, file.path, file.additions, file.deletions
        ));
    }

    // ── Detailed Changes ───────────────────────────────
    doc.push_str("\n## Detailed Changes\n\n");
    for file in &changed_files {
        doc.push_str(&format!("### {}\n\n", file.path));
        if config.include_snippets {
            if let Some(snippet) = &file.snippet {
                doc.push_str(&format!("```{}\n{}\n```\n\n", file.language, snippet));
            }
        }
        if config.include_diffs {
            if let Some(diff) = &file.diff {
                doc.push_str(&format!(
                    "<details><summary>Full diff ({} lines)</summary>\n\n```diff\n{}\n```\n</details>\n\n",
                    diff.lines().count(),
                    diff
                ));
            }
        }
    }

    doc.push_str("---\n\n> Review this walkthrough. Annotate any issues for remediation.\n");
    Ok(doc)
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn extract_completed_tasks(plan: &str) -> Vec<String> {
    plan.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                Some(trimmed.trim_start_matches("- [x]").trim_start_matches("- [X]").trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

fn extract_first_section(content: &str, max_lines: usize) -> String {
    content.lines()
        .skip_while(|l| l.trim().is_empty() || l.starts_with('#'))
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_track_changed_files(root: &Path, track_id: &str) -> anyhow::Result<Vec<ChangedFile>> {
    // Use git log to find commits mentioning this track ID in their message
    let output = Command::new("git")
        .args(["log", "--all", "--oneline", "--grep", track_id, "--name-status", "--diff-filter=ADMR"])
        .current_dir(root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files: Vec<ChangedFile> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in stdout.lines() {
        // Parse git name-status lines: "M\tpath/to/file.rs"
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && !seen.contains(parts[1]) {
            let status = match parts[0].chars().next() {
                Some('A') => FileChangeStatus::Added,
                Some('M') => FileChangeStatus::Modified,
                Some('D') => FileChangeStatus::Deleted,
                Some('R') => FileChangeStatus::Renamed,
                _ => continue,
            };
            let path = parts[1].to_string();
            let language = detect_language(&path);

            // Get diff stats for this file
            let (additions, deletions, diff, snippet) =
                get_file_diff_info(root, &path, track_id);

            seen.insert(path.clone());
            files.push(ChangedFile {
                path, status, language, diff, snippet, additions, deletions,
            });
        }
    }
    Ok(files)
}

fn get_file_diff_info(root: &Path, file_path: &str, track_id: &str) -> (u32, u32, Option<String>, Option<String>) {
    let diff_output = Command::new("git")
        .args(["log", "--all", "-p", "--grep", track_id, "--", file_path])
        .current_dir(root)
        .output()
        .ok();

    let diff_text = diff_output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

    let mut additions = 0u32;
    let mut deletions = 0u32;
    if let Some(ref text) = diff_text {
        for line in text.lines() {
            if line.starts_with('+') && !line.starts_with("+++") { additions += 1; }
            if line.starts_with('-') && !line.starts_with("---") { deletions += 1; }
        }
    }

    // Extract a key snippet (first 30 non-diff lines of the current version)
    let snippet = fs::read_to_string(root.join(file_path))
        .ok()
        .map(|content| {
            content.lines().take(30).collect::<Vec<_>>().join("\n")
        });

    (additions, deletions, diff_text, snippet)
}

fn status_icon(status: &FileChangeStatus) -> &'static str {
    match status {
        FileChangeStatus::Added => "🆕",
        FileChangeStatus::Modified => "✏️",
        FileChangeStatus::Deleted => "🗑️",
        FileChangeStatus::Renamed => "📝",
    }
}

fn detect_language(path: &str) -> String {
    match path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("hpp") | Some("cc") => "cpp",
        Some("md") => "markdown",
        Some("json") => "json",
        Some("toml") => "toml",
        Some("yaml") | Some("yml") => "yaml",
        _ => "text",
    }.to_string()
}
```

#### 8.1.5 Cockpit TUI Pane — `crates/cockpit/src/tracklens/pane.rs`

```rust
// crates/cockpit/src/tracklens/pane.rs

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// TrackLens review status for the TUI
#[derive(Debug, Clone, Default)]
pub struct TrackLensPane {
    pub active: bool,
    pub current_review: Option<ReviewStatus>,
    pub history: Vec<ReviewHistoryEntry>,
}

#[derive(Debug, Clone)]
pub struct ReviewStatus {
    pub track_id: String,
    pub document_type: String,
    pub mode: String,       // "review" | "walkthrough"
    pub server_url: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ReviewHistoryEntry {
    pub track_id: String,
    pub document_type: String,
    pub approved: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub annotation_count: usize,
}

impl TrackLensPane {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // current status
                Constraint::Min(8),    // history
            ])
            .split(area);

        self.render_current_status(f, chunks[0]);
        self.render_history(f, chunks[1]);
    }

    fn render_current_status(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" TrackLens Status ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.current_review.is_some() {
                Color::Green
            } else {
                Color::DarkGray
            }));

        let content = if let Some(review) = &self.current_review {
            vec![
                Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Green)),
                    Span::raw(format!("Reviewing: {} ({})", review.document_type, review.track_id)),
                ]),
                Line::from(vec![
                    Span::styled("  URL: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&review.server_url, Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
                ]),
                Line::from(vec![
                    Span::styled("  Mode: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&review.mode),
                ]),
            ]
        } else {
            vec![Line::from(Span::styled(
                "No active review",
                Style::default().fg(Color::DarkGray),
            ))]
        };

        let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_history(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Review History ")
            .borders(Borders::ALL);

        if self.history.is_empty() {
            let p = Paragraph::new("No reviews yet.")
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, area);
            return;
        }

        let items: Vec<ListItem> = self.history.iter().rev().take(20).map(|entry| {
            let icon = if entry.approved { "✓" } else { "✗" };
            let color = if entry.approved { Color::Green } else { Color::Red };
            let ann_info = if entry.annotation_count > 0 {
                format!(" ({} annotations)", entry.annotation_count)
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(color)),
                Span::raw(format!("{} — {}{ann_info}", entry.track_id, entry.document_type)),
            ]))
        }).collect();

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}
```

#### 8.1.6 CLI Subcommand — `crates/cli/src/commands/tracklens.rs`

```rust
// crates/cli/src/commands/tracklens.rs

use clap::{Args, Subcommand};
use leindex_core::tracklens::{
    TrackLensServer, TrackLensServerOptions, TrackLensMode, TrackLensOrigin,
    WalkthroughConfig, generate_walkthrough,
};
use std::path::PathBuf;

#[derive(Args)]
pub struct TrackLensArgs {
    #[command(subcommand)]
    pub command: TrackLensCommand,
}

#[derive(Subcommand)]
pub enum TrackLensCommand {
    /// Review a markdown document interactively
    Review {
        /// Path to the markdown file to review
        #[arg(short, long)]
        file: PathBuf,

        /// Document type label (e.g., "spec.md", "plan.md")
        #[arg(short, long, default_value = "document")]
        doc_type: String,

        /// Track ID for context
        #[arg(short, long)]
        track_id: Option<String>,

        /// Port to serve on (default: random)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Generate and review a track completion walkthrough
    Walkthrough {
        /// Track ID to generate walkthrough for
        track_id: String,

        /// Maestro project root
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Include full git diffs
        #[arg(long, default_value = "true")]
        include_diffs: bool,

        /// Include code snippets
        #[arg(long, default_value = "true")]
        include_snippets: bool,

        /// Port to serve on (default: random)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Review git diff interactively (code review mode)
    CodeReview {
        /// Git ref to diff against (default: HEAD)
        #[arg(short, long, default_value = "HEAD")]
        git_ref: String,

        /// Port to serve on (default: random)
        #[arg(short, long)]
        port: Option<u16>,
    },
}

pub async fn run(args: TrackLensArgs) -> anyhow::Result<()> {
    // Load pre-built HTML bundles from embedded assets or dist/
    let html_content = load_html_bundle()?;

    match args.command {
        TrackLensCommand::Review { file, doc_type, track_id, port } => {
            let markdown = std::fs::read_to_string(&file)?;
            let mut server = TrackLensServer::start(TrackLensServerOptions {
                markdown, document_type: doc_type.clone(), track_id,
                mode: TrackLensMode::Review, origin: TrackLensOrigin::Maestro,
                html_content, port, open_browser: true,
            }).await?;

            println!("TrackLens reviewing {doc_type} at {}", server.url());
            let decision = server.wait_for_decision().await?;
            server.stop();

            if decision.approved {
                println!("✓ Approved");
            } else {
                println!("✗ Changes requested:\n{}", decision.feedback.unwrap_or_default());
            }
        }

        TrackLensCommand::Walkthrough { track_id, root, include_diffs, include_snippets, port } => {
            let root = std::fs::canonicalize(&root)?;
            let track_dir = root.join("maestro/tracks").join(&track_id);
            if !track_dir.exists() {
                anyhow::bail!("Track directory not found: {}", track_dir.display());
            }

            let walkthrough = generate_walkthrough(&WalkthroughConfig {
                track_id: track_id.clone(),
                root: root.clone(),
                track_dir,
                is_subtrack: false,
                parent_track_id: None,
                include_diffs,
                include_snippets,
                max_snippet_lines: 30,
            })?;

            let mut server = TrackLensServer::start(TrackLensServerOptions {
                markdown: walkthrough, document_type: "walkthrough".into(),
                track_id: Some(track_id.clone()),
                mode: TrackLensMode::Walkthrough, origin: TrackLensOrigin::Maestro,
                html_content, port, open_browser: true,
            }).await?;

            println!("TrackLens walkthrough for {} at {}", track_id, server.url());
            let decision = server.wait_for_decision().await?;
            server.stop();

            if decision.approved {
                // Save final walkthrough
                let out_path = root.join("maestro/tracks").join(&track_id).join("walkthrough-final.md");
                std::fs::write(&out_path, &decision.feedback.unwrap_or_default())?;
                println!("✓ Walkthrough approved. Saved to {}", out_path.display());
            } else {
                println!("✗ Walkthrough requires remediation:\n{}", decision.feedback.unwrap_or_default());
                std::process::exit(1);
            }
        }

        TrackLensCommand::CodeReview { git_ref, port } => {
            let diff = get_git_diff(&git_ref)?;
            let mut server = TrackLensServer::start(TrackLensServerOptions {
                markdown: diff, document_type: "code-review".into(),
                track_id: None,
                mode: TrackLensMode::CodeReview, origin: TrackLensOrigin::Maestro,
                html_content, port, open_browser: true,
            }).await?;

            println!("TrackLens code review at {}", server.url());
            let decision = server.wait_for_decision().await?;
            server.stop();

            if let Some(feedback) = decision.feedback {
                println!("{feedback}");
            }
        }
    }
    Ok(())
}

fn load_html_bundle() -> anyhow::Result<String> {
    // Try embedded asset first, then fall back to dist/ directory
    let dist_path = std::env::var("TRACKLENS_HTML_PATH")
        .unwrap_or_else(|_| {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".maestro/tracklens/dist/tracklens-editor.html")
                .to_string_lossy().to_string()
        });
    std::fs::read_to_string(&dist_path)
        .map_err(|e| anyhow::anyhow!("Failed to load TrackLens HTML bundle at {dist_path}: {e}"))
}

fn get_git_diff(git_ref: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(["diff", git_ref])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### 8.2 Dependency Flow (One-Way Rule Preserved)

```
cli → cockpit + leindex-core
       │              │
       │              ├── tracklens/mod.rs      (re-exports)
       │              ├── tracklens/types.rs     (shared types)
       │              ├── tracklens/server.rs    (axum HTTP server)
       │              └── tracklens/walkthrough.rs (doc generator)
       │
       └── tracklens/pane.rs (TUI status/history)
```

`leindex-core ↛ cockpit` — **NEVER violated.**

### 8.3 Cargo.toml Additions

**`src/leindex/Cargo.toml`** — add to `[dependencies]`:
```toml
axum = "0.8"
opener = "0.7"
dirs = "6"
```

**`crates/cockpit/Cargo.toml`** — no new deps (already uses ratatui, chrono).

**`crates/cli/Cargo.toml`** — no new deps (already uses clap, references leindex-core).

---

## 9. File-by-File Porting Manifest

### 9.1 All Ported Components

| Source (Plannotator) | Target (Maestro) | Treatment |
|---|---|---|
| **CLAUDE CODE INTEGRATION** | | |
| `apps/hook/.claude-plugin/plugin.json` | `apps/tracklens-hook/.claude-plugin/plugin.json` | Rebrand name/description |
| `apps/hook/hooks/hooks.json` | `apps/tracklens-hook/hooks/hooks.json` | Change command → `tracklens` |
| `apps/hook/commands/plannotator-review.md` | `apps/tracklens-hook/commands/tracklens-review.md` | Rebrand |
| `apps/hook/commands/plannotator-annotate.md` | `apps/tracklens-hook/commands/tracklens-annotate.md` | Rebrand |
| `apps/hook/server/index.ts` | `apps/tracklens-hook/server/index.ts` | Rebrand imports + naming; remove sharing env vars |
| `apps/hook/index.tsx` | `apps/tracklens-hook/index.tsx` | Change `@plannotator/editor` → `@maestro/tracklens-editor` |
| `apps/hook/index.html` | `apps/tracklens-hook/index.html` | Update title |
| `apps/hook/vite.config.ts` | `apps/tracklens-hook/vite.config.ts` | Port unchanged |
| `apps/hook/dev-mock-api.ts` | `apps/tracklens-hook/dev-mock-api.ts` | Port unchanged |
| **OPENCODE INTEGRATION** | | |
| `apps/opencode-plugin/index.ts` | `apps/tracklens-opencode/index.ts` | Rebrand; keep agent switching; remove share URL env vars |
| **PI-MONO INTEGRATION** | | |
| `apps/pi-extension/index.ts` | `pi-maestro/src/tracklens/extension/index.ts` | Rebrand + integrate with newTrack/implement |
| `apps/pi-extension/server.ts` | `pi-maestro/src/tracklens/extension/server.ts` | Rebrand all server names |
| `apps/pi-extension/utils.ts` | `pi-maestro/src/tracklens/extension/utils.ts` | Port unchanged |
| **SERVER PACKAGES** | | |
| `packages/server/index.ts` | `packages/tracklens-server/index.ts` | Rebrand; remove sharing routes |
| `packages/server/review.ts` | `packages/tracklens-server/review.ts` | Rebrand |
| `packages/server/annotate.ts` | `packages/tracklens-server/annotate.ts` | Rebrand |
| `packages/server/storage.ts` | `packages/tracklens-server/storage.ts` | `~/.plannotator/` → `~/.maestro/tracklens/` |
| `packages/server/git.ts` | `packages/tracklens-server/git.ts` | Port unchanged |
| `packages/server/browser.ts` | `packages/tracklens-server/browser.ts` | `PLANNOTATOR_BROWSER` → `MAESTRO_BROWSER` |
| `packages/server/repo.ts` | `packages/tracklens-server/repo.ts` | Port unchanged |
| `packages/server/image.ts` | `packages/tracklens-server/image.ts` | Port unchanged |
| `packages/server/ide.ts` | `packages/tracklens-server/ide.ts` | Port unchanged |
| `packages/server/remote.ts` | `packages/tracklens-server/remote.ts` | Rename env vars |
| `packages/server/integrations.ts` | `packages/tracklens-server/integrations.ts` | Port: `extractTags()`, `generateFrontmatter()`, `generateFilename()`, `saveToObsidian()`, `saveToBear()`. Rebrand tags from `plannotator` → `tracklens` |
| **UI PACKAGES — ALL PORTED** | | |
| `packages/editor/App.tsx` | `packages/tracklens-editor/App.tsx` | Rebrand; remove sharing UI; keep all other functionality |
| `packages/review-editor/App.tsx` | `packages/tracklens-review-editor/App.tsx` | Rebrand; remove sharing UI |
| `packages/review-editor/components/*` | `packages/tracklens-review-editor/components/*` | DiffViewer, ReviewPanel, FileTree, etc. |
| `packages/review-editor/hooks/*` | `packages/tracklens-review-editor/hooks/*` | useAnnotationToolbar, useTabIndent |
| `packages/review-editor/utils/*` | `packages/tracklens-review-editor/utils/*` | patchParser, detectLanguage, formatLineRange, renderInlineMarkdown |
| `packages/ui/types.ts` | `packages/tracklens-ui/types.ts` | Port unchanged |
| `packages/ui/utils/parser.ts` | `packages/tracklens-ui/utils/parser.ts` | Port unchanged |
| `packages/ui/utils/storage.ts` | `packages/tracklens-ui/utils/storage.ts` | Change key prefix to `tracklens-` |
| `packages/ui/utils/identity.ts` | `packages/tracklens-ui/utils/identity.ts` | Port unchanged |
| `packages/ui/utils/annotationHelpers.ts` | `packages/tracklens-ui/utils/annotationHelpers.ts` | Port unchanged |
| `packages/ui/utils/planDiffEngine.ts` | `packages/tracklens-ui/utils/planDiffEngine.ts` | Port unchanged |
| `packages/ui/utils/editorMode.ts` | `packages/tracklens-ui/utils/editorMode.ts` | Port unchanged |
| `packages/ui/utils/obsidian.ts` | `packages/tracklens-ui/utils/obsidian.ts` | **KEPT** — vault detection & path resolution |
| `packages/ui/utils/bear.ts` | `packages/tracklens-ui/utils/bear.ts` | **KEPT** — Bear settings |
| `packages/ui/utils/defaultNotesApp.ts` | `packages/tracklens-ui/utils/defaultNotesApp.ts` | **KEPT** — notes app selection |
| `packages/ui/utils/agentSwitch.ts` | `packages/tracklens-ui/utils/agentSwitch.ts` | **KEPT** — OpenCode agent switching; Claude Code routes to maestro agents via orchestrator |
| `packages/ui/utils/permissionMode.ts` | `packages/tracklens-ui/utils/autonomyMode.ts` | **KEPT + MERGED** — Renamed to "autonomy mode"; merged with maestro conductor autonomy levels |
| `packages/ui/utils/planSave.ts` | `packages/tracklens-ui/utils/docSave.ts` | **KEPT** — controls auto-save of reviewed documents |
| `packages/ui/utils/uiPreferences.ts` | `packages/tracklens-ui/utils/uiPreferences.ts` | **KEPT** — sticky actions, ToC, sidebar defaults |
| `packages/ui/utils/planDiffMarketing.ts` | — | **REMOVED** — marketing dialog |
| `packages/ui/components/Viewer.tsx` | `packages/tracklens-ui/components/Viewer.tsx` | Remove tater sprites; keep all annotation/highlight functionality |
| `packages/ui/components/AnnotationPanel.tsx` | Port unchanged | |
| `packages/ui/components/AnnotationToolbar.tsx` | Port unchanged | |
| `packages/ui/components/AnnotationSidebar.tsx` | Port unchanged | |
| `packages/ui/components/MermaidBlock.tsx` | Port unchanged | |
| `packages/ui/components/ThemeProvider.tsx` | Port unchanged | |
| `packages/ui/components/ConfirmDialog.tsx` | Port unchanged | |
| `packages/ui/components/CompletionOverlay.tsx` | Rebrand agent labels to Maestro | |
| `packages/ui/components/ExportModal.tsx` | Port unchanged | |
| `packages/ui/components/ModeSwitcher.tsx` | Remove tater references only | |
| `packages/ui/components/ModeToggle.tsx` | Port unchanged | |
| `packages/ui/components/ResizeHandle.tsx` | Port unchanged | |
| `packages/ui/components/TableOfContents.tsx` | Port unchanged | |
| `packages/ui/components/ImageThumbnail.tsx` | Port unchanged | |
| `packages/ui/components/AttachmentsButton.tsx` | Port unchanged | |
| `packages/ui/components/Settings.tsx` | `packages/tracklens-ui/components/Settings.tsx` | **PORT FULL** — all 3 tabs (General, Display, Saving). Remove only sharing-specific controls. Keep identity, auto-close, agent switch, permission/autonomy mode, plan save, Obsidian, Bear, UI prefs |
| `packages/ui/components/PermissionModeSetup.tsx` | `packages/tracklens-ui/components/AutonomyModeSetup.tsx` | **KEPT** — Renamed; merged with maestro autonomy levels (`bypassPermissions` → `full-auto`, `acceptEdits` → `semi-auto`, `default` → `checkpoint`) |
| `packages/ui/components/UIFeaturesSetup.tsx` | `packages/tracklens-ui/components/UIFeaturesSetup.tsx` | **KEPT** — first-run feature setup |
| `packages/ui/components/ImportModal.tsx` | — | **REMOVED** — depends on sharing |
| `packages/ui/components/Landing.tsx` | — | **REMOVED** — not needed |
| `packages/ui/components/UpdateBanner.tsx` | — | **REMOVED** — phones home to npm |
| `packages/ui/components/TaterSprite*.tsx` | — | **REMOVED** — mascot |
| `packages/ui/hooks/usePlanDiff.ts` | Port unchanged | |
| `packages/ui/hooks/useActiveSection.ts` | Port unchanged | |
| `packages/ui/hooks/useSidebar.ts` | Port unchanged | |
| `packages/ui/hooks/useResizablePanel.ts` | Port unchanged | |
| `packages/ui/hooks/useAutoClose.ts` | Port unchanged | |
| `packages/ui/hooks/useLinkedDoc.ts` | Port unchanged | |
| `packages/ui/hooks/useDismissOnOutsideAndEscape.ts` | Port unchanged | |
| `packages/ui/hooks/useVaultBrowser.ts` | `packages/tracklens-ui/hooks/useVaultBrowser.ts` | **KEPT** — browse Obsidian vault / maestro tracks |
| `packages/ui/hooks/useAgents.ts` | `packages/tracklens-ui/hooks/useAgents.ts` | **KEPT** — OpenCode agent listing + validation |
| `packages/ui/hooks/useUpdateCheck.ts` | — | **REMOVED** — npm version check |
| **SHARED PACKAGES** | | |
| `packages/shared/compress.ts` | `packages/tracklens-shared/compress.ts` | **KEPT** — deflate compression for walkthrough storage optimization |
| `packages/shared/crypto.ts` | — | **REMOVED** — E2E encryption for sharing only |
| `packages/web-highlighter/` | `packages/tracklens-web-highlighter/` | Port as-is |
| **NEW FILES** | | |
| — | `pi-maestro/src/tracklens/walkthrough/generator.ts` | NEW: walkthrough generation |
| — | `pi-maestro/src/tracklens/extension/tools.ts` | NEW: tracklens_review + tracklens_walkthrough tools |
| — | `src/leindex/src/tracklens/server.rs` | NEW: Rust axum server |
| — | `src/leindex/src/tracklens/walkthrough.rs` | NEW: Rust walkthrough generator |
| — | `src/leindex/src/tracklens/mod.rs` | NEW: module declaration |
| — | `crates/cockpit/src/tracklens/pane.rs` | NEW: TUI pane |
| — | `crates/cli/src/commands/tracklens.rs` | NEW: CLI subcommand |

### 9.2 Files NOT Ported (Final — 8 components only)

| File | Reason |
|---|---|
| `apps/paste-service/*` | External sharing service |
| `apps/portal/*` | Web portal for shared content |
| `apps/marketing/*` | Marketing website |
| `packages/shared/crypto.ts` | E2E encryption for sharing only |
| `packages/ui/utils/sharing.ts` + `useSharing.ts` | URL sharing pipeline |
| `packages/ui/utils/planDiffMarketing.ts` | Marketing dialog |
| `packages/ui/components/Landing.tsx` | Standalone landing page |
| `packages/ui/components/UpdateBanner.tsx` + `useUpdateCheck.ts` | npm version check |
| `packages/ui/components/ImportModal.tsx` | Import from shared URL |
| `packages/ui/components/TaterSprite*.tsx` (4 files) | Mascot branding |

---

## 10. Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Goal:** Create directory structures, port core types and utilities, apply global rebranding.

**Tasks:**
1. Create directory tree:
   ```
   apps/tracklens-hook/
   apps/tracklens-opencode/
   packages/tracklens-server/
   packages/tracklens-editor/
   packages/tracklens-review-editor/
   packages/tracklens-ui/
   packages/tracklens-shared/
   packages/tracklens-web-highlighter/
   pi-maestro/src/tracklens/
   src/leindex/src/tracklens/
   crates/cockpit/src/tracklens/
   crates/cli/src/commands/
   ```

2. Port `packages/ui/types.ts` → `packages/tracklens-ui/types.ts` (unchanged)

3. Port `packages/ui/utils/parser.ts` → `packages/tracklens-ui/utils/parser.ts` (unchanged)

4. Port `packages/ui/utils/storage.ts` — change localStorage key prefix:
   ```typescript
   // packages/tracklens-ui/utils/storage.ts
   const STORAGE_PREFIX = "tracklens-";  // was: "plannotator-"
   
   export function getStorageKey(key: string): string {
     return `${STORAGE_PREFIX}${key}`;
   }
   ```

5. Port `packages/shared/compress.ts` → `packages/tracklens-shared/compress.ts` (unchanged — deflate/inflate for walkthrough storage)

6. Port `packages/ui/utils/identity.ts` (unchanged)
7. Port `packages/ui/utils/annotationHelpers.ts` (unchanged)
8. Port `packages/ui/utils/planDiffEngine.ts` (unchanged)
9. Port `packages/ui/utils/editorMode.ts` (unchanged)

10. Port and rebrand autonomy mode:
    ```typescript
    // packages/tracklens-ui/utils/autonomyMode.ts
    // Merged from plannotator's permissionMode.ts + maestro conductor levels
    
    export type AutonomyMode = "full-auto" | "semi-auto" | "checkpoint";
    
    // Mapping from plannotator permission modes
    const LEGACY_MAP: Record<string, AutonomyMode> = {
      bypassPermissions: "full-auto",
      acceptEdits: "semi-auto",
      default: "checkpoint",
    };
    
    export function getAutonomyMode(): AutonomyMode {
      const stored = localStorage.getItem("tracklens-autonomy-mode");
      if (stored && isValidMode(stored)) return stored as AutonomyMode;
      // Check legacy key for migration
      const legacy = localStorage.getItem("plannotator-permission-mode");
      if (legacy && LEGACY_MAP[legacy]) {
        const mapped = LEGACY_MAP[legacy];
        setAutonomyMode(mapped);
        localStorage.removeItem("plannotator-permission-mode");
        return mapped;
      }
      return "checkpoint";
    }
    
    export function setAutonomyMode(mode: AutonomyMode): void {
      localStorage.setItem("tracklens-autonomy-mode", mode);
    }
    
    export function isValidMode(mode: string): mode is AutonomyMode {
      return ["full-auto", "semi-auto", "checkpoint"].includes(mode);
    }
    
    export function modeToClaudeCodePermission(mode: AutonomyMode): string {
      // Maps back to Claude Code's PermissionRequest format
      switch (mode) {
        case "full-auto": return "bypassPermissions";
        case "semi-auto": return "acceptEdits";
        case "checkpoint": return "default";
      }
    }
    ```

11. Port `packages/ui/utils/docSave.ts` (renamed from `planSave.ts`):
    ```typescript
    // packages/tracklens-ui/utils/docSave.ts
    export function getDocSaveEnabled(): boolean {
      return localStorage.getItem("tracklens-doc-save") !== "false";
    }
    export function setDocSaveEnabled(enabled: boolean): void {
      localStorage.setItem("tracklens-doc-save", String(enabled));
    }
    ```

12. Port remaining utils unchanged: `obsidian.ts`, `bear.ts`, `defaultNotesApp.ts`, `agentSwitch.ts`, `uiPreferences.ts`

13. Global rebranding sweep — `sed` pass across all ported files:
    ```bash
    # Verify zero "plannotator" references remain
    grep -r "plannotator" packages/tracklens-*/  # must be empty
    grep -r "Plannotator" packages/tracklens-*/  # must be empty
    grep -r "tater" packages/tracklens-*/        # must be empty (mascot)
    ```

### Phase 2: Server Layer (Week 2-3)

**Goal:** Port all three Node.js server types plus Rust server.

**Tasks:**
1. Port `packages/server/index.ts` → `packages/tracklens-server/index.ts`:
   - Rename `startPlannotatorServer` → `startTrackLensServer`
   - Remove sharing routes (`/api/share`, `/api/paste`)
   - Remove `PLANNOTATOR_SHARE_URL`, `PLANNOTATOR_PASTE_URL` env vars
   - Keep all `/api/approve`, `/api/deny`, `/api/state` routes

2. Port `packages/server/review.ts` → rename `startReviewServer` → `startTrackLensReviewServer`

3. Port `packages/server/annotate.ts` → rename `startAnnotateServer` → `startTrackLensAnnotateServer`

4. Port `packages/server/storage.ts`:
   ```typescript
   // packages/tracklens-server/storage.ts
   import { homedir } from "os";
   import { join } from "path";
   
   export function getTrackLensDir(): string {
     // was: ~/.plannotator/ → now: ~/.maestro/tracklens/
     return join(homedir(), ".maestro", "tracklens");
   }
   ```

5. Port `packages/server/browser.ts` — `PLANNOTATOR_BROWSER` → `MAESTRO_BROWSER`

6. Port `packages/server/remote.ts` — `PLANNOTATOR_REMOTE` → `TRACKLENS_REMOTE`, `PLANNOTATOR_PORT` → `TRACKLENS_PORT`

7. Port `packages/server/integrations.ts` — rebrand tags:
   ```typescript
   // packages/tracklens-server/integrations.ts
   export function extractTags(content: string): string[] { /* unchanged logic */ }
   export function generateFrontmatter(doc: { title: string; tags: string[] }): string {
     return `---\ntags: [tracklens, ${doc.tags.join(", ")}]\n---\n`;  // was: plannotator
   }
   export function saveToObsidian(content: string, vaultPath: string, filename: string): void { /* unchanged */ }
   export function saveToBear(content: string, title: string, tags: string[]): void { /* unchanged */ }
   ```

8. Port remaining server files unchanged: `git.ts`, `repo.ts`, `image.ts`, `ide.ts`

9. Write Rust `TrackLensServer` in `src/leindex/src/tracklens/` (see Section 8.1.3)

10. Write Rust types in `src/leindex/src/tracklens/types.rs` (see Section 8.1.2)

### Phase 3: UI Components (Week 3-4)

**Goal:** Port all React components, hooks, and build system.

**Tasks:**
1. Port `packages/web-highlighter/` → `packages/tracklens-web-highlighter/` (as-is)

2. Port all UI components (see Section 9.1 for full list):
   - `Viewer.tsx` — remove `TaterSprite*` imports, keep all annotation/highlight logic
   - `ModeSwitcher.tsx` — remove tater mascot references only
   - `CompletionOverlay.tsx` — rebrand agent labels from plannotator → maestro
   - `Settings.tsx` — **full port**, remove only sharing tab controls, keep all 3 tabs:
     - General: identity, auto-close, agent switch, autonomy mode
     - Display: sidebar, ToC, sticky actions
     - Saving: doc save, Obsidian vault, Bear integration

3. Port `PermissionModeSetup.tsx` → `AutonomyModeSetup.tsx`:
   ```tsx
   // packages/tracklens-ui/components/AutonomyModeSetup.tsx
   const MODES = [
     { value: "checkpoint", label: "Checkpoint", description: "Confirm all actions (default)" },
     { value: "semi-auto", label: "Semi-Auto", description: "Auto-approve file edits" },
     { value: "full-auto", label: "Full Auto", description: "Auto-approve all tool calls" },
   ] as const;
   ```

4. Port `UIFeaturesSetup.tsx` (unchanged — first-run feature toggles)

5. Port all hooks unchanged: `usePlanDiff`, `useActiveSection`, `useSidebar`, `useResizablePanel`, `useAutoClose`, `useLinkedDoc`, `useDismissOnOutsideAndEscape`

6. Port hooks with modifications:
   - `useVaultBrowser.ts` — add maestro tracks directory as browsable location
   - `useAgents.ts` — keep OpenCode agent listing; add maestro agent list for Claude Code context

7. Remove these files entirely (do NOT port):
   - `Landing.tsx`, `UpdateBanner.tsx`, `ImportModal.tsx`, `TaterSprite*.tsx`
   - `useUpdateCheck.ts`, `useSharing.ts`, `sharing.ts`, `planDiffMarketing.ts`

8. Set up Vite builds:
   ```typescript
   // apps/tracklens-hook/vite.config.ts
   export default defineConfig({
     build: {
       rollupOptions: {
         input: { editor: "index.html" },
         output: { inlineDynamicImports: true },
       },
       outDir: "dist",
     },
     resolve: {
       alias: {
         "@maestro/tracklens-ui": resolve(__dirname, "../../packages/tracklens-ui"),
         "@maestro/tracklens-editor": resolve(__dirname, "../../packages/tracklens-editor"),
       },
     },
   });
   ```

### Phase 4: Claude Code Integration (Week 4-5)

**Goal:** Port the hook app for Claude Code compatibility.

**Tasks:**
1. Create `apps/tracklens-hook/.claude-plugin/plugin.json` (see Section 7.1)
2. Create `apps/tracklens-hook/hooks/hooks.json` (see Section 7.1)
3. Create slash command files:
   - `commands/tracklens-review.md` (see Section 7.1)
   - `commands/tracklens-annotate.md` (see Section 7.1)
4. Port `server/index.ts` with three modes (see Section 7.1 CLI entry code)
5. Port `index.tsx` — change `@plannotator/editor` → `@maestro/tracklens-editor`
6. Port `index.html` — update title to "TrackLens"
7. Port `vite.config.ts` and `dev-mock-api.ts` (unchanged)

**Testing:**
```bash
# Test hook fires on ExitPlanMode
echo '{"tool_input":{"plan":"# Test Plan"},"permission_mode":"default"}' | node apps/tracklens-hook/server/index.ts

# Test slash commands
node apps/tracklens-hook/server/index.ts review
node apps/tracklens-hook/server/index.ts annotate test.md
```

### Phase 5: OpenCode Integration (Week 5)

**Goal:** Port the OpenCode plugin with tool registrations.

**Tasks:**
1. Port `apps/opencode-plugin/index.ts` → `apps/tracklens-opencode/index.ts` (see Section 7.2)
2. Rebrand tool names: `plannotator` → `tracklens`, `plannotator-review` → `tracklens-review`, `plannotator-annotate` → `tracklens-annotate`
3. Remove `PLANNOTATOR_SHARE_URL` env var references
4. Keep `agentSwitch` field in tool responses (OpenCode agent routing)
5. Keep `sharingEnabled` → rename to `savingEnabled` (controls doc persistence, not external sharing)

**Testing:**
```bash
bun test apps/tracklens-opencode/
```

### Phase 6: Pi-mono + newTrack Integration (Week 5-6)

**Goal:** Register TrackLens tools and modify newTrack/implement workflows.

**Tasks:**
1. Register `tracklens_review` tool (see Section 5.2)

2. Register `tracklens_walkthrough` tool:
   ```typescript
   // pi-maestro/src/tracklens/extension/tools.ts
   
   pi.registerTool("tracklens_walkthrough", {
     description: "Generate and present a track completion walkthrough via TrackLens",
     parameters: {
       type: "object",
       properties: {
         trackId: { type: "string", description: "Track ID" },
         isSubtrack: { type: "boolean", description: "Is this a subtrack?" },
         parentTrackId: { type: "string", description: "Parent track ID (if subtrack)" },
       },
       required: ["trackId"],
     },
     async execute(_toolCallId, params, _signal, _onUpdate, ctx) {
       const { trackId, isSubtrack, parentTrackId } = params;
       const root = findMaestroProjectRoot(process.cwd());
       if (!root) return { content: [{ type: "text", text: "Not in a maestro project" }] };
   
       const trackDir = join(root, "maestro/tracks", trackId);
       const walkthrough = generateWalkthrough({
         trackId, root, trackDir,
         isSubtrack: isSubtrack || false,
         parentTrackId,
       });
   
       const htmlBundle = readFileSync(join(__dirname, "../../dist/tracklens-editor.html"), "utf-8");
       const server = startTrackLensServer({
         markdown: walkthrough,
         documentType: "walkthrough",
         trackId,
         mode: "walkthrough",
         origin: "maestro",
         htmlContent: htmlBundle,
       });
   
       openBrowser(server.url);
       ctx.ui.notify(`TrackLens: Walkthrough for ${trackId} at ${server.url}`);
   
       const decision = await server.waitForDecision();
       server.stop();
   
       if (decision.approved) {
         // Save final walkthrough
         const outPath = join(trackDir, "walkthrough-final.md");
         writeFileSync(outPath, walkthrough, "utf-8");
         return {
           content: [{ type: "text", text: `Walkthrough approved. Saved to ${outPath}` }],
           details: { approved: true },
         };
       } else {
         return {
           content: [{
             type: "text",
             text: `Walkthrough requires remediation:\n\n${decision.feedback}\n\nAddress each annotation, then call tracklens_walkthrough again.`,
           }],
           details: {
             approved: false,
             feedback: decision.feedback,
             annotations: decision.annotations,
           },
         };
       }
     },
   });
   ```

3. Modify `buildNewTrackWorkflow()` in `pi-maestro/src/commands/newTrack.ts`:
   - After spec draft (step 3.5), insert step 3.6: call `tracklens_review` (see Section 5.3)
   - After plan draft (step 4.4), insert step 4.5: call `tracklens_review` for plan.md
   - After artifact creation (step 5.6), insert step 5.7: consolidated final review

4. Register `/tracklens` toggle command:
   ```typescript
   // pi-maestro/src/tracklens/extension/index.ts
   pi.registerCommand("tracklens", {
     description: "Toggle TrackLens visual review on/off",
     handler: async (args, ctx) => {
       const enabled = getTrackLensEnabled();
       setTrackLensEnabled(!enabled);
       ctx.ui.notify(`TrackLens ${!enabled ? "enabled" : "disabled"}`);
     },
   });
   ```

**Testing:**
```bash
bun test pi-maestro/src/tracklens/
# Manual: run /maestro:newTrack and verify TrackLens opens at spec/plan approval steps
```

### Phase 7: Walkthrough System (Week 6-7)

**Goal:** Full walkthrough generation + remediation loop for track completion.

**Tasks:**
1. Implement TS walkthrough generator (see Section 6.1)
2. Implement Rust walkthrough generator (see Section 8.1.4)

3. Modify `buildMaestroWorkflow()` in `pi-maestro/src/commands/implement.ts` — replace Section 4.0:
   ```
   ## 4.0 FINALIZE TRACK WITH WALKTHROUGH
   
   When all tasks in plan.md are complete:
   
   1. Call the `tracklens_walkthrough` tool with:
      - `trackId`: the track ID
      - `isSubtrack`: false (or true if this is a subtrack)
      - `parentTrackId`: parent track ID (if subtrack)
   
   2. The tool generates a walkthrough document with:
      - Completed tasks from plan.md
      - Changed files with diffs and code snippets
      - Spec summary for cross-reference
   
   3. TrackLens opens in the browser for review
   
   4. **If approved:**
      a. walkthrough-final.md is saved to the track directory
      b. Update track status to complete in tracks.md
      c. Bank memory with completion summary
      d. Announce completion
   
   5. **If denied with annotations:**
      a. Parse each annotation as a remediation task
      b. Execute remediations (fix code, update docs, etc.)
      c. Call `tracklens_walkthrough` again to generate a NEW walkthrough
      d. The new walkthrough reflects the remediated state
      e. Repeat until approved
   
   **CRITICAL:** Do NOT skip the walkthrough. Every track completion MUST go
   through TrackLens review unless the user has disabled it via /tracklens toggle.
   ```

4. Implement walkthrough compression for storage:
   ```typescript
   // pi-maestro/src/tracklens/walkthrough/storage.ts
   import { compress, decompress } from "@maestro/tracklens-shared/compress";
   
   export async function saveCompressedWalkthrough(trackDir: string, content: string): Promise<void> {
     const compressed = await compress(content);
     writeFileSync(join(trackDir, "walkthrough.compressed"), compressed);
     writeFileSync(join(trackDir, "walkthrough-final.md"), content);
   }
   
   export async function loadCompressedWalkthrough(trackDir: string): Promise<string | null> {
     const compressedPath = join(trackDir, "walkthrough.compressed");
     if (!existsSync(compressedPath)) return null;
     const compressed = readFileSync(compressedPath);
     return decompress(compressed);
   }
   ```

**Testing:**
```bash
# Full E2E: create track → implement → walkthrough → deny → remediate → approve
bun test pi-maestro/src/tracklens/walkthrough/
cargo test -p leindex-core tracklens::walkthrough
```

### Phase 8: Cockpit TUI + CLI (Week 7-8)

**Goal:** Wire TUI pane and CLI subcommand, polish, E2E test.

**Tasks:**
1. Create `crates/cockpit/src/tracklens/pane.rs` (see Section 8.1.5)
2. Create `crates/cockpit/src/tracklens/mod.rs`:
   ```rust
   pub mod pane;
   pub use pane::TrackLensPane;
   ```

3. Wire pane into Cockpit's tab system (`crates/cockpit/src/app.rs`):
   - Add `TrackLens` variant to the tab enum
   - Render `TrackLensPane` in the tab body
   - Update pane state when TrackLens server starts/stops

4. Create `crates/cli/src/commands/tracklens.rs` (see Section 8.1.6)

5. Wire CLI subcommand into `crates/cli/src/main.rs`:
   ```rust
   // In the Command enum:
   #[command(about = "TrackLens visual review and walkthrough")]
   Tracklens(tracklens::TrackLensArgs),
   
   // In the match:
   Command::Tracklens(args) => tracklens::run(args).await,
   ```

6. Build HTML bundles and embed/distribute:
   ```bash
   cd apps/tracklens-hook && bun run build
   # Output: dist/tracklens-editor.html (single-file bundle)
   # Copy to: ~/.maestro/tracklens/dist/
   ```

7. Add `src/leindex/src/tracklens/mod.rs` to leindex-core's `lib.rs`:
   ```rust
   pub mod tracklens;
   ```

**Testing:**
```bash
# Rust tests
cargo test -p leindex-core tracklens
cargo test -p maestro-cockpit tracklens

# CLI integration
cargo run -- tracklens review --file test.md --doc-type spec.md
cargo run -- tracklens walkthrough my-track-id --root .
cargo run -- tracklens code-review --git-ref HEAD~3

# Full E2E across all platforms
# 1. Claude Code: hook fires → TrackLens → approve → decision JSON
# 2. OpenCode: tool → server → review → feedback + agent switch
# 3. Pi-mono: newTrack → Q&A → spec → TrackLens → plan → TrackLens → done
# 4. Pi-mono: implement → tasks → complete → walkthrough → TrackLens → remediate → approve
# 5. CLI: maestro tracklens review/walkthrough/code-review
# 6. Cockpit: TrackLens pane shows active review + history
```

**Rebranding final audit:**
```bash
# MUST return zero results across ALL ported code:
grep -r "plannotator" apps/tracklens-*/ packages/tracklens-*/ pi-maestro/src/tracklens/ src/leindex/src/tracklens/ crates/cockpit/src/tracklens/ crates/cli/src/commands/tracklens.rs
grep -r "Plannotator" apps/tracklens-*/ packages/tracklens-*/ pi-maestro/src/tracklens/
grep -r "tater" apps/tracklens-*/ packages/tracklens-*/ pi-maestro/src/tracklens/
grep -r "backnotprop" apps/tracklens-*/ packages/tracklens-*/
grep -r "PLANNOTATOR_" apps/tracklens-*/ packages/tracklens-*/ pi-maestro/src/tracklens/
```

---

## 11. Testing Strategy

### 11.1 Per-Platform Tests
```bash
# Claude Code hook
bun test apps/tracklens-hook/

# OpenCode plugin
bun test apps/tracklens-opencode/

# Pi-mono extension
bun test pi-maestro/src/tracklens/

# Rust
cargo test -p leindex-core tracklens
cargo test -p maestro-cockpit tracklens
```

### 11.2 Rebranding Audit
```bash
# Must return ZERO results:
grep -r "plannotator" apps/tracklens-hook/ apps/tracklens-opencode/ pi-maestro/src/tracklens/ packages/tracklens-*/
```

### 11.3 End-to-End
1. **Claude Code:** Hook fires on ExitPlanMode → TrackLens opens → approve → decision JSON correct
2. **OpenCode:** Tool called → server starts → review → feedback returned with agent switch
3. **Pi-mono:** `maestro:newTrack` → Q&A questions (unchanged) → spec drafted → TrackLens review → approve/deny loop
4. **Walkthrough:** Track completes → walkthrough generated → TrackLens review → deny → remediate → new walkthrough → approve

---

## Appendix: Autonomy Mode Mapping

| Plannotator Permission | Maestro Autonomy Level | Behavior |
|---|---|---|
| `bypassPermissions` | `full-auto` | Agent auto-approves all tool calls |
| `acceptEdits` | `semi-auto` | Auto-approve file edits, confirm others |
| `default` | `checkpoint` | Confirm everything (default) |

This merges seamlessly with the conductor's existing execution modes.
