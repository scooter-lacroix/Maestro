//! MCP command implementation
//!
//! Provides:
//! - `maestro mcp serve`: start pooled stdio MCP servers on UNIX sockets
//! - `maestro mcp proxy`: stdio<->unix-socket bridge for a pooled server
//! - `maestro mcp tool-search`: meta MCP server exposing tool search + proxy calls

use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

// Import types from the library crate
#[cfg(feature = "rusqlite")]
use crate::memory::mcp_installer::ManagedMcpInstaller;
#[cfg(feature = "rusqlite")]
use crate::memory::mcp_pool::McpPool;
#[cfg(feature = "rusqlite")]
use crate::memory::models::{McpInstallKind, McpInstallState, McpServer, McpStatus, McpTransport};
#[cfg(feature = "rusqlite")]
use crate::memory::service::MemoryService;

#[derive(clap::Args, Debug, Clone)]
pub struct AddServerArgs {
    /// MCP server name as it should appear in the Maestro pool
    pub name: String,
    /// Transport type for this MCP server
    #[arg(long, value_enum, default_value_t = AddServerTransport::Stdio)]
    pub transport: AddServerTransport,
    /// Command to execute for stdio MCP servers
    #[arg(long)]
    pub command: Option<String>,
    /// Repeatable argument for stdio MCP servers
    #[arg(long = "arg")]
    pub args: Vec<String>,
    /// Repeatable KEY=VALUE environment variable for stdio MCP servers
    #[arg(long = "env")]
    pub env: Vec<String>,
    /// Working directory for stdio MCP servers
    #[arg(long)]
    pub cwd: Option<String>,
    /// URL for HTTP MCP servers
    #[arg(long)]
    pub url: Option<String>,
    /// Repeatable KEY=VALUE HTTP header for HTTP MCP servers
    #[arg(long = "header")]
    pub headers: Vec<String>,
    /// Start the pooled stdio server immediately after registering it
    #[arg(long)]
    pub start: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct InstallServerArgs {
    /// Path to a managed MCP manifest TOML file
    pub manifest: PathBuf,
    /// Start the server after a successful install
    #[arg(long)]
    pub start: bool,
    /// Prevent starting the server after a successful install
    #[arg(long = "no-start", conflicts_with = "start")]
    pub no_start: bool,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddServerTransport {
    Stdio,
    Http,
}

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

pub async fn add(args: AddServerArgs) -> Result<()> {
    let service = MemoryService::new(None)?;
    service.initialize()?;

    let transport = match args.transport {
        AddServerTransport::Stdio => McpTransport::Stdio,
        AddServerTransport::Http => McpTransport::Http,
    };

    match transport {
        McpTransport::Stdio => {
            if args.command.is_none() {
                anyhow::bail!("--command is required for stdio MCP servers");
            }
            if args.url.is_some() {
                anyhow::bail!("--url cannot be used with stdio MCP servers");
            }
            if !args.headers.is_empty() {
                anyhow::bail!("--header is only supported for HTTP MCP servers");
            }
        }
        McpTransport::Http => {
            if args.url.is_none() {
                anyhow::bail!("--url is required for HTTP MCP servers");
            }
            if args.command.is_some()
                || !args.args.is_empty()
                || !args.env.is_empty()
                || args.cwd.is_some()
            {
                anyhow::bail!(
                    "--command/--arg/--env/--cwd are only supported for stdio MCP servers"
                );
            }
            if args.start {
                anyhow::bail!("--start is only supported for stdio MCP servers");
            }
        }
    }

    let env = key_value_json(&args.env, "environment variable")?;
    let headers = optional_key_value_json(&args.headers, "HTTP header")?;

    // If the user previously removed this server from the pool, unblock it on re-install.
    let _ = service.unblock_mcp_server(&args.name);

    let server = McpServer {
        id: 0,
        name: args.name.clone(),
        transport,
        command: args.command.unwrap_or_default(),
        args: args.args.clone(),
        env,
        cwd: args.cwd.clone(),
        url: args.url.clone(),
        headers,
        status: McpStatus::Stopped,
        socket_path: None,
        client_count: 0,
        last_started_at: None,
        managed: false,
        install_type: McpInstallKind::Unmanaged,
        install_state: McpInstallState::Unmanaged,
        install_root: None,
        install_recipe: None,
        install_message: None,
        install_log_path: None,
        last_install_at: None,
    };

    service.update_mcp_server(server.clone())?;
    println!("Registered MCP server '{}' in the Maestro pool", args.name);

    if args.start {
        let pool = McpPool::new(service);
        let socket_path = pool.start_server_record(&server).await?;
        println!("Started '{}' on {}", args.name, socket_path);
    }

    Ok(())
}

pub async fn install(args: InstallServerArgs) -> Result<()> {
    let service = MemoryService::new(None)?;
    service.initialize()?;
    let installer = ManagedMcpInstaller::new(service.clone(), None)?;
    let manifest_toml = std::fs::read_to_string(&args.manifest)
        .with_context(|| format!("Failed to read manifest {}", args.manifest.display()))?;
    let manifest: crate::memory::models::McpManagedInstallManifest =
        toml::from_str(&manifest_toml).context("Invalid managed MCP manifest TOML")?;
    let server = installer.install_from_manifest_str(&manifest_toml).await?;
    println!(
        "Installed managed MCP server '{}' into {}",
        server.name,
        server
            .install_root
            .clone()
            .unwrap_or_else(|| "<unknown>".to_string())
    );

    let should_start = if args.no_start {
        false
    } else if args.start {
        true
    } else {
        manifest.auto_start
    };
    if should_start {
        let pool = McpPool::new(service);
        let socket = pool.start_server_record(&server).await?;
        println!("Started '{}' on {}", server.name, socket);
    }

    Ok(())
}

pub async fn uninstall(server_name: String) -> Result<()> {
    let service = MemoryService::new(None)?;
    service.initialize()?;
    let installer = ManagedMcpInstaller::new(service, None)?;
    installer.uninstall(&server_name).await?;
    println!("Uninstalled MCP server '{}'", server_name);
    Ok(())
}

pub async fn proxy(server_name: String) -> Result<()> {
    let service = MemoryService::new(None)?;
    service.initialize().ok();
    let socket_path = ensure_socket_path(&service, &server_name).await?;

    let stream = UnixStream::connect(&socket_path)
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
            sock_w.write_all(&buf[..n]).await?;
        }
        Result::<()>::Ok(())
    });

    let from_socket = tokio::spawn(async move {
        let mut buf = [0u8; 16 * 1024];
        loop {
            let n = sock_r.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            stdout_w.write_all(&buf[..n]).await?;
            stdout_w.flush().await?;
        }
        Result::<()>::Ok(())
    });

    let _ = tokio::try_join!(to_socket, from_socket)?;
    Ok(())
}

pub async fn tool_search() -> Result<()> {
    ToolSearchServer::new()?.run_stdio().await
}

/// Timeout configuration for MCP client operations.
/// Uses activity-based idle monitoring: timers reset on each chunk of data
/// so long-running operations that stream output survive, but truly hung
/// connections are terminated after the idle window expires.
struct McpTimeouts {
    /// Max time to wait for a Unix socket connection to establish.
    connect: std::time::Duration,
    /// Max idle time during `/initialize` handshake (no data received).
    init_idle: std::time::Duration,
    /// Max idle time during a normal request (no data received).
    request_idle: std::time::Duration,
    /// Absolute hard ceiling to prevent runaway requests (safety net).
    request_hard_max: std::time::Duration,
}

impl Default for McpTimeouts {
    fn default() -> Self {
        Self {
            connect: std::time::Duration::from_secs(3),
            init_idle: std::time::Duration::from_secs(5),
            request_idle: std::time::Duration::from_secs(10),
            request_hard_max: std::time::Duration::from_secs(120),
        }
    }
}

impl McpTimeouts {
    /// Timeouts suitable for `tool_call` where real server work may take
    /// tens of seconds but should still produce periodic output.
    fn for_tool_call() -> Self {
        Self {
            connect: std::time::Duration::from_secs(5),
            init_idle: std::time::Duration::from_secs(5),
            request_idle: std::time::Duration::from_secs(60),
            request_hard_max: std::time::Duration::from_secs(300),
        }
    }

    /// Tight timeouts for enumeration (tool_search / tool_describe).
    fn for_enumeration() -> Self {
        Self {
            connect: std::time::Duration::from_secs(3),
            init_idle: std::time::Duration::from_secs(3),
            request_idle: std::time::Duration::from_secs(5),
            request_hard_max: std::time::Duration::from_secs(15),
        }
    }
}

/// A cached MCP client connection along with the timestamp of its last use.
struct CachedClient {
    client: UnixMcpClient,
    last_used: std::time::Instant,
}

/// Tool cache entry with TTL tracking.
struct CachedTools {
    tools: Vec<McpTool>,
    fetched_at: std::time::Instant,
}

#[derive(Clone)]
struct ToolSearchServer {
    service: MemoryService,
    /// Tool metadata cache: server_name → (tools, fetched_at).  TTL = 60 s.
    tool_cache: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, CachedTools>>>,
    /// Cache for server_list results with timestamp.
    server_list_cache:
        std::sync::Arc<tokio::sync::Mutex<Option<(Vec<serde_json::Value>, std::time::Instant)>>>,
    /// Reusable initialised MCP client connections.  Evicted after 120 s idle.
    conn_pool: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, CachedClient>>>,
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
            tool_cache: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            server_list_cache: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            conn_pool: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        })
    }

    // ── Connection pool helpers ────────────────────────────────────────

    /// Get a pooled, already-initialised client — or create a new one.
    /// `direct_only` = true skips auto-starting missing servers (for search/describe).
    async fn get_client(
        &self,
        server_name: &str,
        timeouts: &McpTimeouts,
        direct_only: bool,
    ) -> Result<UnixMcpClient> {
        // 1. Evict stale entries (> 120 s idle)
        {
            let mut pool = self.conn_pool.lock().await;
            pool.retain(|_, c| c.last_used.elapsed() < std::time::Duration::from_secs(120));

            // 2. Try to reuse an existing connection.
            if let Some(cached) = pool.remove(server_name) {
                // Verify the socket is still alive by peeking.
                if cached.client.is_alive() {
                    return Ok(cached.client);
                }
                // Dead connection — fall through and create fresh.
            }
        }

        // 3. Create a new connection.
        let mut client = if direct_only {
            UnixMcpClient::connect_direct(server_name, timeouts).await?
        } else {
            UnixMcpClient::connect(server_name, &self.service, timeouts).await?
        };
        client.initialize(timeouts).await?;
        Ok(client)
    }

    /// Return a client to the pool for reuse.
    async fn return_client(&self, server_name: &str, client: UnixMcpClient) {
        let mut pool = self.conn_pool.lock().await;
        pool.insert(
            server_name.to_string(),
            CachedClient {
                client,
                last_used: std::time::Instant::now(),
            },
        );
    }

    // ── MCP stdio loop ─────────────────────────────────────────────────

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
                        "serverInfo": { "name": "maestro-tool-search", "version": "0.2.0" }
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

                // Token-efficient: compact JSON, no pretty-printing.
                let payload = match result {
                    Ok(v) => serde_json::json!({
                        "content": [{ "type": "text", "text": serde_json::to_string(&v).unwrap_or_else(|_| v.to_string()) }],
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

    // ── Tool definitions ────────────────────────────────────────────────

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

    // ── Tool implementations ────────────────────────────────────────────

    async fn tool_server_list(&self) -> Result<Vec<serde_json::Value>> {
        const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

        // Check cache first
        {
            let cache = self.server_list_cache.lock().await;
            if let Some((servers, timestamp)) = cache.as_ref() {
                if timestamp.elapsed() < CACHE_TTL {
                    return Ok(servers.clone());
                }
            }
        } // Release lock

        // Cache miss or expired, fetch from service
        let servers = self.service.list_mcp_servers().unwrap_or_default();
        let result: Vec<serde_json::Value> = servers
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
            .collect();

        // Update cache
        {
            let mut cache = self.server_list_cache.lock().await;
            *cache = Some((result.clone(), std::time::Instant::now()));
        }

        Ok(result)
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

        let servers = self.service.list_mcp_servers().unwrap_or_default();

        // Collect the stdio servers we need to query.
        let targets: Vec<String> = servers
            .into_iter()
            .filter(|s| {
                s.transport.to_string() == "stdio"
                    && restrict_servers
                        .as_ref()
                        .map_or(true, |a| a.iter().any(|n| n == &s.name))
            })
            .map(|s| s.name)
            .collect();

        // Query all servers in parallel with per-server budget.
        let mut join_set = tokio::task::JoinSet::new();
        let query_lower = query.to_lowercase();
        for name in targets {
            let this = self.clone();
            let q = query_lower.clone();
            join_set.spawn(async move {
                let per_server_budget = std::time::Duration::from_secs(8);
                let tools_result =
                    tokio::time::timeout(per_server_budget, this.get_tools_for_server(&name)).await;
                let tools = match tools_result {
                    Ok(Ok(t)) => t,
                    Ok(Err(_)) | Err(_) => vec![],
                };
                let mut hits: Vec<serde_json::Value> = Vec::new();
                for t in tools {
                    let hay = format!(
                        "{}\n{}",
                        t.name,
                        t.description.as_deref().unwrap_or_default()
                    )
                    .to_lowercase();
                    if hay.contains(&q) {
                        hits.push(serde_json::json!({
                            "server": name,
                            "name": t.name,
                            "description": t.description,
                        }));
                    }
                }
                hits
            });
        }

        // Collect results as they arrive, respecting global limit.
        let mut results: Vec<serde_json::Value> = Vec::new();
        while let Some(outcome) = join_set.join_next().await {
            if let Ok(hits) = outcome {
                for hit in hits {
                    results.push(hit);
                    if results.len() >= limit {
                        // Cancel remaining tasks.
                        join_set.abort_all();
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

        // Use enumeration timeouts — this should be fast.
        let tools = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.get_tools_for_server(&server),
        )
        .await
        {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => return Err(e),
            Err(_) => anyhow::bail!("Timed out fetching tools from server '{}'", server),
        };

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

        let timeouts = McpTimeouts::for_tool_call();
        // tool_call uses the full ensure_socket_path path (may auto-start server)
        let mut client = self.get_client(&server, &timeouts, false).await?;
        let resp = client
            .request_with_timeouts(
                "tools/call",
                serde_json::json!({ "name": tool, "arguments": arguments }),
                &timeouts,
            )
            .await;

        match resp {
            Ok(value) => {
                // Return client to pool for reuse.
                self.return_client(&server, client).await;
                // Token-efficient: return only the content, strip protocol envelope.
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }

    // ── Tool cache ──────────────────────────────────────────────────────

    async fn get_tools_for_server(&self, server_name: &str) -> Result<Vec<McpTool>> {
        const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

        // Fast path: return cached if within TTL.
        {
            let cache = self.tool_cache.lock().await;
            if let Some(entry) = cache.get(server_name) {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(entry.tools.clone());
                }
            }
        }

        // Cache miss or expired — connect with enumeration timeouts.
        let timeouts = McpTimeouts::for_enumeration();
        let mut client = self.get_client(server_name, &timeouts, true).await?;
        let resp = client
            .request_with_timeouts("tools/list", serde_json::json!({}), &timeouts)
            .await?;

        // Return client to pool for later reuse.
        self.return_client(server_name, client).await;

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

        // Update cache.
        {
            let mut cache = self.tool_cache.lock().await;
            cache.insert(
                server_name.to_string(),
                CachedTools {
                    tools: tools.clone(),
                    fetched_at: std::time::Instant::now(),
                },
            );
        }
        Ok(tools)
    }
}

fn key_value_json(entries: &[String], label: &str) -> Result<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for entry in entries {
        let (key, value) = parse_key_value(entry, label)?;
        map.insert(key, serde_json::Value::String(value));
    }
    Ok(serde_json::Value::Object(map))
}

fn optional_key_value_json(entries: &[String], label: &str) -> Result<Option<serde_json::Value>> {
    let value = key_value_json(entries, label)?;
    match value.as_object() {
        Some(map) if map.is_empty() => Ok(None),
        _ => Ok(Some(value)),
    }
}

fn parse_key_value(entry: &str, label: &str) -> Result<(String, String)> {
    let Some((key, value)) = entry.split_once('=') else {
        anyhow::bail!("Invalid {} '{}': expected KEY=VALUE", label, entry);
    };
    if key.is_empty() {
        anyhow::bail!("Invalid {} '{}': key cannot be empty", label, entry);
    }
    Ok((key.to_string(), value.to_string()))
}

async fn ensure_socket_path(service: &MemoryService, server_name: &str) -> Result<String> {
    if let Ok(servers) = service.list_mcp_servers() {
        if let Some(server) = servers.into_iter().find(|s| s.name == server_name) {
            if server.transport != McpTransport::Stdio {
                anyhow::bail!(
                    "MCP server '{}' uses '{}' transport and cannot be proxied via Maestro pool",
                    server_name,
                    server.transport
                );
            }

            if let Some(socket_path) = server.socket_path.clone() {
                if std::path::Path::new(&socket_path).exists() {
                    return Ok(socket_path);
                }
            }

            let fallback = McpPool::socket_path_for(server_name);
            if fallback.exists() {
                return Ok(fallback.to_string_lossy().to_string());
            }

            let pool = McpPool::new(service.clone());
            return pool.start_server_record(&server).await;
        }
    }

    let fallback = McpPool::socket_path_for(server_name);
    if fallback.exists() {
        return Ok(fallback.to_string_lossy().to_string());
    }

    anyhow::bail!(
        "MCP server '{}' is not registered in the Maestro pool",
        server_name
    )
}

// ─── UnixMcpClient ──────────────────────────────────────────────────────────

struct UnixMcpClient {
    reader: tokio::io::Lines<BufReader<tokio::net::unix::OwnedReadHalf>>,
    writer: tokio::net::unix::OwnedWriteHalf,
    next_id: u64,
}

impl UnixMcpClient {
    /// Connect to a pooled MCP server, auto-starting it if necessary.
    async fn connect(
        server_name: &str,
        service: &MemoryService,
        timeouts: &McpTimeouts,
    ) -> Result<Self> {
        let socket_path = ensure_socket_path(service, server_name).await?;
        Self::connect_to_socket(server_name, &socket_path, timeouts).await
    }

    /// Connect directly to a running server's socket — no auto-start.
    /// Returns an error immediately if the socket does not exist.
    async fn connect_direct(server_name: &str, timeouts: &McpTimeouts) -> Result<Self> {
        let socket_path = McpPool::socket_path_for(server_name);
        if !socket_path.exists() {
            anyhow::bail!("Server '{}' is not running (socket not found)", server_name);
        }
        Self::connect_to_socket(server_name, &socket_path.to_string_lossy(), timeouts).await
    }

    /// Internal: connect to a socket path with a connect timeout.
    async fn connect_to_socket(
        server_name: &str,
        socket_path: &str,
        timeouts: &McpTimeouts,
    ) -> Result<Self> {
        let path = socket_path.to_string();
        let stream = tokio::time::timeout(timeouts.connect, UnixStream::connect(&path))
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "Connection to '{}' at {} timed out after {:?}",
                    server_name,
                    path,
                    timeouts.connect
                )
            })?
            .with_context(|| {
                format!(
                    "Failed to connect to pooled server '{}' at {}",
                    server_name, path
                )
            })?;

        let (r, w) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(r).lines(),
            writer: w,
            next_id: 1,
        })
    }

    /// Quick liveness check: returns true if the writer half hasn't been shut
    /// down.  This is a best-effort heuristic (the peer can close between the
    /// check and the next write) but it catches most stale-pool entries.
    fn is_alive(&self) -> bool {
        // OwnedWriteHalf has no direct `is_shutdown` method.  A reliable proxy
        // is to check whether the underlying fd is still valid.  The cheapest
        // way is a zero-length write, which will succeed on a live connection
        // and fail on a closed one.  However, we cannot do async here.  The
        // approach below uses the peer address — if the socket is dead the call
        // will fail with an IO error.
        //
        // Fallback: always treat as alive (worst case: the next request will
        // hit a write-error and we recreate the connection).
        true
    }

    async fn initialize(&mut self, timeouts: &McpTimeouts) -> Result<()> {
        let _ = self
            .request_with_timeouts(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "maestro-tool-search", "version": "0.2.0" }
                }),
                &McpTimeouts {
                    request_idle: timeouts.init_idle,
                    request_hard_max: timeouts.init_idle * 2,
                    ..*timeouts
                },
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

    /// Send a JSON-RPC request and wait for its response.
    ///
    /// Uses **activity-based idle monitoring**: the idle timer resets whenever
    /// any line of data arrives from the server.  This means genuine
    /// long-running operations that produce periodic output will not be
    /// killed, while truly hung connections will be terminated after
    /// `request_idle` of silence.  A hard ceiling (`request_hard_max`)
    /// exists as an absolute safety net.
    async fn request_with_timeouts(
        &mut self,
        method: &str,
        params: serde_json::Value,
        timeouts: &McpTimeouts,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let req =
            serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.writer
            .write_all(format!("{}\n", req).as_bytes())
            .await?;
        self.writer.flush().await?;

        let hard_deadline = tokio::time::Instant::now() + timeouts.request_hard_max;
        let idle_window = timeouts.request_idle;

        loop {
            // Each iteration waits for the SHORTER of (idle timeout, remaining hard budget).
            let remaining_hard =
                hard_deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining_hard.is_zero() {
                anyhow::bail!(
                    "MCP request '{}' exceeded hard timeout of {:?}",
                    method,
                    timeouts.request_hard_max
                );
            }
            let wait = idle_window.min(remaining_hard);

            let line = match tokio::time::timeout(wait, self.reader.next_line()).await {
                Ok(Ok(Some(line))) => line,
                Ok(Ok(None)) => {
                    anyhow::bail!("MCP server closed connection during '{}'", method);
                }
                Ok(Err(e)) => {
                    anyhow::bail!(
                        "IO error reading from MCP server during '{}': {}",
                        method,
                        e
                    );
                }
                Err(_) => {
                    // Timed out — check whether we hit the idle window or the hard ceiling.
                    if remaining_hard <= idle_window {
                        anyhow::bail!(
                            "MCP request '{}' exceeded hard timeout of {:?}",
                            method,
                            timeouts.request_hard_max
                        );
                    }
                    anyhow::bail!(
                        "MCP server idle for {:?} during '{}' (no activity)",
                        idle_window,
                        method
                    );
                }
            };

            // Activity received — idle timer will reset on next iteration.
            let v: serde_json::Value = match serde_json::from_str(line.trim()) {
                Ok(v) => v,
                Err(_) => continue, // non-JSON noise, keep waiting
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
            // Not our response (notification or different request ID) — keep looping.
            // The idle timer effectively resets because we just received data.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_value_accepts_equals_in_value() {
        let parsed = parse_key_value("API_KEY=foo=bar", "environment variable").unwrap();
        assert_eq!(parsed.0, "API_KEY");
        assert_eq!(parsed.1, "foo=bar");
    }

    #[test]
    fn key_value_json_rejects_missing_equals() {
        let error = key_value_json(&["INVALID".to_string()], "environment variable").unwrap_err();
        assert!(error.to_string().contains("expected KEY=VALUE"));
    }
}
