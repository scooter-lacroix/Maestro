# Maestro Cockpit Comprehensive Remediation Plan

**Date:** 2026-02-25  
**Scope:** MaesterClaw Tab, Compilation Errors, Conductor Tab, Settings, Memory Tab  
**Status:** PLANNING  
**Estimated Complexity:** Very High (35+ files, 5 major subsystems)

---

## Table of Contents

1. [MaesterClaw Tab - Keyboard Shortcuts](#1-maesterclaw-tab---keyboard-shortcuts)
2. [MaesterClaw Tab - Setup/Walkthrough Wizard](#2-maesterclaw-tab---setupwalkthrough-wizard)
3. [MaesterClaw Tab - Agent Integration](#3-maesterclaw-tab---agent-integration)
4. [MaesterClaw Tab - Configurable Parameters](#4-maesterclaw-tab---configurable-parameters-in-settings)
5. [Compilation Errors](#5-compilation-errors)
6. [Tzar Review Remediation](#6-tzar-review-remediation)
7. [Conductor Tab Functionality](#7-conductor-tab-functionality)
8. [Settings Save Button](#8-settings-save-button)
9. [Memory Tab - Banking Integration](#9-memory-tab---banking-integration)
10. [Memory Tab - Tree Dependency Graph](#10-memory-tab---tree-dependency-graph)
11. [Memory Tab - Interval-Based Saving](#11-memory-tab---interval-based-saving)

---

## 1. MaesterClaw Tab - Keyboard Shortcuts

### Current State
The MaesterClaw tab (index 1, `tabs::MAESTERCLAW`) currently only handles **Up/Down navigation** to cycle between three sections (`CronJobs`, `McpServers`, `Sandbox`). The UI renders keyboard hints (`[N] New Job`, `[E] Edit`, `[D] Delete`, `[T] Toggle`, `[R] Run Now`, `[A] Add Server`, `[C] Connect`, `[X] Disconnect`, `[P] Change Policy`, `[W] Enable WASM`, `[D] Enable Docker`) in `capabilities.rs` but **none of these shortcuts have actual handlers** in the key dispatch logic of `app.rs`.

### Root Cause Analysis
- **`app.rs` lines 2975–2988 / 3119–3132 / 4501–4514**: MaesterClaw only handles `Up`/`Down` for section cycling. No `Enter`, `N`, `E`, `D`, `T`, `R`, `A`, `C`, `X`, `P`, `W` handlers exist when `tab_index == MAESTERCLAW`.
- There is no `handle_maesterclaw_key()` function analogous to `handle_key_event()` in `conductor/keybindings.rs`.
- The capabilities tab renders shortcut hints as decoration only.

### Files to Modify
| File | Purpose |
|------|---------|
| `crates/cockpit/src/tabs/capabilities.rs` | Add item selection state, action handling |
| `crates/cockpit/src/app.rs` | Add key dispatch for MaesterClaw-specific keys |
| `crates/cockpit/src/state/types.rs` | Add `MaesterClawFocus` enum, list states |

### Implementation Plan

#### Phase 1.1: Add MaesterClaw State Types
**File:** `crates/cockpit/src/state/types.rs`
- Add `MaesterClawFocus` enum: `CronJobs`, `McpServers`, `Sandbox`
- Add `MaesterClawAction` enum: `NewJob`, `EditJob`, `DeleteJob`, `ToggleJob`, `RunJob`, `AddServer`, `ConnectServer`, `DisconnectServer`, `RefreshTools`, `ChangePolicy`, `EnableWasm`, `EnableDocker`

#### Phase 1.2: Add Selection State to App
**File:** `crates/cockpit/src/app.rs`
- Add `cron_job_selected: usize` (already exists as `cron_job_state`)
- Add `mcp_server_selected: usize` for MCP server list
- Add `maesterclaw_focus: MaesterClawFocus` field

#### Phase 1.3: Create MaesterClaw Keybinding Module
**File:** `crates/cockpit/src/maesterclaw/keybindings.rs` (NEW)
- Create `handle_maesterclaw_key(app: &mut App, key: KeyEvent) -> bool` function
- Implement all 12 keyboard shortcuts:
  - **CronJobs section**: `N` (new job modal), `E` (edit selected), `D` (delete selected), `T` (toggle enabled), `R` (run now via `CronManager`)
  - **McpServers section**: `A` (add server modal), `C` (connect selected), `X` (disconnect selected), `R` (refresh tools)
  - **Sandbox section**: `P` (cycle autonomy level via `SandboxManager`), `W` (toggle WASM runtime), `D` (toggle Docker runtime)
  - **Cross-section**: `Enter` (select/expand item), `j`/`k` (item navigation within section)

#### Phase 1.4: Wire Keybindings into App Event Loop
**File:** `crates/cockpit/src/app.rs`
- In the `else` (non-input-mode) key dispatch block, add a MaesterClaw catch-all similar to Conductor's at line 3267:
  ```rust
  _ if app.tab_index == tabs::MAESTERCLAW => {
      if handle_maesterclaw_key(&mut app, key) {
          continue;
      }
  }
  ```
- Ensure this is placed AFTER the global Up/Down handlers but BEFORE the generic key handlers

#### Phase 1.5: Add Modals for CronJob/MCP Actions
**File:** `crates/cockpit/src/maesterclaw/modals.rs` (NEW)
- `NewCronJobModal`: Fields for name, schedule (cron/interval/at), type (shell/agent), command
- `NewMcpServerModal`: Fields for name, transport (stdio/sse), command/URL
- `PolicyModal`: Dropdown for autonomy level selection
- Wire modals into `InputMode` enum in `state/types.rs`

#### Phase 1.6: Implement CronJob CRUD via CronManager
**Files:** `crates/core/src/` (existing `CronManager` API)
- `N`: `app.cron_manager.create_job(...)` → toast success
- `E`: Open edit modal pre-filled with selected job
- `D`: Confirmation prompt → `app.cron_manager.delete_job(id)`
- `T`: `app.cron_manager.toggle_job(id)` → flip enabled flag
- `R`: `app.cron_manager.run_job(id)` → execute immediately

#### Phase 1.7: Implement MCP Server Actions
**Files:** `crates/cockpit/src/app.rs`, `maesterclaw/mod.rs`
- `A`: Modal for adding server → `app.mcp_manager.register_server(...)`
- `C`: `app.mcp_manager.connect(name).await` (needs async handling)
- `X`: `app.mcp_manager.disconnect(name)`
- `R`: `app.mcp_manager.refresh_tools(name)` → re-enumerate tools

#### Phase 1.8: Implement Sandbox Policy Actions
**File:** `crates/cockpit/src/app.rs`
- `P`: Cycle through `HumanApproval` → `Supervised` → `Autonomous`
- `W`: Toggle WASM in `sandbox_manager.enable_runtime("wasm")`
- `D`: Toggle Docker in `sandbox_manager.enable_runtime("docker")`

---

## 2. MaesterClaw Tab - Setup/Walkthrough Wizard

### Current State
No setup wizard exists. Users land on the MaesterClaw tab with empty sections and no guidance on how to configure agents, MCP servers, or security policies.

### Files to Create/Modify
| File | Purpose |
|------|---------|
| `crates/cockpit/src/maesterclaw/wizard.rs` (NEW) | Wizard state machine and rendering |
| `crates/cockpit/src/tabs/capabilities.rs` | Detect first-run and show wizard |
| `crates/cockpit/src/state/types.rs` | Add `WizardStep` enum |
| `crates/cockpit/src/app.rs` | Wire wizard key events |

### Implementation Plan

#### Phase 2.1: Define Wizard State Machine
**File:** `crates/cockpit/src/maesterclaw/wizard.rs`
```rust
pub enum WizardStep {
    Welcome,           // Introduction page
    DetectAgents,      // Auto-detect installed CLIs
    ConfigureAgents,   // Select default agent, configure API keys
    SetupMcpServers,   // Configure MCP server connections
    SecurityPolicy,    // Set autonomy level
    CronSetup,         // Optional: set up first cron job
    Complete,          // Summary and finish
}

pub struct SetupWizard {
    pub step: WizardStep,
    pub detected_agents: Vec<DetectedAgent>,
    pub selected_agents: Vec<String>,
    pub api_keys: HashMap<String, String>,
    pub is_active: bool,
    pub scroll: u16,
}
```

#### Phase 2.2: Agent Detection
- Call `which` for: `iflow`, `claude`, `codex`, `gemini`, `opencode`, `aider`, `amp`
- Check for API keys in env: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GOOGLE_API_KEY`, `OPENROUTER_API_KEY`
- Display detection results with checkmarks/crosses
- Store results in `detected_agents` for UI display

#### Phase 2.3: Wizard Rendering
- Each step renders as a full-screen overlay within the MaesterClaw tab
- Navigation: `Enter` (next step), `Esc` (back/cancel), `Tab` (switch fields)
- Auto-detect on step entry, allow manual override
- Progress indicator at top showing current step

#### Phase 2.4: First-Run Detection
**File:** `crates/cockpit/src/tabs/capabilities.rs`
- Check `~/.config/maestro/wizard_complete` flag file
- If absent and no cron jobs / no MCP servers: auto-trigger wizard
- Add "Run Setup Wizard" option accessible via `Shift+W` key

#### Phase 2.5: Persist Wizard Results
- Save configured agents to `config.toml` (extend `Config` struct)
- Save MCP server configs to the memory service DB
- Create cron jobs if any were configured
- Write `~/.config/maestro/wizard_complete` marker

---

## 3. MaesterClaw Tab - Agent Integration

### Current State
- **iFlow**: Only "somewhat integrated" — the capabilities tab shows an iFlow hint at the bottom (`iflow -p "<prompt>"`), but there's no agent management UI.
- **Codex, Claude Code, Gemini, OpenCode**: Completely absent from the MaesterClaw tab. Session creation (`app.rs:1516`) uses `new_session_tool` which supports arbitrary tool names, but MaesterClaw has no agent-specific panels, status indicators, or launch mechanisms.
- **Pi-Mono**: Agent mapping exists in `crates/pi-mono/src/agents/mapping.rs` but is only used by the Conductor's `AgentExecutor`, not by MaesterClaw.

### Files to Create/Modify
| File | Purpose |
|------|---------|
| `crates/cockpit/src/maesterclaw/agents.rs` (NEW) | Agent panel: detection, status, launch |
| `crates/cockpit/src/tabs/capabilities.rs` | Add 4th section "Agents" |
| `crates/cockpit/src/state/types.rs` | Add `AgentInfo` struct |
| `crates/cockpit/src/app.rs` | Wire agent actions |

### Implementation Plan

#### Phase 3.1: Add Agents Section to CapabilitiesSection
**File:** `crates/cockpit/src/tabs/capabilities.rs`
- Add `CapabilitiesSection::Agents` variant
- Change layout to 4 section tabs: Agents | CronJobs | McpServers | Sandbox
- Agents section should be the **default** (first) section since it's the primary function

#### Phase 3.2: Define AgentInfo Types
**File:** `crates/cockpit/src/state/types.rs`
```rust
pub struct AgentInfo {
    pub name: String,           // "claude", "codex", "gemini", "opencode", "iflow"
    pub display_name: String,   // "Claude Code", "Codex CLI", etc.
    pub binary_path: Option<String>,  // Detected path
    pub is_installed: bool,
    pub is_configured: bool,    // Has API key
    pub api_key_env: String,    // "ANTHROPIC_API_KEY"
    pub version: Option<String>,
    pub active_sessions: usize, // How many sessions are using this agent
    pub last_used: Option<String>,
}
```

#### Phase 3.3: Implement Agent Detection & Status Panel
**File:** `crates/cockpit/src/maesterclaw/agents.rs`
- `detect_agents() -> Vec<AgentInfo>`: Run `which <agent>` for each known CLI
- `render_agents_section(frame, app, area)`:
  - Show table: Agent | Status | Version | Sessions | Last Used
  - Installed = green, Not Installed = red, Configured = blue
  - Help bar: `[L] Launch Session  [C] Configure  [I] Install Guide  [R] Refresh`

#### Phase 3.4: Wire Agent Actions
- `L` (Launch): Open session creation wizard pre-filled with selected agent
- `C` (Configure): Open API key configuration modal
- `I` (Install): Show installation instructions for the selected agent
- `R` (Refresh): Re-detect all agents

#### Phase 3.5: Per-Agent Configuration Modals
- Claude: Anthropic API key, model selection (claude-4, sonnet, haiku)
- Codex: OpenAI API key, model selection
- Gemini: Google API key, model selection
- OpenCode: Config path
- iFlow: iFlow config path, non-interactive mode settings

---

## 4. MaesterClaw Tab - Configurable Parameters in Settings

### Current State
The Settings tab (`tabs/settings.rs`) only has 5 options: Editor, Theme, Transparent, InstallPath, Save. There are NO MaesterClaw-specific configuration options (agent defaults, security policy, MCP server auto-start, cron job defaults, etc.).

### Files to Modify
| File | Purpose |
|------|---------|
| `src/leindex/src/config.rs` | Extend Config struct with MaesterClaw fields |
| `crates/cockpit/src/tabs/settings.rs` | Add new settings sections |
| `crates/cockpit/src/state/types.rs` | Extend SettingsOption enum |
| `crates/cockpit/src/app.rs` | Wire new settings options |

### Implementation Plan

#### Phase 4.1: Extend Config Struct
**File:** `src/leindex/src/config.rs`
```rust
pub struct Config {
    // Existing
    pub editor: String,
    pub install_path: String,
    pub theme: String,
    pub selected_tools: Vec<String>,
    pub transparent: bool,
    // NEW: MaesterClaw settings
    pub default_agent: String,          // "claude", "codex", etc.
    pub autonomy_level: String,         // "human_approval", "supervised", "autonomous"
    pub mcp_auto_start: bool,           // Auto-start MCP servers on launch
    pub memory_auto_bank: bool,         // Auto-bank memories during execution
    pub memory_bank_interval_secs: u64, // Memory banking interval
    pub max_agent_iterations: u64,      // Default max iterations for agent loops
    pub agent_timeout_secs: u64,        // Agent execution timeout
    pub sandbox_runtime: String,        // "native", "wasm", "docker"
    pub enable_telemetry: bool,         // Conductor telemetry
    pub conductor_poll_interval_ms: u64,// Conductor polling interval
    pub api_keys: HashMap<String, String>, // Agent API keys (encrypted?)
}
```

#### Phase 4.2: Add Settings Sections
**File:** `crates/cockpit/src/tabs/settings.rs`
- Restructure into tabbed sections: General | Agents | Security | Memory | Conductor
- Add `SettingsOption` variants for each new field
- Add dropdown selectors for enum-like fields (agent, autonomy, runtime)

#### Phase 4.3: Wire Settings Navigation
**File:** `crates/cockpit/src/app.rs`
- Extend `SettingsOption` cycle in Up/Down handlers
- Add Enter handlers for new options (dropdowns, toggles, text inputs)
- All changes auto-persist via `config.save()`

---

## 5. Compilation Errors

### Current State
**The workspace currently compiles successfully.** `cargo check --workspace` completes with only warnings (16 dead-code warnings in `maestro-claw`, 2 unused-variable warnings in `gateway`). No compilation errors exist.

The user pasted 184 lines of compilation errors that likely represent a **prior state** that has since been resolved, or errors from a different branch/checkout.

### Verification
```bash
$ cargo check --workspace 2>&1 | grep "^error"
# (no output — zero errors)
```

### Plan
#### Phase 5.1: Address Remaining Warnings (Non-Error)
**File:** `crates/maestro-claw/src/providers/openai.rs`
- Add `#[allow(dead_code)]` to response struct fields used for deserialization (or prefix with `_`)
- These fields are required for serde deserialization even if not read directly

**File:** `crates/maestro-claw/src/providers/anthropic.rs`
- Same treatment for `AnthropicResponse.id`, `response_type`, `role`
- Same for stream event variants `ContentBlockStop.index`, `MessageStart.message`, `MessageDelta.delta`

**File:** `crates/maestro-claw/src/providers/ollama.rs`
- Same for `OllamaResponse.total_duration`, `OllamaResponseMessage.role`

**File:** `crates/maestro-claw/src/providers/openrouter.rs`
- Same for `OpenRouterResponse.id`, `provider`, `OpenRouterChoice.index`, etc.

**File:** `crates/gateway/src/ws.rs`
- Prefix unused `state` params with `_state` (lines 328, 337)

#### Phase 5.2: If Errors Reappear
- Run `cargo check --workspace 2>&1` to capture current errors
- Categorize by: type mismatch, missing field, missing import, trait bound
- Fix each error through proper implementation, never code removal
- Run `cargo test --workspace` after each fix batch

---

## 6. Tzar Review Remediation

### Current State
The Tzar review (`maesterclaw-rebuild-tzar-review.md`) identified:
- **0 Critical issues** (all prior criticals resolved)
- **5 Improvements** (IMP-1 through IMP-5)
- **3 Optimizations** (OPT-1 through OPT-3)
- **5 Edge Cases** (EDGE-1 through EDGE-5)
- **5 Security items** (all passing ✅)
- **4 Performance items** (all passing ✅)

### Remediation Plan (ALL items regardless of severity)

#### Phase 6.1: IMP-2 — Anthropic warmup token waste
**File:** `crates/maestro-claw/src/providers/anthropic.rs` (lines 636-661)
- Change `warmup()` to use `"max_tokens": 1` in the request body
- Add TTL cache for warmup result (e.g., `last_warmup: Option<Instant>`, skip if < 5 min)
- Alternative: Anthropic has no public health endpoint, so use a minimal token request

#### Phase 6.2: IMP-3 — OpenRouter warmup token waste
**File:** `crates/maestro-claw/src/providers/openrouter.rs` (lines 566-588)
- Change `warmup()` to delegate to `self.health_check().await` like OpenAI does
- OpenRouter has a `/models` endpoint that can be used for health checking

#### Phase 6.3: IMP-4 — MemoryHook uses std::sync::Mutex
**File:** `crates/maestro-claw/src/hooks/builtin/memory.rs` (line 18)
- Replace `std::sync::Mutex` with `tokio::sync::Mutex` for consistency
- Update all `.lock()` calls to `.lock().await`
- Verify no synchronous contexts depend on this lock

#### Phase 6.4: IMP-5 — Session naming API
**File:** `crates/maestro-claw/src/session/session.rs`
- Add `Session::named(title: String) -> Self` constructor
- Keep `Session::new()` for backward compatibility
- Update integration tests to use `Session::named()` where appropriate

#### Phase 6.5: OPT-1 — ToolRegistry::list() allocation
**File:** `crates/maestro-claw/src/tools/registry.rs` (line 58)
- Return `impl Iterator<Item = &str>` instead of `Vec<String>`
- Or cache the list and invalidate on register/unregister

#### Phase 6.6: OPT-2 — SSE carryover buffer optimization
**Files:** `openai.rs`, `anthropic.rs`, `ollama.rs`, `openrouter.rs`
- Replace `carry[pos + 1..].to_string()` with `carry.drain(..=pos)` pattern
- Use `String::drain()` to avoid tail allocation
- This is a minor optimization but improves correctness

#### Phase 6.7: EDGE-1 — Agent loop summarization i18n
**File:** `crates/maestro-claw/src/agent/loop.rs` (lines 240-246)
- Make summarization prompt configurable via `AgentConfig`
- Add `summary_prompt: Option<String>` field to `AgentConfig`
- Default to English prompt, allow override

#### Phase 6.8: EDGE-2 — System prompt preservation during trim
**File:** `crates/maestro-claw/src/session/thread.rs` (lines 162-168)
- Modify `trim_old_turns(keep)` to always preserve the first System turn
- `turns.retain(|t| t.role == Role::System || turns.len() - t.index <= keep)`

#### Phase 6.9: EDGE-3 — Shell classification bypass (defense hardening)
**File:** `crates/maestro-claw/src/tools/builtin/shell.rs` (lines 85-173)
- Add normalization step: strip NUL bytes, collapse whitespace, resolve shell escape sequences before classification
- Add test cases for `ba\sh`, `b""ash`, backtick substitution

#### Phase 6.10: EDGE-4 — Session garbage collection
**File:** `crates/gateway/src/routes.rs` (lines 673-696)
- Add `SessionConfig { max_sessions: usize, ttl_secs: u64 }` to `GatewayState`
- Implement LRU eviction: on new session creation, if `sessions.len() > max_sessions`, remove oldest
- Add periodic cleanup task via `tokio::spawn` that removes sessions older than TTL

---

## 7. Conductor Tab Functionality

### Current State
The Conductor tab has substantial infrastructure but needs deeper integration review.

### Track: `conductor-ralph-parity_20260213`

#### Investigation Summary
- **State Machine** (`state_machine.rs`): Fully implemented with all transitions (Ready→Running→Paused→Completed→Failed)
- **Keybindings** (`keybindings.rs`): Full Ralph-style shortcuts implemented (s/p/r/?/Ctrl+R/Ctrl+S/Ctrl+A)
- **Polling** (`polling.rs`): Polls `~/.maestro/orchestrate/<track>/session.json` and `events.jsonl`
- **Observer** (`observer.rs`): File-based session observation with steering commands
- **Telemetry** (`telemetry.rs`): Global broadcast bus implemented but **underutilized**

#### Gaps Identified
| Gap | Description | Severity |
|-----|-------------|----------|
| G1 | Telemetry bus `BUS` is created but never subscribed to by the UI loop | Medium |
| G2 | `poll_engine_state()` does direct file I/O on main thread — should use telemetry bus | Medium |
| G3 | `observer.rs` `FileBasedObserver` spawns scan tasks but UI doesn't render observer state changes | Low |
| G4 | Parallel view exists but task parallel execution is not wired | Low |
| G5 | Memory browser in conductor stores memories but doesn't trigger auto-banking | Medium |

### Track: `conductor-loop-telemetry_20260127`

#### Investigation Summary
- The telemetry bus exists (`telemetry.rs:31` — `lazy_static! BUS`)
- Events are defined in `model.rs` (`ConductorEvent` enum with ~15 variants)
- **But**: The polling module (`polling.rs`) reads files directly and calls `state.transition()` without broadcasting through the bus
- The bus has no subscribers in the UI rendering loop

#### Remediation Plan

##### Phase 7.1: Wire Telemetry Bus into Polling
**File:** `crates/cockpit/src/conductor/polling.rs`
- After each `state.transition(event)` call, also `telemetry::BUS.broadcast(event.clone())`
- This decouples the state update from any UI refresh trigger

##### Phase 7.2: Subscribe UI Loop to Telemetry
**File:** `crates/cockpit/src/app.rs`
- In `run_app()`, create a `telemetry::BUS.subscribe()` receiver
- In the event loop, check `rx.try_recv()` alongside keyboard events
- On telemetry event: force UI refresh, update status bar

##### Phase 7.3: Observer State Rendering
**File:** `crates/cockpit/src/conductor/pane.rs`
- Add `render_observer_status()` function
- Show: active observed sessions, last event timestamps, observer errors
- Wire into the Conductor dashboard overlay

##### Phase 7.4: Parallel View Wiring
**File:** `crates/cockpit/src/conductor/parallel_view.rs`
- Connect to `ConductorState.parallel_tasks` if/when multiple tasks run concurrently
- Render task dependency graph with status indicators

##### Phase 7.5: Memory Integration in Conductor
**File:** `crates/cockpit/src/conductor/keybindings.rs`
- After `ConductorEvent::TaskCompleted`, auto-store a memory summarizing the task result
- After `ConductorEvent::AllComplete`, store a track completion memory
- Use `ConductorAction::StoreMemory` pattern already established

---

## 8. Settings Save Button

### Current State
The save button handler exists at `app.rs:4732-4735`:
```rust
SettingsOption::Save => {
    let _ = app.config.save();
    app.status_message = "Configuration saved to ~/.config/maestro/config.toml".to_string();
}
```

### Issues Found
1. **`let _ = app.config.save()`** silently swallows errors — the `_` discards the `Result`
2. **`config.save()` returns `Ok(())` even when `dirs::config_dir()` returns `None`** — it silently does nothing if the config directory can't be determined
3. No visual feedback beyond status message (no toast notification)
4. No validation of config values before save

### Remediation Plan

#### Phase 8.1: Fix Error Handling in Save Handler
**File:** `crates/cockpit/src/app.rs` (line 4732)
```rust
SettingsOption::Save => {
    match app.config.save() {
        Ok(()) => {
            app.toast_queue.success("Configuration saved to ~/.config/maestro/config.toml");
            app.status_message = "Configuration saved".to_string();
        }
        Err(e) => {
            app.toast_queue.error(format!("Failed to save: {}", e));
            app.status_message = format!("Save failed: {}", e);
        }
    }
}
```

#### Phase 8.2: Fix Config::save() Silent Failure
**File:** `src/leindex/src/config.rs` (line 43)
```rust
pub fn save(&self) -> anyhow::Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let maestro_conf = config_dir.join("maestro");
    if !maestro_conf.exists() {
        fs::create_dir_all(&maestro_conf)?;
    }
    let config_path = maestro_conf.join("config.toml");
    let toml_string = toml::to_string(self)?;
    fs::write(&config_path, toml_string)?;
    Ok(())
}
```

#### Phase 8.3: Add Config Validation
**File:** `src/leindex/src/config.rs`
- Add `Config::validate() -> Result<()>` method
- Validate: editor binary exists, install_path is writable, theme is known
- Call `validate()` before `save()` in the settings handler

---

## 9. Memory Tab - Banking Integration

### Current State
- Memory banking occurred twice during the overhaul track and then stopped
- `MemoryService::store_memory()` exists and works (proven by manual `n` key in Memory tab and Conductor memory browser)
- Agent sessions launched via `SessionManager::create_session()` at `app.rs:1516` do NOT inject memory hooks
- The `PersistentMemoryHook` in `maestro-claw` (`integration/memory.rs`) exists but is only used when agents are launched through the MaesterClaw gateway, NOT through the cockpit's session spawning

### Root Cause
The cockpit spawns sessions via `TmuxMultiplexer` which launches CLI agents (iflow, claude, etc.) as subprocesses. These processes have NO hook into the Maestro memory system because they run independently.

### Remediation Plan

#### Phase 9.1: Create Memory Banking Bridge for CLI Agents
**File:** `crates/cockpit/src/maesterclaw/memory_bridge.rs` (NEW)
```rust
pub struct AgentMemoryBridge {
    service: Arc<MemoryService>,
    session_id: String,
    agent_name: String,
    bank_interval: Duration,
}

impl AgentMemoryBridge {
    /// Start background memory banking for a CLI agent session
    pub async fn start_banking(&self) -> JoinHandle<()> {
        // Monitor agent's output (via tmux capture-pane)
        // Parse for key events (task completion, errors, discoveries)
        // Store memories at configured intervals
    }
}
```

#### Phase 9.2: Hook Memory Banking into Session Creation
**File:** `crates/cockpit/src/app.rs` (around line 1523)
- After successful `manager.create_session()`, spawn `AgentMemoryBridge`
- Pass `MemoryService`, session_id, and agent tool name
- Store the `JoinHandle` in `App` for cleanup on session teardown

#### Phase 9.3: Implement Output Monitoring
**File:** `crates/cockpit/src/maesterclaw/memory_bridge.rs`
- Use `TmuxMultiplexer::capture_pane(session_id)` periodically
- Parse output for memory-worthy events:
  - Pattern: `"Task completed"`, `"Error:"`, `"Created:"`, `"Modified:"`
  - Use heuristics: lines containing file paths, error messages, completion markers
- Store parsed insights as categorized memories

#### Phase 9.4: Conductor Auto-Banking
**File:** `crates/cockpit/src/conductor/keybindings.rs`
- Extend `ConductorAction` with `AutoBankMemory { event_type, content }`
- In `app.rs` conductor event handling, after `ConductorEvent::TaskCompleted` etc., auto-trigger memory storage

---

## 10. Memory Tab - Tree Dependency Graph

### Current State
- Memories are displayed as a flat list with expand/collapse per item
- `MemoryInfo` struct has `project_id`, `track_id`, `session_id`, `tags` fields that could express relationships
- No tree/graph visualization exists
- The detail panel shows metadata but no relational connections

### Remediation Plan

#### Phase 10.1: Add Dependency Graph Data Model
**File:** `crates/cockpit/src/state/types.rs`
```rust
pub struct MemoryNode {
    pub memory: MemoryInfo,
    pub children: Vec<usize>,  // Indices into the tree
    pub parent: Option<usize>,
    pub depth: usize,
    pub is_expanded: bool,
}

pub struct MemoryTreeState {
    pub nodes: Vec<MemoryNode>,
    pub root_indices: Vec<usize>,
    pub selected: usize,
    pub view_mode: MemoryViewMode,
}

pub enum MemoryViewMode {
    FlatList,
    TreeByProject,
    TreeByTrack,
    TreeBySession,
    TreeByCategory,
    DependencyGraph,
}
```

#### Phase 10.2: Build Tree from Memory Relationships
**File:** `crates/cockpit/src/tabs/memory.rs`
- `build_memory_tree(memories: &[MemoryInfo], mode: MemoryViewMode) -> MemoryTreeState`
- Group by: project_id → track_id → session_id → category → individual memories
- Support toggling between flat list and tree view via `v` key

#### Phase 10.3: Render Tree with Expand/Collapse
**File:** `crates/cockpit/src/tabs/memory.rs`
- Render tree with indentation and tree drawing characters (├──, └──, │)
- `Enter` expands/collapses nodes
- Arrow Right/Left for tree navigation
- Show memory count per group node

#### Phase 10.4: User-Editable Memory Details
**File:** `crates/cockpit/src/tabs/memory.rs`
- In `MemoryDetail` mode, add `e` key to enter edit mode
- Editable fields: content, category, importance, tags
- Changes saved via `MemoryService::update_memory(id, fields)`
- Add `MemoryService::update_memory()` if it doesn't exist

#### Phase 10.5: Add Dependency Graph Rendering
- In `DependencyGraph` view mode, show memories connected by:
  - Same session_id (temporal dependencies)
  - Same project_id (project scope)
  - Tag overlap (semantic connections)
- Use ASCII art for connections between related memories

---

## 11. Memory Tab - Interval-Based Saving

### Current State
Memory banking only happens manually (user presses `n` or uses Conductor memory browser). No automatic interval-based saving exists.

### Required Intervals
Per user specification, memories MUST be saved at:
1. Track/spec creation
2. Task completion
3. Insight discovery
4. Module creation
5. Checkpoints
6. Review
7. Remediation completion
8. Track completion

### Remediation Plan

#### Phase 11.1: Define Memory Event Types
**File:** `crates/cockpit/src/maesterclaw/memory_events.rs` (NEW)
```rust
pub enum MemoryTrigger {
    TrackCreated { track_id: String, spec_summary: String },
    TaskCompleted { task_id: String, result: String },
    InsightDiscovered { content: String, source: String },
    ModuleCreated { module_path: String, description: String },
    Checkpoint { checkpoint_id: String, state_summary: String },
    ReviewCompleted { review_type: String, findings: Vec<String> },
    RemediationCompleted { item_id: String, resolution: String },
    TrackCompleted { track_id: String, summary: String },
}
```

#### Phase 11.2: Implement Trigger Detection in Conductor
**File:** `crates/cockpit/src/conductor/polling.rs`
- After parsing `events.jsonl` entries, detect each trigger type:
  - `EngineEvent::TaskCompleted` → `MemoryTrigger::TaskCompleted`
  - `EngineEvent::TrackStarted` → `MemoryTrigger::TrackCreated`
  - `EngineEvent::AllComplete` → `MemoryTrigger::TrackCompleted`
  - Custom heuristics for insight/module/review detection from agent output

#### Phase 11.3: Implement Trigger Detection in Session Spawning
**File:** `crates/cockpit/src/app.rs`
- On session creation → `MemoryTrigger::TrackCreated` (if associated with a track)
- On session detach after work → `MemoryTrigger::Checkpoint`

#### Phase 11.4: Auto-Banking Service
**File:** `crates/cockpit/src/maesterclaw/memory_events.rs`
```rust
pub struct MemoryAutoBank {
    service: Arc<MemoryService>,
    rx: mpsc::Receiver<MemoryTrigger>,
}

impl MemoryAutoBank {
    pub async fn run(&mut self) {
        while let Some(trigger) = self.rx.recv().await {
            let (content, category) = trigger.to_memory_content();
            let _ = self.service.store_memory(&content, category);
        }
    }
}
```

#### Phase 11.5: Wire Triggers into Event Flow
**File:** `crates/cockpit/src/app.rs`
- Create `mpsc::channel()` at app startup
- Pass `tx` to Conductor, Session Manager, and Agent Bridge
- Spawn `MemoryAutoBank::run()` as background task
- Each subsystem sends `MemoryTrigger` events through the channel

#### Phase 11.6: Periodic Checkpoint Banking
- Add timer-based checkpoint every N minutes (configurable via `memory_bank_interval_secs`)
- On checkpoint: capture current app state summary, store as `MemoryTrigger::Checkpoint`
- Include: active sessions, track progress, recent changes

---

## Execution Priority Order

| Priority | Item | Est. Complexity | Dependencies |
|----------|------|-----------------|--------------|
| P0 | §8 Settings Save Button | Low | None |
| P0 | §5 Compilation Warnings | Low | None |
| P1 | §6 Tzar Review Remediation | Medium | None |
| P1 | §1 MaesterClaw Keyboard Shortcuts | High | None |
| P2 | §4 MaesterClaw Settings | Medium | §1, §8 |
| P2 | §3 Agent Integration | High | §1 |
| P2 | §7 Conductor Tab Gaps | Medium | None |
| P3 | §2 Setup Wizard | High | §1, §3 |
| P3 | §9 Memory Banking Integration | High | §7, §11 |
| P3 | §11 Interval-Based Memory Saving | High | §9 |
| P4 | §10 Tree Dependency Graph | Medium | §9 |

---

## Testing Strategy

### Per-Phase Verification
1. `cargo check --workspace` — zero errors
2. `cargo clippy --workspace --all-targets` — zero warnings (or only pre-existing)
3. `cargo test --workspace` — all existing tests pass
4. Manual TUI testing for each new keybinding/feature
5. `cargo test -p maestro-cockpit` — new tests added for each phase

### Integration Testing
- Launch TUI, navigate to each tab, verify all shortcuts
- Create session with each agent type, verify memory banking
- Run conductor with a track, verify telemetry bus events
- Change settings and verify save/load cycle

---

*This plan constitutes the comprehensive remediation blueprint for the Maestro Cockpit TUI.*
