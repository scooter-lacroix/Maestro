# Sub-Track 03: Orchestrate Pane (Ralph Port) Integrated into Cockpit - Plan

## Phase 1: Concept Mapping & Data Model

### [x] Task 1.1: Define Track/Task model in Rust (lossless parsing of tracks + plan.md)
- [x] Parse `maestro/tracks.md` into:
  - track id
  - description
  - status ([ ]/[~]/[x])
  - link path
- [x] Parse `maestro/tracks/<track_id>/metadata.json` for type + subtracks
- [x] Parse `plan.md` into a task tree:
  - hierarchical bullets
  - status markers `[ ]/[~]/[x]`
  - preserve text + ordering
- [x] Implement a lossless writer:
  - only change status markers / small appended notes
  - never reflow user prose unexpectedly

**Completion:** Data models defined in `src/orchestrate/model.rs` with Track, Task, TrackPlan, and supporting types. Lossless parser implemented in `src/orchestrate/parser.rs`.

### [x] Task 1.2: Map Ralph "tracker" abstraction to Maestro tracks (source-of-truth policy)
- [x] Decide what the orchestrate engine treats as canonical state:
  - `tracks.md` vs per-track `plan.md` vs `metadata.json`
- [x] Define dependency semantics:
  - task-level dependencies (if present) vs phase ordering
  - track-level dependencies (subtracks)
- [x] Define "actionable" vs "blocked" rule set (Ralph parity):
  - actionable if dependencies are satisfied
  - blocked if any dependency incomplete

**Completion:** TaskDependency and DependencyType (Hard/Soft) defined. `is_actionable()` and `is_blocked()` methods implemented.

### [x] Task 1.3: Define iteration state persistence format (lock + journal + logs)
- [x] Define on-disk state directory (recommended):
  - `~/.maestro/orchestrate/`
- [x] Persist:
  - session metadata (selected track, mode, agent tool/model, sandbox)
  - current iteration counter
  - current task id/path
  - last N iteration summaries (for prompt context)
- [x] Locking model:
  - lock file per project path + track id
  - stale lock detection + safe recovery
- [x] Journal model:
  - append-only event log (for crash recovery)

**Completion:** StateManager, SessionLock, LockGuard, SessionState, IterationLog implemented in `src/orchestrate/state.rs`.

## Phase 2: Execution Engine (Rust)

### [x] Task 2.1: Implement iteration lifecycle (select → prompt → run → detect completion → update)
- [x] Task selection:
  - choose the highest-priority actionable plan task
  - mark it `[~]` before execution
- [x] Prompt build:
  - include track spec context + selected task details
  - include LeIndex context bundle (balanced mode)
  - include "recent progress summary" (last N iterations)
- [x] Agent execution:
  - run in isolated context (new process / new tmux pane / sandbox)
  - capture stdout/stderr continuously
- [x] Completion detection:
  - require plan update + git commit + backpressure success (preferred)
  - accept `<promise>COMPLETE</promise>` only as an additional signal
- [x] Finalize iteration:
  - mark task `[x]`, update plan notes, write session state, rotate logs

**Completion:** OrchestrateEngine with start/pause/resume/abort methods implemented in `src/orchestrate/engine.rs`. Loop lifecycle with timeout support.

### [x] Task 2.2: Implement error strategies (retry/skip/abort) + backoff
- [x] Implement per-task retry counters
- [x] Implement "skip task" tracking (avoid infinite loops)
- [x] Add exponential backoff for repeated failures

**Completion:** ErrorStrategy enum (Retry/Skip/Abort) and handle_task_failure method implemented.

### [x] Task 2.3: Implement structured logging (per-iteration, searchable)
- [x] Per-iteration log file:
  - timestamped JSONL events (start/stop/output/chosen-task/errors)
- [x] Session summary file:
  - compact markdown/JSON summary for Cockpit display
- [x] Optional: persist "codebase patterns" learned (Ralph's progress.md analog)

**Completion:** IterationLog with JSONL persistence. Recent iterations query for context.

### [x] Task 2.4: Implement interruption handling (Ctrl-C, graceful stop)
- [x] First Ctrl-C pauses loop and persists state
- [x] Second Ctrl-C aborts and marks session as interrupted
- [x] Ensure terminal state + tmux state remains consistent

**Completion:** SessionStatus enum (Idle/Running/Paused/Completed/Failed/Interrupted). pause/resume/abort methods implemented.

### [ ] Task 2.5: Implement rate-limit detection + recovery policy (recommended for autonomy)
- [ ] Detect rate-limit patterns per tool:
  - Claude CLI: known messages + HTTP 429
  - others: configurable regex patterns
- [ ] Recovery policy:
  - wait-and-retry for primary agent
  - optional fallback agent/tool selection (if configured)

**Note:** Deferred to future enhancement.

## Phase 3: Agent Runner Integrations

### [x] Task 3.1: Define agent runner interface (claude/codex/opencode/amp/shell)
- [x] Normalize "run one iteration" across tools:
  - stdin prompt mode vs file prompt
  - working directory handling
  - environment variables
  - output format options (plain vs structured)
- [x] Define runner capabilities matrix:
  - supports non-interactive mode
  - supports auto-approval / dangerous mode
  - supports structured output

**Completion:** DynAgentRunner trait and AgentRunner wrapper implemented in `src/orchestrate/runner.rs`. Support for claude/gemini/qwen/opencode tools.

### [x] Task 3.2: Implement runner(s) with consistent output capture and completion detection
- [x] Implement process runner with:
  - streaming output capture
  - timeouts
  - exit code capture
- [ ] Optional: tmux runner using existing multiplexer (for parity with Cockpit sessions)
- [x] Implement output parser hooks:
  - `<promise>COMPLETE</promise>` detection
  - subagent trace parsing (if tool supports structured output)

**Completion:** CliRunner implemented with streaming output capture via BufReader, 300s timeout, exit code capture. Completion detection via `<promise>COMPLETE</promise>` pattern matching.

### [ ] Task 3.3: Support headless execution + optional sandbox boundary
- [x] Headless mode:
  - orchestrate loop runs without Cockpit UI, logs to disk/stdout
- [ ] Sandbox mode (recommended for dangerous auto-approval):
  - Linux: bubblewrap (bwrap) profile
  - macOS: sandbox-exec profile (or documented alternative)
  - Explicit file allowlist (project dir + minimal tool cache)

**Completion:** Engine supports headless execution (can run without TUI). Sandbox mode deferred to future enhancement.

## Phase 4: Orchestrate UI (Cockpit Tab)

### [x] Task 4.1: Add Orchestrate tab and routing in Cockpit
- [x] Add a new tab index for Orchestrate
- [x] Wire tab navigation + state storage

**Completion:** Orchestrate tab (index 4) already integrated in Cockpit. Full routing implemented in `crates/cockpit/src/app.rs` with keybindings.

### [x] Task 4.2: Implement Ralph-like panels (left tree, right details/output)
- [x] Left:
  - tracks list with expand/collapse
  - selected track shows nested plan tasks
  - status indicators: active/pending/actionable/blocked/completed
- [x] Right:
  - selected task details (spec excerpt, acceptance criteria, dependencies)
  - live output viewer with scroll and "follow tail" mode
- [x] Bottom:
  - iteration stats (duration, commits, tests run)

**Completion:** Full Ralph-like UI implemented in `crates/cockpit/src/orchestrate.rs`. Left panel shows track/task tree with expand/collapse. Right panel shows task details and live output with scroll.

### [x] Task 4.3: Implement keybindings + help overlay
- [x] Navigation:
  - arrows/jk for list movement
  - enter to expand/collapse
- [x] Control:
  - `s` start
  - `p` pause
  - `r` resume
  - `x` abort
  - `?` help

**Completion:** All keybindings implemented in app.rs for orchestrate tab. Track navigation (o/O), task expansion (Space), control keys (s/p/r/x/c), and help overlay updated.

### [ ] Task 4.4: Implement setup-on-first-run flow within pane
- [ ] Detect missing config / missing tool binaries and provide guided setup
- [ ] Offer to run `maestro-setup` (or embedded setup flow) from within Cockpit
- [ ] Persist chosen defaults (tool/model/sandbox) to config

**Note:** Setup flow deferred to future enhancement. Current implementation assumes tracks.md exists.

## Phase 5: LeIndex + Prompt Integration

### [x] Task 5.1: Define prompt templates (planning/building) for Maestro flavor
- [x] Planning prompt:
  - forbid implementation/commits
  - require plan regeneration / reprioritization
  - require LeIndex usage for "don't assume not implemented"
- [x] Building prompt:
  - exactly one task per iteration
  - require tests/backpressure
  - require plan update + commit
- [x] Keep templates short and deterministic (Ralph playbook principle)

**Completion:** PromptBuilder implemented in `src/orchestrate/prompts.rs` with separate planning and building templates. Used by `build_prompt()` method in `src/orchestrate/engine.rs`.

### [x] Task 5.2: Integrate LeIndex context bundles (5-phase, targeted extraction)
- [x] Provide "context budget" policy:
  - phase1/phase2 ultra for orientation
  - balanced for actionable code generation
- [x] Ensure the engine can request:
  - targeted file context
  - callers/callees
  - cfg/dfg/slice for risk spots

**Completion:** LeIndex 5-phase analysis integrated via `get_leindex_context()` method. Phase1 and Phase2 summaries included in prompts. Context budget configurable via `context_budget` setting.

### [x] Task 5.3: Implement CLI command interface (`maestro orchestrate`)
- [x] Add `orchestrate` subcommand to maestro CLI
- [x] Subcommands: start, pause, resume, abort, status, list
- [x] Support for planning/building modes
- [x] Agent configuration (tool, model, dangerous mode, sandbox)
- [x] Error strategy configuration (retry/skip/abort)

**Completion:** CLI commands implemented in `src/cli/orchestrate.rs` and integrated in `crates/cli/src/main.rs`. Full `maestro orchestrate` subcommand with start/pause/resume/abort/status/list operations.

### [ ] Task 5.4: Integrate Maestro commands (`setup`, `newTrack`, `implement`) into loop steps
- [ ] When no track exists:
  - orchestrate pane can trigger `maestro:newTrack` flow (or equivalent)
- [ ] When track exists:
  - orchestrate pane can run building loop against it
- [ ] Optional hybrid:
  - orchestrate loop uses `maestro implement` to run in tmux sessions managed by Cockpit

**Note:** Integration with Maestro commands deferred to future enhancement. Current implementation uses direct agent runner execution.

## Phase 6: Documentation + Credits

### [x] Task 6.1: Add credits in `README.md`
- [x] Credit `subsy/ralph-tui` (MIT) and link to repo
- [x] Credit `ghuntley/how-to-ralph-wiggum` and link to repo
- [x] Add any additional upstream inspirations used

**Completion:** Credits added to README.md acknowledging Ralph TUI inspirations.

### [x] Task 6.2: Add user docs: how to use Orchestrate pane + safety notes
- [x] Document:
  - planning vs building mode
  - dangerous mode implications
  - sandbox expectations and recommended configuration
  - crash recovery and stale lock handling

**Completion:** Documentation added to README.md with Orchestrate pane usage guide.

### [x] Task 6.3: Maestro - User Manual Verification 'Sub-Track 03' (Protocol in workflow.md)

**Completion:** User manual verification complete. All core functionality documented and tested.
