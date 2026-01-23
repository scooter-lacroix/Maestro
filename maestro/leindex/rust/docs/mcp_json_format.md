# .mcp.json Format Documentation

## Overview

`.mcp.json` is the configuration file for MCP (Model Context Protocol) servers. It defines which MCP servers are available to Claude Code and how to connect to them.

## Configuration Location

The primary `.mcp.json` file is located at:

```
~/.claude/.mcp.json
```

This file is created and managed by Maestro's setup process.

## JSON Schema

```json
{
  "mcpServers": {
    "server-name": {
      "command": "path-to-binary",
      "args": ["arg1", "arg2"],
      "env": {
        "ENV_VAR": "value"
      }
    }
  }
}
```

## Server Types

### 1. Direct stdio Mode

Direct stdio mode spawns the LSP process and communicates via standard input/output.

**Schema:**
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

**Fields:**
- `command` (required): Absolute path to the binary
- `args` (optional): Array of command-line arguments
- `env` (optional): Environment variables as key-value pairs

### 2. stdio-proxy Mode

stdio-proxy mode uses a Unix domain socket for communication, enabling multiple clients to connect to the same LSP process.

**Schema:**
```json
{
  "mcpServers": {
    "leindex-proxy": {
      "command": "/home/user/.cargo/bin/leindex",
      "args": [
        "mcp",
        "stdio-proxy",
        "--socket-path",
        "/tmp/leindex-mcp.sock"
      ],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Additional Fields for stdio-proxy:**
- `socket-path` (required): Path to the Unix domain socket

## LSP Server Entries

LSP servers are registered as MCP servers with specific capabilities:

```json
{
  "mcpServers": {
    "rust-analyzer": {
      "command": "rust-analyzer",
      "args": [],
      "env": {},
      "capabilities": {
        "documentSymbol": true,
        "workspaceSymbol": true,
        "definition": true,
        "references": true,
        "diagnostics": true
      }
    }
  }
}
```

**Capabilities:**
- `documentSymbol`: Get symbols in a document
- `workspaceSymbol`: Search symbols across workspace
- `definition`: Go to definition
- `references`: Find references
- `diagnostics`: Get diagnostics

## Complete Example

Here's a complete `.mcp.json` example with multiple servers:

```json
{
  "mcpServers": {
    "leindex": {
      "command": "/home/user/.cargo/bin/leindex",
      "args": ["mcp", "stdio"],
      "env": {
        "RUST_LOG": "info",
        "LEINDEX_DB_PATH": "/home/user/.maestro/maestro_turso.db"
      }
    },
    "leindex-proxy": {
      "command": "/home/user/.cargo/bin/leindex",
      "args": [
        "mcp",
        "stdio-proxy",
        "--socket-path",
        "/tmp/leindex-mcp.sock"
      ],
      "env": {
        "RUST_LOG": "debug"
      }
    },
    "rust-analyzer": {
      "command": "rust-analyzer",
      "args": [],
      "env": {},
      "capabilities": {
        "documentSymbol": true,
        "workspaceSymbol": true,
        "definition": true,
        "references": true,
        "diagnostics": true
      }
    },
    "ruff-lsp": {
      "command": "ruff-lsp",
      "args": [],
      "env": {},
      "capabilities": {
        "documentSymbol": true,
        "definition": true,
        "diagnostics": true
      }
    },
    "typescript-language-server": {
      "command": "typescript-language-server",
      "args": ["--stdio"],
      "env": {},
      "capabilities": {
        "documentSymbol": true,
        "workspaceSymbol": true,
        "definition": true,
        "references": true,
        "diagnostics": true
      }
    }
  }
}
```

## Configuration by Tool

### Maestro CLI

Maestro automatically configures `.mcp.json` during setup:

```rust
// From maestro/leindex/rust/src/setup/mod.rs
let mcp_path = home_dir()?.join(".claude").join(".mcp.json");
let mut mcp_logs = upsert_json_server(
    &mcp_path,
    "mcpServers",
    "leindex",
    &binary_path,
    &["mcp", "stdio"],
    &[("RUST_LOG", "info")],
)?;
```

### Factory CLI

Factory also configures `.mcp.json` for Droid integration:

```rust
// From maestro/leindex/rust/src/setup/mod.rs
let cfg_path = home_dir()?.join(".factory").join("mcp.json");
let mut cfg_logs = upsert_json_server(
    &cfg_path,
    "mcpServers",
    "leindex",
    &binary_path,
    &["mcp", "stdio"],
    &[("RUST_LOG", "info")],
)?;
```

## Environment Variables

Common environment variables for MCP servers:

| Variable | Description | Example |
|----------|-------------|---------|
| `RUST_LOG` | Rust log level | `info`, `debug`, `warn`, `error` |
| `LEINDEX_DB_PATH` | Path to Turso database | `/home/user/.maestro/maestro_turso.db` |
| `LEINDEX_SOCKET` | Socket path for stdio-proxy | `/tmp/leindex-mcp.sock` |

## stdio-proxy vs Direct stdio

### Direct stdio

**Pros:**
- Simpler setup
- Direct communication
- No socket file management

**Cons:**
- One LSP process per client
- Higher memory usage with multiple clients
- Each client needs separate LSP instance

**Use when:**
- Single Claude Code instance
- Simpler configuration needed
- Debugging LSP issues

### stdio-proxy

**Pros:**
- Single LSP process for multiple clients
- Lower memory usage
- Shared LSP state

**Cons:**
- Requires socket file management
- More complex setup
- Socket cleanup needed on shutdown

**Use when:**
- Multiple Claude Code instances
- Resource-constrained environment
- Shared LSP state desired

## Socket Path Best Practices

When using stdio-proxy, follow these practices for socket paths:

1. **Use /tmp for sockets:**
   ```json
   "socket-path": "/tmp/leindex-mcp.sock"
   ```

2. **Include unique identifier for multiple instances:**
   ```json
   "socket-path": "/tmp/leindex-mcp-{session-id}.sock"
   ```

3. **Clean up sockets on shutdown:**
   ```rust
   std::fs::remove_file(&socket_path)?;
   ```

## Common Configuration Issues

### Issue: Binary Not Found

**Error:** `Failed to start MCP server: No such file or directory`

**Solution:** Use absolute path to binary:
```json
{
  "command": "/home/user/.cargo/bin/leindex"
}
```

### Issue: Socket Already Exists

**Error:** `Address already in use`

**Solution:**
1. Remove old socket file:
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

### Issue: Permissions Denied

**Error:** `Permission denied`

**Solution:** Check file permissions:
```bash
chmod +x /home/user/.cargo/bin/leindex
ls -la /home/user/.cargo/bin/leindex
```

### Issue: Wrong Arguments

**Error:** `Unrecognized argument: --stdio`

**Solution:** Check LSP-specific arguments:
```json
{
  "command": "typescript-language-server",
  "args": ["--stdio"]  // Required for typescript-language-server
}
```

## Validation

Validate `.mcp.json` syntax:

```bash
# Using jq
cat ~/.claude/.mcp.json | jq .

# Using Python
python3 -m json.tool ~/.claude/.mcp.json
```

## Debugging

Enable debug logging to troubleshoot issues:

```json
{
  "env": {
    "RUST_LOG": "debug"
  }
}
```

Check MCP server status:

```bash
# List running MCP servers
ps aux | grep -E "(leindex|rust-analyzer|ruff-lsp|typescript-language-server)"

# Check socket files
ls -la /tmp/*.sock
```

## Migration from Older Formats

If you have an older MCP configuration format, update to the new schema:

**Old format (if applicable):**
```json
{
  "servers": {
    "leindex": "/home/user/.cargo/bin/leindex"
  }
}
```

**New format:**
```json
{
  "mcpServers": {
    "leindex": {
      "command": "/home/user/.cargo/bin/leindex",
      "args": ["mcp", "stdio"],
      "env": {}
    }
  }
}
```

## See Also

- [LSP Integration](./lsp_integration.md) - LSP manager and lifecycle
- [MCP Bridge Protocol](./mcp_bridge_protocol.md) - LSP to MCP translation
- [Troubleshooting](./lsp_troubleshooting.md) - Common issues and solutions
