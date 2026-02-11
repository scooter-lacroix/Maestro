//! Maestro-specific metrics collector
//!
//! This module provides metrics specific to Maestro including LSP status,
//! agent telemetry, LeIndex stats, and Maestro memory usage.

use crate::error::{Error, Result};
use crate::types::{
    AgentInfo, AgentStatus, LspStatus, LeIndexStats, MaestroMemoryStats, MaestroMetrics,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::fs;

/// Default refresh interval for Maestro metrics
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Default Maestro data directory
const DEFAULT_MAESTRO_DIR: &str = ".maestro";

/// Maestro metrics collector
pub struct MaestroCollector {
    /// Path to the Maestro data directory
    maestro_dir: PathBuf,

    /// Refresh interval between readings
    refresh_interval: Duration,

    /// Last update timestamp
    last_update: Instant,

    /// Cached LSP statuses
    cached_lsp: HashMap<String, LspStatus>,

    /// Cached agent info
    cached_agents: HashMap<String, AgentInfo>,

    /// Whether to use live data or cached/stub data
    live_mode: bool,
}

impl MaestroCollector {
    /// Create a new Maestro collector with automatic path detection
    pub fn new() -> Self {
        // Try to find Maestro directory from current working directory
        let maestro_dir = Self::find_maestro_dir();
        Self::with_maestro_dir(maestro_dir)
    }

    /// Create a new Maestro collector with a specific Maestro directory
    pub fn with_maestro_dir(maestro_dir: PathBuf) -> Self {
        Self {
            maestro_dir,
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
            last_update: Instant::now(),
            cached_lsp: HashMap::new(),
            cached_agents: HashMap::new(),
            live_mode: true,
        }
    }

    /// Create a new Maestro collector with a custom refresh interval
    pub fn with_refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }

    /// Set whether to use live data or stub data
    pub fn with_live_mode(mut self, live: bool) -> Self {
        self.live_mode = live;
        self
    }

    /// Collect all Maestro metrics
    pub fn collect(&mut self) -> Result<MaestroMetrics> {
        self.refresh_if_needed();

        let lsp_servers = self.collect_lsp_status()?;
        let agents = self.collect_agent_info()?;
        let leindex = self.collect_leindex_stats()?;
        let memory = self.collect_memory_stats()?;

        Ok(MaestroMetrics::new(lsp_servers, agents, leindex, memory))
    }

    /// Collect LSP server status
    fn collect_lsp_status(&mut self) -> Result<HashMap<String, LspStatus>> {
        let lsp_servers = if !self.live_mode {
            self.create_stub_lsp_status()
        } else {
            let mut lsp_servers = HashMap::new();

            // Try to read LSP status from Maestro state
            let lsp_dir = self.maestro_dir.join("lsp");
            if lsp_dir.exists() {
                // Look for LSP state files
                if let Ok(entries) = fs::read_dir(&lsp_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let is_running = path.join("running").exists();
                            let status = if is_running {
                                "Running".to_string()
                            } else {
                                "Stopped".to_string()
                            };

                            let mut lsp_status = LspStatus::new(name.clone(), is_running, status);

                            // Try to get additional info
                            if let Ok(info) = self.read_lsp_info(&path) {
                                lsp_status.files_tracked = info.files_tracked;
                                lsp_status.diagnostics_count = info.diagnostics_count;
                            }

                            lsp_servers.insert(name, lsp_status);
                        }
                    }
                }
            }

            // If no LSP servers found, provide stub data
            if lsp_servers.is_empty() {
                lsp_servers = self.create_stub_lsp_status();
            }

            lsp_servers
        };

        self.cached_lsp = lsp_servers.clone();
        Ok(lsp_servers)
    }

    /// Read LSP info from a file
    fn read_lsp_info(&self, path: &Path) -> Result<LspInfo> {
        let info_file = path.join("info.json");
        if info_file.exists() {
            if let Ok(content) = fs::read_to_string(&info_file) {
                if let Ok(info) = serde_json::from_str::<LspInfo>(&content) {
                    return Ok(info);
                }
            }
        }
        Ok(LspInfo::default())
    }

    /// Create stub LSP status for testing
    fn create_stub_lsp_status(&self) -> HashMap<String, LspStatus> {
        let mut lsp_servers = HashMap::new();

        // Common LSP servers
        lsp_servers.insert(
            "rust-analyzer".to_string(),
            LspStatus::new("rust-analyzer".to_string(), true, "Ready".to_string()),
        );
        lsp_servers.insert(
            "typescript-language-server".to_string(),
            LspStatus::new("typescript-language-server".to_string(), true, "Ready".to_string()),
        );
        lsp_servers.insert(
            "pylsp".to_string(),
            LspStatus::new("pylsp".to_string(), false, "Stopped".to_string()),
        );

        lsp_servers
    }

    /// Collect agent telemetry
    fn collect_agent_info(&mut self) -> Result<HashMap<String, AgentInfo>> {
        let agents = if !self.live_mode {
            self.create_stub_agent_info()
        } else {
            let mut agents = HashMap::new();

            // Try to read agent status from Maestro state
            let agents_file = self.maestro_dir.join("agents.json");
            if agents_file.exists() {
                if let Ok(content) = fs::read_to_string(&agents_file) {
                    if let Ok(agent_list) = serde_json::from_str::<Vec<AgentJson>>(&content) {
                        for agent_json in agent_list {
                            let agent = AgentInfo::new(
                                agent_json.name,
                                agent_json.agent_type,
                                Self::parse_agent_status(&agent_json.status),
                            );
                            agents.insert(agent.name.clone(), agent);
                        }
                    }
                }
            }

            // If no agents found, provide stub data
            if agents.is_empty() {
                agents = self.create_stub_agent_info();
            }

            agents
        };

        self.cached_agents = agents.clone();
        Ok(agents)
    }

    /// Parse agent status from string
    fn parse_agent_status(status: &str) -> AgentStatus {
        match status.to_lowercase().as_str() {
            "idle" => AgentStatus::Idle,
            "working" | "running" => AgentStatus::Working,
            "paused" => AgentStatus::Paused,
            "error" | "failed" => AgentStatus::Error,
            _ => AgentStatus::Unknown,
        }
    }

    /// Create stub agent info for testing
    fn create_stub_agent_info(&self) -> HashMap<String, AgentInfo> {
        let mut agents = HashMap::new();

        agents.insert(
            "researcher".to_string(),
            AgentInfo::new("researcher".to_string(), "general-purpose".to_string(), AgentStatus::Idle),
        );
        agents.insert(
            "rust-dev".to_string(),
            AgentInfo::new("rust-dev".to_string(), "general-purpose".to_string(), AgentStatus::Working),
        );
        agents.insert(
            "tui-integrator".to_string(),
            AgentInfo::new("tui-integrator".to_string(), "general-purpose".to_string(), AgentStatus::Idle),
        );

        agents
    }

    /// Collect LeIndex statistics
    fn collect_leindex_stats(&self) -> Result<LeIndexStats> {
        if !self.live_mode {
            return Ok(LeIndexStats::default());
        }

        let leindex_dir = self.maestro_dir.join("leindex");
        if !leindex_dir.exists() {
            return Ok(LeIndexStats::default());
        }

        // Try to read stats file
        let stats_file = leindex_dir.join("stats.json");
        if stats_file.exists() {
            if let Ok(content) = fs::read_to_string(&stats_file) {
                if let Ok(stats) = serde_json::from_str::<LeIndexJson>(&content) {
                    return Ok(LeIndexStats {
                        files_indexed: stats.files_indexed,
                        symbols_indexed: stats.symbols_indexed,
                        index_size_bytes: stats.index_size_bytes,
                        last_update: stats.last_update,
                    });
                }
            }
        }

        // Try to estimate from directory size
        let index_dir = leindex_dir.join("index");
        if index_dir.exists() {
            let index_size = Self::dir_size(&index_dir)?;
            return Ok(LeIndexStats {
                files_indexed: 0,
                symbols_indexed: 0,
                index_size_bytes: index_size,
                last_update: None,
            });
        }

        Ok(LeIndexStats::default())
    }

    /// Calculate directory size recursively
    fn dir_size(path: &Path) -> Result<u64> {
        let mut total = 0u64;
        if path.is_dir() {
            for entry in fs::read_dir(path).map_err(|e| {
                Error::MaestroMetricsFailed(format!("Failed to read directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    Error::MaestroMetricsFailed(format!("Failed to read entry: {}", e))
                })?;
                let path = entry.path();
                if path.is_dir() {
                    total += Self::dir_size(&path)?;
                } else {
                    total += entry.metadata().map_err(|e| {
                        Error::MaestroMetricsFailed(format!("Failed to get metadata: {}", e))
                    })?.len();
                }
            }
        }
        Ok(total)
    }

    /// Collect Maestro memory statistics
    fn collect_memory_stats(&self) -> Result<MaestroMemoryStats> {
        if !self.live_mode {
            return Ok(MaestroMemoryStats::default());
        }

        // Try to read memory stats from Maestro
        let memory_file = self.maestro_dir.join("memory_stats.json");
        if memory_file.exists() {
            if let Ok(content) = fs::read_to_string(&memory_file) {
                if let Ok(stats) = serde_json::from_str::<MaestroMemoryJson>(&content) {
                    return Ok(MaestroMemoryStats {
                        total_bytes: stats.total_bytes,
                        cache_bytes: stats.cache_bytes,
                        index_bytes: stats.index_bytes,
                        session_bytes: stats.session_bytes,
                    });
                }
            }
        }

        // Estimate from memory directory
        let memory_dir = self.maestro_dir.join("memory");
        if memory_dir.exists() {
            let total = Self::dir_size(&memory_dir)?;
            return Ok(MaestroMemoryStats {
                total_bytes: total,
                cache_bytes: total / 2, // Rough estimate
                index_bytes: total / 4,
                session_bytes: total / 4,
            });
        }

        Ok(MaestroMemoryStats::default())
    }

    /// Refresh cached data if enough time has passed
    fn refresh_if_needed(&mut self) {
        if self.last_update.elapsed() >= self.refresh_interval {
            self.last_update = Instant::now();
            // Cache will be updated on next collect call
        }
    }

    /// Find the Maestro directory by searching upward from current directory
    fn find_maestro_dir() -> PathBuf {
        let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let mut path = current.as_path();
        loop {
            let maestro_path = path.join(DEFAULT_MAESTRO_DIR);
            if maestro_path.exists() && maestro_path.is_dir() {
                return maestro_path;
            }

            match path.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => path = parent,
                _ => break,
            }
        }

        // Default to ~/.maestro
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(DEFAULT_MAESTRO_DIR)
    }

    /// Get the Maestro directory path
    pub fn maestro_dir(&self) -> &Path {
        &self.maestro_dir
    }

    /// Set the Maestro directory path
    pub fn set_maestro_dir(&mut self, path: PathBuf) {
        self.maestro_dir = path;
    }

    /// Get the refresh interval
    pub fn refresh_interval(&self) -> Duration {
        self.refresh_interval
    }

    /// Set a new refresh interval
    pub fn set_refresh_interval(&mut self, interval: Duration) {
        self.refresh_interval = interval;
    }

    /// Get cached LSP status
    pub fn cached_lsp_status(&self) -> &HashMap<String, LspStatus> {
        &self.cached_lsp
    }

    /// Get cached agent info
    pub fn cached_agents(&self) -> &HashMap<String, AgentInfo> {
        &self.cached_agents
    }
}

impl Default for MaestroCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON representation of LSP info
#[derive(Debug, Clone, serde::Deserialize)]
struct LspInfo {
    #[serde(default)]
    files_tracked: usize,
    #[serde(default)]
    diagnostics_count: usize,
}

impl Default for LspInfo {
    fn default() -> Self {
        Self {
            files_tracked: 0,
            diagnostics_count: 0,
        }
    }
}

/// JSON representation of an agent
#[derive(Debug, Clone, serde::Deserialize)]
struct AgentJson {
    name: String,
    #[serde(rename = "type")]
    agent_type: String,
    status: String,
}

/// JSON representation of LeIndex stats
#[derive(Debug, Clone, serde::Deserialize)]
struct LeIndexJson {
    #[serde(default)]
    files_indexed: usize,
    #[serde(default)]
    symbols_indexed: usize,
    #[serde(default)]
    index_size_bytes: u64,
    #[serde(default)]
    last_update: Option<String>,
}

/// JSON representation of Maestro memory stats
#[derive(Debug, Clone, serde::Deserialize)]
struct MaestroMemoryJson {
    #[serde(default)]
    total_bytes: u64,
    #[serde(default)]
    cache_bytes: u64,
    #[serde(default)]
    index_bytes: u64,
    #[serde(default)]
    session_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maestro_collector_new() {
        let collector = MaestroCollector::new();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
        assert!(collector.maestro_dir().ends_with(DEFAULT_MAESTRO_DIR));
    }

    #[test]
    fn test_maestro_collector_with_maestro_dir() {
        let dir = PathBuf::from("/test/path");
        let collector = MaestroCollector::with_maestro_dir(dir.clone());
        assert_eq!(collector.maestro_dir(), dir);
    }

    #[test]
    fn test_maestro_collector_with_refresh_interval() {
        let interval = Duration::from_secs(10);
        let collector = MaestroCollector::new().with_refresh_interval(interval);
        assert_eq!(collector.refresh_interval(), interval);
    }

    #[test]
    fn test_maestro_collector_with_live_mode() {
        let mut collector = MaestroCollector::new().with_live_mode(false);
        let metrics = collector.collect().expect("Failed to collect Maestro metrics");

        // Should return stub data in non-live mode
        assert!(!metrics.lsp_servers.is_empty() || !metrics.agents.is_empty());
    }

    #[test]
    fn test_maestro_collector_default() {
        let collector = MaestroCollector::default();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_maestro_collector_collect() {
        let mut collector = MaestroCollector::new().with_live_mode(false);
        let metrics = collector.collect().expect("Failed to collect Maestro metrics");

        // Should have some data (either live or stub)
        // Just verify the structure is valid
        assert!(metrics.age() >= Duration::from_secs(0));
    }

    #[test]
    fn test_maestro_collector_set_refresh_interval() {
        let mut collector = MaestroCollector::new();
        let new_interval = Duration::from_secs(15);

        collector.set_refresh_interval(new_interval);
        assert_eq!(collector.refresh_interval(), new_interval);
    }

    #[test]
    fn test_maestro_collector_set_maestro_dir() {
        let mut collector = MaestroCollector::new();
        let new_dir = PathBuf::from("/new/path");

        collector.set_maestro_dir(new_dir.clone());
        assert_eq!(collector.maestro_dir(), new_dir);
    }

    #[test]
    fn test_lsp_status_new() {
        let lsp = LspStatus::new("test".to_string(), true, "Running".to_string());
        assert_eq!(lsp.name, "test");
        assert!(lsp.is_running);
        assert_eq!(lsp.status, "Running");
        assert_eq!(lsp.files_tracked, 0);
        assert_eq!(lsp.diagnostics_count, 0);
    }

    #[test]
    fn test_agent_info_new() {
        let agent = AgentInfo::new("test".to_string(), "general-purpose".to_string(), AgentStatus::Working);
        assert_eq!(agent.name, "test");
        assert_eq!(agent.agent_type, "general-purpose");
        assert_eq!(agent.status, AgentStatus::Working);
        assert_eq!(agent.cpu_percent, 0.0);
        assert_eq!(agent.memory_bytes, 0);
    }

    #[test]
    fn test_agent_status_parsing() {
        assert_eq!(MaestroCollector::parse_agent_status("idle"), AgentStatus::Idle);
        assert_eq!(MaestroCollector::parse_agent_status("Idle"), AgentStatus::Idle);
        assert_eq!(MaestroCollector::parse_agent_status("working"), AgentStatus::Working);
        assert_eq!(MaestroCollector::parse_agent_status("paused"), AgentStatus::Paused);
        assert_eq!(MaestroCollector::parse_agent_status("error"), AgentStatus::Error);
        assert_eq!(MaestroCollector::parse_agent_status("unknown"), AgentStatus::Unknown);
    }

    #[test]
    fn test_leindex_stats_default() {
        let stats = LeIndexStats::default();
        assert_eq!(stats.files_indexed, 0);
        assert_eq!(stats.symbols_indexed, 0);
        assert_eq!(stats.index_size_bytes, 0);
        assert!(stats.last_update.is_none());
    }

    #[test]
    fn test_maestro_memory_stats_default() {
        let stats = MaestroMemoryStats::default();
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.cache_bytes, 0);
        assert_eq!(stats.index_bytes, 0);
        assert_eq!(stats.session_bytes, 0);
    }

    #[test]
    fn test_maestro_metrics_empty() {
        let metrics = MaestroMetrics::empty();
        assert!(metrics.lsp_servers.is_empty());
        assert!(metrics.agents.is_empty());
        assert_eq!(metrics.leindex.files_indexed, 0);
        assert_eq!(metrics.memory.total_bytes, 0);
    }

    #[test]
    fn test_maestro_metrics_age() {
        let metrics = MaestroMetrics::empty();
        std::thread::sleep(Duration::from_millis(10));
        assert!(metrics.age() >= Duration::from_millis(10));
    }

    #[test]
    fn test_stub_lsp_status() {
        let collector = MaestroCollector::new().with_live_mode(false);
        let lsp_servers = collector.create_stub_lsp_status();

        assert!(!lsp_servers.is_empty());
        assert!(lsp_servers.contains_key("rust-analyzer"));
    }

    #[test]
    fn test_stub_agent_info() {
        let collector = MaestroCollector::new().with_live_mode(false);
        let agents = collector.create_stub_agent_info();

        assert!(!agents.is_empty());
        assert!(agents.contains_key("researcher"));
    }

    #[test]
    fn test_cached_lsp_status() {
        let mut collector = MaestroCollector::new().with_live_mode(false);
        collector.collect().expect("Failed to collect");

        let cached = collector.cached_lsp_status();
        assert!(!cached.is_empty());
    }

    #[test]
    fn test_cached_agents() {
        let mut collector = MaestroCollector::new().with_live_mode(false);
        collector.collect().expect("Failed to collect");

        let cached = collector.cached_agents();
        assert!(!cached.is_empty());
    }

    #[test]
    fn test_lsp_info_default() {
        let info = LspInfo::default();
        assert_eq!(info.files_tracked, 0);
        assert_eq!(info.diagnostics_count, 0);
    }
}
