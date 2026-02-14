# Task Dependency Determination Logic - Analysis Document

## Phase 1 Task 1.1: Analysis

### 1. Data Flow Overview

```
plan.md (markdown)
    │
    ▼
parse_plan_md() ──► TrackPlan { tasks: Vec<Task> }
    │                         │
    │                         ▼
    │                   Task {
    │                     id: String,
    │                     dependencies: Vec<TaskDependency>,
    │                     status: TrackStatus,
    │                     ...
    │                   }
    │                         │
    │                         ▼
    │                   completed_tasks_map() ──► HashMap<String, bool>
    │                         │                        │
    │                         ▼                        ▼
    │                   is_blocked() ◄────────── completion status
    │                   is_actionable() ◄───────── completion status
    │                         │
    ▼                         ▼
get_selectable_items() ◄────── results
    │
    ▼
render_track_tree() ──► UI indicators
```

### 2. Dependency Parsing

**Location**: `maestro/leindex/rust/src/orchestrate/parser.rs`

**Function**: `parse_task_line()` (lines 251-369)

**How it works**:
1. Parses `**Dependencies**:` section from plan.md task entries
2. Normalizes dependency task IDs to match task ID format:
   - Lowercase conversion
   - Replaces spaces, dots, colons, underscores with hyphens
   - Filters to alphanumeric + hyphen only
   - Adds `task-` prefix if not present
3. Creates `TaskDependency` with `dependency_type: TaskDependencyType::Hard`

**Key Code** (lines 329-361):
```rust
if next_trimmed.starts_with("**Dependencies**:") {
    // Parse dependencies from following lines
    // ...
    let normalized_id = if dep_id.starts_with("task-") {
        dep_id
    } else {
        format!("task-{}", dep_id)
    };
    dependencies.push(TaskDependency {
        task_id: normalized_id,
        dependency_type: TaskDependencyType::Hard,
    });
}
```

### 3. Dependency Data Structure

**Location**: `maestro/leindex/rust/src/orchestrate/model.rs`

```rust
// Line 42-53
pub struct TaskDependency {
    pub task_id: String,
    pub dependency_type: TaskDependencyType,
}

pub enum TaskDependencyType {
    Hard, // Must complete before this task can start
    Soft, // Should complete, but not blocking
}
```

### 4. Blocked/Actionable Status Determination

**Location**: `maestro/leindex/rust/src/orchestrate/model.rs`

**Task::is_blocked()** (lines 82-87):
```rust
pub fn is_blocked(&self, completed_tasks: &HashMap<String, bool>) -> bool {
    self.dependencies
        .iter()
        .filter(|d| d.dependency_type == TaskDependencyType::Hard)
        .any(|d| !completed_tasks.get(&d.task_id).copied().unwrap_or(false))
}
```
- Returns `true` if ANY hard dependency is NOT completed
- Returns `false` if no hard dependencies exist or all are completed

**Task::is_actionable()** (lines 70-79):
```rust
pub fn is_actionable(&self, completed_tasks: &HashMap<String, bool>) -> bool {
    if !self.status.is_actionable() {
        return false;
    }
    self.dependencies
        .iter()
        .filter(|d| d.dependency_type == TaskDependencyType::Hard)
        .all(|d| completed_tasks.get(&d.task_id).copied().unwrap_or(false))
}
```
- Returns `false` if status is NOT Pending or InProgress
- Returns `true` only if ALL hard dependencies ARE completed

### 5. Usage in UI

**Location**: `crates/cockpit/src/conductor/pane.rs`

`get_selectable_items()` (lines 651-672):
```rust
fn add_tasks_to_selectable_items(&self, task: &Task, ...) {
    let is_blocked = task.is_blocked(completed_tasks);
    let is_actionable = task.is_actionable(completed_tasks);
    // ... creates SelectableItem::Task with is_blocked, is_actionable
}
```

**Location**: `crates/cockpit/src/conductor/track_tree.rs`

Rendering logic (lines 156-168):
```rust
match status {
    TrackStatus::Pending => {
        if *is_blocked {
            (STATUS_BLOCKED, conductor_theme.task_blocked)     // ⊘ red
        } else if *is_actionable {
            (STATUS_PENDING, conductor_theme.task_actionable)  // ○ green
        } else {
            (STATUS_PENDING, conductor_theme.task_pending)    // ○ gray
        }
    }
    TrackStatus::InProgress => (STATUS_ACTIVE, conductor_theme.task_active),
    TrackStatus::Completed => (STATUS_DONE, conductor_theme.task_done),
}
```

### 6. Constants

**Location**: `crates/cockpit/src/conductor/theme.rs`

```rust
pub const STATUS_ACTIVE: &str = "▶";
pub const STATUS_ACTIONABLE: &str = "○";
pub const STATUS_PENDING: &str = "○";
pub const STATUS_BLOCKED: &str = "⊘";
```

**Colors** (line 63-65):
```rust
task_actionable: Color::Rgb(158, 206, 106),  // Green (same as task_active!)
task_pending: Color::Rgb(86, 95, 137),        // Gray
task_blocked: Color::Rgb(247, 118, 142),     // Red
```

### 7. Current Issues / Observations

1. **Same color for actionable and active**: Both `task_actionable` and `task_active` use the same RGB value (158, 206, 106 - green), making them visually indistinguishable.

2. **No soft dependency support**: While `TaskDependencyType::Soft` exists, the parser always creates `Hard` dependencies. Soft dependencies would allow "should complete but not blocking" semantics.

3. **ID normalization edge cases**: The ID normalization in parser uses character filtering that could potentially cause collisions for similar task names.

### 8. Files Analyzed

- `maestro/leindex/rust/src/orchestrate/model.rs` - Task struct, is_blocked, is_actionable methods
- `maestro/leindex/rust/src/orchestrate/parser.rs` - Dependency parsing from plan.md
- `crates/cockpit/src/conductor/pane.rs` - get_selectable_items integration
- `crates/cockpit/src/conductor/track_tree.rs` - UI rendering
- `crates/cockpit/src/conductor/theme.rs` - Status constants and colors
- `crates/cockpit/src/conductor/model.rs` - SelectableItem definition
