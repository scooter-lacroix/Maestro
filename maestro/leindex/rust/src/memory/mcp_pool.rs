//! Centralized MCP Pooling
//!
//! Provides socket-based proxying and multiplexing for shared MCP servers.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, error};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use super::models::McpStatus;
use super::service::MemoryService;

pub struct McpPool {
    proxies: Arc<RwLock<HashMap<String, Arc<SocketProxy>>>>,
    _service: MemoryService,
}

impl McpPool {
    pub fn new(service: MemoryService) -> Self {
        Self {
            proxies: Arc::new(RwLock::new(HashMap::new())),
            _service: service,
        }
    }

    /// Start an MCP server in the pool
    pub async fn start_server(&self, name: &str, command: &str, args: Vec<String>, env: HashMap<String, String>) -> Result<String> {
        let mut proxies = self.proxies.write().await;
        
        if let Some(proxy) = proxies.get(name) {
            if proxy.is_running().await {
                return Ok(proxy.socket_path.to_string_lossy().to_string());
            }
        }

        let socket_path = self.get_socket_path(name)?;
        let proxy = SocketProxy::new(name, command, args, env, socket_path.clone())?;
        let proxy_arc = Arc::new(proxy);
        
        let proxy_clone = proxy_arc.clone();
        let name_str = name.to_string();
        tokio::spawn(async move {
            if let Err(e) = proxy_clone.run().await {
                error!("MCP Proxy {} failed: {}", name_str, e);
            }
        });

        proxies.insert(name.to_string(), proxy_arc);
        
        Ok(socket_path.to_string_lossy().to_string())
    }

    fn get_socket_path(&self, name: &str) -> Result<PathBuf> {
        let mut p = std::env::temp_dir();
        p.push(format!("maestro-mcp-{}.sock", name));
        Ok(p)
    }
}

pub struct SocketProxy {
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    socket_path: PathBuf,
    status: RwLock<McpStatus>,
}

impl SocketProxy {
    pub fn new(name: &str, command: &str, args: Vec<String>, env: HashMap<String, String>, socket_path: PathBuf) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            command: command.to_string(),
            args,
            env,
            socket_path,
            status: RwLock::new(McpStatus::Stopped),
        })
    }

    pub async fn is_running(&self) -> bool {
        *self.status.read().await == McpStatus::Running
    }

    pub async fn run(&self) -> Result<()> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .context("Failed to bind UNIX socket")?;
        
        info!("MCP Proxy {} listening on {}", self.name, self.socket_path.display());
        *self.status.write().await = McpStatus::Running;

        // Spawn actual MCP server process
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .envs(&self.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn MCP server")?;
        
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        
        // Channel for multiplexing client requests to stdio
        let (_request_tx, mut request_rx) = mpsc::channel::<(Vec<u8>, mpsc::Sender<Vec<u8>>)>(32);

        // Task: Handle process output (multiplex back to clients)
        // Note: Simple implementation - forwarding to all clients or matching IDs if JSON-RPC
        tokio::spawn(async move {
            let mut line = String::new();
            while stdout.read_line(&mut line).await.is_ok() {
                if line.is_empty() { break; }
                // Here we would need to parse JSON-RPC to route back to the correct client
                // For now, simpler: we assume 1-to-1 or use a smarter multiplexer
                line.clear();
            }
        });

        // Task: Handle process input
        tokio::spawn(async move {
            while let Some((data, _reply_tx)) = request_rx.recv().await {
                let _ = stdin.write_all(&data).await;
            }
        });

        // Main Loop: Accept socket connections
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0; 1024];
                while let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 { break; }
                }
            });
        }

        Ok(())
    }
}
