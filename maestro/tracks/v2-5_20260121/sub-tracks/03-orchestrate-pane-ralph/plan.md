# Sub-Track 03: Orchestrate Pane (Ralph Port) Integrated into Cockpit - Plan

## Phase 1: Concept Mapping & Data Model

### [ ] Task 1.1: Define Track/Task model in Rust (lossless parsing of tracks + plan.md)
- [ ] Parse `maestro/tracks.md` into:
  - track id
  - description
  - status ([ ]/[~]/[x])
  - link path
- [ ] Parse `maestro/tracks/<track_id>/metadata.json` for type + subtracks
- [ ] Parse `plan.md` into a task tree:
  - hierarchical bullets
  - status markers `[ ]/[~]/[x]`
  - preserve text + ordering
- [ ] Implement a lossless writer:
  - only change status markers / small appended notes
  - never reflow user prose unexpectedly

### [ ] Task 1.2: Map Ralph “tracker” abstraction to Maestro tracks (source-of-truth policy)
- [ ] Decide what the orchestrate engine treats as canonical state:
  - `tracks.md` vs per-track `plan.md` vs `metadata.json`
- [ ] Define dependency semantics:
  - task-level dependencies (if present) vs phase ordering
  - track-level dependencies (subtracks)
- [ ] Define “actionable” vs “blocked” rule set (Ralph parity):
  - actionable if dependencies are satisfied
  - blocked if any dependency incomplete

### [ ] Task 1.3: Define iteration state persistence format (lock + journal + logs)
- [ ] Define on-disk state directory (recommended):
  - `~/.maestro/orchestrate/`
- [ ] Persist:
  - session metadata (selected track, mode, agent tool/model, sandbox)
  - current iteration counter
  - current task id/path
  - last N iteration summaries (for prompt context)
- [ ] Locking model:
  - lock file per project path + track id
  - stale lock detection + safe recovery
- [ ] Journal model:
  - append-only event log (for crash recovery)

## Phase 2: Execution Engine (Rust)

### [ ] Task 2.1: Implement iteration lifecycle (select → prompt → run → detect completion → update)
- [ ] Task selection:
  - choose the highest-priority actionable plan task
  - mark it `[~]` before execution
- [ ] Prompt build:
  - include track spec context + selected task details
  - include LeIndex context bundle (balanced mode)
  - include “recent progress summary” (last N iterations)
- [ ] Agent execution:
  - run in isolated context (new process / new tmux pane / sandbox)
  - capture stdout/stderr continuously
- [ ] Completion detection:
  - require plan update + git commit + backpressure success (preferred)
  - accept `<promise>COMPLETE</promise>` only as an additional signal
- [ ] Finalize iteration:
  - mark task `[x]`, update plan notes, write session state, rotate logs

### [ ] Task 2.2: Implement error strategies (retry/skip/abort) + backoff
- [ ] Implement per-task retry counters
- [ ] Implement “skip task” tracking (avoid infinite loops)
- [ ] Add exponential backoff for repeated failures

### [ ] Task 2.3: Implement structured logging (per-iteration, searchable)
- [ ] Per-iteration log file:
  - timestamped JSONL events (start/stop/output/chosen-task/errors)
- [ ] Session summary file:
  - compact markdown/JSON summary for Cockpit display
- [ ] Optional: persist “codebase patterns” learned (Ralph’s progress.md analog)

### [ ] Task 2.4: Implement interruption handling (Ctrl-C, graceful stop)
- [ ] First Ctrl-C pauses loop and persists state
- [ ] Second Ctrl-C aborts and marks session as interrupted
- [ ] Ensure terminal state + tmux state remains consistent

### [ ] Task 2.5: Implement rate-limit detection + recovery policy (recommended for autonomy)
- [ ] Detect rate-limit patterns per tool:
  - Claude CLI: known messages + HTTP 429
  - others: configurable regex patterns
- [ ] Recovery policy:
  - wait-and-retry for primary agent
  - optional fallback agent/tool selection (if configured)

## Phase 3: Agent Runner Integrations

### [ ] Task 3.1: Define agent runner interface (claude/codex/opencode/amp/shell)
- [ ] Normalize “run one iteration” across tools:
  - stdin prompt mode vs file prompt
  - working directory handling
  - environment variables
  - output format options (plain vs structured)
- [ ] Define runner capabilities matrix:
  - supports non-interactive mode
  - supports auto-approval / dangerous mode
  - supports structured output

### [ ] Task 3.2: Implement runner(s) with consistent output capture and completion detection
- [ ] Implement process runner with:
  - streaming output capture
  - timeouts
  - exit code capture
- [ ] Optional: tmux runner using existing multiplexer (for parity with Cockpit sessions)
- [ ] Implement output parser hooks:
  - `<promise>COMPLETE</promise>` detection
  - subagent trace parsing (if tool supports structured output)

### [ ] Task 3.3: Support headless execution + optional sandbox boundary
- [ ] Headless mode:
  - orchestrate loop runs without Cockpit UI, logs to disk/stdout
- [ ] Sandbox mode (recommended for dangerous auto-approval):
  - Linux: bubblewrap (bwrap) profile
  - macOS: sandbox-exec profile (or documented alternative)
  - Explicit file allowlist (project dir + minimal tool cache)

## Phase 4: Orchestrate UI (Cockpit Tab)

### [ ] Task 4.1: Add Orchestrate tab and routing in Cockpit
- [ ] Add a new tab index for Orchestrate
- [ ] Wire tab navigation + state storage

### [ ] Task 4.2: Implement Ralph-like panels (left tree, right details/output)
- [ ] Left:
  - tracks list with expand/collapse
  - selected track shows nested plan tasks
  - status indicators: active/pending/actionable/blocked/completed
- [ ] Right:
  - selected task details (spec excerpt, acceptance criteria, dependencies)
  - live output viewer with scroll and “follow tail” mode
- [ ] Bottom:
  - iteration stats (duration, commits, tests run)

### [ ] Task 4.3: Implement keybindings + help overlay
- [ ] Navigation:
  - arrows/jk for list movement
  - enter to expand/collapse
- [ ] Control:
  - `s` start
  - `p` pause
  - `r` resume
  - `x` abort
  - `?` help

### [ ] Task 4.4: Implement setup-on-first-run flow within pane
- [ ] Detect missing config / missing tool binaries and provide guided setup
- [ ] Offer to run `maestro-setup` (or embedded setup flow) from within Cockpit
- [ ] Persist chosen defaults (tool/model/sandbox) to config

## Phase 5: LeIndex + Prompt Integration

### [ ] Task 5.1: Define prompt templates (planning/building) for Maestro flavor
- [ ] Planning prompt:
  - forbid implementation/commits
  - require plan regeneration / reprioritization
  - require LeIndex usage for “don’t assume not implemented”
- [ ] Building prompt:
  - exactly one task per iteration
  - require tests/backpressure
  - require plan update + commit
- [ ] Keep templates short and deterministic (Ralph playbook principle)

### [ ] Task 5.2: Integrate LeIndex context bundles (5-phase, targeted extraction)
- [ ] Provide “context budget” policy:
  - phase1/phase2 ultra for orientation
  - balanced for actionable code generation
- [ ] Ensure the engine can request:
  - targeted file context
  - callers/callees
  - cfg/dfg/slice for risk spots

### [ ] Task 5.3: Integrate Maestro commands (`setup`, `newTrack`, `implement`, `orchestrate`) into loop steps
- [ ] When no track exists:
  - orchestrate pane can trigger `maestro:newTrack` flow (or equivalent)
- [ ] When track exists:
  - orchestrate pane can run building loop against it
- [ ] Optional hybrid:
  - orchestrate loop uses `maestro implement` to run in tmux sessions managed by Cockpit

## Phase 6: Documentation + Credits

### [ ] Task 6.1: Add credits in `README.md`
- [ ] Credit `subsy/ralph-tui` (MIT) and link to repo
- [ ] Credit `ghuntley/how-to-ralph-wiggum` and link to repo
- [ ] Add any additional upstream inspirations used

### [ ] Task 6.2: Add user docs: how to use Orchestrate pane + safety notes
- [ ] Document:
  - planning vs building mode
  - dangerous mode implications
  - sandbox expectations and recommended configuration
  - crash recovery and stale lock handling

### [ ] Task 6.3: Maestro - User Manual Verification 'Sub-Track 03' (Protocol in workflow.md)
