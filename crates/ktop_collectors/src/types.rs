//! Core data types for system metrics
//!
//! This module defines the data structures used across all collectors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// CPU usage statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// Overall CPU usage percentage (0.0 - 100.0)
    pub usage_percent: f32,

    /// Number of CPU cores
    pub core_count: usize,

    /// Per-core usage percentages
    pub per_core_usage: Vec<f32>,

    /// CPU frequency in MHz
    pub frequency_mhz: Option<f32>,

    /// Load averages (1min, 5min, 15min)
    pub load_average: (f32, f32, f32),

    /// Timestamp of when these metrics were collected
    /// Note: Not serialized due to Instant limitations
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

impl CpuMetrics {
    /// Create new CPU metrics
    pub fn new(
        usage_percent: f32,
        core_count: usize,
        per_core_usage: Vec<f32>,
        frequency_mhz: Option<f32>,
        load_average: (f32, f32, f32),
    ) -> Self {
        Self {
            usage_percent: usage_percent.clamp(0.0, 100.0),
            core_count,
            per_core_usage,
            frequency_mhz,
            load_average,
            timestamp: Instant::now(),
        }
    }

    /// Get age of these metrics
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Memory statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Total RAM in bytes
    pub total_bytes: u64,

    /// Used RAM in bytes
    pub used_bytes: u64,

    /// Available RAM in bytes
    pub available_bytes: u64,

    /// Buffered memory in bytes
    pub buffers_bytes: u64,

    /// Cached memory in bytes
    pub cached_bytes: u64,

    /// Total swap in bytes
    pub swap_total_bytes: u64,

    /// Used swap in bytes
    pub swap_used_bytes: u64,

    /// Timestamp of when these metrics were collected
    /// Note: Not serialized due to Instant limitations
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

impl MemoryMetrics {
    /// Create new memory metrics
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        total_bytes: u64,
        used_bytes: u64,
        available_bytes: u64,
        buffers_bytes: u64,
        cached_bytes: u64,
        swap_total_bytes: u64,
        swap_used_bytes: u64,
    ) -> Self {
        Self {
            total_bytes,
            used_bytes,
            available_bytes,
            buffers_bytes,
            cached_bytes,
            swap_total_bytes,
            swap_used_bytes,
            timestamp: Instant::now(),
        }
    }

    /// Get RAM usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f32 / self.total_bytes as f32) * 100.0
    }

    /// Get swap usage percentage
    pub fn swap_usage_percent(&self) -> f32 {
        if self.swap_total_bytes == 0 {
            return 0.0;
        }
        (self.swap_used_bytes as f32 / self.swap_total_bytes as f32) * 100.0
    }

    /// Get age of these metrics
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Process information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,

    /// Process name
    pub name: String,

    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage percentage
    pub memory_percent: f32,

    /// Resident Set Size (RSS) in bytes
    pub rss_bytes: u64,

    /// Shared memory in bytes
    pub shared_bytes: u64,

    /// Process status
    pub status: ProcessStatus,

    /// Command line
    pub command: Option<String>,

    /// User CPU time in seconds
    pub user_time_seconds: u64,

    /// System CPU time in seconds
    pub system_time_seconds: u64,

    /// Total CPU time in seconds
    pub total_cpu_time: u64,

    /// Virtual memory size in bytes
    pub vms_bytes: u64,

    /// Text (code) segment size in bytes
    pub text_bytes: u64,
}

/// Process status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// Running
    Running,
    /// Sleeping
    Sleeping,
    /// Stopped
    Stopped,
    /// Zombie
    Zombie,
    /// Dead
    Dead,
    /// Unknown status
    Unknown,
}

impl From<sysinfo::ProcessStatus> for ProcessStatus {
    fn from(status: sysinfo::ProcessStatus) -> Self {
        match status {
            sysinfo::ProcessStatus::Run => ProcessStatus::Running,
            sysinfo::ProcessStatus::Sleep => ProcessStatus::Sleeping,
            sysinfo::ProcessStatus::Stop => ProcessStatus::Stopped,
            sysinfo::ProcessStatus::Zombie => ProcessStatus::Zombie,
            sysinfo::ProcessStatus::Dead => ProcessStatus::Dead,
            _ => ProcessStatus::Unknown,
        }
    }
}

impl ProcessInfo {
    /// Create new process info
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pid: u32,
        name: String,
        cpu_percent: f32,
        memory_percent: f32,
        rss_bytes: u64,
        shared_bytes: u64,
        status: ProcessStatus,
        command: Option<String>,
        user_time_seconds: u64,
        system_time_seconds: u64,
        total_cpu_time: u64,
        vms_bytes: u64,
        text_bytes: u64,
    ) -> Self {
        Self {
            pid,
            name,
            cpu_percent: cpu_percent.max(0.0),
            memory_percent: memory_percent.max(0.0),
            rss_bytes,
            shared_bytes,
            status,
            command,
            user_time_seconds,
            system_time_seconds,
            total_cpu_time,
            vms_bytes,
            text_bytes,
        }
    }
}

/// Network interface statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Per-interface statistics
    pub interfaces: HashMap<String, InterfaceStats>,

    /// Total bytes received (across all interfaces)
    pub total_recv_bytes: u64,

    /// Total bytes sent (across all interfaces)
    pub total_sent_bytes: u64,

    /// Calculated download speed (bytes/sec)
    pub download_speed_bps: u64,

    /// Calculated upload speed (bytes/sec)
    pub upload_speed_bps: u64,

    /// Timestamp of when these metrics were collected
    /// Note: Not serialized due to Instant limitations
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

/// Statistics for a single network interface
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceStats {
    /// Interface name
    pub name: String,

    /// Bytes received
    pub recv_bytes: u64,

    /// Bytes sent
    pub sent_bytes: u64,

    /// Packets received
    pub recv_packets: u64,

    /// Packets sent
    pub sent_packets: u64,

    /// Errors on receive
    pub recv_errors: u64,

    /// Errors on send
    pub send_errors: u64,
}

impl InterfaceStats {
    /// Create new interface stats
    pub fn new(name: String) -> Self {
        Self {
            name,
            recv_bytes: 0,
            sent_bytes: 0,
            recv_packets: 0,
            sent_packets: 0,
            recv_errors: 0,
            send_errors: 0,
        }
    }
}

impl NetworkMetrics {
    /// Create new network metrics
    pub fn new(
        interfaces: HashMap<String, InterfaceStats>,
        total_recv_bytes: u64,
        total_sent_bytes: u64,
        download_speed_bps: u64,
        upload_speed_bps: u64,
    ) -> Self {
        Self {
            interfaces,
            total_recv_bytes,
            total_sent_bytes,
            download_speed_bps,
            upload_speed_bps,
            timestamp: Instant::now(),
        }
    }

    /// Get age of these metrics
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Disk usage and I/O statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskMetrics {
    /// Per-mount-point statistics
    pub mounts: Vec<DiskMount>,

    /// Total bytes read
    pub read_bytes: u64,

    /// Total bytes written
    pub write_bytes: u64,

    /// Read speed (bytes/sec)
    pub read_speed_bps: u64,

    /// Write speed (bytes/sec)
    pub write_speed_bps: u64,

    /// Timestamp of when these metrics were collected
    /// Note: Not serialized due to Instant limitations
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

/// Statistics for a single disk mount point
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskMount {
    /// Mount point path
    pub mount_point: String,

    /// Device name
    pub device: String,

    /// File system type
    pub fs_type: String,

    /// Total bytes
    pub total_bytes: u64,

    /// Available bytes
    pub available_bytes: u64,

    /// Used bytes
    pub used_bytes: u64,

    /// Is this a read-only filesystem?
    pub is_read_only: bool,
}

impl DiskMount {
    /// Create new disk mount info
    pub fn new(
        mount_point: String,
        device: String,
        fs_type: String,
        total_bytes: u64,
        available_bytes: u64,
        used_bytes: u64,
        is_read_only: bool,
    ) -> Self {
        Self {
            mount_point,
            device,
            fs_type,
            total_bytes,
            available_bytes,
            used_bytes,
            is_read_only,
        }
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f32 / self.total_bytes as f32) * 100.0
    }
}

impl DiskMetrics {
    /// Create new disk metrics
    pub fn new(
        mounts: Vec<DiskMount>,
        read_bytes: u64,
        write_bytes: u64,
        read_speed_bps: u64,
        write_speed_bps: u64,
    ) -> Self {
        Self {
            mounts,
            read_bytes,
            write_bytes,
            read_speed_bps,
            write_speed_bps,
            timestamp: Instant::now(),
        }
    }

    /// Get age of these metrics
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Maestro-specific metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaestroMetrics {
    /// LSP server status
    pub lsp_servers: HashMap<String, LspStatus>,

    /// Agent telemetry
    pub agents: HashMap<String, AgentInfo>,

    /// LeIndex statistics
    pub leindex: LeIndexStats,

    /// Maestro memory usage
    pub memory: MaestroMemoryStats,

    /// Timestamp of when these metrics were collected
    /// Note: Not serialized due to Instant limitations
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

/// LSP server status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LspStatus {
    /// Server name
    pub name: String,

    /// Is the server running?
    pub is_running: bool,

    /// Server status message
    pub status: String,

    /// Number of files being tracked
    pub files_tracked: usize,

    /// Diagnostics count
    pub diagnostics_count: usize,
}

impl LspStatus {
    /// Create new LSP status
    pub fn new(name: String, is_running: bool, status: String) -> Self {
        Self {
            name,
            is_running,
            status,
            files_tracked: 0,
            diagnostics_count: 0,
        }
    }
}

/// Agent information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent name
    pub name: String,

    /// Agent type
    pub agent_type: String,

    /// Agent status
    pub status: AgentStatus,

    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage in bytes
    pub memory_bytes: u64,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is idle
    Idle,
    /// Agent is working
    Working,
    /// Agent is paused
    Paused,
    /// Agent is in error state
    Error,
    /// Unknown status
    Unknown,
}

impl AgentInfo {
    /// Create new agent info
    pub fn new(name: String, agent_type: String, status: AgentStatus) -> Self {
        Self {
            name,
            agent_type,
            status,
            cpu_percent: 0.0,
            memory_bytes: 0,
        }
    }
}

/// LeIndex statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LeIndexStats {
    /// Number of indexed files
    pub files_indexed: usize,

    /// Number of indexed symbols
    pub symbols_indexed: usize,

    /// Index size in bytes
    pub index_size_bytes: u64,

    /// Last index update time
    pub last_update: Option<String>,
}

/// Maestro memory statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MaestroMemoryStats {
    /// Total memory allocated by Maestro
    pub total_bytes: u64,

    /// Memory used by caches
    pub cache_bytes: u64,

    /// Memory used by indexes
    pub index_bytes: u64,

    /// Memory used by active sessions
    pub session_bytes: u64,
}

impl MaestroMetrics {
    /// Create new Maestro metrics
    pub fn new(
        lsp_servers: HashMap<String, LspStatus>,
        agents: HashMap<String, AgentInfo>,
        leindex: LeIndexStats,
        memory: MaestroMemoryStats,
    ) -> Self {
        Self {
            lsp_servers,
            agents,
            leindex,
            memory,
            timestamp: Instant::now(),
        }
    }

    /// Get age of these metrics
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }

    /// Create empty/default Maestro metrics
    pub fn empty() -> Self {
        Self {
            lsp_servers: HashMap::new(),
            agents: HashMap::new(),
            leindex: LeIndexStats::default(),
            memory: MaestroMemoryStats::default(),
            timestamp: Instant::now(),
        }
    }
}

/// Unified system metrics container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU metrics
    pub cpu: Option<CpuMetrics>,

    /// Memory metrics
    pub memory: Option<MemoryMetrics>,

    /// Top processes by CPU usage
    pub top_cpu_processes: Vec<ProcessInfo>,

    /// Top processes by memory usage
    pub top_memory_processes: Vec<ProcessInfo>,

    /// Network metrics
    pub network: Option<NetworkMetrics>,

    /// Disk metrics
    pub disk: Option<DiskMetrics>,

    /// Maestro-specific metrics
    pub maestro: Option<MaestroMetrics>,

    /// Timestamp of when these metrics were collected
    /// Note: Not serialized due to Instant limitations
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: Instant,
}

impl SystemMetrics {
    /// Create new empty system metrics
    pub fn new() -> Self {
        Self {
            cpu: None,
            memory: None,
            top_cpu_processes: Vec::new(),
            top_memory_processes: Vec::new(),
            network: None,
            disk: None,
            maestro: None,
            timestamp: Instant::now(),
        }
    }

    /// Get age of these metrics
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }

    /// Check if all metrics are populated
    pub fn is_complete(&self) -> bool {
        self.cpu.is_some()
            && self.memory.is_some()
            && self.network.is_some()
            && self.disk.is_some()
            && self.maestro.is_some()
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_metrics_new() {
        let cpu = CpuMetrics::new(
            50.0,
            8,
            vec![10.0, 20.0, 30.0],
            Some(3200.0),
            (1.0, 0.8, 0.5),
        );
        assert_eq!(cpu.usage_percent, 50.0);
        assert_eq!(cpu.core_count, 8);
        assert_eq!(cpu.per_core_usage.len(), 3);
        assert_eq!(cpu.frequency_mhz, Some(3200.0));
        assert_eq!(cpu.load_average, (1.0, 0.8, 0.5));
    }

    #[test]
    fn test_cpu_metrics_clamp() {
        let cpu = CpuMetrics::new(-10.0, 8, vec![150.0], None, (0.0, 0.0, 0.0));
        assert_eq!(cpu.usage_percent, 0.0); // Clamped to 0
    }

    #[test]
    fn test_memory_metrics_usage_percent() {
        let mem = MemoryMetrics::new(16_000_000_000, 8_000_000_000, 8_000_000_000, 0, 0, 0, 0);
        assert_eq!(mem.usage_percent(), 50.0);
    }

    #[test]
    fn test_memory_metrics_zero_total() {
        let mem = MemoryMetrics::new(0, 0, 0, 0, 0, 0, 0);
        assert_eq!(mem.usage_percent(), 0.0);
        assert_eq!(mem.swap_usage_percent(), 0.0);
    }

    #[test]
    fn test_memory_metrics_swap_usage() {
        let mem = MemoryMetrics::new(
            16_000_000_000,
            8_000_000_000,
            8_000_000_000,
            0,
            0,
            4_000_000_000,
            2_000_000_000,
        );
        assert_eq!(mem.swap_usage_percent(), 50.0);
    }

    #[test]
    fn test_process_info_new() {
        let proc = ProcessInfo::new(
            1234,
            "test".to_string(),
            5.5,
            10.0,
            1_000_000,
            500_000,
            ProcessStatus::Running,
            Some("test --arg".to_string()),
            100,
            50,
            150,
            2_000_000,
            64_000,
        );
        assert_eq!(proc.pid, 1234);
        assert_eq!(proc.name, "test");
        assert_eq!(proc.cpu_percent, 5.5);
        assert_eq!(proc.memory_percent, 10.0);
        assert_eq!(proc.status, ProcessStatus::Running);
        assert_eq!(proc.user_time_seconds, 100);
        assert_eq!(proc.system_time_seconds, 50);
        assert_eq!(proc.total_cpu_time, 150);
        assert_eq!(proc.vms_bytes, 2_000_000);
        assert_eq!(proc.text_bytes, 64_000);
    }

    #[test]
    fn test_process_info_clamp_negative_cpu() {
        let proc = ProcessInfo::new(
            1,
            "test".to_string(),
            -5.0,
            -10.0,
            0,
            0,
            ProcessStatus::Unknown,
            None,
            0,
            0,
            0,
            0,
            0,
        );
        assert_eq!(proc.cpu_percent, 0.0);
        assert_eq!(proc.memory_percent, 0.0);
    }

    #[test]
    fn test_interface_stats_new() {
        let iface = InterfaceStats::new("eth0".to_string());
        assert_eq!(iface.name, "eth0");
        assert_eq!(iface.recv_bytes, 0);
        assert_eq!(iface.sent_bytes, 0);
    }

    #[test]
    fn test_network_metrics_new() {
        let mut interfaces = HashMap::new();
        interfaces.insert("eth0".to_string(), InterfaceStats::new("eth0".to_string()));

        let net = NetworkMetrics::new(interfaces, 1000, 500, 100, 50);
        assert_eq!(net.total_recv_bytes, 1000);
        assert_eq!(net.total_sent_bytes, 500);
        assert_eq!(net.download_speed_bps, 100);
        assert_eq!(net.upload_speed_bps, 50);
    }

    #[test]
    fn test_disk_mount_usage_percent() {
        let mount = DiskMount::new(
            "/".to_string(),
            "/dev/sda1".to_string(),
            "ext4".to_string(),
            1000,
            500,
            500,
            false,
        );
        assert_eq!(mount.usage_percent(), 50.0);
    }

    #[test]
    fn test_disk_mount_zero_total() {
        let mount = DiskMount::new(
            "/".to_string(),
            "/dev/sda1".to_string(),
            "ext4".to_string(),
            0,
            0,
            0,
            false,
        );
        assert_eq!(mount.usage_percent(), 0.0);
    }

    #[test]
    fn test_disk_metrics_new() {
        let mounts = vec![DiskMount::new(
            "/".to_string(),
            "/dev/sda1".to_string(),
            "ext4".to_string(),
            1_000_000_000_000,
            500_000_000_000,
            500_000_000_000,
            false,
        )];

        let disk = DiskMetrics::new(mounts, 1024, 2048, 100, 200);
        assert_eq!(disk.read_bytes, 1024);
        assert_eq!(disk.write_bytes, 2048);
        assert_eq!(disk.mounts.len(), 1);
    }

    #[test]
    fn test_lsp_status_new() {
        let lsp = LspStatus::new("rust-analyzer".to_string(), true, "Ready".to_string());
        assert_eq!(lsp.name, "rust-analyzer");
        assert!(lsp.is_running);
        assert_eq!(lsp.status, "Ready");
        assert_eq!(lsp.files_tracked, 0);
        assert_eq!(lsp.diagnostics_count, 0);
    }

    #[test]
    fn test_agent_info_new() {
        let agent = AgentInfo::new(
            "researcher".to_string(),
            "general-purpose".to_string(),
            AgentStatus::Working,
        );
        assert_eq!(agent.name, "researcher");
        assert_eq!(agent.agent_type, "general-purpose");
        assert_eq!(agent.status, AgentStatus::Working);
        assert_eq!(agent.cpu_percent, 0.0);
        assert_eq!(agent.memory_bytes, 0);
    }

    #[test]
    fn test_maestro_metrics_empty() {
        let maestro = MaestroMetrics::empty();
        assert!(maestro.lsp_servers.is_empty());
        assert!(maestro.agents.is_empty());
        assert_eq!(maestro.leindex.files_indexed, 0);
        assert_eq!(maestro.memory.total_bytes, 0);
    }

    #[test]
    fn test_system_metrics_new() {
        let metrics = SystemMetrics::new();
        assert!(metrics.cpu.is_none());
        assert!(metrics.memory.is_none());
        assert!(metrics.top_cpu_processes.is_empty());
        assert!(metrics.top_memory_processes.is_empty());
        assert!(metrics.network.is_none());
        assert!(metrics.disk.is_none());
        assert!(metrics.maestro.is_none());
    }

    #[test]
    fn test_system_metrics_is_complete() {
        let mut metrics = SystemMetrics::new();
        assert!(!metrics.is_complete());

        metrics.cpu = Some(CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0)));
        assert!(!metrics.is_complete());

        metrics.memory = Some(MemoryMetrics::new(
            16_000_000_000,
            8_000_000_000,
            8_000_000_000,
            0,
            0,
            0,
            0,
        ));
        assert!(!metrics.is_complete());

        metrics.network = Some(NetworkMetrics::new(HashMap::new(), 0, 0, 0, 0));
        assert!(!metrics.is_complete());

        metrics.disk = Some(DiskMetrics::new(vec![], 0, 0, 0, 0));
        assert!(!metrics.is_complete());

        metrics.maestro = Some(MaestroMetrics::empty());
        assert!(metrics.is_complete());
    }

    #[test]
    fn test_system_metrics_default() {
        let metrics = SystemMetrics::default();
        assert!(metrics.cpu.is_none());
        assert!(metrics.memory.is_none());
    }

    #[test]
    fn test_process_status_from_sysinfo() {
        use sysinfo::ProcessStatus as SysStatus;

        assert_eq!(ProcessStatus::from(SysStatus::Run), ProcessStatus::Running);
        assert_eq!(
            ProcessStatus::from(SysStatus::Sleep),
            ProcessStatus::Sleeping
        );
        assert_eq!(ProcessStatus::from(SysStatus::Stop), ProcessStatus::Stopped);
        assert_eq!(
            ProcessStatus::from(SysStatus::Zombie),
            ProcessStatus::Zombie
        );
        assert_eq!(ProcessStatus::from(SysStatus::Dead), ProcessStatus::Dead);
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
    fn test_metrics_age() {
        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(cpu.age() >= std::time::Duration::from_millis(10));
    }
}
