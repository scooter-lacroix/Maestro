# TUI User Guide for LSP Features

## Overview

The Terminal User Interface (TUI) provides comprehensive LSP management capabilities, including:

- **LSP Status Indicators** on session cards
- **Dedicated LSPs Tab** for detailed management
- **Manual Controls** (Start, Stop, Restart)
- **Log Viewing** for debugging
- **Refresh Controls** for status updates

## LSP Status Indicators

### Session Card Indicators

Each session card displays colored indicators for its active LSPs:

```
┌─────────────────────────────────────────────────────────────────┐
│  my-rust-project                            [● R] [○ P]         │
│  Active • 245 files                        rust: running       │
│                                             python: stopped     │
└─────────────────────────────────────────────────────────────────┘
```

### Indicator Colors and Icons

| Status | Icon | Color | Description |
|--------|------|-------|-------------|
| **Running** | `●` | GREEN | LSP is running and healthy |
| **Starting** | `○` | YELLOW | LSP is starting up |
| **Error** | `x` | RED | LSP crashed or failed to start |
| **Stopped** | `○` | GRAY | LSP is not running |

### Language Short Codes

| Code | Language | LSP Binary |
|------|----------|------------|
| R | Rust | rust-analyzer |
| P | Python | ruff-lsp |
| T | TypeScript | typescript-language-server |
| J | JavaScript | typescript-language-server |

## LSPs Tab

### Accessing the LSPs Tab

1. Navigate to Dashboard mode
2. Press `5` or use arrow keys to select "LSPs" tab

### LSPs Tab Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ LSPs                                    [r] Refresh [l] Logs    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ● rust-analyzer    Running   my-rust-project                   │
│  ■ ruff-lsp         Stopped   my-rust-project                   │
│  ⚠ typescript-...   Error     my-ts-project                     │
│                                                                  │
│  ─────────────────────────────────────────────────────────      │
│  [Enter] Toggle Start/Stop  [R] Restart  [l] View Logs          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### LSP List Columns

1. **Status Icon:** `●` Running, `■` Stopped, `⚠` Error
2. **LSP Name:** Full LSP server name
3. **Status Text:** Running, Stopped, Error, Starting
4. **Session:** Session name the LSP belongs to

## LSP Controls

### 1. Start/Stop LSP (Toggle)

**Action:** Press `Enter` on selected LSP

**Behavior:**
- If Stopped/Error → Starts the LSP
- If Running/Starting → Stops the LSP

**Status Message:**
```
Starting 'rust-analyzer'... (press 'r' to refresh)
```

**Implementation:**
```rust
fn toggle_lsp(&mut self, session_id: &str, lsp_name: &str, status: LspStatus) {
    let result = match status {
        LspStatus::Stopped | LspStatus::Error => {
            // Start the LSP
            lsp_manager.start_lsp(&session_id, lsp_type, None).await
        }
        LspStatus::Running | LspStatus::Starting => {
            // Stop the LSP
            lsp_manager.stop_lsp(&session_id, lsp_type).await
        }
    };
}
```

### 2. Restart LSP

**Action:** Press `R` on selected LSP

**Behavior:**
1. Stops the LSP (if running)
2. Waits for clean shutdown
3. Starts the LSP again
4. Returns to LSPs tab

**Status Message:**
```
Restarting 'rust-analyzer'... (press 'r' to refresh)
```

**Use cases:**
- LSP is in Error state
- LSP is not responding
- Need to reload LSP configuration

### 3. Refresh LSP Status

**Action:** Press `r` in LSPs tab

**Behavior:**
- Queries Turso database for current LSP states
- Updates in-memory cache
- Refreshes display

**Status Message:**
```
LSP status refreshed
```

**Refresh Throttling:**
- Manual refresh: No throttling (force=true)
- Auto-refresh: Throttled to prevent excessive DB queries

### 4. View LSP Logs

**Action:** Press `l` on selected LSP

**Behavior:**
- Opens LSP log viewer overlay
- Shows recent LSP output
- Scrollable with arrow keys/Page Up/Page Down
- Press `q` or `Esc` to close

**Log Viewer Layout:**
```
┌─────────────────────────────────────────────────────────────────┐
│ LSP Logs: rust-analyzer                              [q] Close  │
├─────────────────────────────────────────────────────────────────┤
│ 2024-01-22 10:30:45 INFO  Starting rust-analyzer...            │
│ 2024-01-22 10:30:45 DEBUG Spawning process: rust-analyzer      │
│ 2024-01-22 10:30:46 INFO  LSP initialized successfully         │
│ 2024-01-22 10:30:47 DEBUG Received notification:               │
│ 2024-01-22 10:30:47 DEBUG   textDocument/publishDiagnostics    │
│ 2024-01-22 10:30:50 INFO  Diagnostics published: 3 errors      │
│                                                                  │
│                                                                  │
│  ↓ Scroll for more logs...                                      │
└─────────────────────────────────────────────────────────────────┘
```

## Navigation

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `5` | Switch to LSPs tab |
| `↑` / `↓` | Navigate LSP list |
| `Enter` | Toggle Start/Stop |
| `R` | Restart LSP |
| `r` | Refresh status |
| `l` | View logs |
| `q` / `Esc` | Close log viewer / Return to dashboard |

### Tab Navigation

```
┌─────────────────────────────────────────────────────────────────┐
│ [1] Files  [2] Chunks  [3] Symbols  [4] Preview  [5] LSPs       │
└─────────────────────────────────────────────────────────────────┘
```

Press number keys to switch between tabs:
- `1` - Files tab
- `2` - Chunks tab
- `3` - Symbols tab
- `4` - Preview tab
- `5` - LSPs tab

## LSP Status Refresh

### Automatic Refresh

LSP status is NOT auto-refreshed periodically. This prevents excessive database queries.

**Manual refresh required:**
- After starting/stopping LSPs
- After restarting LSPs
- To check current LSP state

### Refresh Implementation

```rust
fn refresh_lsp_status_impl(&mut self, force: bool) -> bool {
    if !force {
        // Throttle refreshes (optional - currently not enforced)
        let last_refresh = self.last_lsp_status_refresh;
        if last_refresh.elapsed() < Duration::from_secs(1) {
            return false;
        }
    }

    self.last_lsp_status_refresh = Instant::now();
    self.pending_lsp_refresh = true;
    true
}
```

### Async Refresh in Event Loop

```rust
async fn do_refresh_lsp_status(&mut self) {
    let Some(storage) = self.storage_backend.clone() else {
        return;
    };

    let mut new_cache: HashMap<String, Vec<(String, LspStatus)>> = HashMap::new();

    for session in &self.sessions {
        let session_id = session.session_id.clone();

        match storage.get_session_lsp_states(&session_id).await {
            Ok(states) => {
                let lsp_statuses: Vec<(String, LspStatus)> = states
                    .into_iter()
                    .map(|state| (state.lsp_name, state.status))
                    .collect();
                new_cache.insert(session_id, lsp_statuses);
            }
            Err(e) => {
                self.status_message = format!("Failed to refresh LSP status: {}", e);
            }
        }
    }

    self.lsp_status_cache = new_cache;
    self.pending_lsp_refresh = false;
}
```

## LSP Installation Guidance

### rust-analyzer

**Installation:**
```bash
# Via rustup
rustup component add rust-analyzer

# Or download pre-built binary
# Visit: https://github.com/rust-lang/rust-analyzer#installing
```

**Verification:**
```bash
rust-analyzer --version
```

### ruff-lsp

**Installation:**
```bash
pip install ruff-lsp
```

**Verification:**
```bash
ruff-lsp --version
```

### typescript-language-server

**Installation:**
```bash
npm install -g typescript-language-server
```

**Verification:**
```bash
typescript-language-server --version
```

## Troubleshooting in TUI

### LSP Not Starting

**Symptoms:**
- Status shows "Error"
- Status shows "Stopped" after trying to start

**Diagnostic Steps:**
1. Press `l` to view LSP logs
2. Look for error messages like "Failed to spawn"
3. Check if LSP binary is installed

**Common Error Messages:**
```
Failed to spawn LSP 'rust-analyzer'. Is it installed and in PATH?
```

**Solution:**
Install the missing LSP (see installation guidance above)

### LSP Crashed

**Symptoms:**
- Status changes from "Running" to "Error"
- Session card shows red indicator

**Diagnostic Steps:**
1. Press `l` to view LSP logs
2. Look for crash messages or stack traces
3. Check system resources (memory, CPU)

**Solution:**
1. Press `R` to restart the LSP
2. If crash persists, file a bug report

### LSP Not Responding

**Symptoms:**
- Status stuck on "Starting"
- Operations timeout

**Diagnostic Steps:**
1. Check LSP logs for hang indicators
2. Verify LSP process is running (`ps aux | grep rust-analyzer`)
3. Check database connectivity

**Solution:**
1. Press `R` to restart the LSP
2. If problem persists, restart Maestro

### Old Status Displayed

**Symptoms:**
- LSP status doesn't reflect current state
- Recent start/stop not shown

**Solution:**
Press `r` to refresh LSP status from database

## Advanced Usage

### Multi-Session LSP Management

The LSPs tab shows all LSPs across all sessions:

```
┌─────────────────────────────────────────────────────────────────┐
│ LSPs                                    [r] Refresh             │
├─────────────────────────────────────────────────────────────────┤
│  ● rust-analyzer    Running   my-rust-project                   │
│  ● rust-analyzer    Running   another-rust-project               │
│  ■ ruff-lsp         Stopped   my-py-project                     │
└─────────────────────────────────────────────────────────────────┘
```

**Key points:**
- Same LSP can run for multiple sessions
- Each session has its own LSP instance
- Status is tracked per session

### Bulk Operations

**Starting all LSPs for a session:**
1. Go to Files tab
2. Select session
3. LSPs auto-start based on file types

**Stopping all LSPs for a session:**
1. Go to LSPs tab
2. Manually stop each LSP with `Enter`

## Status Color Reference

### Session Card LSP Indicators

```rust
let (icon, color) = match status {
    LspStatus::Running => (" ● ", Color::Green),
    LspStatus::Starting => (" ◐ ", Color::Yellow),
    LspStatus::Error => (" x ", Color::Red),
    LspStatus::Stopped => (" ○ ", Color::Gray),
};
```

### LSPs Tab Status Text

```rust
let (status_text, status_color, icon) = match status {
    LspStatus::Running => ("Running", Color::Green, "●"),
    LspStatus::Stopped => ("Stopped", Color::Red, "■"),
    LspStatus::Error => ("Error", Color::Red, "⚠"),
    LspStatus::Starting => ("Starting", Color::Yellow, "○"),
};
```

## Tips and Best Practices

1. **Refresh after changes:** Always press `r` after starting/stopping LSPs
2. **Check logs first:** When troubleshooting, view logs before restarting
3. **Use restart for errors:** Press `R` to restart LSPs in Error state
4. **Monitor resources:** LSPs can use significant memory for large projects
5. **Batch operations:** Start multiple LSPs before refreshing to save time

## See Also

- [LSP Integration](./lsp_integration.md) - LSP manager architecture
- [MCP Bridge Protocol](./mcp_bridge_protocol.md) - LSP to MCP translation
- [Troubleshooting](./lsp_troubleshooting.md) - Detailed troubleshooting guide
