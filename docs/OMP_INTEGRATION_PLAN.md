# OMP (oh-my-pi) Integration Plan for Maestro Cockpit

## Executive Summary

Integrate oh-my-pi (OMP) as a first-class tool provider within Maestro Cockpit, leveraging existing Maestro infrastructure for LSP/MCP/session management while preserving OMP's unique TypeScript/Bun-based agent capabilities.

## Component Overlap Analysis

### 1. LSP Integration

| Feature | OMP (TypeScript) | Maestro (Rust) | Recommendation |
|---------|-----------------|----------------|----------------|
| **Client Management** | `lsp/client.ts` - per-file client tracking | `lsp_pool.rs` - pooled instances | **Use Maestro** - shared pool is more efficient |
| **Diagnostics** | Pull-based via tool | Push to memory via MCP bridge | **Use Maestro** - bridge pattern integrates with TUI |
| **Protocol** | vscode-languageserver-node | lsp-types crate | **Use Maestro** - native Rust, no FFI |
| **Warmup** | Parallel startup in `warmupLspServers()` | `auto_start_lsps_for_session()` | **Use Maestro** - better integration with session lifecycle |

**Decision**: OMP's LSP tool should call through to Maestro's `LspPool` via IPC.

### 2. MCP Integration

| Feature | OMP (TypeScript) | Maestro (Rust) | Recommendation |
|---------|-----------------|----------------|----------------|
| **Transport** | stdio + HTTP | stdio only | **Hybrid** - OMP HTTP transport for remote MCPs |
| **Discovery** | `.mcp.json` parsing | System-wide + per-project | **Use Maestro** - centralized discovery |
| **Tool Bridge** | `MCPTool` wrapper | Direct tool exposure | **Use OMP pattern** - better tool cache |
| **OAuth Flow** | Full OAuth discovery | None | **Use OMP** - unique capability |
| **Connection Pool** | Per-session | Global `McpPool` | **Use Maestro** - shared across sessions |

**Decision**: Maestro manages MCP server lifecycle; OMP provides HTTP transport and OAuth flow.

### 3. Session Management

| Feature | OMP (TypeScript) | Maestro (Rust) | Recommendation |
|---------|-----------------|----------------|----------------|
| **Storage Format** | JSONL (append-only) | SQLite via Turso | **Use Maestro** - better query support |
| **Compaction** | Token-based pruning | Not implemented | **Port OMP** - critical for long sessions |
| **Branch Summarization** | LLM-generated summaries | Not implemented | **Port OMP** - enables context recovery |
| **Session Fork** | Yes | No | **Port OMP** - useful for experiments |
| **Auth Storage** | SQLite credential store | Integrated with session | **Use OMP** - better OAuth handling |

**Decision**: Use Maestro's SQLite storage; port OMP's compaction and summarization algorithms.

### 4. TUI Rendering

| Feature | OMP (TypeScript) | Maestro (Rust) | Recommendation |
|---------|-----------------|----------------|----------------|
| **Framework** | Custom reactive TUI | ratatui | **Use Maestro** - native performance |
| **Streaming** | Differential rendering | Full re-render | **Port OMP pattern** - more efficient |
| **Components** | Composable widgets | ratatui widgets | **Hybrid** - adapt OMP patterns |

**Decision**: OMP runs as subprocess; output rendered in Maestro TUI pane.

### 5. Tool Execution

| Tool | OMP | Maestro | Overlap |
|------|-----|---------|---------|
| BashTool | ✅ Virtual terminal | ✅ tmux integration | **Use Maestro** - tmux persistence |
| PythonTool | ✅ IPython kernel | ❌ | **Use OMP** |
| EditTool | ✅ Patch-based | ✅ sed-based | **Use OMP** - patch is safer |
| ReadTool | ✅ | ✅ | Either |
| GrepTool | ✅ ripgrep WASM | ✅ system grep | **Use OMP** - WASM is faster |
| FindTool | ✅ glob WASM | ✅ system find | **Use OMP** - WASM is faster |

## Architecture Design

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     MAESTRO COCKPIT (Rust/TUI)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │ Conductor   │  │  Sessions   │  │    LSPs     │  │    MCP     │ │
│  │    Pane     │  │   Manager   │  │    Pool     │  │    Pool    │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └─────┬──────┘ │
│         │                │                │                │        │
│         └────────────────┴────────────────┴────────────────┘        │
│                                    │                                 │
│                          ┌─────────▼─────────┐                      │
│                          │   OMP BRIDGE      │                      │
│                          │   (IPC/RPC)       │                      │
│                          └─────────┬─────────┘                      │
└────────────────────────────────────┼────────────────────────────────┘
                                     │
                          ┌─────────▼─────────┐
                          │  OMP WORKER       │
                          │  (Bun/TypeScript) │
                          │  ┌─────────────┐  │
                          │  │ Agent Loop  │  │
                          │  │ Tool Runner │  │
                          │  │ Compaction  │  │
                          │  │ IPython     │  │
                          │  └─────────────┘  │
                          └───────────────────┘
```

### Communication Protocol

1. **Control Channel**: Unix socket for JSON-RPC commands
2. **Output Channel**: stdout streaming for TUI rendering
3. **Event Channel**: SSE for real-time updates (diagnostics, progress)

### Key Integration Points

#### 1. OMP Tool Provider Registration

```rust
// In crates/cockpit/src/tools/omp.rs
pub struct OmpToolProvider {
    bridge: OmpBridge,
}

impl ToolProvider for OmpToolProvider {
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition::new("omp:python", "Execute Python code"),
            ToolDefinition::new("omp:edit", "Apply patch-based edits"),
            ToolDefinition::new("omp:grep", "Search with ripgrep WASM"),
            ToolDefinition::new("omp:find", "Find files with glob WASM"),
        ]
    }
    
    async fn execute(&self, tool: &str, input: Value) -> Result<ToolResult> {
        self.bridge.invoke(tool, input).await
    }
}
```

#### 2. Startup Optimization

**Current OMP Startup Path** (slow):
```
main.ts → initTheme → Settings.init → discoverAuthStorage 
→ ModelRegistry → resolveModelScope → createAgentSession 
→ discoverExtensions → loadSkills → warmupLspServers 
→ discoverMCP → InteractiveMode.init
```

**Optimized Path** (fast, like pi-mono):
```
main.ts (worker) → receive config via IPC → skip discovery 
→ use pre-warmed pools from Maestro → ready in <100ms
```

**Optimization Strategies**:

1. **Lazy Discovery**: Defer extension/skill/MCP discovery until first use
2. **Pre-warmed Pools**: Share LSP/MCP pools with Maestro cockpit
3. **Parallel Init**: Run model registry + auth storage in parallel
4. **Config Injection**: Receive all config via IPC from cockpit (no file I/O)
5. **WASM Warmup**: Pre-compile WASM modules on first worker spawn

#### 3. Conductor Integration

```rust
// In crates/cockpit/src/conductor/omp_integration.rs
impl ConductorPane {
    pub fn spawn_omp_agent(&mut self, track: &Track) -> Result<()> {
        let session_id = self.create_session_for_track(track);
        
        // OMP worker receives context from conductor
        let config = OmpWorkerConfig {
            session_id: session_id.clone(),
            project_path: track.link_path.clone(),
            model: self.config.default_model.clone(),
            tools: vec!["python", "edit", "grep", "find", "read", "write"],
            // Use Maestro's pre-warmed pools
            lsp_pool: Some(self.app.lsp_pool.clone()),
            mcp_pool: Some(self.app.mcp_pool.clone()),
        };
        
        // Spawn OMP as subprocess with IPC
        let worker = OmpWorker::spawn(config)?;
        
        // Wire up output to iteration output panel
        self.wire_worker_output(worker, track.id.clone());
        
        Ok(())
    }
}
```

## Implementation Phases

### Phase 1: OMP Worker Bridge (Week 1)

**Goal**: Establish IPC communication between Maestro and OMP

1. Create `crates/cockpit/src/omp/` module
2. Implement `OmpBridge` with Unix socket IPC
3. Define JSON-RPC protocol for tool invocation
4. Create OMP worker entry point (`packages/coding-agent/src/worker.ts`)

### Phase 2: Tool Delegation (Week 2)

**Goal**: Route tool calls through appropriate backend

1. Implement `ToolProvider` trait for OMP
2. Map OMP tools to Maestro equivalents where overlap exists
3. Add Python tool support (unique to OMP)
4. Port WASM-based grep/find for performance

### Phase 3: Startup Optimization (Week 3)

**Goal**: Achieve pi-mono launch speed (<100ms to ready)

1. Profile current OMP startup with `PI_DEBUG_STARTUP=1`
2. Implement config injection via IPC
3. Create shared pool access protocol
4. Add lazy discovery for extensions/skills
5. Benchmark against pi-mono

### Phase 4: Conductor Integration (Week 4)

**Goal**: Deep integration with orchestrate workflow

1. Add OMP as tool option in Conductor pane
2. Wire OMP output to iteration log
3. Support track-specific tool configuration
4. Add session resumption with OMP workers

### Phase 5: Compaction Port (Week 5)

**Goal**: Enable long-running session support

1. Port OMP's token-based compaction algorithm
2. Implement branch summarization
3. Add session fork capability
4. Integrate with Maestro's memory system

## File Structure

```
maestro/
├── crates/
│   └── cockpit/
│       └── src/
│           ├── omp/
│           │   ├── mod.rs           # Module exports
│           │   ├── bridge.rs        # IPC bridge to OMP worker
│           │   ├── protocol.rs      # JSON-RPC protocol definitions
│           │   ├── provider.rs      # ToolProvider implementation
│           │   └── worker.rs        # Worker process management
│           └── conductor/
│               └── omp_agent.rs     # Conductor-OMP integration
└── docs/
    └── OMP_INTEGRATION_PLAN.md      # This document

oh-my-pi/  (cloned to maestro's vendor/)
└── packages/
    └── coding-agent/
        └── src/
            ├── worker.ts            # New: Worker entry point
            ├── ipc/
            │   ├── server.ts        # IPC server for Maestro
            │   └── protocol.ts      # Shared protocol types
            └── tools/
                └── maestro-bridge.ts # Tools that delegate to Maestro
```

## Performance Targets

| Metric | Current OMP | Target | Rationale |
|--------|-------------|--------|-----------|
| Cold start | ~2-3s | <500ms | pi-mono achieves ~100ms |
| Warm start | ~1s | <100ms | Shared pools, no discovery |
| Tool invocation | 50-100ms | <20ms | Direct IPC, no HTTP |
| Memory overhead | ~200MB | <50MB | Shared pools, single process |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| IPC overhead | Medium | Medium | Use shared memory for large outputs |
| Protocol drift | Low | High | Generate types from shared schema |
| Bun compatibility | Low | High | Fallback to Node.js runtime |
| WASM compilation | Medium | Low | Pre-compile on first spawn |

## Success Criteria

1. ✅ OMP tools accessible from Maestro Cockpit
2. ✅ Startup time <500ms cold, <100ms warm
3. ✅ Python tool working in Cockpit sessions
4. ✅ Session compaction preserving context
5. ✅ Conductor can spawn OMP agents for tracks

## Implementation Status

### Completed

1. **OMP Module Structure** (`crates/cockpit/src/omp/`)
   - `mod.rs` - Module exports
   - `protocol.rs` - JSON-RPC protocol definitions
   - `worker.rs` - Subprocess management with IPC
   - `bridge.rs` - High-level tool invocation API
   - `provider.rs` - ToolProvider trait implementation

2. **Conductor Integration** (`crates/cockpit/src/conductor/omp_agent.rs`)
   - `OmpAgent` - Per-track agent instance
   - `OmpAgentManager` - Manages multiple agents
   - Integration with Conductor pane

3. **OMP Worker** (`packages/coding-agent/src/worker.ts`)
   - JSON-RPC over stdio
   - Lazy initialization
   - Tool invocation routing

### In Progress

1. **Startup Optimization**
   - Profile current startup path with `PI_DEBUG_STARTUP=1`
   - Implement shared pool access via Unix sockets
   - Add config injection to skip discovery

### Pending

1. **Session Compaction Port** - Port OMP's token-based compaction
2. **Branch Summarization** - Enable context recovery for long sessions
3. **HTTP MCP Transport** - Add HTTP transport support from OMP
4. **OAuth Flow** - Integrate OMP's OAuth discovery for MCP servers

### Performance Benchmarks

Current (unoptimized):
```
cold start: ~2-3s (full discovery)
warm start: ~1s (cached models)
tool invoke: ~50-100ms (JSON-RPC overhead)
```

Target (after optimization):
```
cold start: <500ms (config injection, shared pools)
warm start: <100ms (worker reuse)
tool invoke: <20ms (direct IPC)
```
