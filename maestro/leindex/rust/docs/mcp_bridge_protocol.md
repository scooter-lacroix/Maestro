# MCP Bridge Protocol Documentation

## Overview

The MCP (Model Context Protocol) Bridge translates between Language Server Protocol (LSP) and MCP, enabling Claude Code to interact with LSP servers through standardized MCP tools and events.

## Architecture

```
┌─────────────┐     JSON-RPC      ┌─────────────┐     JSON-RPC      ┌─────────────┐
│   MCP       │  ─────────────▶   │   Bridge    │  ─────────────▶   │   LSP       │
│   Client    │                    │             │                    │   Server    │
│             │  ◀─────────────    │             │  ◀─────────────    │             │
└─────────────┘                    └─────────────┘                    └─────────────┘
                                           │
                                           ▼
                                    ┌─────────────┐
                                    │ Diagnostics │
                                    │   Cache     │
                                    └─────────────┘
```

## Protocol Translation

### LSP → MCP Tools

The bridge exposes LSP capabilities as MCP tools:

| MCP Tool | LSP Method | Description |
|----------|-----------|-------------|
| `lsp/document_symbols` | `textDocument/documentSymbol` | Get symbols (functions, classes) in a document |
| `lsp/workspace_symbols` | `workspace/symbol` | Search for symbols across the workspace |
| `lsp/definition` | `textDocument/definition` | Go to definition of a symbol |
| `lsp/references` | `textDocument/references` | Find all references to a symbol |
| `lsp/diagnostics` | Cached diagnostics | Get current diagnostics for a document |

### LSP → MCP Events

The bridge converts LSP notifications to MCP events:

| MCP Event | LSP Notification | Description |
|-----------|------------------|-------------|
| `diagnostics/published` | `textDocument/publishDiagnostics` | New diagnostics available |
| `lsp/log_message` | `window/logMessage` | LSP log message |

## JSON-RPC Message Format

### LSP Message Framing

LSP uses Content-Length framing with `\r\n\r\n` headers:

```
Content-Length: 1234\r\n
\r\n
{"jsonrpc":"2.0","id":"1","method":"...","params":{...}}
```

### Request Format

```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "method": "textDocument/documentSymbol",
  "params": {
    "textDocument": {
      "uri": "file:///path/to/file.rs"
    }
  }
}
```

### Response Format

```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "result": [
    {
      "name": "function_name",
      "kind": 12,
      "range": {
        "start": {"line": 0, "character": 0},
        "end": {"line": 10, "character": 0}
      },
      "children": []
    }
  ]
}
```

### Notification Format

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/publishDiagnostics",
  "params": {
    "uri": "file:///path/to/file.rs",
    "diagnostics": [
      {
        "range": {
          "start": {"line": 5, "character": 10},
          "end": {"line": 5, "character": 20}
        },
        "severity": 1,
        "code": "E0001",
        "source": "rust-analyzer",
        "message": "error message here"
      }
    ]
  }
}
```

## MCP Tool Definitions

### 1. document_symbols

Get symbols (functions, classes, etc.) in a document.

**Tool Name:** `lsp/document_symbols`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "uri": {
      "type": "string",
      "description": "Document URI (e.g., file:///path/to/file.rs)"
    }
  },
  "required": ["uri"]
}
```

**Example Request:**
```json
{
  "uri": "file:///home/user/project/src/main.rs"
}
```

**Example Response:**
```json
{
  "result": [
    {
      "name": "main",
      "kind": 12,
      "detail": "fn() -> Result<()>",
      "range": {
        "start": {"line": 0, "character": 0},
        "end": {"line": 10, "character": 1}
      },
      "selectionRange": {
        "start": {"line": 0, "character": 3},
        "end": {"line": 0, "character": 7}
      },
      "children": []
    }
  ]
}
```

### 2. workspace_symbols

Search for symbols across the entire workspace.

**Tool Name:** `lsp/workspace_symbols`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query string"
    }
  },
  "required": ["query"]
}
```

**Example Request:**
```json
{
  "query": "parse"
}
```

**Example Response:**
```json
{
  "result": [
    {
      "name": "parse_config",
      "kind": 12,
      "location": {
        "uri": "file:///home/user/project/src/config.rs",
        "range": {
          "start": {"line": 15, "character": 0},
          "end": {"line": 30, "character": 1}
        }
      },
      "containerName": "Config"
    }
  ]
}
```

### 3. definition

Go to definition of a symbol.

**Tool Name:** `lsp/definition`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "uri": {
      "type": "string",
      "description": "Document URI"
    },
    "line": {
      "type": "integer",
      "description": "Line number (0-based)"
    },
    "character": {
      "type": "integer",
      "description": "Character position (0-based)"
    }
  },
  "required": ["uri", "line", "character"]
}
```

**Example Request:**
```json
{
  "uri": "file:///home/user/project/src/main.rs",
  "line": 5,
  "character": 10
}
```

**Example Response:**
```json
{
  "result": [
    {
      "uri": "file:///home/user/project/src/lib.rs",
      "range": {
        "start": {"line": 10, "character": 0},
        "end": {"line": 15, "character": 1}
      }
    }
  ]
}
```

### 4. references

Find all references to a symbol.

**Tool Name:** `lsp/references`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "uri": {
      "type": "string",
      "description": "Document URI"
    },
    "line": {
      "type": "integer",
      "description": "Line number (0-based)"
    },
    "character": {
      "type": "integer",
      "description": "Character position (0-based)"
    }
  },
  "required": ["uri", "line", "character"]
}
```

**Example Request:**
```json
{
  "uri": "file:///home/user/project/src/main.rs",
  "line": 5,
  "character": 10
}
```

**Example Response:**
```json
{
  "result": [
    {
      "uri": "file:///home/user/project/src/main.rs",
      "range": {
        "start": {"line": 5, "character": 10},
        "end": {"line": 5, "character": 20}
      }
    },
    {
      "uri": "file:///home/user/project/src/test.rs",
      "range": {
        "start": {"line": 12, "character": 5},
        "end": {"line": 12, "character": 15}
      }
    }
  ]
}
```

### 5. diagnostics

Get current diagnostics for a document (from cache).

**Tool Name:** `lsp/diagnostics`

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "uri": {
      "type": "string",
      "description": "Document URI"
    }
  },
  "required": ["uri"]
}
```

**Example Request:**
```json
{
  "uri": "file:///home/user/project/src/main.rs"
}
```

**Example Response:**
```json
{
  "uri": "file:///home/user/project/src/main.rs",
  "diagnostics": [
    {
      "range": {
        "start": {"line": 5, "character": 10},
        "end": {"line": 5, "character": 20}
      },
      "severity": 1,
      "code": "E0001",
      "source": "rust-analyzer",
      "message": "expected type, found `i32`"
    }
  ]
}
```

## MCP Events

### diagnostics/published

Emitted when LSP publishes new diagnostics.

**Event Data:**
```json
{
  "name": "diagnostics/published",
  "data": {
    "uri": "file:///home/user/project/src/main.rs",
    "diagnostics": [
      {
        "range": {
          "start": {"line": 5, "character": 10},
          "end": {"line": 5, "character": 20}
        },
        "severity": 1,
        "code": "E0001",
        "source": "rust-analyzer",
        "message": "error message here"
      }
    ]
  }
}
```

### lsp/log_message

Emitted when LSP sends a log message.

**Event Data:**
```json
{
  "name": "lsp/log_message",
  "data": {
    "type": 3,
    "message": "LSP log message here"
  }
}
```

## Connection Handling

### LSP Initialization Sequence

1. **Spawn LSP Process**
   ```rust
   let (mut stdin, mut stdout, child) = bridge.start_lsp_process().await?;
   ```

2. **Send Initialize Request**
   ```json
   {
     "jsonrpc": "2.0",
     "id": "1",
     "method": "initialize",
     "params": {
       "processId": 12345,
       "rootUri": "file:///home/user/project",
       "capabilities": {
         "workspace": {
           "symbol": true
         },
         "textDocument": {
           "definition": true,
           "references": true,
           "documentSymbol": true,
           "publishDiagnostics": true
         }
       },
       "clientInfo": {
         "name": "maestro-lsp-mcp-bridge",
         "version": "2.0.0"
       }
     }
   }
   ```

3. **Send Initialized Notification**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "initialized",
     "params": {}
   }
   ```

### Tool Call Handling

When an MCP tool is called:

1. Bridge translates tool call to LSP request
2. Sends request to LSP via stdin
3. Reads response from stdout (with timeout)
4. Translates response back to MCP format
5. Returns result to MCP client

### Background LSP Reader

A background task continuously reads LSP notifications:

```rust
pub async fn run_lsp_reader(
    self,
    mut stdout: ChildStdout,
    event_tx: mpsc::Sender<McpEvent>,
) -> Result<()> {
    loop {
        match self.read_next_lsp_message(&mut stdout).await {
            Ok(message) => {
                if message.get("id").is_none() {
                    // This is a notification
                    if let Some(event) = self.handle_lsp_notification(method, params) {
                        event_tx.send(event).await?;
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from LSP: {}", e);
                break;
            }
        }
    }
    Ok(())
}
```

## Diagnostics Cache

The bridge maintains an in-memory cache of diagnostics:

```rust
diagnostics_cache: Arc<RwLock<HashMap<String, Vec<Diagnostic>>>>
```

- **Key:** Document URI (String)
- **Value:** List of diagnostics for that document
- **Updated:** When `textDocument/publishDiagnostics` received
- **Used by:** `lsp/diagnostics` tool for fast access

## File URI Encoding

The bridge properly encodes file paths to file:// URIs:

```rust
// /home/user/project with spaces
// → file:///home/user/project%20with%20spaces

fn path_to_file_uri(path: &str) -> Result<String> {
    let canonical = Path::new(path).canonicalize()?;
    let path_str = canonical.to_str()?;

    // URL-encode each path component
    let encoded_components: Vec<String> = path_str
        .split('/')
        .map(|component| urlencoding::encode(component).to_string())
        .collect();
    let encoded = encoded_components.join("/");

    // Add file:// prefix
    if path_str.starts_with('/') {
        Ok(format!("file:///{}", encoded))
    } else {
        Ok(format!("file://{}", encoded))
    }
}
```

## Message Size Limits

To prevent DoS attacks, the bridge enforces a maximum message size:

```rust
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16MB
```

If an LSP message exceeds this limit, the bridge returns an error and closes the connection.

## Graceful Shutdown

The bridge implements proper LSP shutdown:

```rust
pub async fn shutdown(
    &mut self,
    stdin: &mut ChildStdin,
    mut child: tokio::process::Child,
) -> Result<()> {
    // Send shutdown request
    self.send_lsp_request(stdin, "shutdown", "shutdown", &json!(null)).await?;

    // Send exit notification
    self.send_lsp_notification(stdin, "exit", &json!(null)).await?;

    // Wait for process to exit (with timeout)
    let _ = timeout(Duration::from_secs(5), child.wait()).await;

    Ok(())
}
```

## Error Handling

All bridge operations return `Result<T>` with descriptive errors:

```rust
use anyhow::{anyhow, Result};

match bridge.document_symbols(uri, &mut stdin, &mut stdout).await {
    Ok(result) => {
        // Process result
    }
    Err(e) => {
        if e.to_string().contains("Failed to spawn") {
            // LSP not installed
        } else if e.to_string().contains("timeout") {
            // LSP not responding
        } else {
            // Other error
        }
    }
}
```

## Example Usage

### Creating a Bridge

```rust
use leindex_analyzers::lsp::mcp_bridge::{McpBridge, McpTool, McpEvent};
use leindex_analyzers::memory::lsp_manager::LspType;

let bridge = McpBridge::new(LspType::Rust, "/home/user/project");
```

### Getting Available Tools

```rust
let tools = bridge.get_tools();

for tool in tools {
    println!("Tool: {} - {}", tool.name, tool.description);
}
```

### Calling a Tool

```rust
let result = bridge.call_tool(
    "lsp/document_symbols",
    json!({"uri": "file:///home/user/project/src/main.rs"}),
    &mut stdin,
    &mut stdout
).await?;
```

### Handling Events

```rust
let (event_tx, mut event_rx) = mpsc::channel::<McpEvent>(100);

tokio::spawn(async move {
    bridge.run_lsp_reader(stdout, event_tx).await
});

while let Some(event) = event_rx.recv().await {
    match event.name.as_str() {
        "diagnostics/published" => {
            // Handle new diagnostics
        }
        "lsp/log_message" => {
            // Handle log message
        }
        _ => {}
    }
}
```

## See Also

- [LSP Integration](./lsp_integration.md) - LSP manager and lifecycle
- [.mcp.json Format](./mcp_json_format.md) - Configuration format
- [TUI User Guide](./lsp_tui_user_guide.md) - Using LSPs in the terminal UI
