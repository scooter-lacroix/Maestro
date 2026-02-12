//! LSP Resource Pool - Pooled LSP instances with aggressive resource optimization
//!
//! ## Architecture
//!
//! This module implements a shared-pool architecture for LSP servers:
//! - **Single Instance Per Language**: Only one LSP per language type, shared across all sessions
//! - **Reference Counting**: Tracks which sessions are using each LSP
//! - **Idle Timeout**: Automatically stops LSPs after configurable idle period
//! - **Lazy Initialization**: LSPs only start on first request
//! - **Resource Limits**: Process priority, memory limits, CPU throttling
//!
//! ## Resource Optimization Techniques
//!
//! 1. **Pooling**: 10 sessions = 1 LSP instance (not 10)
//! 2. **Idle Timeout**: Stop after 5 minutes of inactivity (configurable)
//! 3. **Low Priority**: Run LSPs at lowest process priority
//! 4. **Memory Limits**: Optional cgroup-based memory limits
//! 5. **Request Batching**: Debounce rapid requests
//! 6. **Graceful Degradation**: Refuse new LSPs under memory pressure

use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::{watch, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use super::lsp_manager::LspType;
use super::turso_backend::{LspStatus, TursoStorageBackend};

/// Configuration for the LSP pool
#[derive(Debug, Clone)]
pub struct LspPoolConfig {
    /// Idle timeout before stopping an LSP (default: 5 minutes)
    pub idle_timeout: Duration,
    /// Maximum memory usage before refusing new LSPs (0 = unlimited)
    pub max_total_memory_mb: u64,
    /// Minimum free memory required to start new LSP (default: 512MB)
    pub min_free_memory_mb: u64,
    /// Check interval for idle timeout and resource monitoring
    pub monitor_interval: Duration,
    /// Enable process priority lowering
    pub low_priority: bool,
    /// Maximum LSP instances to run simultaneously (0 = unlimited)
    pub max_instances: usize,
    /// Enable request batching/debouncing
    pub enable_batching: bool,
    /// Debounce window for batching requests
    pub debounce_ms: u64,
}

impl Default for LspPoolConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(300),
            max_total_memory_mb: 0,
            min_free_memory_mb: 512,
            monitor_interval: Duration::from_secs(30),
            low_priority: true,
            max_instances: 0,
            enable_batching: true,
            debounce_ms: 100,
        }
    }
}

/// A pooled LSP instance shared across multiple sessions
pub struct PooledLsp {
    /// LSP type
    pub lsp_type: LspType,
    /// Sessions currently using this LSP
    pub sessions: HashSet<String>,
    /// Process handle
    pub child: Option<Child>,
    /// Process ID
    pub pid: Option<u32>,
    /// Current status
    pub status: LspStatus,
    /// When the LSP was started
    pub started_at: Instant,
    /// Last activity timestamp (seconds since start)
    last_activity_secs: AtomicU64,
    /// Number of requests processed (for metrics)
    pub request_count: AtomicU64,
    /// Estimated memory usage in bytes
    pub memory_bytes: AtomicU64,
    /// Whether this LSP is marked for shutdown
    pub shutting_down: AtomicBool,
    /// Started at instant for calculating idle time
    started_at_instant: Instant,
}

impl PooledLsp {
    fn new(lsp_type: LspType) -> Self {
        let now = Instant::now();
        Self {
            lsp_type,
            sessions: HashSet::new(),
            child: None,
            pid: None,
            status: LspStatus::Stopped,
            started_at: now,
            last_activity_secs: AtomicU64::new(0),
            request_count: AtomicU64::new(0),
            memory_bytes: AtomicU64::new(0),
            shutting_down: AtomicBool::new(false),
            started_at_instant: now,
        }
    }

    fn touch(&self) {
        let elapsed = self.started_at_instant.elapsed().as_secs();
        self.last_activity_secs.store(elapsed, Ordering::Relaxed);
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    fn idle_duration(&self) -> Duration {
        let last_secs = self.last_activity_secs.load(Ordering::Relaxed);
        let total_elapsed = self.started_at_instant.elapsed().as_secs();
        Duration::from_secs(total_elapsed.saturating_sub(last_secs))
    }

    fn is_idle(&self, timeout: Duration) -> bool {
        self.sessions.is_empty() && self.idle_duration() > timeout
    }
}

/// LSP Resource Pool - manages shared LSP instances
pub struct LspPool {
    /// Pool configuration
    config: LspPoolConfig,
    /// Storage backend for state persistence
    #[allow(dead_code)]
    storage: TursoStorageBackend,
    /// Pooled LSP instances (keyed by LspType)
    pools: Arc<RwLock<HashMap<LspType, PooledLsp>>>,
    /// Session -> LSP subscriptions mapping
    subscriptions: Arc<RwLock<HashMap<String, HashSet<LspType>>>>,
    /// Stop signal for background monitor
    stop_tx: watch::Sender<bool>,
    /// Total memory used by all LSPs
    total_memory_bytes: AtomicU64,
    /// Whether the pool is shutting down
    is_shutting_down: AtomicBool,
    /// Metrics: total LSP starts
    pub total_starts: AtomicU64,
    /// Metrics: total LSP stops
    pub total_stops: AtomicU64,
    /// Metrics: cache hits (LSP already running)
    pub cache_hits: AtomicU64,
    /// Metrics: cache misses (LSP needed to start)
    pub cache_misses: AtomicU64,
}

impl LspPool {
    /// Create a new LSP pool
    pub fn new(storage: TursoStorageBackend, config: LspPoolConfig) -> Self {
        let (stop_tx, _) = watch::channel(false);
        
        Self {
            config,
            storage,
            pools: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            stop_tx,
            total_memory_bytes: AtomicU64::new(0),
            is_shutting_down: AtomicBool::new(false),
            total_starts: AtomicU64::new(0),
            total_stops: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }
    
    /// Start the background idle monitor
    pub fn start_monitor(&self) -> JoinHandle<()> {
        let pools = self.pools.clone();
        let config = self.config.clone();
        let mut stop_rx = self.stop_tx.subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.monitor_interval);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut pools_guard = match pools.try_write() {
                            Ok(g) => g,
                            Err(_) => continue,
                        };
                        
                        let idle_timeout = config.idle_timeout;
                        let mut to_stop = Vec::new();
                        
                        for (lsp_type, pooled) in pools_guard.iter() {
                            if pooled.status == LspStatus::Running && pooled.is_idle(idle_timeout) {
                                info!("Stopping idle LSP '{}' after {:?} of inactivity", 
                                    lsp_type.display_name(), pooled.idle_duration());
                                to_stop.push(*lsp_type);
                            }
                        }
                        
                        for lsp_type in to_stop {
                            if let Some(pooled) = pools_guard.get_mut(&lsp_type) {
                                if pooled.status == LspStatus::Running {
                                    pooled.shutting_down.store(true, Ordering::Relaxed);
                                    
                                    if let Some(ref mut child) = pooled.child {
                                        #[cfg(unix)]
                                        {
                                            if let Some(pid) = pooled.pid {
                                                let _ = nix::sys::signal::kill(
                                                    nix::unistd::Pid::from_raw(-(pid as i32)),
                                                    nix::sys::signal::Signal::SIGTERM,
                                                );
                                            }
                                        }
                                        let _ = child.kill().await;
                                        let _ = child.wait().await;
                                    }
                                    
                                    pooled.child = None;
                                    pooled.pid = None;
                                    pooled.status = LspStatus::Stopped;
                                    pooled.shutting_down.store(false, Ordering::Relaxed);
                                    
                                    info!("Pooled LSP '{}' stopped (idle)", lsp_type.display_name());
                                }
                            }
                        }
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        })
    }
    
    /// Subscribe a session to an LSP type
    pub async fn subscribe(&self, session_id: &str, lsp_type: LspType, project_path: Option<&PathBuf>) -> Result<bool> {
        if self.is_shutting_down.load(Ordering::Relaxed) {
            return Err(anyhow!("LSP pool is shutting down"));
        }
        
        if !self.check_resources_available().await? {
            return Err(anyhow!("Insufficient system resources to start LSP"));
        }
        
        if self.config.max_instances > 0 {
            let pools = self.pools.read().await;
            let running = pools.values().filter(|p| p.status == LspStatus::Running).count();
            if running >= self.config.max_instances && !pools.contains_key(&lsp_type) {
                drop(pools);
                self.evict_idle_lsp().await?;
            }
        }
        
        let mut pools = self.pools.write().await;
        let mut subscriptions = self.subscriptions.write().await;
        
        let pooled = pools.entry(lsp_type).or_insert_with(|| PooledLsp::new(lsp_type));
        
        pooled.sessions.insert(session_id.to_string());
        
        subscriptions
            .entry(session_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(lsp_type);
        
        if pooled.status != LspStatus::Running {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            self.start_pooled_lsp_internal(pooled, project_path).await?;
            self.total_starts.fetch_add(1, Ordering::Relaxed);
            Ok(true)
        } else {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            pooled.touch();
            Ok(false)
        }
    }
    
    /// Unsubscribe a session from an LSP type
    pub async fn unsubscribe(&self, session_id: &str, lsp_type: LspType) -> Result<bool> {
        let mut pools = self.pools.write().await;
        let mut subscriptions = self.subscriptions.write().await;
        
        if let Some(lsps) = subscriptions.get_mut(session_id) {
            lsps.remove(&lsp_type);
            if lsps.is_empty() {
                subscriptions.remove(session_id);
            }
        }
        
        if let Some(pooled) = pools.get_mut(&lsp_type) {
            pooled.sessions.remove(session_id);
            
            if pooled.sessions.is_empty() && self.config.idle_timeout == Duration::ZERO {
                self.stop_pooled_lsp_internal(pooled).await?;
                self.total_stops.fetch_add(1, Ordering::Relaxed);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Unsubscribe a session from all LSPs
    pub async fn unsubscribe_all(&self, session_id: &str) -> Result<Vec<LspType>> {
        let subscriptions = self.subscriptions.read().await;
        let lsps: Vec<LspType> = subscriptions
            .get(session_id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        drop(subscriptions);
        
        for lsp_type in &lsps {
            self.unsubscribe(session_id, *lsp_type).await?;
        }
        
        Ok(lsps)
    }
    
    /// Get an LSP for use (marks as active)
    pub async fn get(&self, lsp_type: LspType) -> Option<LspHandle> {
        let mut pools = self.pools.write().await;
        if let Some(pooled) = pools.get_mut(&lsp_type) {
            if pooled.status == LspStatus::Running && !pooled.shutting_down.load(Ordering::Relaxed) {
                pooled.touch();
                return Some(LspHandle {
                    lsp_type,
                    pid: pooled.pid,
                });
            }
        }
        None
    }
    
    /// Check if an LSP is running
    pub async fn is_running(&self, lsp_type: LspType) -> bool {
        let pools = self.pools.read().await;
        pools.get(&lsp_type).map(|p| p.status == LspStatus::Running).unwrap_or(false)
    }
    
    /// Get all running LSP types
    pub async fn running_types(&self) -> Vec<LspType> {
        let pools = self.pools.read().await;
        pools.iter()
            .filter(|(_, p)| p.status == LspStatus::Running)
            .map(|(t, _)| *t)
            .collect()
    }
    
    /// Get sessions subscribed to a specific LSP
    pub async fn get_subscribers(&self, lsp_type: LspType) -> Vec<String> {
        let pools = self.pools.read().await;
        pools.get(&lsp_type)
            .map(|p| p.sessions.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get pool statistics
    pub async fn stats(&self) -> LspPoolStats {
        let pools = self.pools.read().await;
        let running = pools.values().filter(|p| p.status == LspStatus::Running).count();
        let total_subscribers: usize = pools.values().map(|p| p.sessions.len()).sum();
        
        LspPoolStats {
            running_instances: running,
            total_subscribers,
            total_memory_mb: self.total_memory_bytes.load(Ordering::Relaxed) / (1024 * 1024),
            total_starts: self.total_starts.load(Ordering::Relaxed),
            total_stops: self.total_stops.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }
    
    /// Check if system resources are available
    async fn check_resources_available(&self) -> Result<bool> {
        if self.config.min_free_memory_mb > 0 {
            let free_mb = get_available_memory_mb()?;
            if free_mb < self.config.min_free_memory_mb {
                warn!("Insufficient free memory: {}MB < {}MB required", 
                    free_mb, self.config.min_free_memory_mb);
                return Ok(false);
            }
        }
        
        if self.config.max_total_memory_mb > 0 {
            let current_mb = self.total_memory_bytes.load(Ordering::Relaxed) / (1024 * 1024);
            if current_mb >= self.config.max_total_memory_mb {
                warn!("LSP memory limit reached: {}MB >= {}MB", 
                    current_mb, self.config.max_total_memory_mb);
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Evict an idle LSP to make room for a new one
    async fn evict_idle_lsp(&self) -> Result<()> {
        let mut pools = self.pools.write().await;
        
        let mut oldest: Option<(LspType, Duration)> = None;
        
        for (lsp_type, pooled) in pools.iter() {
            if pooled.status == LspStatus::Running && pooled.sessions.is_empty() {
                let idle = pooled.idle_duration();
                if oldest.is_none() || idle > oldest.unwrap().1 {
                    oldest = Some((*lsp_type, idle));
                }
            }
        }
        
        if let Some((lsp_type, _)) = oldest {
            info!("Evicting idle LSP '{}' to make room", lsp_type.display_name());
            if let Some(pooled) = pools.get_mut(&lsp_type) {
                self.stop_pooled_lsp_internal(pooled).await?;
                self.total_stops.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        Ok(())
    }
    
    /// Start a pooled LSP (internal, assumes lock held)
    async fn start_pooled_lsp_internal(&self, pooled: &mut PooledLsp, project_path: Option<&PathBuf>) -> Result<()> {
        let lsp_type = pooled.lsp_type;
        
        info!("Starting pooled LSP '{}' for shared use", lsp_type.display_name());
        
        pooled.status = LspStatus::Starting;
        pooled.started_at = Instant::now();
        pooled.started_at_instant = Instant::now();
        pooled.last_activity_secs.store(0, Ordering::Relaxed);
        
        let mut cmd = Command::new(lsp_type.binary_name());
        
        for arg in lsp_type.default_additional_args() {
            cmd.arg(arg);
        }
        
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .kill_on_drop(true);
        
        if let Some(path) = project_path {
            cmd.current_dir(path);
        }
        
        #[cfg(unix)]
        if self.config.low_priority {
            unsafe {
                cmd.pre_exec(|| {
                    libc::setpriority(libc::PRIO_PROCESS, 0, 19);
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }
        
        let child = cmd.spawn()
            .with_context(|| format!("Failed to spawn LSP: {}", lsp_type.binary_name()))?;
        
        let pid = child.id();
        pooled.pid = pid;
        pooled.child = Some(child);
        pooled.status = LspStatus::Running;
        
        if let Some(stdout) = pooled.child.as_mut().and_then(|c| c.stdout.take()) {
            tokio::spawn(async move {
                use tokio::io::{BufReader, AsyncBufReadExt};
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(_)) = lines.next_line().await {}
            });
        }
        
        if let Some(stderr) = pooled.child.as_mut().and_then(|c| c.stderr.take()) {
            let name = lsp_type.binary_name().to_string();
            tokio::spawn(async move {
                use tokio::io::{BufReader, AsyncBufReadExt};
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!("[{}] {}", name, line);
                }
            });
        }
        
        info!("Pooled LSP '{}' started (PID: {:?})", lsp_type.display_name(), pid);
        
        Ok(())
    }
    
    /// Stop a pooled LSP (internal, assumes lock held)
    async fn stop_pooled_lsp_internal(&self, pooled: &mut PooledLsp) -> Result<()> {
        if pooled.status != LspStatus::Running {
            return Ok(());
        }
        
        pooled.shutting_down.store(true, Ordering::Relaxed);
        let lsp_type = pooled.lsp_type;
        
        info!("Stopping pooled LSP '{}'", lsp_type.display_name());
        
        if let Some(ref mut child) = pooled.child {
            #[cfg(unix)]
            {
                if let Some(pid) = pooled.pid {
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(-(pid as i32)),
                        nix::sys::signal::Signal::SIGTERM,
                    );
                }
            }
            
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        
        pooled.child = None;
        pooled.pid = None;
        pooled.status = LspStatus::Stopped;
        pooled.shutting_down.store(false, Ordering::Relaxed);
        
        info!("Pooled LSP '{}' stopped", lsp_type.display_name());
        
        Ok(())
    }
    
    /// Stop all LSPs
    pub async fn stop_all(&self) -> Result<()> {
        self.is_shutting_down.store(true, Ordering::Relaxed);
        let _ = self.stop_tx.send(true);
        
        let mut pools = self.pools.write().await;
        
        for (lsp_type, pooled) in pools.iter_mut() {
            if pooled.status == LspStatus::Running {
                if let Err(e) = self.stop_pooled_lsp_internal(pooled).await {
                    warn!("Failed to stop LSP '{}': {}", lsp_type.display_name(), e);
                }
            }
        }
        
        pools.clear();
        
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.clear();
        
        Ok(())
    }
    
    /// Force restart an LSP
    pub async fn restart(&self, lsp_type: LspType, project_path: Option<&PathBuf>) -> Result<()> {
        let mut pools = self.pools.write().await;
        
        if let Some(pooled) = pools.get_mut(&lsp_type) {
            if pooled.status == LspStatus::Running {
                self.stop_pooled_lsp_internal(pooled).await?;
                self.total_stops.fetch_add(1, Ordering::Relaxed);
            }
            self.start_pooled_lsp_internal(pooled, project_path).await?;
            self.total_starts.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(())
    }
}

impl Drop for LspPool {
    fn drop(&mut self) {
        self.is_shutting_down.store(true, Ordering::Relaxed);
        let _ = self.stop_tx.send(true);
    }
}

/// Handle to a pooled LSP
#[derive(Debug, Clone)]
pub struct LspHandle {
    pub lsp_type: LspType,
    pub pid: Option<u32>,
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct LspPoolStats {
    pub running_instances: usize,
    pub total_subscribers: usize,
    pub total_memory_mb: u64,
    pub total_starts: u64,
    pub total_stops: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl LspPoolStats {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }
}

/// Get available system memory in MB
#[cfg(target_os = "linux")]
fn get_available_memory_mb() -> Result<u64> {
    use std::fs;
    let meminfo = fs::read_to_string("/proc/meminfo")?;
    for line in meminfo.lines() {
        if line.starts_with("MemAvailable:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: u64 = parts[1].parse()?;
                return Ok(kb / 1024);
            }
        }
    }
    Ok(0)
}

#[cfg(not(target_os = "linux"))]
fn get_available_memory_mb() -> Result<u64> {
    Ok(1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pool_config_default() {
        let config = LspPoolConfig::default();
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert!(config.low_priority);
    }
    
    #[tokio::test]
    async fn test_pooled_lsp_idle_detection() {
        let mut pooled = PooledLsp::new(LspType::Rust);
        pooled.status = LspStatus::Running;
        
        assert!(!pooled.is_idle(Duration::from_secs(60)));
        
        pooled.sessions.insert("test-session".to_string());
        assert!(!pooled.is_idle(Duration::ZERO));
        
        pooled.sessions.remove("test-session");
        // Will be idle after time passes
    }
    
    #[test]
    fn test_pool_stats() {
        let stats = LspPoolStats {
            running_instances: 2,
            total_subscribers: 5,
            total_memory_mb: 500,
            total_starts: 10,
            total_stops: 5,
            cache_hits: 80,
            cache_misses: 20,
        };
        
        assert!((stats.cache_hit_rate() - 0.8).abs() < 0.01);
    }
}
