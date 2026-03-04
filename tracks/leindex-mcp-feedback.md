# LeIndex MCP Server Feedback — Detailed Experience Report

**Date:** 2026-03-04  
**Context:** Investigating plannotator project (~92 files, TypeScript/React monorepo) and maestro project (~24,000 indexed nodes) to produce a detailed porting plan.  
**Thread:** T-019cb764-0577-70ef-9bae-1ec3ee43fff5

---

## 1. Overall Assessment

**Rating: 7/10** — LeIndex provided strong structural analysis and symbol-level understanding that would be impractical to assemble manually, but several friction points forced fallback to `Read`.

---

## 2. What Worked Well

### 2.1 `leindex_project_map` — Excellent
This was the single most valuable tool. It gave me:
- A **complete 92-file inventory** of plannotator with symbol counts and complexity scores
- **Top symbols per file** — immediately revealing which files contain critical logic
- **Complexity hotspot ranking** — `packages/editor/App.tsx` (complexity 66, 66 symbols) stood out immediately as the UI hub

For initial project comprehension, this alone replaced what would have been 15-20 `find`/`ls` commands and manual file-by-file scanning. The `include_symbols=true` flag was critical for this to be useful.

### 2.2 `leindex_phase_analysis` — Strong
The 5-phase analysis provided:
- **201 internal import edges** mapped across the monorepo
- **Entry point detection**: correctly identified `apps/pi-extension/index.ts:execute`, `packages/server/storage.ts:saveFinalSnapshot`, and `apps/paste-service/core/handler.ts:handleRequest` as key entry points
- **Focus files**: the top-5 focus files were exactly right for understanding the project
- **Hotspot scoring**: `packages/editor/App.tsx` (score 0.515, impact 64) correctly identified as the highest-impact module

### 2.3 `leindex_file_summary` — Very Good
This was the workhorse tool for understanding individual files. The `include_source=true` flag provided:
- **Symbol signatures with source snippets** — enough to understand each function's purpose without reading the full file
- **Cross-file dependency maps** — e.g., seeing that `packages/editor/App.tsx` depends on 32 other modules
- **Caller/callee relationships** — e.g., `plannotator()` depends on `togglePlanMode`, `exitToIdle`, `enterPlanning`, `persistState`, `updateWidget`, `updateStatus`, `resolvePlanPath`, `getTextContent`, `isAssistantMessage`

For most files, this was sufficient. The source snippets were truncated but provided enough context to understand the API surface.

### 2.4 `leindex_deep_analyze` — Good for Discovery
When searching for "newTrack command implementation" across maestro's ~24,000 nodes, it:
- Found `buildNewTrackWorkflow` (the exact function I needed) ranked #1 with score 0.318
- Found `registerNewTrack` ranked #5
- Found `get_new_track_command` in the Rust cockpit crate ranked #4
- Found `TrackPlan` and `MaestroTrack` Rust models

The semantic search quality was solid — it understood the intent of "track creation flow" and returned relevant results.

### 2.5 `leindex_search` — Good
The semantic search for "implement command track execution completion workflow" correctly returned:
- `Command` import in `implement.rs` (#1)
- `simulate_command_execution` in test files (#2)
- `buildMaestroWorkflow` in both TS and JS (#5, #6)
- `ImplementSessionTarget` (#7)

### 2.6 `leindex_index` — Fast
Both projects indexed instantly (0ms reported) with cached results, making re-indexing friction-free.

---

## 3. What Forced Fallback to `Read`

### 3.1 Source Code Retrieval Failures — The Primary Issue

**This was the dominant problem.** When `leindex_deep_analyze` found relevant symbols, it **could not read the source code** for any file under the maestro project:

```
// Symbol: buildNewTrackWorkflow
// File: /run/media/scooter/W.D SSD/Prod/maestro/pi-maestro/src/commands/newTrack.ts
// [Error: Could not read file: /run/media/scooter/W.D SSD/Prod/maestro/pi-maestro/src/commands/newTrack.ts]
```

This error appeared for **every single file** in the context expansion. The issue is a **mount point mismatch**: LeIndex indexes the project at `/run/media/scooter/W.D SSD/Prod/maestro` but the actual filesystem mount is `/mnt/WD-SSD/Prod/maestro`. The index stores file paths using one mount point, but at read time it cannot find them at the other.

**Impact:** I had to use `Read` for:
- `pi-maestro/src/commands/newTrack.ts` (308 lines) — the entire newTrack workflow
- `pi-maestro/src/commands/implement.ts` (304 lines) — the implement workflow  
- `pi-maestro/src/lib/project.ts` (137 lines) — project utilities
- `apps/pi-extension/index.ts` (668 lines) — plannotator extension entry
- `apps/pi-extension/server.ts` (513 lines) — server implementations
- `apps/pi-extension/utils.ts` (103 lines) — utility functions
- `packages/server/git.ts` (150 lines) — git utilities
- `packages/review-editor/App.tsx` (930 lines) — review editor
- `packages/editor/App.tsx` — needed full source for UI understanding
- Several other files

**If file reading had worked**, I could have relied entirely on `leindex_file_summary` with `include_source=true` and `leindex_read_symbol` for targeted symbol reading, which would have been far more token-efficient.

### 3.2 `leindex_read_symbol` — Not Tested Due to File Read Failures

I avoided `leindex_read_symbol` because if `leindex_deep_analyze` couldn't read files, `leindex_read_symbol` would likely fail the same way. This tool would have been the ideal replacement for targeted `Read` calls (reading only specific functions instead of entire files).

### 3.3 `leindex_context` — Not Used for Same Reason

The gravity traversal in `leindex_context` would have been perfect for understanding how `buildNewTrackWorkflow` connects to `registerNewTrack` and the event system, but the file read errors would have produced empty context blocks.

---

## 4. What Was Insufficient

### 4.1 Token Budget Limits on `leindex_file_summary`

For large files like `packages/editor/App.tsx` (1539 lines, 66 symbols), even `token_budget=5000` only provided:
- Source snippets for the top 2-3 symbols (the rest got truncated)
- Full dependency lists (which were very useful)
- But not enough source to understand the complete approval/deny flow

I needed `Read` to understand the `handleAnnotateFeedback`, `handleApprove`, `handleDeny` logic in the editor — the exact code paths relevant to the porting plan.

### 4.2 No Full-File Read Capability

LeIndex doesn't have a "read this file" tool — it's designed for structural/symbolic analysis. This is by design, but when you need to understand specific imperative logic (e.g., the HTTP server request handler's routing logic in `server.ts:fetch()`), structural analysis isn't enough. You need to read the actual code.

**Recommendation:** A `leindex_read_file` tool that reads a file with line ranges (like `Read`) but with the added benefit of annotating symbols and cross-references inline would bridge this gap perfectly.

### 4.3 Cross-Project Analysis

I indexed both `plannotator` and `maestro` separately, which worked, but there's no way to query **across** projects. For a porting task, being able to ask "find all symbols in plannotator that have equivalents in maestro" would have been transformative.

---

## 5. Detailed Recommendations

### 5.1 Critical: Fix Mount Point Path Resolution

**Priority: P0**

The `/run/media/scooter/W.D SSD/` vs `/mnt/WD-SSD/` mount point discrepancy caused every file read to fail. The index stores absolute paths from one mount point, but reads attempt the other.

**Suggestions:**
- Store relative paths in the index, resolving to absolute at query time using the provided `project_path`
- Or: detect and handle symlinks/mount aliases when indexing
- Or: allow a `path_prefix_remap` option: `{"/run/media/scooter/W.D SSD/": "/mnt/WD-SSD/"}`

### 5.2 High: Add `leindex_read_file` Tool

**Priority: P1**

A tool that reads file content (with line ranges) but adds inline symbol annotations:

```
// Reading: packages/server/index.ts [lines 50-80]
// Symbols in range: startPlannotatorServer (Function, complexity 2, 15 callers)
50: async function startPlannotatorServer(
51:   options: ServerOptions  // → ServerOptions (interface, line 10)
52: ): Promise<ServerResult> {  // → ServerResult (interface, line 25)
```

This would make `Read` unnecessary for 90% of cases.

### 5.3 Medium: Higher Default Token Budget for `file_summary`

**Priority: P2**

Default of 1000 tokens is too low for files with 30+ symbols. Recommend 3000 default, with the option to go to 10000 for complex files.

### 5.4 Medium: Cross-Project Symbol Matching

**Priority: P2**

For porting/migration tasks, a tool like `leindex_cross_match` that takes two indexed projects and finds:
- Symbols with similar names
- Symbols with similar signatures
- Modules with similar dependency patterns

### 5.5 Low: Batch `file_summary` 

**Priority: P3**

Allow `leindex_file_summary` to accept an array of file paths and return summaries for all of them in one call. This would reduce round-trips when surveying a directory of related files.

### 5.6 Low: Export Index as Markdown

**Priority: P3**

A `leindex_export` tool that outputs the full project index as a structured markdown document — useful for feeding into planning prompts or documentation generation.

---

## 6. Usage Statistics for This Session

| Tool | Calls | Purpose | Satisfaction |
|------|-------|---------|-------------|
| `leindex_index` | 2 | Index plannotator + maestro | ✅ Perfect |
| `leindex_project_map` | 1 | Full project overview of plannotator | ✅ Excellent |
| `leindex_phase_analysis` | 1 | 5-phase analysis of plannotator | ✅ Very good |
| `leindex_deep_analyze` | 3 | Search for newTrack, track completion, conductor | ⚠️ Results good, file reads failed |
| `leindex_search` | 1 | Search for implement command | ✅ Good |
| `leindex_file_summary` | 11 | Detailed file analysis | ✅ Very good (when token budget sufficient) |
| `Read` (fallback) | 12 | Read full file contents | ⚠️ Forced by file read failures |

**Estimated token savings if file reads had worked:** ~60-70%. Instead of reading 12 full files (~4500 lines total), I could have used `leindex_read_symbol` for the ~25 specific functions I needed (~800 lines equivalent).

---

## 7. Summary

LeIndex is a strong tool for **structural comprehension** — understanding project architecture, symbol relationships, dependency graphs, and complexity hotspots. The `project_map` and `phase_analysis` tools are best-in-class for initial project understanding.

The critical blocker in this session was the **file path mount point mismatch** causing all file reads to fail. If this single issue were fixed, the LeIndex toolset would have been sufficient for 90%+ of the analysis work with significantly lower token consumption.

The tool fills a genuine gap between "grep for strings" and "read entire files" — it provides **semantic, structural understanding** that neither approach offers. For a task like porting a 92-file project, that structural understanding is exactly what's needed.
