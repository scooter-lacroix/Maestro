//! MCP command implementation
//!
//! Provides:
//! - `maestro mcp serve`: start pooled stdio MCP servers on UNIX sockets
//! - `maestro mcp proxy`: stdio<->unix-socket bridge for a pooled server
//! - `maestro mcp tool-search`: meta MCP server exposing tool search + proxy calls

#![cfg(feature = "rusqlite")]

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::memory::{McpPool, MemoryService};
use crate::memory::models::McpServer;

pub async fn serve() -> Result<()> {
    let service = MemoryService::new(None)?;
    service.initialize()?;
    let _ = service.sync_mcp_servers_from_system();

    let pool = McpPool::new(service.clone());
    let started = pool.start_all_from_db().await.unwrap_or(0);
    println!(
        "maestro mcp serve: started {} pooled MCP server(s)",
        started
    );

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

pub async fn proxy(server_name: String) -> Result<()> {
    // Prefer DB socket_path if present; otherwise fallback to deterministic /tmp path.
    let service = MemoryService::new(None)?;
    service.initialize().ok();
    let socket_path = service
        .list_mcp_servers()
        .ok()
        .and_then(|list: Vec<McpServer>| {
            list.into_iter()
                .find(|s| s.name == server_name)
                .and_then(|s| s.socket_path)
        })
        .unwrap_or_else(|| {
            McpPool::socket_path_for(&server_name)
                .to_string_lossy()
                .to_string()
        });

    let stream: tokio::net::UnixStream = UnixStream::connect(&socket_path)
        .await
        .with_context(|| format!("Failed to connect to MCP pool socket {}", socket_path))?;

    let (mut sock_r, mut sock_w) = stream.into_split();
    let (mut stdin_r, mut stdout_w) = (tokio::io::stdin(), tokio::io::stdout());

    let to_socket = tokio::spawn(async move {
        let mut buf = [0u8; 16 * 1024];
        loop {
            let n = stdin_r.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            <tokio::net::unix::OwnedWriteHalf as tokio::io::AsyncWriteExt>::write_all(&mut sock_w, &buf[..n]).await?;
        }
        Result::<()>::Ok(())
    });

    let from_socket = tokio::spawn(async move {
        let mut buf = [0u8; 16 * 1024];
        loop {
            let n = <tokio::net::unix::OwnedReadHalf as tokio::io::AsyncReadExt>::read(&mut sock_r, &mut buf).await?;
            if n == 0 {
                break;
            }
            <tokio::io::Stdout as tokio::io::AsyncWriteExt>::write_all(&mut stdout_w, &buf[..n]).await?;
            <tokio::io::Stdout as tokio::io::AsyncWriteExt>::flush(&mut stdout_w).await?;
        }
        Result::<()>::Ok(())
    });

    let _ = tokio::try_join!(to_socket, from_socket)?;
    Ok(())
}

pub async fn tool_search() -> Result<()> {
    ToolSearchServer::new()?.run_stdio().await
}

#[derive(Clone)]
struct ToolSearchServer {
    service: MemoryService,
    cache: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<McpTool>>>>,
}

#[derive(Clone, Debug)]
struct McpTool {
    name: String,
    description: Option<String>,
    input_schema: serde_json::Value,
}

impl ToolSearchServer {
    fn new() -> Result<Self> {
        let service = MemoryService::new(None)?;
        service.initialize().ok();
        Ok(Self {
            service,
            cache: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        })
    }

    async fn run_stdio(self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin).lines();
        let mut out = tokio::io::BufWriter::new(stdout);

        while let Some(line) = reader.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let req: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if req.get("method").and_then(|v| v.as_str()) == Some("initialize") {
                let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "maestro-tool-search", "version": "0.1.0" }
                    }
                });
                out.write_all(format!("{}\n", resp).as_bytes()).await?;
                out.flush().await?;
                continue;
            }

            if req.get("method").and_then(|v| v.as_str()) == Some("tools/list") {
                let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": self.meta_tools()
                    }
                });
                out.write_all(format!("{}\n", resp).as_bytes()).await?;
                out.flush().await?;
                continue;
            }

            if req.get("method").and_then(|v| v.as_str()) == Some("tools/call") {
                let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
                let tool_name = req
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let args = req
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                let result = match tool_name.as_str() {
                    "server_list" => self.tool_server_list().await.map(serde_json::Value::Array),
                    "tool_search" => self.tool_search(args).await,
                    "tool_describe" => self.tool_describe(args).await,
                    "tool_call" => self.tool_call(args).await,
                    _ => Ok(serde_json::json!({"error": "unknown tool"})),
                };

                let payload = match result {
                    Ok(v) => serde_json::json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()) }],
                        "isError": false
                    }),
                    Err(e) => serde_json::json!({
                        "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                        "isError": true
                    }),
                };

                let resp = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": payload });
                out.write_all(format!("{}\n", resp).as_bytes()).await?;
                out.flush().await?;
                continue;
            }

            // Ignore notifications like "notifications/initialized"
        }

        Ok(())
    }

    fn meta_tools(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "name": "server_list",
                "description": "List MCP servers available in the Maestro pool.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            }),
            serde_json::json!({
                "name": "tool_search",
                "description": "Search tools across pooled MCP servers by name/description.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 20 },
                        "servers": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }
            }),
            serde_json::json!({
                "name": "tool_describe",
                "description": "Get a tool schema from a pooled MCP server.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "server": { "type": "string" },
                        "tool": { "type": "string" }
                    },
                    "required": ["server", "tool"],
                    "additionalProperties": false
                }
            }),
            serde_json::json!({
                "name": "tool_call",
                "description": "Call a tool on a pooled MCP server (without listing every tool).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "server": { "type": "string" },
                        "tool": { "type": "string" },
                        "arguments": { "type": "object" }
                    },
                    "required": ["server", "tool"],
                    "additionalProperties": false
                }
            }),
        ]
    }

    async fn tool_server_list(&self) -> Result<Vec<serde_json::Value>> {
        let servers = self.service.list_mcp_servers().unwrap_or_default();
        Ok(servers
            .into_iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "transport": s.transport.to_string(),
                    "status": s.status.to_string(),
                    "socket_path": s.socket_path,
                    "url": s.url,
                })
            })
            .collect())
    }

    async fn tool_search(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if query.is_empty() {
            return Ok(serde_json::json!({"results": []}));
        }
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(200) as usize;
        let restrict_servers: Option<Vec<String>> =
            args.get("servers").and_then(|v| v.as_array()).map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            });

        let mut results: Vec<serde_json::Value> = Vec::new();
        let servers = self.service.list_mcp_servers().unwrap_or_default();

        for s in servers {
            if let Some(ref allowed) = restrict_servers {
                if !allowed.iter().any(|n| n == &s.name) {
                    continue;
                }
            }
            if s.transport.to_string() != "stdio" {
                continue;
            }

            let tools = self.get_tools_for_server(&s.name).await.unwrap_or_default();
            for t in tools {
                let hay = format!("{}\n{}", t.name, t.description.clone().unwrap_or_default())
                    .to_lowercase();
                if hay.contains(&query.to_lowercase()) {
                    results.push(serde_json::json!({
                        "server": s.name,
                        "name": t.name,
                        "description": t.description,
                    }));
                    if results.len() >= limit {
                        return Ok(serde_json::json!({ "results": results }));
                    }
                }
            }
        }

        Ok(serde_json::json!({ "results": results }))
    }

    async fn tool_describe(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let server = args
            .get("server")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tool = args
            .get("tool")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tools = self.get_tools_for_server(&server).await.unwrap_or_default();
        let found = tools.into_iter().find(|t| t.name == tool);
        Ok(match found {
            Some(t) => serde_json::json!({
                "server": server,
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema,
            }),
            None => serde_json::json!({"error": "tool not found"}),
        })
    }

    async fn tool_call(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let server = args
            .get("server")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tool = args
            .get("tool")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let arguments = args
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let mut client = UnixMcpClient::connect(&server, &self.service).await?;
        client.initialize().await?;
        let resp = client
            .request(
                "tools/call",
                serde_json::json!({ "name": tool, "arguments": arguments }),
            )
            .await?;
        Ok(resp)
    }

    async fn get_tools_for_server(&self, server_name: &str) -> Result<Vec<McpTool>> {
        // Fast path cache
        if let Some(cached) = self.cache.lock().await.get(server_name).cloned() {
            return Ok(cached);
        }

        let mut client = UnixMcpClient::connect(server_name, &self.service).await?;
        client.initialize().await?;
        let resp = client.request("tools/list", serde_json::json!({})).await?;

        let tools_val = resp
            .get("tools")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let mut tools: Vec<McpTool> = Vec::new();
        if let Some(arr) = tools_val.as_array() {
            for t in arr {
                let name = t
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() {
                    continue;
                }
                let description = t
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let input_schema = t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                tools.push(McpTool {
                    name,
                    description,
                    input_schema,
                });
            }
        }

        self.cache
            .lock()
            .await
            .insert(server_name.to_string(), tools.clone());
        Ok(tools)
    }
}

struct UnixMcpClient {
    reader: tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
    writer: tokio::net::unix::OwnedWriteHalf,
    next_id: u64,
}

impl UnixMcpClient {
    async fn connect(server_name: &str, service: &MemoryService) -> Result<Self> {
        let socket_path: String = service
            .list_mcp_servers()
            .ok()
            .and_then(|list: Vec<McpServer>| {
                list.into_iter()
                    .find(|s| s.name == server_name)
                    .and_then(|s| s.socket_path.clone())
            })
            .unwrap_or_else(|| {
                McpPool::socket_path_for(server_name)
                    .to_string_lossy()
                    .to_string()
            });

        let stream = tokio::net::UnixStream::connect(&socket_path).await.with_context(|| {
            format!(
                "Failed to connect to pooled server '{}' at {}",
                server_name, socket_path
            )
        })?;

        let (r, w) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(r).lines(),
            writer: w,
            next_id: 1,
        })
    }

    async fn initialize(&mut self) -> Result<()> {
        let _ = self
            .request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "maestro-tool-search", "version": "0.1.0" }
                }),
            )
            .await?;
        // MCP requires an initialized notification.
        let note = serde_json::json!({ "jsonrpc":"2.0", "method":"notifications/initialized", "params":{} });
        self.writer
            .write_all(format!("{}\n", note).as_bytes())
            .await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let req =
            serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.writer
            .write_all(format!("{}\n", req).as_bytes())
            .await?;
        self.writer.flush().await?;

        while let Some(line) = self.reader.next_line().await? {
            let v: serde_json::Value = match serde_json::from_str(line.trim()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("id").and_then(|x| x.as_u64()) == Some(id) {
                if let Some(err) = v.get("error") {
                    return Ok(serde_json::json!({ "error": err }));
                }
                return Ok(v
                    .get("result")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})));
            }
        }

        anyhow::bail!("No response from MCP server for {}", method)
    }
}
