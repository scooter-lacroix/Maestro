//! ktop_collectors - System metrics collectors for Maestro TUI
//!
//! This crate provides collectors for system metrics including CPU, memory,
//! processes, network, disk, and Maestro-specific metrics. It uses TDD
//! (Test-Driven Development) principles and provides reactive state management
//! with delta update optimization.
//!
//! # Example
//!
//! ```no_run
//! use ktop_collectors::{CpuCollector, MemoryCollector, ProcessCollector};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut cpu_collector = CpuCollector::new();
//!     let mut mem_collector = MemoryCollector::new();
//!     let mut proc_collector = ProcessCollector::new();
//!
//!     let cpu_metrics = cpu_collector.collect()?;
//!     let mem_metrics = mem_collector.collect()?;
//!     let (top_cpu, top_mem) = proc_collector.collect_top_both()?;
//!
//!     println!("CPU: {}% across {} cores", cpu_metrics.usage_percent, cpu_metrics.core_count);
//!     println!("Memory: {}% used", mem_metrics.usage_percent());
//!
//!     Ok(())
//! }
//! ```

pub mod cpu;
pub mod disk;
pub mod error;
pub mod maestro;
pub mod memory;
pub mod network;
pub mod proc_parser;
pub mod process;
pub mod state;
pub mod types;

// Re-export commonly used types
pub use cpu::CpuCollector;
pub use disk::DiskCollector;
pub use error::{Error, Result};
pub use maestro::MaestroCollector;
pub use memory::MemoryCollector;
pub use network::NetworkCollector;
pub use process::ProcessCollector;
pub use state::{MetricsState, StateUpdate, UpdateFlags};
pub use types::{
    AgentInfo, AgentStatus, CpuMetrics, DiskMetrics, DiskMount, InterfaceStats, LeIndexStats,
    LspStatus, MaestroMemoryStats, MaestroMetrics, MemoryMetrics, NetworkMetrics, ProcessInfo,
    ProcessStatus, SystemMetrics,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default refresh intervals for each collector type
pub mod intervals {
    use std::time::Duration;

    /// Default CPU refresh interval (500ms)
    pub const CPU: Duration = Duration::from_millis(500);

    /// Default memory refresh interval (2s)
    pub const MEMORY: Duration = Duration::from_secs(2);

    /// Default process refresh interval (5s)
    pub const PROCESS: Duration = Duration::from_secs(5);

    /// Default network refresh interval (1s)
    pub const NETWORK: Duration = Duration::from_secs(1);

    /// Default disk refresh interval (5s)
    pub const DISK: Duration = Duration::from_secs(5);

    /// Default Maestro metrics refresh interval (5s)
    pub const MAESTRO: Duration = Duration::from_secs(5);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_intervals_defined() {
        assert!(intervals::CPU.as_millis() > 0);
        assert!(intervals::MEMORY.as_secs() > 0);
        assert!(intervals::PROCESS.as_secs() > 0);
        assert!(intervals::NETWORK.as_secs() > 0);
        assert!(intervals::DISK.as_secs() > 0);
        assert!(intervals::MAESTRO.as_secs() > 0);
    }
}
