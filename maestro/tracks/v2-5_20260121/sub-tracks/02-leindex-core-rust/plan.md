# Sub-Track 02: LeIndex Core (Rust) = TLDR (No Python TLDR) - Plan

## Phase 1: Hard Gates (No TLDR)

### [ ] Task 1.1: Add repo gate: forbid `maestro.tldr` imports outside `maestro/archive/`
- [ ] Add CI job (or pre-commit hook) that fails on:
  - `rg -n "maestro\\.tldr" --glob '!maestro/archive/**'`
- [ ] Add explicit allowlist for planning docs if needed (but default: forbid in runtime)

### [ ] Task 1.2: Add repo gate: forbid `archive/tldr` execution paths (documentation-only)
- [ ] Fail build if runtime code references `maestro/archive/tldr` or attempts to execute those files
- [ ] Verify no `__init__.py` is added under `maestro/archive/` that would make it importable

### [ ] Task 1.3: Define compatibility policy for `/maestro:tldr` (alias vs removal)
- [ ] Decide:
  - A) keep `/maestro:tldr` as a compatibility alias (implemented via LeIndex), or
  - B) remove it and rebrand everything to `/maestro:leindex`
- [ ] Document the mapping table (old TLDR command → new LeIndex command)

## Phase 2: LeIndex CLI Surface (Canonical)

### [ ] Task 2.1: Specify commands and output formats (json, llm/balanced, ultra)
- [ ] Define canonical “analysis surface” commands:
  - file-level: `ast`, `callgraph`, `cfg`, `dfg`, `slicing`
  - project-level: phase1–phase5
  - search: `search`, `answer` (if supported)
- [ ] Define output formats:
  - `json`: machine readable (for Cockpit/orchestrate parsing)
  - `llm` (balanced): LLM-actionable, token efficient
  - `ultra`: exploration-only, maximum compression
- [ ] Define hard output caps per command (chars/lines) and truncation policy

### [ ] Task 2.2: Implement (or confirm existing) 5-layer commands in Rust CLI
- [ ] Confirm coverage across supported languages (tree-sitter):
  - Python/TS/JS/Rust/Go/Java/C/C++
- [ ] Ensure “callers/callees” UX exists (either as separate subcommands or query flags)
- [ ] Ensure slicing can target either:
  - (file, line) OR (function, line) deterministically

### [ ] Task 2.3: Implement 5-phase workflow helpers (phase1–phase5) as stable commands
- [ ] Convert the current `/phase1` UX in Cockpit analysis hub into first-class CLI commands:
  - `maestro leindex phase1 <root> --mode ultra|balanced --files N --chars N`
  - … through phase5
- [ ] Ensure phases are composable from orchestrate engine (machine readable mode)

### [ ] Task 2.4: Define stable “context bundle” format for orchestrate loops
- [ ] Define a JSON schema for:
  - task id + description
  - selected files + excerpts (token-truncated)
  - analysis summaries per layer
  - “commands to run” backpressure hints
- [ ] Provide both:
  - `json` bundle (for orchestrate engine)
  - `llm` bundle (for direct prompt injection)

## Phase 3: Hooks Migration

### [ ] Task 3.1: Replace TLDR-era hooks behavior with LeIndex-backed behavior
- [ ] Pre-tool-use read hook:
  - on `Read`, attach LeIndex AST/callgraph summaries for code files
- [ ] Pre-tool-use context hook:
  - on `Task` (and optionally `Edit`), attach LeIndex context bundle based on prompt intent
- [ ] Post-tool-use edit notify:
  - on `Edit`/`Write` success, invalidate LeIndex caches or trigger incremental reindex

### [ ] Task 3.2: Remove LeIndex hooks’ TLDR fallbacks
- [ ] Confirm no hooks attempt to import `maestro.tldr.*`
- [ ] Ensure hooks degrade gracefully when LeIndex is unavailable (no hard failures)

### [ ] Task 3.3: Ensure hooks fail safe (never corrupt tool payloads) and are fast
- [ ] Add timeouts for any subprocess/rust calls
- [ ] Ensure hook JSON output always includes the original tool payload unchanged
- [ ] Add lightweight unit tests for hook behavior on:
  - non-code files
  - missing files
  - very large files
  - permission errors

## Phase 4: Skills + Docs Migration

### [ ] Task 4.1: Update `maestro/skills/analysis/tldr-*` to reference LeIndex (or rename)
- [ ] Replace `tldr ...` CLI examples with the canonical LeIndex CLI surface
- [ ] Ensure router skill points at *real* commands (no dead/imagined CLI)
- [ ] Decide whether to rename directories `tldr-*` → `leindex-*` with aliases

### [ ] Task 4.2: Update `claude-code/commands/maestro:tldr.md` and `maestro:configure.md`
- [ ] Remove all `from maestro.tldr ...` snippets
- [ ] Describe `/maestro:tldr` as a LeIndex-backed compatibility interface (if retained)
- [ ] Ensure examples match actual CLI and hook names (`leindex-*`)

### [ ] Task 4.3: Update `README.md` and `plugin.json` CLI command list to match reality
- [ ] README:
  - update TUI build/run instructions (Rust)
  - update analysis commands to canonical LeIndex surface
- [ ] plugin.json:
  - list accurate commands installed by the plugin
  - remove Go dependency
  - ensure “TLDR” terminology is strictly legacy/alias, not implementation

## Phase 5: Cockpit Analysis UX

### [ ] Task 5.1: Define analysis workflows that match the 5-phase system
- [ ] Define “fast orientation” workflow:
  - phase1 structural scan (ultra)
  - phase2 dependency map (ultra)
- [ ] Define “implementation-ready” workflow:
  - targeted file/function context (balanced)
  - cfg/dfg/slice on demand

### [ ] Task 5.2: Update analysis tab UI/commands to use LeIndex as the engine
- [ ] Provide guided UI actions for phases (not just freeform command input)
- [ ] Persist analysis history with bounded storage
- [ ] Ensure analysis tab supports orchestrate engine by exporting bundles

### [ ] Task 5.3: Ensure analysis outputs are directly usable for implementation (balanced mode)
- [ ] Balanced mode includes:
  - signatures
  - line numbers
  - imports/exports
  - key call edges
- [ ] Ultra mode explicitly marked “exploration only”

## Phase 6: Verification

### [ ] Task 6.1: Manual validation: common analysis tasks succeed (AST/callers/cfg/dfg/slice)
- [ ] Validate on Maestro repo itself (Python + Rust mixed)
- [ ] Validate on a representative TypeScript repo (if available)

### [ ] Task 6.2: Token-efficiency validation (ultra vs balanced tradeoffs)
- [ ] Measure output sizes on small/medium/large files
- [ ] Confirm truncation policy preserves the most relevant context

### [ ] Task 6.3: Maestro - User Manual Verification 'Sub-Track 02' (Protocol in workflow.md)
