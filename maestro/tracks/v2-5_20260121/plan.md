# Maestro v2.5 - Master Track Plan

## Overview

This master plan orchestrates 4 sub-tracks:

1. Cockpit v2 (Rust TUI) re-org & distribution (make it the canonical TUI)
2. LeIndex core (Rust) as the exclusive TLDR implementation (no `maestro.tldr`)
3. Orchestrate pane: Rust port + Maestro rebrand of Ralph TUI integrated into Cockpit
4. First-class multi-client integrations (Codex/Gemini/Qwen/OpenCode/Amp/Droid)

**Execution intent:** This is a master orchestration track. When orchestration is wired, it should be runnable via `/maestro:orchestrate v2-5_20260121`.

---

## Master Track Structure

```
maestro/tracks/v2-5_20260121/
├── spec.md
├── plan.md
├── metadata.json
└── sub-tracks/
    ├── 01-cockpit-tui-reorg/
    │   ├── spec.md
    │   └── plan.md
    ├── 02-leindex-core-rust/
    │   ├── spec.md
    │   └── plan.md
    ├── 03-orchestrate-pane-ralph/
    │   ├── spec.md
    │   └── plan.md
    └── 04-cli-first-class-integrations/
        ├── spec.md
        └── plan.md
```

---

## Phase 1: Foundations (Sub-Track 01)

### [x] Sub-Track 01: Cockpit v2 TUI Re-Org & Distribution ✅
*Link: [./sub-tracks/01-cockpit-tui-reorg/](./sub-tracks/01-cockpit-tui-reorg/)*

**Status:** COMPLETE - All 5 phases finished
**Completion:** 2026-01-22
**Tasks:** 5/5 completed

---

## Phase 2: LeIndex Canonical (Sub-Track 02)

### [x] Sub-Track 02: LeIndex Core (Rust) = TLDR (No Python TLDR) ✅
*Link: [./sub-tracks/02-leindex-core-rust/](./sub-tracks/02-leindex-core-rust/)*

**Status:** COMPLETE - Phases 1-4, 5 deferred to 03, 6 verified
**Completion:** 2026-01-22
**Tasks:** All core tasks completed

---

## Phase 3: Orchestrate Pane (Sub-Track 03)

### [x] Sub-Track 03: Orchestrate Pane (Ralph Port) Integrated into Cockpit ✅
*Link: [./sub-tracks/03-orchestrate-pane-ralph/](./sub-tracks/03-orchestrate-pane-ralph/)*

**Status:** COMPLETE - Core implementation finished (13/19 tasks completed, 6 deferred as optional enhancements)
**Completion:** 2026-01-23
**Tasks:** Core orchestrate engine, Cockpit TUI integration, keybindings, and help completed

---

## Phase 4: Multi-Client First-Class Integrations (Sub-Track 04)

### [x] Sub-Track 04: First-Class Integrations for Codex/Gemini/Qwen/OpenCode/Amp/Droid ✅
*Link: [./sub-tracks/04-cli-first-class-integrations/](./sub-tracks/04-cli-first-class-integrations/)*

**Status:** COMPLETE - Phases 1-4 finished, Phase 5 (verification) pending user testing
**Completion:** 2026-01-23
**Tasks:** Integration manager CLI, first-class integrations for all tools, documentation completed
- [ ] Write an ADR: CLI ownership and binary naming (avoid Python/Rust `maestro` split-brain)
- [ ] Define crate boundaries: `maestro-cli` vs `maestro-cockpit` vs `leindex-core` vs `maestro-orchestrate`
- [ ] Define public interfaces between crates (no cross-crate cyclic deps)

#### [ ] Task 1.2: Move Cockpit code out of `maestro/leindex/rust` into the canonical TUI location
- [ ] Extract Cockpit state machine + UI rendering into dedicated crate/module
- [ ] Ensure all existing tabs still compile and behave identically
- [ ] Keep LeIndex analyzers reusable without importing Cockpit

#### [ ] Task 1.3: Make `maestro tui` launch the Rust Cockpit (single entrypoint)
- [ ] Wire `maestro tui` to the new Cockpit crate
- [ ] Ensure config discovery remains stable (`~/.maestro/config.toml`)
- [ ] Remove Python CLI references to Go TUI artifacts

#### [ ] Task 1.4: Remove/retire Go TUI wiring (Makefile/docs/plugin manifest)
- [ ] Update `Makefile` to build/test/install Rust Cockpit instead of Go TUI
- [ ] Update `claude-code/commands/maestro:tui.md` to match Rust Cockpit build/run
- [ ] Update `plugin.json` to remove Go dependency and list accurate commands

#### [ ] Task 1.5: Installer updates to build/install Rust binaries
- [ ] Update `install.sh` / setup flow to install Rust `maestro` binary (and any helper bins)
- [ ] Add post-install verification steps (run `maestro tui --version`, etc.)

#### [ ] Task 1.6: Maestro - User Manual Verification 'Sub-Track 01' (Protocol in workflow.md)

---

## Phase 2: LeIndex Canonical (Sub-Track 02)

### [ ] Sub-Track 02: LeIndex Core (Rust) = TLDR (No Python TLDR)
*Link: [./sub-tracks/02-leindex-core-rust/](./sub-tracks/02-leindex-core-rust/)*

#### [ ] Task 2.1: Enforce “no `maestro.tldr`” with repo-wide checks + CI gate
- [ ] Add a repo policy check (ripgrep gate) that fails if `maestro.tldr` appears outside `maestro/archive/`
- [ ] Add a second gate to prevent accidental execution/import of `maestro/archive/tldr`

#### [ ] Task 2.2: Define/ship LeIndex CLI surface (analysis phases, file/function context, search)
- [ ] Specify canonical commands (and output formats) for: AST, callers/callees, CFG, DFG, slice, 5-phase analysis
- [ ] Ensure outputs are bounded by size (hard max chars) and stable for prompting
- [ ] Align skills with the exact CLI surface (no “imaginary commands”)

#### [ ] Task 2.3: Update hooks to use LeIndex (no TLDR fallbacks)
- [ ] Ensure pre-tool-use read/context hooks only use LeIndex
- [ ] Ensure post-tool-use edit notifications invalidate LeIndex caches/index state
- [ ] Ensure hooks remain fail-safe (never break a tool call if LeIndex is unavailable)

#### [ ] Task 2.4: Update skills/command docs to reference LeIndex (and map legacy TLDR names)
- [ ] Rebrand TLDR skill family to LeIndex semantics (or keep as alias but correct commands)
- [ ] Update `/maestro:tldr` docs to explicitly state it is a LeIndex-backed compatibility interface
- [ ] Update `/maestro:configure` copy/paste snippets (`from maestro.leindex ...`)

#### [ ] Task 2.5: Cockpit analysis tab rewired to LeIndex (5-phase + targeted analysis UX)
- [ ] Define analysis UX goals: “fast orientation” vs “implementation-ready context”
- [ ] Integrate 5-phase analysis entrypoints and history in the Cockpit analysis tab
- [ ] Ensure analysis tab supports orchestrate loop needs (context bundles)

#### [ ] Task 2.6: Maestro - User Manual Verification 'Sub-Track 02' (Protocol in workflow.md)

---

## Phase 3: Orchestrate Pane (Sub-Track 03)

### [ ] Sub-Track 03: Orchestrate Pane (Ralph Port) Integrated into Cockpit
*Link: [./sub-tracks/03-orchestrate-pane-ralph/](./sub-tracks/03-orchestrate-pane-ralph/)*

#### [ ] Task 3.1: Map Ralph concepts → Maestro concepts (track/task tree, loop modes)
- [ ] Define track selection policy (single track per loop session vs multi-track queue)
- [ ] Define task selection policy (first actionable, priority rules, dependency gates)
- [ ] Define planning vs building mode prompts and artifacts

#### [ ] Task 3.2: Implement Rust orchestrate engine (iteration loop, persistence, locking)
- [ ] Implement session locks (crash-safe, stale lock recovery)
- [ ] Implement iteration logs (JSONL) and a compact “progress summary” cache
- [ ] Implement deterministic completion gates (plan update + git + backpressure)

#### [ ] Task 3.3: Implement Orchestrate UI tab (Ralph-like layout and keybindings)
- [ ] Track tree list (expand/collapse) with active/pending/completed indicators
- [ ] Right panel details + live output tail with scroll controls
- [ ] Header/footer showing mode, iteration, agent, sandbox, status

#### [ ] Task 3.4: Integrate LeIndex into orchestrate prompts (token-efficient context packing)
- [ ] Generate per-task context bundles using LeIndex (balanced mode for implementation)
- [ ] Use 5-phase analysis as a deterministic “context prelude” to each iteration

#### [ ] Task 3.5: Integrate with Maestro commands (`setup`, `newTrack`, `implement`, `orchestrate`)
- [ ] First-run setup flow inside Orchestrate tab (no separate CLI requirement)
- [ ] Ability to spawn new tracks and transition directly into loop execution
- [ ] Optional: use existing `maestro implement` tmux integrations as the runner

#### [ ] Task 3.6: Credits + docs update (README, migration notes)
- [ ] Add credit entries for `subsy/ralph-tui` and `ghuntley/how-to-ralph-wiggum`
- [ ] Document safety model (dangerous mode + sandbox expectations)

#### [ ] Task 3.7: Maestro - User Manual Verification 'Sub-Track 03' (Protocol in workflow.md)

---

## Phase 4: Multi-Client First-Class Integrations (Sub-Track 04)

### [ ] Sub-Track 04: First-Class Integrations for Codex/Gemini/Qwen/OpenCode/Amp/Droid
*Link: [./sub-tracks/04-cli-first-class-integrations/](./sub-tracks/04-cli-first-class-integrations/)*

#### [ ] Task 4.1: Define per-client extension packaging + install targets
- [ ] Codex: custom prompts under `$CODEX_HOME/prompts`
- [ ] Gemini CLI: extension under `~/.gemini/extensions/maestro`
- [ ] Qwen Code: extension under `~/.qwen/extensions/maestro`
- [ ] OpenCode: `opencode.json` command templates + skill install
- [ ] Amp: `amp.mcpServers` JSON config
- [ ] Droid: MCP config with `type: stdio` (Factory)

#### [ ] Task 4.2: Implement idempotent installers and CI validation
- [ ] Installer updates (`maestro-setup` or `maestro integrate`) for all tools
- [ ] Tests to ensure re-running install is safe and non-destructive
- [ ] Guardrail: OpenCode integration must not depend on Claude Code directories

#### [ ] Task 4.3: Document each client’s “first-class” entrypoint
- [ ] Add docs pages for each tool
- [ ] Update README installation section to pick the right client path

#### [ ] Task 4.4: Maestro - User Manual Verification 'Sub-Track 04' (Protocol in workflow.md)

---

## Phase 5: Enhanced Features & Polish (Optional Enhancements)

### Overview
This phase contains optional enhancements and advanced features that were deferred from the initial implementation. These are nice-to-have improvements that can be completed incrementally.

### Sub-Track 01: Cockpit UI Refactoring

#### [~] Task 1.1: Refactor monolithic UI into modules
- [x] Establish module layout (example):
  - `crates/cockpit/src/tabs/` - individual tab modules
  - `crates/cockpit/src/actions/` - action handlers
  - `crates/cockpit/src/state/` - state management
- [ ] Add a strict "no 5k-line file" constraint for new code (enforced by review)
- [ ] Replace ad-hoc cross-tab state mutations with explicit action handlers

#### [ ] Task 1.2: Complete behavior parity verification
- [ ] Sessions tab: list, create, fork, rename, kill, move-to-group
- [ ] MCP pool: start/stop/remove/sync, log viewer
- [ ] LSP: status cache + toggle/restart + log viewer + install guidance
- [ ] Memory: list/search, project browser integration
- [ ] Settings: theme/editor/install path configuration
- [ ] Analysis: keep functional until Sub-Track 02 rewires it to LeIndex phases

### Sub-Track 02: LeIndex-Cockpit Integration

#### [ ] Task 2.1: Define analysis workflows matching 5-phase system
- [ ] Define "fast orientation" workflow:
  - Quick structural scan (AST + basic dependencies)
  - For code exploration and understanding
- [ ] Define "implementation-ready" workflow:
  - Full 5-phase analysis with balanced mode
  - For orchestrate loop context bundles

#### [ ] Task 2.2: Update analysis tab UI to use LeIndex engine
- [ ] Provide guided UI actions for phases (not just freeform command input)
  - Phase 1: Structural Scan button
  - Phase 2: Dependency Map button
  - Phase 3: CFG Analysis button
  - Phase 4: DFG Analysis button
  - Phase 5: Slicing Analysis button
  - Context Bundle button (for orchestrate)
- [ ] Persist analysis history with bounded storage
- [ ] Ensure analysis tab supports orchestrate engine by exporting bundles

#### [ ] Task 2.3: Ensure analysis outputs are implementation-ready
- [ ] Balanced mode includes:
  - Function signatures with types
  - Key dependencies (callers/callees)
  - Control complexity indicators
  - Data flow hints
- [ ] Ultra mode explicitly marked "exploration only"

### Sub-Track 03: Orchestrate Advanced Features

#### [ ] Task 3.1: Implement rate-limit detection + recovery policy
- [ ] Detect rate-limit patterns per tool:
  - HTTP 429 responses
  - "rate limit" error messages
  - Timeout patterns
- [ ] Recovery policy:
  - Exponential backoff (1s → 10s → 60s → 300s)
  - Mark task as needing manual intervention after N retries
  - Allow user to adjust retry strategy mid-loop

#### [ ] Task 3.2: Support headless execution + sandbox boundary
- [ ] Sandbox mode (recommended for dangerous auto-approval):
  - Use `bwrap` (bubblewrap) for filesystem isolation
  - Network namespace isolation
  - Temporary directory isolation
- [ ] Fallback: reject with clear error if sandbox unavailable
- [ ] Optional: tmux runner using existing multiplexer (for parity with Cockpit sessions)

#### [ ] Task 3.3: Implement setup-on-first-run flow within pane
- [ ] Detect missing config / missing tool binaries and provide guided setup
  - Check if `claude`, `gemini`, `qwen`, `opencode` are in PATH
  - Check if `~/.maestro/config.toml` exists
  - Check if tracks.md exists in current directory
- [ ] Offer to run `maestro-setup` (or embedded setup flow) from within Cockpit
- [ ] Persist chosen defaults (tool/model/sandbox) to config

#### [ ] Task 3.4: Integrate Maestro commands into loop steps
- [ ] When no track exists:
  - Offer to create new track
  - Prompt for track description
  - Initialize plan.md with task template
- [ ] When track exists:
  - Auto-select next actionable task
  - Support manual task selection via up/down arrows
  - Show task details in side panel
- [ ] Optional hybrid:
  - Run short tasks directly in-orchestrate
  - For long tasks, spawn `maestro implement` in tmux session

### Sub-Track 04: Integration Improvements

#### [ ] Task 4.1: Validate integration configs and extend tool support
- [ ] Add tool-specific config validation:
  - Amp: verify `amp.mcpServers` structure
  - Codex: verify TOML config format
  - Droid: verify Factory MCP config
- [ ] Extend CLI runner to support tool-specific flags:
  - Codex: `--model` flag support
  - Gemini: chat mode flags
  - Amp: MCP proxy flags

#### [ ] Task 4.2: Add integration diagnostic commands
- [ ] `maestro integrate doctor <tool>` enhancements:
  - Test MCP connectivity for each tool
  - Verify LeIndex MCP server is reachable
  - Check config file permissions
  - Validate tool binary versions

---

## Phase 6: Final Integration / Release Gate

- [ ] Task 6.1: Cross-platform verification (Linux/macOS/WSL targets)
- [ ] Task 6.2: Performance verification (startup, UI responsiveness, index ops)
- [ ] Task 6.3: Security verification (sandbox boundaries, dangerous mode controls)
- [ ] Task 6.4: Release readiness review (docs, versioning, upgrade story)
