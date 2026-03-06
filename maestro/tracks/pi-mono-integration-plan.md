# Plan: Wire pi-mono into Conductor's omp_agent

## Context

The conductor pane currently uses `omp_agent.rs` which spawns TypeScript/Bun subprocess workers for agent execution. The `maestro-pi-mono` crate provides native Rust agent execution with better performance, streaming, cancellation support, and multi-agent orchestration (Scout, Planner, Reviewer, Worker roles). This plan integrates pi-mono as a parallel backend with automatic fallback to OMP.

**Why this change:**
- Native Rust execution eliminates subprocess overhead
- Built-in CancellationToken support for TUI responsiveness
- Rich streaming callbacks for real-time UI updates
- Multi-agent orchestration (chain, parallel execution)
- Usage metrics for cost tracking

## Architecture

```
ConductorPane
  └── AgentExecutor (new)
        ├── PiMonoBackend (native Rust, preferred)
        └── OmpBackend (subprocess, fallback)
```

## Implementation

### Phase 1: Core AgentExecutor Abstraction

**New file:** `crates/cockpit/src/conductor/agent_executor.rs`

Create unified trait-based interface:

```rust
// Core types
pub struct AgentConfig {
    pub model: String,
    pub tools: Vec<String>,
    pub timeout_secs: u64,
    pub agent_role: Option<AgentRole>,
}

pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub usage: Option<UsageMetrics>,
}

pub enum StreamEvent {
    Started, Output(String), Progress{message, percent}, Error(String), Completed{success}
}

// Unified backend trait
#[async_trait]
pub trait AgentBackend: Send + Sync {
    fn is_available(&self) -> bool;
    async fn execute(&self, task: &str, config: &AgentConfig, cancel: Option<CancellationToken>) -> Result<AgentResult>;
    async fn execute_with_streaming(&self, task, config, cancel, callback) -> Result<AgentResult>;
}

// Pi-mono backend implementation
pub struct PiMonoBackend {
    registry: Arc<AgentRegistry>,
    runner: Arc<SubagentRunner>,
    detection: Arc<PiDetection>,
}

// OMP wrapper backend
pub struct OmpBackend { manager: Arc<OmpAgentManager> }

// Combined executor with preference logic
pub struct AgentExecutor {
    pi_mono_backend: Option<Arc<PiMonoBackend>>,
    omp_backend: Option<Arc<OmpBackend>>,
}
```

**Files to modify:**
- `crates/cockpit/Cargo.toml` - Add `maestro-pi-mono = { path = "../pi-mono" }`
- `crates/cockpit/src/conductor/mod.rs` - Add `pub mod agent_executor;`

### Phase 2: ConductorPane Integration

**File:** `crates/cockpit/src/conductor/pane.rs`

Add fields to `ConductorPane`:
```rust
pub agent_executor: Option<AgentExecutor>,
pub pi_mono_config: Option<Arc<PiMonoConfig>>,
pub selected_agent_role: Option<AgentRole>,
pub cancellation_token: Option<Arc<CancellationToken>>,
```

Initialize in `default()`:
```rust
let pi_mono_config = maestro_pi_mono::load_config().ok();
let agent_executor = Some(AgentExecutor::new(pi_mono_config.clone(), omp_manager));
```

### Phase 3: ConductorState Updates

**File:** `crates/cockpit/src/conductor/model.rs`

Add to `ConductorState`:
```rust
pub pi_mono_available: bool,
pub selected_agent_role: Option<String>,
pub active_backend: Option<String>,
```

### Phase 4: UI Keybindings

**File:** `crates/cockpit/src/conductor/keybindings.rs`

Add keybindings:
- `A` - Cycle agent role: Scout → Architect → Critic → Kraken → Scout
- `Ctrl+C` - Cancel active execution via CancellationToken

### Phase 5: Streaming Integration

**File:** `crates/cockpit/src/conductor/polling.rs`

Add `process_stream_event()` handler:
```rust
fn process_stream_event(&mut self, event: StreamEvent) {
    match event {
        StreamEvent::Started => self.add_output("--- Agent started ---".into()),
        StreamEvent::Output(text) => self.add_output(text),
        StreamEvent::Progress{message, percent} => self.add_output(format!("[{}%] {}", percent, message)),
        StreamEvent::Error(err) => self.add_output(format!("ERROR: {}", err)),
        StreamEvent::Completed{success} => self.add_output(format!("--- {} ---", if success {"completed"} else {"failed"})),
    }
}
```

### Phase 6: Header Display

**File:** `crates/cockpit/src/conductor/header.rs`

Display current agent role and active backend in header.

## Critical Files

| File | Purpose |
|------|---------|
| `crates/cockpit/src/conductor/pane.rs` | Integration point - instantiate AgentExecutor |
| `crates/cockpit/src/conductor/omp_agent.rs` | Pattern to follow for agent API |
| `crates/pi-mono/src/execution/runner.rs` | SubagentRunner to wrap |
| `crates/pi-mono/src/agents/mapping.rs` | AgentRole, PiAgentType mapping |
| `crates/pi-mono/src/lib.rs` | Re-exports: load_config, SubagentRunner, AgentRole |

## Key Reusable Components

From pi-mono crate:
- `maestro_pi_mono::load_config()` - Load PiMonoConfig from `~/.config/maestro/pi-mono.toml`
- `maestro_pi_mono::SubagentRunner` - Main execution interface
- `maestro_pi_mono::AgentRole` - Enum: Scout, Architect, Critic, Kraken
- `maestro_pi_mono::PiDetection::detect()` - Find pi executable
- `maestro_pi_mono::AgentRegistry` - Role-to-agent-type mapping

From omp_agent (pattern to follow):
- `OmpAgent::new(track_id, project_path, config)` - Constructor pattern
- `OmpAgentManager` - Multi-agent management pattern
- `execute_task()` - Async execution pattern

## Verification

1. **Build:** `cargo build -p maestro-cockpit`
2. **Unit tests:** `cargo test -p maestro-cockpit agent_executor`
3. **Integration test:** Launch TUI with `maestro tui`, press `A` to cycle roles, verify backend detection
4. **Fallback test:** Run with pi-mono unavailable, verify OMP fallback works
5. **Cancellation test:** Start execution, press `Ctrl+C`, verify clean cancellation

## Scope Notes

**In scope (this PR):**
- AgentExecutor abstraction with PiMonoBackend + OmpBackend
- Single-agent execution via `SubagentRunner::run()`
- Streaming callbacks to polling system
- Cancellation support
- Agent role selection via keybinding

**Out of scope (future PRs):**
- `execute_chain()` for multi-agent pipelines
- `execute_parallel()` for parallel task execution
- Workflow presets (implement, implement-and-review)
