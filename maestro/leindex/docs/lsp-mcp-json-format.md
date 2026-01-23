# LSP .mcp.json Format Design

## Overview

This document describes the JSON schema format for exposing Language Server Protocol (LSP) servers via `.mcp.json` configuration files. This format enables CLI tools (Claude Code, Gemini, etc.) to communicate directly with LSP processes via stdio for latency-sensitive operations like code completion, inlay hints, and go-to-definition.

## Schema Version

**Current Version:** 1.0

The root structure includes a `schemaVersion` field for future compatibility:

```json
{
  "schemaVersion": "1.0",
  "mcpServers": { ... },
  "lsp": { ... }
}
```

## Design Goals

1. **Backward Compatibility**: Existing `.mcp.json` files without LSP entries must continue to work
2. **Standard MCP Format**: Follow the existing Claude Code `.mcp.json` schema pattern
3. **Hybrid Exposure**: Support both direct stdio (for low-latency) and MCP bridge (for async operations)
4. **Graceful Degradation**: Tools should work normally when LSPs are unavailable

## Security Considerations

### Shell Injection Prevention

All paths and user-provided values in `.mcp.json` are properly shell-escaped when used in command strings. The implementation uses single-quote escaping with proper quote replacement to prevent shell injection attacks.

### Path Validation

- All project paths are validated and sanitized before use
- Symbolic link attacks are prevented using atomic file operations with O_EXCL
- Temporary files are created using `tempfile::NamedTempFile` for secure atomic writes

### LSP Process Isolation

LSP processes are spawned with:
- Explicit working directories
- Limited environment variables
- No inherited file descriptors beyond stdio
- Kill-on-drop semantics for proper cleanup

## Schema Definition

### Root Structure

```json
{
  "schemaVersion": "1.0",
  "mcpServers": {
    // Existing MCP servers (backward compatible)
    "maestro-tool-search": {
      "command": "maestro",
      "args": ["mcp", "tool-search"],
      "type": "stdio"
    }
  },
  "lsp": {
    // NEW: LSP server entries for direct stdio exposure
    "servers": [
      // LSP server definitions
    ]
  }
}
```

### Field Naming Convention

- **Rust code**: `snake_case` for struct field names
- **JSON output**: `camelCase` for JSON-RPC protocol compliance
- Example: `session_id` in Rust → `sessionId` in JSON

### LSP Server Entry Schema

Each LSP server entry in `lsp.servers` has the following structure:

```json
{
  "lsp": {
    "servers": [
      {
        "name": "rust-analyzer-session123",
        "language": "rust",
        "displayName": "rust-analyzer",
        "command": "rust-analyzer",
        "args": [],
        "type": "stdio",
        "session_id": "session123",
        "project_path": "/path/to/project",
        "capabilities": [
          "completion",
          "inlayHint",
          "definition",
          "references",
          "documentSymbol",
          "workspaceSymbol"
        ],
        "transport": "stdio",
        "stdio_proxy": {
          "socket_path": "/tmp/maestro-lsp-rust-session123.sock",
          "enabled": true
        }
      }
    ]
  }
}
```

### Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique identifier for this LSP instance (format: `{lsp_name}-{session_id}`) |
| `language` | string | Yes | Language identifier (rust, python, typescript) |
| `displayName` | string | Yes | Human-readable LSP name (rust-analyzer, ruff-lsp, typescript-language-server) |
| `command` | string | Yes | LSP binary to execute |
| `args` | array[string] | Yes | Arguments to pass to LSP binary |
| `type` | string | Yes | Transport type - always "stdio" for LSPs |
| `session_id` | string | Yes | Maestro session ID this LSP belongs to |
| `project_path` | string | Yes | Project root path for LSP workspace |
| `capabilities` | array[string] | Yes | LSP capabilities exposed (completion, inlayHint, definition, etc.) |
| `transport` | string | Yes | Communication mechanism - "stdio" (direct) or "stdio-proxy" (via socket) |
| `stdio_proxy` | object | No | Configuration for stdio proxy (if transport is "stdio-proxy") |
| `stdio_proxy.socket_path` | string | No | Unix socket path for proxy communication |
| `stdio_proxy.enabled` | boolean | No | Whether proxy is enabled |

### Transport Modes

#### 1. Direct stdio (`transport: "stdio"`)

CLI tools communicate directly with LSP via stdio:
- Pros: Lowest latency, simplest setup
- Cons: Single client per LSP instance

Example:
```json
{
  "name": "rust-analyzer-session123",
  "transport": "stdio",
  "command": "rust-analyzer",
  "args": []
}
```

#### 2. stdio-proxy (`transport: "stdio-proxy"`) - NOT YET IMPLEMENTED

**Implementation Status:** The stdio-proxy transport mode is currently **not implemented**.

The `stdio-proxy` mode would allow CLI tools to communicate via a Unix socket proxy:
- Pros: Multiple concurrent clients
- Cons: Slightly higher latency, requires proxy process

Example (future):
```json
{
  "name": "rust-analyzer-session123",
  "transport": "stdio-proxy",
  "stdio_proxy": {
    "socket_path": "/tmp/maestro-lsp-rust-session123.sock",
    "enabled": true
  }
}
```

**Note:** The proxy implementation exists in `src/lsp/stdio_proxy.rs` but is not integrated with the session manager. This is intentional - the proxy requires additional production testing and configuration options.

### Capability Flags

LSP capabilities indicate which operations are supported:

| Capability | Description |
|------------|-------------|
| `completion` | Code completion |
| `inlayHint` | Inline type hints |
| `definition` | Go-to-definition |
| `references` | Find references |
| `documentSymbol` | Document symbols (outline) |
| `workspaceSymbol` | Workspace-wide symbol search |
| `diagnostics` | Publish diagnostics (via MCP bridge only) |
| `hover` | Hover information |
| `rename` | Symbol renaming |
| `codeAction` | Code actions/refactoring |

## Example Configurations

### Rust Project (Single LSP)

```json
{
  "schemaVersion": "1.0",
  "mcpServers": {
    "maestro-tool-search": {
      "command": "maestro",
      "args": ["mcp", "tool-search"],
      "type": "stdio"
    }
  },
  "lsp": {
    "servers": [
      {
        "name": "rust-analyzer-session123",
        "language": "rust",
        "displayName": "rust-analyzer",
        "command": "rust-analyzer",
        "args": [],
        "type": "stdio",
        "session_id": "session123",
        "project_path": "/home/user/projects/my-rust-app",
        "capabilities": ["completion", "inlayHint", "definition", "hover"],
        "transport": "stdio"
      }
    ]
  }
}
```

### Multi-Language Project (Multiple LSPs)

```json
{
  "schemaVersion": "1.0",
  "mcpServers": {
    "maestro-tool-search": {
      "command": "maestro",
      "args": ["mcp", "tool-search"],
      "type": "stdio"
    }
  },
  "lsp": {
    "servers": [
      {
        "name": "rust-analyzer-session123",
        "language": "rust",
        "displayName": "rust-analyzer",
        "command": "rust-analyzer",
        "args": [],
        "type": "stdio",
        "session_id": "session123",
        "project_path": "/home/user/projects/polyglot-app",
        "capabilities": ["completion", "inlayHint", "definition", "hover"],
        "transport": "stdio"
      },
      {
        "name": "ruff-lsp-session123",
        "language": "python",
        "displayName": "ruff-lsp",
        "command": "ruff-lsp",
        "args": [],
        "type": "stdio",
        "session_id": "session123",
        "project_path": "/home/user/projects/polyglot-app",
        "capabilities": ["completion", "definition", "hover"],
        "transport": "stdio"
      },
      {
        "name": "typescript-language-server-session123",
        "language": "typescript",
        "displayName": "typescript-language-server",
        "command": "typescript-language-server",
        "args": ["--stdio"],
        "type": "stdio",
        "session_id": "session123",
        "project_path": "/home/user/projects/polyglot-app",
        "capabilities": ["completion", "definition", "references"],
        "transport": "stdio"
      }
    ]
  }
}
```

### With MCP Bridge Entries

```json
{
  "schemaVersion": "1.0",
  "mcpServers": {
    "maestro-tool-search": {
      "command": "maestro",
      "args": ["mcp", "tool-search"],
      "type": "stdio"
    },
    "maestro-lsp-rust-diagnostics": {
      "command": "maestro",
      "args": ["lsp-mcp-bridge", "--session-id", "session123", "--lsp", "rust-analyzer"],
      "type": "stdio",
      "description": "Rust diagnostics and symbols via MCP"
    }
  },
  "lsp": {
    "servers": [
      {
        "name": "rust-analyzer-session123",
        "language": "rust",
        "displayName": "rust-analyzer",
        "command": "rust-analyzer",
        "args": [],
        "type": "stdio",
        "session_id": "session123",
        "project_path": "/home/user/projects/my-rust-app",
        "capabilities": ["completion", "inlayHint", "definition"],
        "transport": "stdio"
      }
    ]
  }
}
```

## Integration with Maestro TUI

### File Location

`.mcp.json` files are written to the temp directory with session-specific naming:
```
/tmp/maestro-mcp-config-{session_id}.json
```

### Generation Flow

1. User creates session in Maestro TUI
2. Language detection determines applicable LSPs
3. LSP manager starts LSP processes
4. `.mcp.json` is generated with:
   - Existing `mcpServers` entries (tool-search, etc.)
   - New `lsp.servers` entries for each running LSP
5. CLI tool is launched with `--mcp-config /tmp/maestro-mcp-config-{session_id}.json`

### Lifecycle

- **Created**: When session starts and LSPs are launched
- **Updated**: When LSPs are manually started/stopped
- **Deleted**: When session is killed (file cleaned up)

## Backward Compatibility

### Tools Without LSP Support

Tools that don't recognize the `lsp` section will:
- Ignore the unknown `lsp` key
- Continue to use `mcpServers` entries normally
- Experience no behavior change

### Existing Configurations

Existing `.mcp.json` files without `lsp` section:
- Parse successfully
- Work identically to before
- No migration required

## Tool Integration Guide

### For Tool Authors

To consume LSP entries from `.mcp.json`:

1. Parse the JSON file
2. Check for `lsp.servers` array
3. For each LSP server:
   - Spawn the LSP process using `command` + `args`
   - Communicate via JSON-RPC over stdio
   - Handle LSP protocol: initialize, shutdown, exit
4. Use `capabilities` array to determine available operations

### Example Pseudocode

```python
import json
import subprocess

def load_lsp_servers(config_path):
    with open(config_path) as f:
        config = json.load(f)

    lsp_section = config.get("lsp", {})
    return lsp_section.get("servers", [])

def start_lsp(lsp_config):
    proc = subprocess.Popen(
        [lsp_config["command"]] + lsp_config["args"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        cwd=lsp_config["project_path"]
    )
    return proc

# Usage
config_path = "/tmp/maestro-mcp-config-session123.json"
lsp_servers = load_lsp_servers(config_path)

for lsp in lsp_servers:
    if lsp["language"] == "rust":
        process = start_lsp(lsp)
        # Use LSP JSON-RPC protocol...
```

## Validation Schema (JSON Schema)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "mcpServers": {
      "type": "object",
      "description": "MCP server configurations (existing format)"
    },
    "lsp": {
      "type": "object",
      "properties": {
        "servers": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "language", "displayName", "command", "type", "session_id", "project_path", "capabilities", "transport"],
            "properties": {
              "name": { "type": "string" },
              "language": { "type": "string", "enum": ["rust", "python", "typescript"] },
              "displayName": { "type": "string" },
              "command": { "type": "string" },
              "args": { "type": "array", "items": { "type": "string" } },
              "type": { "type": "string", "enum": ["stdio"] },
              "session_id": { "type": "string" },
              "project_path": { "type": "string" },
              "capabilities": {
                "type": "array",
                "items": { "type": "string" }
              },
              "transport": { "type": "string", "enum": ["stdio", "stdio-proxy"] },
              "stdio_proxy": {
                "type": "object",
                "properties": {
                  "socket_path": { "type": "string" },
                  "enabled": { "type": "boolean" }
                }
              }
            }
          }
        }
      }
    }
  }
}
```

## Future Extensions

### Potential Enhancements

1. **Workspace Folders**: Support multi-root workspaces
2. **LSP Settings**: Per-LSP initialization options
3. **Client Capabilities**: Advertise tool capabilities to LSP
4. **Dynamic Config**: Watch file for changes and reload LSPs

### Compatibility Notes

- Claude Code currently does not support direct LSP stdio consumption
- This format is designed for future adoption by CLI tools
- Tools can opt-in to LSP support incrementally

## References

- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [Claude Code MCP Documentation](https://docs.anthropic.com/claude/docs/mcp)
- [Maestro LSP Integration Spec](../tracks/lsp-integration_20260119/spec.md)
