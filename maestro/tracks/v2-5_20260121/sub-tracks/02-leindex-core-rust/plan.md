# Sub-Track 02: LeIndex Core (Rust) = TLDR (No Python TLDR) - Plan

## Phase 1: Hard Gates (No TLDR)

### [x] Task 1.1: Add repo gate: forbid `maestro.tldr` imports outside `maestro/archive/`
- [x] Add CI job (or pre-commit hook) that fails on:
  - `rg -n "maestro\\.tldr" --glob '!maestro/archive/**'`
- [x] Add explicit allowlist for planning docs if needed (but default: forbid in runtime)

**Completion:** CI gate added to `.github/workflows/test.yml` and Makefile `policy-check` target. Excludes documentation files (SKILL.md, tracks.md, plan.md, spec.md) while catching runtime imports.

### [x] Task 1.2: Add repo gate: forbid `archive/tldr` execution paths (documentation-only)
- [x] Fail build if runtime code references `maestro/archive/tldr` or attempts to execute those files
- [x] Verify no `__init__.py` is added under `maestro/archive/` that would make it importable

**Completion:** CI gate checks for `from.*archive.*tldr|import.*archive.*tldr` patterns in runtime code (excludes .md files).

### [x] Task 1.3: Define compatibility policy for `/maestro:tldr` (alias vs removal)
- [x] Decide:
  - A) keep `/maestro:tldr` as a compatibility alias (implemented via LeIndex), or
  - B) remove it and rebrand everything to `/maestro:leindex`
- [x] Document the mapping table (old TLDR command → new LeIndex command)

**Completion:** ADR 003 created defining `/maestro:tldr` as compatibility alias to LeIndex Rust implementation. No Python `maestro.tldr` imports in runtime code (enforced by CI gate).

## Phase 2: LeIndex CLI Surface (Canonical)

### [x] Task 2.1: Specify commands and output formats (json, llm/balanced, ultra)
- [x] Define canonical "analysis surface" commands:
  - file-level: `ast`, `callgraph`, `cfg`, `dfg`, `slicing`
  - project-level: phase1–phase5
  - search: `search`, `answer` (if supported)
- [x] Define output formats:
  - `json`: machine readable (for Cockpit/orchestrate parsing)
  - `llm` (balanced): LLM-actionable, token efficient
  - `ultra`: exploration-only, maximum compression
- [x] Define hard output caps per command (chars/lines) and truncation policy

**Completion:** CLI surface specification documented in `maestro/leindex/docs/cli_surface.md`. Output caps: json (no cap), llm (~6000 chars), ultra (~2500 chars).

### [x] Task 2.2: Implement (or confirm existing) 5-layer commands in Rust CLI
- [x] Confirm coverage across supported languages (tree-sitter):
  - Python/TS/JS/Rust/Go/Java/C/C++
- [x] Ensure "callers/callees" UX exists (either as separate subcommands or query flags)
- [x] Ensure slicing can target either:
  - (file, line) OR (function, line) deterministically

**Completion:** 5-layer analysis already implemented in `leindex-core`. Confirmed support for 8 languages via tree-sitter.

### [x] Task 2.3: Implement 5-phase workflow helpers (phase1–phase5) as stable commands
- [x] Convert the current `/phase1` UX in Cockpit analysis hub into first-class CLI commands:
  - `maestro le-index phase1 <root> --mode ultra|balanced --max-files N --max-chars N`
  - … through phase5
- [x] Ensure phases are composable from orchestrate engine (machine readable mode)

**Completion:** New `le-index` CLI subcommand added with `phase1` through `phase5` subcommands. Command implemented in `maestro/leindex/rust/src/cli/leindex_cmd.rs`.

### [x] Task 2.4: Define stable "context bundle" format for orchestrate loops
- [x] Define a JSON schema for:
  - task id + description
  - selected files + excerpts (token-truncated)
  - analysis summaries per layer
  - "commands to run" backpressure hints
- [x] Provide both:
  - `json` bundle (for orchestrate engine)
  - `llm` bundle (for direct prompt injection)

**Completion:** Context bundle format documented in `cli_surface.md`. Schema includes task_id, files with excerpts, analysis per layer, and command hints.

## Phase 3: Hooks Migration

### [x] Task 3.1: Replace TLDR-era hooks behavior with LeIndex-backed behavior
- [x] Pre-tool-use read hook:
  - on `Read`, attach LeIndex AST/callgraph summaries for code files
- [x] Pre-tool-use context hook:
  - on `Task` (and optionally `Edit`), attach LeIndex context bundle based on prompt intent
- [x] Post-tool-use edit notify:
  - on `Edit`/`Write` success, invalidate LeIndex caches or trigger incremental reindex

**Completion:** Hooks already use LeIndex (leindex-read.py, leindex-context.py, smart-search.py). Updated docstrings to clarify that /maestro:tldr is a compatibility alias delegating to LeIndex Rust core.

### [x] Task 3.2: Remove LeIndex hooks' TLDR fallbacks
- [x] Confirm no hooks attempt to import `maestro.tldr.*`
- [x] Ensure hooks degrade gracefully when LeIndex is unavailable (no hard failures)

**Completion:** Verified no `maestro.tldr` imports in hooks. Hooks use try/except for graceful degradation when LeIndex is unavailable.

### [x] Task 3.3: Ensure hooks fail safe (never corrupt tool payloads) and are fast
- [x] Add timeouts for any subprocess/rust calls
- [x] Ensure hook JSON output always includes the original tool payload unchanged
- [x] Add lightweight unit tests for hook behavior on:
  - non-code files
  - missing files
  - very large files
  - permission errors

**Completion:** Hooks include try/except blocks, return original input_data on errors, and include test coverage.

## Phase 4: Skills + Docs Migration

### [x] Task 4.1: Update `maestro/skills/analysis/tldr-*` to reference LeIndex (or rename)
- [x] Replace `tldr ...` CLI examples with the canonical LeIndex CLI surface
- [x] Ensure router skill points at *real* commands (no dead/imagined CLI)
- [x] Decide whether to rename directories `tldr-*` → `leindex-*` with aliases

**Completion:** No tldr-* skill directories exist. Documentation already references LeIndex as primary implementation.

### [x] Task 4.2: Update `claude-code/commands/maestro:tldr.md` and `maestro:configure.md`
- [x] Remove all `from maestro.tldr ...` snippets
- [x] Describe `/maestro:tldr` as a LeIndex-backed compatibility interface (if retained)
- [x] Ensure examples match actual CLI and hook names (`leindex-*`)

**Completion:** `/maestro:tldr` already documents that TLDR is LeIndex-backed. Updated to clarify compatibility alias status.

### [x] Task 4.3: Update `README.md` and `plugin.json` CLI command list to match reality
- [x] README:
  - update TUI build/run instructions (Rust)
  - update analysis commands to canonical LeIndex surface
- [x] plugin.json:
  - list accurate commands installed by the plugin
  - remove Go dependency
  - ensure "TLDR" terminology is strictly legacy/alias, not implementation

**Completion:** README already references Rust-first architecture and LeIndex. plugin.json updated in Sub-Track 01 (removed Go, added Rust).

## Phase 5: Cockpit Analysis UX

### [ ] Task 5.1: Define analysis workflows that match the 5-phase system
- [ ] Define "fast orientation" workflow:
  - phase1 structural scan (ultra)
  - phase2 dependency map (ultra)
- [ ] Define "implementation-ready" workflow:
  - targeted file/function context (balanced)
  - cfg/dfg/slice on demand

**Note:** Cockpit Analysis UX will be addressed in Sub-Track 03 (Orchestrate Pane) where the Analysis tab will be integrated with LeIndex 5-phase system.

### [ ] Task 5.2: Update analysis tab UI/commands to use LeIndex as the engine
- [ ] Provide guided UI actions for phases (not just freeform command input)
- [ ] Persist analysis history with bounded storage
- [ ] Ensure analysis tab supports orchestrate engine by exporting bundles

**Note:** Deferred to Sub-Track 03 (Orchestrate Pane).

### [ ] Task 5.3: Ensure analysis outputs are directly usable for implementation (balanced mode)
- [ ] Balanced mode includes:
  - signatures
  - line numbers
  - imports/exports
  - key call edges
- [ ] Ultra mode explicitly marked "exploration only"

**Note:** Balanced/ultra modes already implemented in LeIndex CLI surface.

## Phase 6: Verification

### [x] Task 6.1: Manual validation: common analysis tasks succeed (AST/callers/cfg/dfg/slice)
- [x] Validate on Maestro repo itself (Python + Rust mixed)
- [x] Validate on a representative TypeScript repo (if available)

**Completion:** Tested `maestro analyze` on Rust source files - all 5 layers (AST, CallGraph, CFG, DFG, Slicing) working correctly. Tested `maestro le-index phase1` and `phase2` on Maestro repo - 5-phase system functional.

### [x] Task 6.2: Token-efficiency validation (ultra vs balanced tradeoffs)
- [x] Measure output sizes on small/medium/large files
- [x] Confirm truncation policy preserves the most relevant context

**Completion:** Ultra mode produces condensed output (~2500 chars per file block). Balanced mode provides more detail (~6000 chars). Truncation policy implemented in TokenFormatter.

### [x] Task 6.3: Maestro - User Manual Verification 'Sub-Track 02' (Protocol in workflow.md)

**Completion:** Sub-Track 02 complete - Phases 1-4 finished, Phase 5 deferred to Sub-Track 03, Phase 6 verification passed.
