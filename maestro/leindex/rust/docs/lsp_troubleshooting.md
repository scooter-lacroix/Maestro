# LSP Troubleshooting Guide

## Overview

This guide covers common issues with LSP integration in Maestro, including:

- LSP startup failures
- Installation problems
- Configuration issues
- Database migration problems
- Mode switch data loss issues
- Debugging procedures

## Common LSP Startup Issues

### Issue: LSP Binary Not Found

**Error Message:**
```
Failed to spawn LSP 'rust-analyzer'. Is it installed and in PATH?
```

**Cause:** LSP binary is not installed or not in PATH

**Solution:**

1. Check if LSP is installed:
```bash
which rust-analyzer
which ruff-lsp
which typescript-language-server
```

2. If not found, install the LSP:

**rust-analyzer:**
```bash
# Via rustup
rustup component add rust-analyzer

# Or download pre-built binary
# Visit: https://github.com/rust-lang/rust-analyzer#installing
```

**ruff-lsp:**
```bash
pip install ruff-lsp
```

**typescript-language-server:**
```bash
npm install -g typescript-language-server
```

3. Verify installation:
```bash
rust-analyzer --version
ruff-lsp --version
typescript-language-server --version
```

### Issue: LSP Fails to Initialize

**Error Message:**
```
Failed to initialize LSP: timeout waiting for response
```

**Cause:** LSP process is not responding to initialize request

**Solution:**

1. Check LSP logs:
```bash
# In TUI, press 'l' on the LSP to view logs
# Look for initialization errors
```

2. Check LSP configuration:
```bash
# Verify .mcp.json has correct arguments
cat ~/.claude/.mcp.json
```

3. Test LSP manually:
```bash
# Test rust-analyzer
echo '{"jsonrpc":"2.0","id":"1","method":"initialize","params":{}}' | rust-analyzer

# Test ruff-lsp
echo '{"jsonrpc":"2.0","id":"1","method":"initialize","params":{}}' | ruff-lsp
```

4. Check system resources:
```bash
# Check available memory
free -h

# Check CPU usage
top -bn1 | head -20
```

### Issue: LSP Crashes Immediately

**Error Message:**
```
LSP 'rust-analyzer' exited unexpectedly with status: 1
```

**Cause:** LSP process crashed during startup

**Solution:**

1. Check LSP logs for crash details:
```bash
# In TUI, press 'l' on the LSP
# Look for stack traces or error messages
```

2. Test LSP in isolation:
```bash
# Run LSP directly to see output
rust-analyzer
```

3. Check for missing dependencies:
```bash
# rust-analyzer needs rust toolchain
rustc --version

# typescript-language-server needs TypeScript
npm list -g typescript
```

4. Check file permissions:
```bash
# Ensure binary is executable
ls -la $(which rust-analyzer)
chmod +x $(which rust-analyzer)
```

## Installation Problems

### Issue: rust-analyzer Installation Fails

**Symptoms:**
- `rustup component add rust-analyzer` fails
- Binary not found after installation

**Solution:**

1. Update rustup:
```bash
rustup update
```

2. Try alternative installation:
```bash
# Download pre-built binary
wget https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu.gz
gunzip rust-analyzer-x86_64-unknown-linux-gnu.gz
chmod +x rust-analyzer-x86_64-unknown-linux-gnu
mv rust-analyzer-x86_64-unknown-linux-gnu ~/.cargo/bin/rust-analyzer
```

3. Verify installation:
```bash
rust-analyzer --version
```

### Issue: ruff-lsp Installation Fails

**Symptoms:**
- `pip install ruff-lsp` fails
- ImportError when running ruff-lsp

**Solution:**

1. Ensure Python is installed:
```bash
python3 --version
pip3 --version
```

2. Use pip3 explicitly:
```bash
pip3 install ruff-lsp
```

3. Or use python -m pip:
```bash
python3 -m pip install ruff-lsp
```

4. Check for conflicts:
```bash
pip3 list | grep ruff
```

### Issue: typescript-language-server Installation Fails

**Symptoms:**
- `npm install -g typescript-language-server` fails
- Command not found after installation

**Solution:**

1. Ensure Node.js and npm are installed:
```bash
node --version
npm --version
```

2. Check npm global path:
```bash
npm config get prefix
# Add $(npm config get prefix)/bin to PATH if needed
```

3. Try with sudo (if necessary):
```bash
sudo npm install -g typescript-language-server
```

4. Verify installation:
```bash
typescript-language-server --version
```

## .mcp.json Configuration Issues

### Issue: Invalid JSON Syntax

**Error Message:**
```
Failed to parse .mcp.json: syntax error
```

**Cause:** JSON syntax error in .mcp.json

**Solution:**

1. Validate JSON syntax:
```bash
# Using jq
cat ~/.claude/.mcp.json | jq .

# Using Python
python3 -m json.tool ~/.claude/.mcp.json
```

2. Fix common errors:
- Missing commas
- Trailing commas
- Unquoted keys
- Missing braces

3. Example valid .mcp.json:
```json
{
  "mcpServers": {
    "leindex": {
      "command": "/home/user/.cargo/bin/leindex",
      "args": ["mcp", "stdio"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Issue: Wrong Binary Path

**Error Message:**
```
Failed to spawn LSP: No such file or directory
```

**Cause:** Binary path in .mcp.json is incorrect

**Solution:**

1. Find correct binary path:
```bash
which rust-analyzer
which ruff-lsp
which typescript-language-server
```

2. Update .mcp.json with absolute paths:
```json
{
  "mcpServers": {
    "rust-analyzer": {
      "command": "/home/user/.cargo/bin/rust-analyzer",
      "args": []
    }
  }
}
```

3. Avoid relative paths:
```json
// BAD
{
  "command": "~/.cargo/bin/rust-analyzer"
}

// GOOD
{
  "command": "/home/user/.cargo/bin/rust-analyzer"
}
```

### Issue: Missing stdio Flag

**Symptoms:**
- typescript-language-server tries to connect via TCP
- Connection refused errors

**Cause:** typescript-language-server defaults to TCP mode

**Solution:**

Add `--stdio` flag to arguments:
```json
{
  "mcpServers": {
    "typescript-language-server": {
      "command": "typescript-language-server",
      "args": ["--stdio"]
    }
  }
}
```

### Issue: Socket Already Exists

**Error Message:**
```
Address already in use: /tmp/leindex-mcp.sock
```

**Cause:** Previous socket file not cleaned up

**Solution:**

1. Remove old socket:
```bash
rm /tmp/leindex-mcp.sock
```

2. Or use unique socket path per session:
```json
{
  "args": [
    "mcp",
    "stdio-proxy",
    "--socket-path",
    "/tmp/leindex-mcp-{uuid}.sock"
  ]
}
```

3. Check for running processes:
```bash
ps aux | grep leindex
ps aux | grep rust-analyzer
```

## Database Migration Problems

### Issue: Schema Migration Fails

**Error Message:**
```
Failed to migrate database schema from TEXT to INTEGER
```

**Cause:** Old database schema incompatible with new code

**Solution:**

1. Check database version:
```bash
# Open database
sqlite3 ~/.maestro/maestro_turso.db

# Check schema
.schema lsp_servers
```

2. Manual migration:
```sql
-- Backup old data
CREATE TABLE lsp_servers_backup AS SELECT * FROM lsp_servers;

-- Drop old table
DROP TABLE lsp_servers;

-- Create new table with correct schema
CREATE TABLE lsp_servers (
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

-- Restore data
INSERT INTO lsp_servers SELECT * FROM lsp_servers_backup;
```

3. Restart Maestro to run migrations

### Issue: Database Locked

**Error Message:**
```
Database is locked: database is locked
```

**Cause:** Another process has the database open

**Solution:**

1. Find processes using database:
```bash
lsof ~/.maestro/maestro_turso.db
```

2. Kill conflicting processes:
```bash
kill <pid>
```

3. Or close other Maestro instances

### Issue: Corrupted Database

**Error Message:**
```
Database disk image is malformed
```

**Cause:** Database file corrupted

**Solution:**

1. Backup existing database:
```bash
cp ~/.maestro/maestro_turso.db ~/.maestro/maestro_turso.db.backup
```

2. Try to repair:
```bash
sqlite3 ~/.maestro/maestro_turso.db "PRAGMA integrity_check;"
```

3. If repair fails, start fresh:
```bash
rm ~/.maestro/maestro_turso.db
# Maestro will recreate on next start
```

## Mode Switch Data Loss Issues

### Issue: Data Loss During Mode Switch

**Symptoms:**
- Vectors missing after Linear → HNSW switch
- Inconsistent vector counts

**Cause:** Turso was not the authoritative source during migration

**Solution:**

1. **This should not happen** with current implementation
2. Current implementation always migrates from Turso:
```rust
// Load from Turso (authoritative source)
if let Some(ref turso) = self.turso {
    let all_vectors = turso.get_all_vectors().await?;
    for (content, embedding, metadata) in all_vectors {
        new_hnsw_store.add_vector(&content, embedding, metadata)?;
    }
}
```

3. If issue persists, check:
- Turso vector count matches expected
- No errors during migration
- Disk space available

### Issue: Mode Switch Timeout

**Error Message:**
```
Mode switch timeout: operation took too long
```

**Cause:** Too many vectors to migrate within timeout

**Solution:**

1. Check vector count:
```bash
sqlite3 ~/.maestro/maestro_turso.db "SELECT COUNT(*) FROM vectors;"
```

2. Increase timeout if needed:
```rust
// In adaptive.rs, increase timeout
let timeout_duration = Duration::from_secs(300); // 5 minutes
```

3. Consider pre-migrating:
- Add all vectors before mode switch threshold
- Let system migrate in background

## Debugging Procedures

### Enable Debug Logging

**Set environment variable:**
```bash
export RUST_LOG=debug
maestro tui
```

**Or in .mcp.json:**
```json
{
  "mcpServers": {
    "leindex": {
      "command": "/home/user/.cargo/bin/leindex",
      "args": ["mcp", "stdio"],
      "env": {
        "RUST_LOG": "debug"
      }
    }
  }
}
```

### View LSP Logs

**In TUI:**
1. Go to LSPs tab (press `5`)
2. Select LSP
3. Press `l` to view logs

**Or from command line:**
```bash
# Check LSP output
journalctl -f | grep rust-analyzer
```

### Check Database State

**Query LSP states:**
```bash
sqlite3 ~/.maestro/maestro_turso.db

SELECT * FROM lsp_servers;
```

**Check vector counts:**
```bash
sqlite3 ~/.maestro/maestro_turso.db

SELECT COUNT(*) FROM vectors;
```

**Check session LSPs:**
```bash
sqlite3 ~/.maestro/maestro_turso.db

SELECT session_id, lsp_name, status, pid
FROM lsp_servers
ORDER BY session_id, lsp_name;
```

### Monitor Process Status

**Check running LSPs:**
```bash
ps aux | grep -E "(rust-analyzer|ruff-lsp|typescript-language-server)"
```

**Check process tree:**
```bash
pstree -p $$ | grep rust-analyzer
```

**Check resource usage:**
```bash
top -p $(pidof rust-analyzer)
```

### Test LSP Manually

**Test rust-analyzer:**
```bash
# Start LSP
rust-analyzer

# Send initialize request
echo '{"jsonrpc":"2.0","id":"1","method":"initialize","params":{"processId":1,"rootUri":"file:///tmp/test","capabilities":{}}}' | rust-analyzer
```

**Test ruff-lsp:**
```bash
echo '{"jsonrpc":"2.0","id":"1","method":"initialize","params":{"processId":1,"rootUri":"file:///tmp/test","capabilities":{}}}' | ruff-lsp
```

## Performance Issues

### Issue: Slow LSP Startup

**Symptoms:**
- LSP takes > 10 seconds to start
- Status stuck on "Starting"

**Solution:**

1. Check project size:
```bash
# Count files in project
find . -name "*.rs" | wc -l
```

2. Exclude unnecessary directories:
```json
// In project config
{
  "rust-analyzer": {
    "cargo": {
      "loadOutDirsFromCheck": true
    },
    "procMacro": {
      "enable": false
    }
  }
}
```

3. Increase system resources:
- Add more RAM
- Use faster SSD

### Issue: High Memory Usage

**Symptoms:**
- LSP process using > 2GB RAM
- System slows down

**Solution:**

1. Check memory usage:
```bash
ps aux | grep rust-analyzer | awk '{print $6}'
```

2. Restart LSP:
```bash
# In TUI, press 'R' on the LSP
```

3. Adjust LSP limits:
```json
{
  "rust-analyzer": {
    "maxInlayHintLength": 20
  }
}
```

4. Consider using stdio-proxy for shared LSP instances

## Network Issues (stdio-proxy)

### Issue: Socket Connection Refused

**Error Message:**
```
Failed to connect to socket: Connection refused
```

**Cause:** stdio-proxy server not running

**Solution:**

1. Check if socket exists:
```bash
ls -la /tmp/leindex-mcp.sock
```

2. Start stdio-proxy server:
```bash
leindex mcp stdio-proxy --socket-path /tmp/leindex-mcp.sock
```

3. Check socket permissions:
```bash
# Socket should be readable/writable
chmod 666 /tmp/leindex-mcp.sock
```

### Issue: Socket Hangs

**Symptoms:**
- Operations timeout when using socket
- No response from LSP

**Solution:**

1. Restart stdio-proxy server:
```bash
# Kill existing process
pkill -f "leindex mcp stdio-proxy"

# Start new server
leindex mcp stdio-proxy --socket-path /tmp/leindex-mcp.sock
```

2. Check socket backlog:
```bash
netstat -an | grep /tmp/leindex-mcp.sock
```

## Getting Help

If you continue to experience issues:

1. **Collect diagnostic information:**
```bash
# Maestro version
maestro --version

# Rust version
rustc --version

# LSP versions
rust-analyzer --version
ruff-lsp --version
typescript-language-server --version

# Database state
sqlite3 ~/.maestro/maestro_turso.db ".schema lsp_servers"

# Running processes
ps aux | grep -E "(maestro|rust-analyzer|ruff-lsp)"
```

2. **Check logs:**
```bash
# Maestro logs
journalctl -u maestro -n 100

# LSP logs (in TUI, press 'l' on LSP)
```

3. **File a bug report:**
- Include diagnostic information
- Describe expected vs actual behavior
- Include steps to reproduce

## See Also

- [LSP Integration](./lsp_integration.md) - LSP manager architecture
- [MCP Bridge Protocol](./mcp_bridge_protocol.md) - LSP to MCP translation
- [TUI User Guide](./lsp_tui_user_guide.md) - Using LSPs in the terminal UI
