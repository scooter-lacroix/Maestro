# LSP Integration Documentation

## Overview

The LSP (Language Server Protocol) Integration provides on-demand code intelligence for Maestro sessions. It manages the lifecycle of language server processes, providing features like:

- **Auto-start**: Automatically start appropriate LSPs based on project file types
- **Manual control**: Start, stop, and restart LSPs via TUI
- **State persistence**: LSP state saved to Turso database across restarts
- **Graceful degradation**: Sessions continue operating even when LSPs fail

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Maestro Session                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │ rust-       │    │ ruff-       │    │ typescript- │         │
│  │ analyzer    │    │ lsp         │    │ language-   │         │
│  │ (Rust)      │    │ (Python)    │    │ server      │         │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘         │
│         │                   │                   │                │
│         └───────────────────┼───────────────────┘                │
│                             │                                    │
│                    ┌────────▼────────┐                          │
│                    │   LspManager    │                          │
│                    │  - Lifecycle    │                          │
│                    │  - Monitoring   │                          │
│                    │  - State Pers.  │                          │
│                    └────────┬────────┘                          │
├─────────────────────────────┼──────────────────────────────────┤
│                             ▼                                    │
│                    ┌────────────────┐                           │
│                    │  Turso Backend │                           │
│                    │  lsp_servers   │                           │
│                    └────────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
```

## Supported LSPs

| LSP | Language | Binary Name | Extensions |
|-----|----------|-------------|------------|
| rust-analyzer | Rust | `rust-analyzer` | `.rs` |
| ruff-lsp | Python | `ruff-lsp` | `.py` |
| typescript-language-server | TypeScript/JavaScript | `typescript-language-server` | `.ts`, `.tsx`, `.js`, `.jsx` |

## LSP Lifecycle

### 1. Starting an LSP

LSPs can be started automatically or manually:

**Auto-start:**
- Triggered when a session contains files with matching extensions
- Example: `.rs` files → auto-start rust-analyzer
- Controlled by `auto_start` flag in `LspConfig`

**Manual start:**
```rust
use leindex_analyzers::memory::{LspManager, LspType};

let manager = LspManager::new(storage);
manager.start_lsp("session-123", LspType::Rust, None).await?;
```

### 2. Running State

When an LSP is running:
- Process spawned with stdio pipes connected
- Status set to `LspStatus::Running`
- PID and timestamp tracked
- Background monitoring task watches for crashes

### 3. Stopping an LSP

```rust
manager.stop_lsp("session-123", LspType::Rust).await?;
```

Shutdown sequence:
1. Attempt graceful shutdown (SIGTERM on Unix)
2. Wait up to 5 seconds for clean exit
3. Force kill if necessary (SIGKILL on Unix)
4. Clean up stdio proxy socket (if enabled)
5. Update status to `LspStatus::Stopped`

### 4. Monitoring

Each LSP has a background monitoring task that:
- Checks if process is still alive
- Updates status to `LspStatus::Error` if crashed
- Stores error message for diagnostics

## Language Detection and Auto-Start

Language detection uses file extension mapping:

```rust
impl LspType {
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            LspType::Rust => &["rs"],
            LspType::Python => &["py"],
            LspType::TypeScript => &["ts", "tsx", "js", "jsx"],
        }
    }
}
```

When a session is indexed:
1. Collect all file extensions in the session
2. Match against LSP extension mappings
3. Auto-start LSPs with `auto_start: true`

## State Persistence in Turso

LSP state is persisted in the `lsp_servers` table:

```sql
CREATE TABLE IF NOT EXISTS lsp_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    language TEXT NOT NULL,
    lsp_name TEXT NOT NULL,
    status TEXT NOT NULL,
    pid INTEGER,
    port INTEGER,
    auto_start INTEGER NOT NULL DEFAULT 1,
    last_started TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT,
    UNIQUE(session_id, lsp_name)
);
```

### State Structure

```rust
pub struct LspServerState {
    pub id: i64,
    pub session_id: String,
    pub language: String,        // "rust", "python", "typescript"
    pub lsp_name: String,        // "rust-analyzer", "ruff-lsp", etc.
    pub status: LspStatus,       // Running, Stopped, Error, Starting
    pub pid: Option<i64>,
    pub port: Option<i64>,
    pub auto_start: bool,
    pub last_started: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}
```

## Graceful Degradation Patterns

The LSP integration is designed to fail gracefully:

### 1. LSP Not Installed

If an LSP binary is not found:
- Session continues operating without LSP features
- Error stored in `last_error` field
- User can install LSP later and restart

### 2. LSP Crash During Operation

If an LSP process crashes:
- Background monitoring detects the crash
- Status updated to `LspStatus::Error`
- Session continues without that LSP
- User can manually restart via TUI

### 3. Database Unavailable

If Turso database is unavailable:
- In-memory LSP tracking continues
- State not persisted (lost on restart)
- LSP processes continue running

## LspManager API

### Creating a Manager

```rust
use leindex_analyzers::memory::{LspManager, TursoStorageBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = TursoStorageBackend::new(None, None).await?;
    let manager = LspManager::new(storage);
    Ok(())
}
```

### Starting an LSP

```rust
/// Start an LSP for a session
///
/// # Arguments
/// * `session_id` - Session identifier
/// * `lsp_type` - Type of LSP to start
/// * `config` - Optional LSP configuration (binary_path, args, env_vars)
///
/// # Returns
/// Returns Ok(()) if LSP started successfully or already running
pub async fn start_lsp(
    &self,
    session_id: &str,
    lsp_type: LspType,
    config: Option<LspConfig>,
) -> Result<()>
```

### Stopping an LSP

```rust
/// Stop an LSP for a session
///
/// # Arguments
/// * `session_id` - Session identifier
/// * `lsp_type` - Type of LSP to stop
///
/// # Returns
/// Returns Ok(()) if LSP stopped successfully or was not running
pub async fn stop_lsp(
    &self,
    session_id: &str,
    lsp_type: LspType,
) -> Result<()>
```

### Getting LSP State

```rust
/// Get LSP state for a session
///
/// # Returns
/// Returns HashMap of lsp_name -> LspProcess
pub async fn get_lsps_for_session(
    &self,
    session_id: &str,
) -> Result<HashMap<String, LspProcess>>
```

### Restarting an LSP

```rust
/// Restart an LSP (stop then start)
///
/// # Returns
/// Returns Ok(()) if LSP restarted successfully
pub async fn restart_lsp(
    &self,
    session_id: &str,
    lsp_type: LspType,
) -> Result<()>
```

## LspProcess Structure

```rust
pub struct LspProcess {
    pub lsp_type: LspType,
    pub session_id: String,
    pub pid: Option<u32>,
    pub status: LspStatus,
    pub port: Option<u16>,
    pub auto_start: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    // ... internal fields
}
```

## LspStatus Values

| Status | Description |
|--------|-------------|
| `Starting` | LSP process is being spawned |
| `Running` | LSP is running and healthy |
| `Stopped` | LSP is not running (normal state) |
| `Error` | LSP crashed or failed to start |

## LspConfig Options

```rust
pub struct LspConfig {
    /// Auto-start this LSP when language is detected
    pub auto_start: bool,

    /// Custom path to LSP binary (if not in PATH)
    pub binary_path: Option<PathBuf>,

    /// Additional arguments to pass to LSP
    pub additional_args: Vec<String>,

    /// Environment variables for LSP process
    pub env_vars: HashMap<String, String>,

    /// Use stdio proxy for this LSP (requires feature flag)
    pub use_proxy: bool,
}
```

### Default Configuration

```rust
impl Default for LspConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            binary_path: None,
            additional_args: Vec::new(),
            env_vars: HashMap::new(),
            use_proxy: false,
        }
    }
}
```

## Example Usage

### Basic Auto-Start

```rust
// Session with Rust files
let session_id = "my-rust-project";
let manager = LspManager::new(storage);

// Auto-start will detect .rs files and start rust-analyzer
manager.auto_start_lsps(session_id, &[".rs"]).await?;
```

### Manual Control with Custom Config

```rust
use std::path::PathBuf;
use std::collections::HashMap;

let config = LspConfig {
    auto_start: false,
    binary_path: Some(PathBuf::from("/custom/path/rust-analyzer")),
    additional_args: vec!["--verbose".to_string()],
    env_vars: {
        let mut map = HashMap::new();
        map.insert("RUST_ANALYZER_CONFIG".to_string(), "/path/to/config".to_string());
        map
    },
    use_proxy: false,
};

manager.start_lsp(session_id, LspType::Rust, Some(config)).await?;
```

### Checking LSP Status

```rust
let lsps = manager.get_lsps_for_session(session_id).await?;

for (lsp_name, process) in lsps {
    println!("{}: {} (PID: {:?})",
        lsp_name,
        process.status.as_str(),
        process.pid
    );
}
```

## Error Handling

All LSP manager operations return `Result<()>` with descriptive errors:

```rust
use anyhow::{anyhow, Result};

match manager.start_lsp(session_id, LspType::Rust, None).await {
    Ok(()) => println!("LSP started"),
    Err(e) => {
        if e.to_string().contains("Failed to spawn") {
            eprintln!("LSP binary not found. Please install rust-analyzer.");
        } else {
            eprintln!("Failed to start LSP: {}", e);
        }
    }
}
```

## Process Cleanup

On Unix systems, LSP processes are killed by process group to prevent child process leaks:

```rust
#[cfg(unix)]
unsafe fn unix_kill_process_group(pgid: i32, signal: libc::c_int) -> Result<()> {
    let result = libc::killpg(pgid, signal);
    if result == -1 {
        return Err(anyhow!("Failed to send signal to process group"));
    }
    Ok(())
}
```

Shutdown sequence:
1. Try SIGTERM (graceful shutdown)
2. Wait up to 5 seconds
3. Try SIGKILL (force terminate) if needed
4. Clean up stdio proxy socket file

## See Also

- [MCP Bridge Protocol](./mcp_bridge_protocol.md) - LSP to MCP translation
- [TUI User Guide](./lsp_tui_user_guide.md) - Using LSPs in the terminal UI
- [Troubleshooting](./lsp_troubleshooting.md) - Common issues and solutions
