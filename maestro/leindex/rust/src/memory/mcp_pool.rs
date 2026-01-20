//! Centralized MCP Pooling
//!
//! A lightweight UNIX-socket gateway that keeps one underlying MCP server process
//! running per configured server name, and allows many clients to connect via
//! a shared socket without respawning the real server.
//!
//! Transport:
//! - Each pooled server exposes a UNIX socket at `/tmp/maestro-mcp-<name>.sock`.
//! - Clients speak newline-delimited JSON-RPC (MCP over stdio framing).
//! - The pool rewrites request IDs to avoid collisions and routes responses back.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex, RwLock, watch};
use tracing::{error, info, warn};

use super::models::{McpServer, McpStatus, McpTransport};
#[cfg(feature = "rusqlite")]
use super::service::MemoryService;

#[cfg(feature = "rusqlite")]
pub struct McpPool {
    proxies: Arc<RwLock<HashMap<String, Arc<SocketProxy>>>>,
    service: MemoryService,
}

#[cfg(feature = "rusqlite")]
impl McpPool {
    pub fn new(service: MemoryService) -> Self {
        Self {
            proxies: Arc::new(RwLock::new(HashMap::new())),
            service,
        }
    }

    pub fn socket_path_for(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let safe = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        p.push(format!("maestro-mcp-{}.sock", safe));
        p
    }

    pub fn log_path_for(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let safe = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        p.push(format!("maestro-mcp-{}.log", safe));
        p
    }

    /// Start all stdio MCP servers currently registered in the DB.
    pub async fn start_all_from_db(&self) -> Result<usize> {
        let servers = self.service.list_mcp_servers().unwrap_or_default();
        let mut started = 0usize;
        for s in servers {
            if s.transport != McpTransport::Stdio {
                continue;
            }
            if self.start_server_record(&s).await.is_ok() {
                started += 1;
            }
        }
        Ok(started)
    }

    pub async fn start_server_record(&self, server: &McpServer) -> Result<String> {
        if server.transport != McpTransport::Stdio {
            anyhow::bail!("Cannot start non-stdio MCP server '{}' in pool", server.name);
        }

        let mut proxies = self.proxies.write().await;
        if let Some(proxy) = proxies.get(&server.name) {
            if proxy.is_running().await {
                return Ok(proxy.socket_path.to_string_lossy().to_string());
            }
        }

        let socket_path = Self::socket_path_for(&server.name);
        let env = json_env_to_hashmap(&server.env);
        let proxy = SocketProxy::new(
            &server.name,
            &server.command,
            server.args.clone(),
            env,
            server.cwd.clone(),
            socket_path.clone(),
        )?;
        let proxy_arc = Arc::new(proxy);

        let proxy_clone = proxy_arc.clone();
        let name = server.name.clone();
        tokio::spawn(async move {
            if let Err(e) = proxy_clone.run().await {
                error!("MCP pool server '{}' crashed: {}", name, e);
            }
        });

        proxies.insert(server.name.clone(), proxy_arc);

        // Update DB to reflect socket path (status is updated opportunistically by the UI).
        let mut updated = server.clone();
        updated.socket_path = Some(socket_path.to_string_lossy().to_string());
        updated.status = McpStatus::Running;
        updated.last_started_at = Some(chrono::Utc::now());
        let _ = self.service.update_mcp_server(updated);

        Ok(socket_path.to_string_lossy().to_string())
    }

    pub async fn stop_server(&self, name: &str) -> Result<()> {
        let proxy = { self.proxies.read().await.get(name).cloned() };
        if let Some(p) = proxy {
            p.shutdown().await;
        }

        // Update DB state (best-effort).
        if let Ok(servers) = self.service.list_mcp_servers() {
            if let Some(mut server) = servers.into_iter().find(|s| s.name == name) {
                server.status = McpStatus::Stopped;
                server.socket_path = None;
                server.client_count = 0;
                let _ = self.service.update_mcp_server(server);
            }
        }

        // Remove proxy record (a new start will recreate it).
        self.proxies.write().await.remove(name);
        Ok(())
    }
}

fn json_env_to_hashmap(v: &serde_json::Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(map) = v.as_object() {
        for (k, val) in map {
            if let Some(s) = val.as_str() {
                out.insert(k.clone(), s.to_string());
            } else {
                out.insert(k.clone(), val.to_string());
            }
        }
    }
    out
}

struct PendingRequest {
    client_id: u64,
    original_id: serde_json::Value,
}

pub struct SocketProxy {
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    cwd: Option<String>,
    pub(crate) socket_path: PathBuf,
    status: RwLock<McpStatus>,
    shutdown_tx: watch::Sender<bool>,
    next_client_id: AtomicU64,
}

impl SocketProxy {
    pub fn new(
        name: &str,
        command: &str,
        args: Vec<String>,
        env: HashMap<String, String>,
        cwd: Option<String>,
        socket_path: PathBuf,
    ) -> Result<Self> {
        let (shutdown_tx, _) = watch::channel(false);
        Ok(Self {
            name: name.to_string(),
            command: command.to_string(),
            args,
            env,
            cwd,
            socket_path,
            status: RwLock::new(McpStatus::Stopped),
            shutdown_tx,
            next_client_id: AtomicU64::new(1),
        })
    }

    pub async fn is_running(&self) -> bool {
        *self.status.read().await == McpStatus::Running
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub async fn run(&self) -> Result<()> {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        let listener = UnixListener::bind(&self.socket_path).context("Failed to bind UNIX socket")?;
        info!("MCP pool '{}' listening on {}", self.name, self.socket_path.display());

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args).envs(&self.env);
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server '{}'", self.name))?;

        let stdin = child.stdin.take().context("Missing stdin")?;
        let stdout = child.stdout.take().context("Missing stdout")?;
        let stderr = child.stderr.take().context("Missing stderr")?;

        let stdin = Arc::new(Mutex::new(stdin));

        let clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending: Arc<Mutex<HashMap<String, PendingRequest>>> = Arc::new(Mutex::new(HashMap::new()));

        *self.status.write().await = McpStatus::Running;

        // STDERR logger
        tokio::spawn({
            let name = self.name.clone();
            async move {
                let log_path = McpPool::log_path_for(&name);
                let mut log_file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .await
                    .ok();
                let mut r = BufReader::new(stderr);
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let msg = line.trim_end();
                            if !msg.is_empty() {
                                warn!("mcp[{}] {}", name, msg);
                                if let Some(f) = log_file.as_mut() {
                                    let _ = f
                                        .write_all(format!("{}\n", msg).as_bytes())
                                        .await;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        });

        // STDOUT router
        tokio::spawn({
            let clients = clients.clone();
            let pending = pending.clone();
            async move {
                let mut r = BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(_) => break,
                    }

                    let raw = line.trim_end();
                    if raw.is_empty() {
                        continue;
                    }

                    let parsed: serde_json::Value = match serde_json::from_str(raw) {
                        Ok(v) => v,
                        Err(_) => {
                            // If it's not JSON (some servers log on stdout), broadcast as text.
                            let msg = serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": "maestro/log",
                                "params": { "text": raw }
                            });
                            let bytes = format!("{}\n", msg).into_bytes();
                            let map = clients.lock().await;
                            for tx in map.values() {
                                let _ = tx.send(bytes.clone()).await;
                            }
                            continue;
                        }
                    };

                    let id_str = parsed
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(internal_id) = id_str {
                        let pending_req = { pending.lock().await.remove(&internal_id) };
                        if let Some(pr) = pending_req {
                            let mut msg = parsed;
                            if let Some(obj) = msg.as_object_mut() {
                                obj.insert("id".to_string(), pr.original_id);
                            }
                            let bytes = format!("{}\n", msg).into_bytes();
                            let map = clients.lock().await;
                            if let Some(tx) = map.get(&pr.client_id) {
                                let _ = tx.send(bytes).await;
                            }
                            continue;
                        }
                    }

                    // Notifications or unknown IDs: broadcast.
                    let bytes = format!("{}\n", parsed).into_bytes();
                    let map = clients.lock().await;
                    for tx in map.values() {
                        let _ = tx.send(bytes.clone()).await;
                    }
                }
            }
        });

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                accept = listener.accept() => {
                    let (socket, _) = match accept {
                        Ok(v) => v,
                        Err(e) => {
                            error!("MCP pool '{}' accept failed: {}", self.name, e);
                            continue;
                        }
                    };
                    self.spawn_client(socket, stdin.clone(), clients.clone(), pending.clone()).await;
                }
                status = child.wait() => {
                    match status {
                        Ok(s) => warn!("MCP server '{}' exited: {}", self.name, s),
                        Err(e) => warn!("MCP server '{}' wait error: {}", self.name, e),
                    }
                    break;
                }
            }
        }

        let _ = child.kill().await;
        *self.status.write().await = McpStatus::Stopped;
        let _ = std::fs::remove_file(&self.socket_path);
        Ok(())
    }

    async fn spawn_client(
        &self,
        socket: UnixStream,
        stdin: Arc<Mutex<tokio::process::ChildStdin>>,
        clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>>,
        pending: Arc<Mutex<HashMap<String, PendingRequest>>>,
    ) {
        let client_id = self.next_client_id.fetch_add(1, Ordering::Relaxed);
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);
        clients.lock().await.insert(client_id, tx.clone());

        let (read_half, mut write_half) = socket.into_split();
        let mut reader = BufReader::new(read_half);

        // Writer task
        let clients_writer = clients.clone();
        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if write_half.write_all(&bytes).await.is_err() {
                    break;
                }
            }
            clients_writer.lock().await.remove(&client_id);
        });

        // Reader loop (in task)
        let pending_reader = pending.clone();
        let stdin_reader = stdin.clone();
        let name = self.name.clone();
        tokio::spawn(async move {
            let mut line = String::new();
            let mut req_seq: u64 = 1;
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }

                let raw = line.trim_end();
                if raw.is_empty() {
                    continue;
                }

                let mut msg: serde_json::Value = match serde_json::from_str(raw) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Rewrite IDs to avoid collisions across clients.
                if let Some(orig_id) = msg.get("id").cloned() {
                    let internal_id = format!("c{}-{}", client_id, req_seq);
                    req_seq = req_seq.wrapping_add(1);
                    pending_reader.lock().await.insert(
                        internal_id.clone(),
                        PendingRequest { client_id, original_id: orig_id },
                    );
                    if let Some(obj) = msg.as_object_mut() {
                        obj.insert("id".to_string(), serde_json::Value::String(internal_id));
                    }
                }

                let out = match serde_json::to_string(&msg) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let mut locked = stdin_reader.lock().await;
                if locked.write_all(out.as_bytes()).await.is_err() {
                    warn!("MCP pool '{}' failed writing to child stdin", name);
                    break;
                }
                if locked.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = locked.flush().await;
            }
        });
    }
}
