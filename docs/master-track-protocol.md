# Maestro Master Track Protocol v2.5

> Comprehensive protocol for track-based spec-driven development in Maestro.

---

## Table of Contents

1. [Track Structure](#1-track-structure)
2. [Track Lifecycle](#2-track-lifecycle)
3. [Conductor Integration](#3-conductor-integration)
4. [LeIndex Context](#4-leindex-context)
5. [Completion Gates](#5-completion-gates)
6. [Master Track Orchestration](#6-master-track-orchestration)
7. [Error Recovery](#7-error-recovery)
8. [Reference](#8-reference)

---

## 1. Track Structure

Every track follows a standardized directory layout for consistency and tooling compatibility.

### 1.1 Required Files

```
maestro/tracks/<track_id>/
├── spec.md           # Specification document
├── plan.md           # Implementation plan with task breakdown
└── metadata.json     # Track metadata (optional but recommended)
```

#### spec.md - Specification

The specification defines **what** to build:

```markdown
# Track Title - Specification

## Objective
One-paragraph summary of the goal.

## Requirements
### R1: Requirement Name
- Detailed requirement description
- Acceptance criteria as bullet points

### R2: Requirement Name
...

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
```

#### plan.md - Implementation Plan

The plan defines **how** to build it:

```markdown
# Track Title - Plan

## Phase 1: Phase Name

### [ ] Task 1.1: Task Title
- [ ] Subtask description
- [ ] Another subtask

**Dependencies:** None
**Deliverables:** `path/to/file.rs`

### [ ] Task 1.2: Task Title
...

## Phase 2: Phase Name
...
```

**Task Status Markers:**
| Marker | Status | Description |
|--------|--------|-------------|
| `[ ]` | Pending | Not started |
| `[~]` | In Progress | Currently being worked on |
| `[x]` | Completed | Finished and verified |

#### metadata.json - Track Metadata

```json
{
  "id": "track-name_20260121",
  "type": "track",
  "status": "new",
  "created": "2026-01-21T00:00:00Z",
  "updated": "2026-01-21T00:00:00Z",
  "dependencies": [],
  "tags": ["feature", "rust"]
}
```

**Type Values:**
- `"track"` - Standard implementation track
- `"master"` - Orchestration track with sub-tracks

**Status Values:**
- `"new"` - Not yet started
- `"in_progress"` - Active work
- `"completed"` - All tasks done
- `"blocked"` - Waiting on dependencies

### 1.2 Optional: Sub-tracks Directory

For complex tracks that require decomposition:

```
maestro/tracks/<master_track_id>/
├── spec.md
├── plan.md
├── metadata.json
└── sub-tracks/
    ├── 01-first-subtrack/
    │   ├── spec.md
    │   ├── plan.md
    │   └── metadata.json
    ├── 02-second-subtrack/
    │   ├── spec.md
    │   ├── plan.md
    │   └── metadata.json
    └── ...
```

Master track `metadata.json` references subtracks:

```json
{
  "id": "v2-5_20260121",
  "type": "master",
  "status": "new",
  "subtracks": [
    "01-cockpit-tui-reorg",
    "02-leindex-core-rust",
    "03-orchestrate-pane-ralph",
    "04-cli-first-class-integrations"
  ]
}
```

---

## 2. Track Lifecycle

### 2.1 Modes: Planning vs Building

Derived from the Ralph methodology, Maestro supports two distinct modes:

#### Planning Mode

**Purpose:** Generate or update plan artifacts without implementation.

**Constraints:**
- No code changes allowed
- No commits created
- Only `plan.md` and `spec.md` may be modified
- LeIndex used for "don't assume not implemented" verification

**Prompt Directive:**
```
MODE: PLANNING
You are analyzing and planning only.
- Do NOT implement any code
- Do NOT create commits
- Update plan.md with refined task breakdown
- Use LeIndex to verify what already exists
```

#### Building Mode

**Purpose:** Implement one task per iteration with full verification.

**Constraints:**
- Exactly one task per iteration
- Require tests/backpressure validation
- Require plan update + git commit
- Observable side effects required for completion

**Prompt Directive:**
```
MODE: BUILDING
You are implementing exactly ONE task.
- Implement the task described below
- Run tests and verify they pass
- Update plan.md to mark task complete
- Create a git commit with descriptive message
```

### 2.2 Completion Detection

Completion is detected via **observable side effects**, not verbal claims:

1. **Primary Signals (Required):**
   - `plan.md` task status changed: `[ ]` → `[x]`
   - Git commit created with task-related changes

2. **Secondary Signals (Recommended):**
   - Tests pass (backpressure validation)
   - Linter/typecheck pass

3. **Tertiary Signals (Optional):**
   - `<promise>COMPLETE</promise>` marker in output
   - Exit code 0

**Detection Logic:**
```rust
fn is_iteration_complete(iteration: &Iteration) -> bool {
    let plan_updated = iteration.plan_status_changed_to_complete;
    let commit_created = iteration.git_commit_sha.is_some();
    let tests_passed = iteration.backpressure_result == Some(true);
    
    // Required: plan update AND commit
    // Recommended: tests passed
    plan_updated && commit_created && tests_passed.unwrap_or(true)
}
```

### 2.3 Lock File Management

For crash safety and concurrent execution prevention:

#### Lock File Location

```
~/.maestro/orchestrate/locks/<project_hash>_<track_id>.lock
```

#### Lock File Format

```json
{
  "pid": 12345,
  "hostname": "workstation",
  "started_at": "2026-01-25T10:30:00Z",
  "track_id": "my-track_20260121",
  "mode": "building"
}
```

#### Stale Lock Detection

```rust
fn is_lock_stale(lock: &LockFile) -> bool {
    // Check if PID is still running
    if !process_exists(lock.pid) {
        return true;
    }
    
    // Check if lock is older than timeout (default: 4 hours)
    let age = Utc::now() - lock.started_at;
    if age > Duration::hours(4) {
        return true;
    }
    
    false
}
```

#### Recovery Protocol

```
1. Detect stale lock on startup
2. If stale:
   a. Log warning with lock details
   b. Clear lock file
   c. Set session status to "Interrupted"
   d. Clear active_task_ids
   e. Resume from last persisted state
3. If not stale:
   a. Report "Session already running"
   b. Offer attach/force options
```

---

## 3. Conductor Integration

The Conductor is the TUI pane for orchestrating tracks.

### 3.1 Track Tree Visualization

```
┌─ Tracks ───────────────────────────┐
│ ▼ [~] v2-5_20260121 (Master)       │
│   ├─ [x] 01-cockpit-tui-reorg      │
│   ├─ [~] 02-leindex-core-rust      │
│   │   ├─ [x] Task 1.1: Eliminate...│
│   │   ├─ [~] Task 1.2: LeIndex...  │
│   │   └─ [ ] Task 1.3: Hook...     │
│   ├─ [ ] 03-orchestrate-pane-ralph │
│   └─ [ ] 04-cli-integrations       │
│ ▶ [ ] feature-xyz_20260120         │
└────────────────────────────────────┘
```

**Status Indicators:**
| Symbol | Meaning |
|--------|---------|
| `[ ]` | Pending |
| `[~]` | In Progress |
| `[x]` | Completed |
| `[!]` | Error/Failed |
| `[B]` | Blocked (dependencies) |

**Tree Operations:**
- `↑/↓` or `j/k`: Navigate
- `Enter` or `Space`: Expand/collapse
- `o/O`: Select previous/next track
- `g/G`: Go to top/bottom

### 3.2 Iteration Loop

The core orchestration cycle:

```
┌─────────────────────────────────────────────────────────────────┐
│                     ITERATION LOOP                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  SELECT  │───▶│  PROMPT  │───▶│   RUN    │───▶│  DETECT  │  │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘  │
│       │                                               │         │
│       │                                               ▼         │
│       │                                         ┌──────────┐   │
│       │◀────────────────────────────────────────│  UPDATE  │   │
│       │                                         └──────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### Phase Details

1. **SELECT**: Choose highest-priority actionable task
   - Parse `plan.md` for pending tasks
   - Filter by dependencies (skip blocked)
   - Apply priority rules (phase order, explicit priority)
   - Mark selected task `[~]`

2. **PROMPT**: Build context-rich prompt
   - Include track spec context
   - Include selected task details
   - Inject LeIndex context bundle
   - Include recent progress summary (last N iterations)

3. **RUN**: Execute agent with prompt
   - Spawn agent process (claude/gemini/qwen/opencode)
   - Stream stdout/stderr
   - Apply timeout (default: 300s)
   - Capture exit code

4. **DETECT**: Determine completion
   - Check for plan.md status change
   - Check for git commit
   - Validate tests passed (if configured)
   - Detect `<promise>COMPLETE</promise>` marker

5. **UPDATE**: Persist state and continue
   - Update plan.md with completion/notes
   - Write iteration log
   - Update session state
   - Continue to next task or pause

### 3.3 Session Persistence

Session state stored at:
```
~/.maestro/orchestrate/sessions/<project_hash>/<track_id>/
├── session.json      # Current session state
├── iterations/       # Per-iteration logs
│   ├── 001.jsonl
│   ├── 002.jsonl
│   └── ...
└── lock.json         # Active lock file
```

**session.json Format:**
```json
{
  "track_id": "my-track_20260121",
  "status": "running",
  "mode": "building",
  "current_iteration": 5,
  "current_task_id": "1.3",
  "tasks_completed": 4,
  "total_tasks": 12,
  "started_at": "2026-01-25T10:30:00Z",
  "updated_at": "2026-01-25T11:45:00Z",
  "agent": {
    "tool": "claude",
    "model": "claude-sonnet-4-20250514",
    "dangerous_mode": true
  },
  "active_task_ids": ["1.3"],
  "skipped_task_ids": []
}
```

---

## 4. LeIndex Context

LeIndex provides token-efficient code analysis for agent prompts.

### 4.1 5-Phase Analysis

| Phase | Name | Output | Use Case |
|-------|------|--------|----------|
| 1 | Structure | File tree + summaries | Orientation |
| 2 | Dependencies | Import/call graph | Navigation |
| 3 | Control Flow | CFG + complexity | Risk assessment |
| 4 | Data Flow | DFG + taint tracking | Security review |
| 5 | Slicing | Impact analysis | Change planning |

### 4.2 Context Modes

#### Ultra Mode (< 50K tokens)

For quick iterations and simple tasks:

```
Phase 1: Structural summary only (no file contents)
Phase 2: Direct dependencies only (1-hop)
Token budget: 10K-30K
```

**When to use:**
- Simple bug fixes
- Documentation updates
- Single-file changes

#### Balanced Mode (> 50K tokens)

For complex implementations:

```
Phase 1: Structure + key file excerpts
Phase 2: Full dependency graph (2-hop)
Phase 3: CFG for modified functions
Token budget: 50K-100K
```

**When to use:**
- Feature implementation
- Refactoring
- Multi-file changes

### 4.3 Context Bundle Generation

```rust
async fn build_iteration_context(
    track_id: &str,
    task: &Task,
    budget_tokens: usize,
    mode: FormatMode,
) -> Result<ContextBundle> {
    // 1. Phase 1: Structural scan
    let structure = leindex_phase1(&task.related_files)?;
    
    // 2. Phase 2: Dependency map
    let dependencies = leindex_phase2(&task.related_files)?;
    
    // 3. Task context
    let task_context = format!(
        "## Current Task\n\n**ID**: {}\n**Title**: {}\n**Description**: {}\n",
        task.id, task.title, task.description
    );
    
    // 4. Recent iteration summaries
    let recent = get_recent_iterations(track_id, 5)?;
    let history = format_iteration_summary(&recent);
    
    // 5. Assemble within budget
    ContextBundle::new()
        .add_section("task", task_context)
        .add_section("structure", structure)
        .add_section("dependencies", dependencies)
        .add_section("recent_progress", history)
        .fit_to_budget(budget_tokens, mode)
}
```

---

## 5. Completion Gates

Three gates must pass for iteration completion:

### 5.1 Gate 1: Plan Status Update

**Requirement:** Task marker in `plan.md` changes from `[ ]` to `[x]`

**Detection:**
```rust
fn detect_plan_update(before: &str, after: &str, task_id: &str) -> bool {
    let before_status = extract_task_status(before, task_id);
    let after_status = extract_task_status(after, task_id);
    
    before_status == TaskStatus::Pending && after_status == TaskStatus::Completed
}
```

**Lossless Update Policy:**
- Only modify status markers
- Preserve all user prose
- Append completion notes (optional)
- Never reflow or reformat

### 5.2 Gate 2: Git Commit Creation

**Requirement:** New commit created with task-related changes

**Detection:**
```rust
fn detect_git_commit(before_sha: &str, after_sha: &str) -> Option<CommitInfo> {
    if before_sha == after_sha {
        return None;
    }
    
    // Get commit info
    let commit = git_show(after_sha)?;
    
    Some(CommitInfo {
        sha: after_sha.to_string(),
        message: commit.message,
        files_changed: commit.files,
        additions: commit.additions,
        deletions: commit.deletions,
    })
}
```

**Commit Message Convention:**
```
<type>(<scope>): <description>

Types: feat, fix, refactor, docs, test, chore
Scope: component or track name
```

### 5.3 Gate 3: Backpressure Validation

**Requirement:** Tests pass (when configured)

**Detection:**
```rust
async fn run_backpressure(config: &BackpressureConfig) -> BackpressureResult {
    let mut results = vec![];
    
    // Run configured checks
    if config.run_tests {
        results.push(("tests", run_tests().await));
    }
    if config.run_typecheck {
        results.push(("typecheck", run_typecheck().await));
    }
    if config.run_lint {
        results.push(("lint", run_lint().await));
    }
    
    BackpressureResult {
        passed: results.iter().all(|(_, r)| r.is_ok()),
        checks: results,
    }
}
```

**Configuration:**
```toml
# maestro.toml
[backpressure]
run_tests = true
run_typecheck = true
run_lint = false
timeout_secs = 120
```

---

## 6. Master Track Orchestration

For master tracks with sub-tracks, orchestration follows a hierarchical pattern.

### 6.1 Orchestration Protocol

```
1. SETUP CHECK
   - Verify maestro environment (tech-stack.md, workflow.md, product.md)
   - Load master track metadata
   - Validate all subtracks exist

2. SUBTRACK SELECTION
   - Identify next actionable subtrack
   - Check dependencies (cross-subtrack)
   - Check parallel eligibility

3. SUBTRACK EXECUTION
   - Launch agent with `/maestro:implement <subtrack_id>`
   - Monitor progress
   - Handle errors

4. CHECKPOINT
   - Record completion in master plan
   - Update subtrack metadata
   - Commit progress

5. CONTINUE OR COMPLETE
   - If subtracks remain: goto step 2
   - If all complete: run final verification
```

### 6.2 Parallel Execution

When subtracks have no dependencies, run in parallel:

```markdown
# Master Plan Example

## Phase 2: Core Features (Parallel Execution)

### [ ] Execute subtrack 'ui-pages'
  **Parallel-With:** provider-integrations

### [ ] Execute subtrack 'provider-integrations'
  **Parallel-With:** ui-pages
```

**Parallel Launch:**
```rust
async fn launch_parallel(subtracks: &[&str]) -> Vec<TaskHandle> {
    let mut handles = vec![];
    
    for subtrack in subtracks {
        let handle = spawn_agent(subtrack, true /* background */);
        handles.push(handle);
    }
    
    handles
}

async fn monitor_parallel(handles: Vec<TaskHandle>) -> Vec<SubtrackResult> {
    join_all(handles.iter().map(|h| h.wait())).await
}
```

### 6.3 Error Handling in Orchestration

**On Subtrack Failure:**
1. Halt immediately
2. Kill parallel subtracks (if any)
3. Display error context
4. Offer recovery options:
   - Retry failed subtrack
   - Resume after manual fix
   - Abort orchestration

---

## 7. Error Recovery

### 7.1 Error Strategies

| Strategy | Behavior |
|----------|----------|
| **Retry** | Re-attempt task with exponential backoff |
| **Skip** | Mark task skipped, continue to next |
| **Abort** | Stop orchestration, preserve state |

**Configuration:**
```rust
pub enum ErrorStrategy {
    Retry { max_attempts: u32, backoff_base_secs: u64 },
    Skip { max_skips: u32 },
    Abort,
}
```

### 7.2 Retry Logic

```rust
async fn handle_task_failure(
    task: &Task,
    error: &TaskError,
    strategy: &ErrorStrategy,
    state: &mut SessionState,
) -> ControlFlow {
    match strategy {
        ErrorStrategy::Retry { max_attempts, backoff_base_secs } => {
            let attempts = state.retry_count(task.id);
            
            if attempts >= *max_attempts {
                return ControlFlow::Abort(format!(
                    "Task {} failed after {} attempts",
                    task.id, attempts
                ));
            }
            
            let backoff = Duration::from_secs(
                backoff_base_secs * 2u64.pow(attempts as u32)
            );
            
            sleep(backoff).await;
            state.increment_retry(task.id);
            ControlFlow::Retry
        }
        
        ErrorStrategy::Skip { max_skips } => {
            if state.skipped_count() >= *max_skips {
                return ControlFlow::Abort("Too many skipped tasks".into());
            }
            
            state.mark_skipped(task.id);
            ControlFlow::Continue
        }
        
        ErrorStrategy::Abort => {
            ControlFlow::Abort(error.message.clone())
        }
    }
}
```

### 7.3 Resume Capability

**On Session Resume:**
```rust
async fn resume_session(track_id: &str) -> Result<()> {
    // 1. Load session state
    let session = load_session(track_id)?;
    
    // 2. Detect and recover stale locks
    let recovery = detect_and_recover_stale(track_id)?;
    if recovery.was_stale {
        log::warn!("Recovered from stale session");
    }
    
    // 3. Skip completed tasks
    let tasks = parse_plan(track_id)?;
    let pending = tasks.iter()
        .filter(|t| t.status == TaskStatus::Pending)
        .filter(|t| !session.skipped_task_ids.contains(&t.id))
        .collect::<Vec<_>>();
    
    // 4. Resume from last task
    if let Some(last_task) = session.active_task_ids.first() {
        log::info!("Resuming from task: {}", last_task);
    }
    
    // 5. Continue iteration loop
    run_iteration_loop(pending, session).await
}
```

---

## 8. Reference

### 8.1 Key Paths

| Path | Purpose |
|------|---------|
| `maestro/tracks.md` | Track registry |
| `maestro/tracks/<id>/` | Track directory |
| `maestro/tracks/<id>/spec.md` | Specification |
| `maestro/tracks/<id>/plan.md` | Implementation plan |
| `maestro/tracks/<id>/metadata.json` | Track metadata |
| `~/.maestro/orchestrate/` | Session state |

### 8.2 CLI Commands

```bash
# Start orchestration
maestro conductor start <track_id> --mode building

# Pause running session
maestro conductor pause

# Resume paused session
maestro conductor resume

# Abort session
maestro conductor abort

# Check status
maestro conductor status

# List all tracks
maestro conductor list
```

### 8.3 Keybindings (Conductor Pane)

| Key | Action |
|-----|--------|
| `s` | Start orchestration |
| `p` | Pause |
| `r` | Resume |
| `x` | Abort |
| `c` | Commit current task |
| `?` | Help overlay |
| `o/O` | Previous/next track |
| `Space` | Expand/collapse |
| `j/k` | Navigate up/down |

### 8.4 Credits

This protocol is inspired by:
- [subsy/ralph-tui](https://github.com/subsy/ralph-tui) (MIT) - Ralph TUI loop mechanics
- [ghuntley/how-to-ralph-wiggum](https://github.com/ghuntley/how-to-ralph-wiggum) - Ralph playbook methodology

---

*Last updated: 2026-01-25 | Maestro v2.5*
