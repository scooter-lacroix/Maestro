//! Process metrics collector
//!
//! This module provides process information including CPU and memory usage.

use crate::error::Result;
use crate::proc_parser::{is_proc_available, parse_proc_stat, parse_proc_statm, ticks_to_seconds};
use crate::types::{ProcessInfo, ProcessStatus};
use std::time::Duration;
use sysinfo::{Pid, Process, System};

/// Default refresh interval for process metrics
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Maximum number of processes to return in top lists
const DEFAULT_TOP_N: usize = 10;

/// Process collector
pub struct ProcessCollector {
    /// System interface for reading process stats
    system: System,

    /// Refresh interval between readings
    refresh_interval: Duration,

    /// Last update timestamp
    last_update: std::time::Instant,

    /// Number of top processes to return
    top_n: usize,
}

impl ProcessCollector {
    /// Create a new process collector
    pub fn new() -> Self {
        Self::with_refresh_interval(DEFAULT_REFRESH_INTERVAL)
    }

    /// Create a new process collector with a custom refresh interval
    pub fn with_refresh_interval(interval: Duration) -> Self {
        let mut system = System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true); // Initial refresh

        Self {
            system,
            refresh_interval: interval,
            last_update: std::time::Instant::now(),
            top_n: DEFAULT_TOP_N,
        }
    }

    /// Set the number of top processes to return
    pub fn with_top_n(mut self, top_n: usize) -> Self {
        self.top_n = top_n.max(1);
        self
    }

    /// Collect all processes
    pub fn collect(&mut self) -> Result<Vec<ProcessInfo>> {
        self.refresh_if_needed();

        let total_memory = self.system.total_memory();
        let mut processes = Vec::new();

        for process in self.system.processes().values() {
            let info = self.process_to_info(process, total_memory);
            processes.push(info);
        }

        Ok(processes)
    }

    /// Collect top N processes by CPU usage
    pub fn collect_top_by_cpu(&mut self) -> Result<Vec<ProcessInfo>> {
        let mut processes = self.collect()?;
        processes.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        processes.truncate(self.top_n);
        Ok(processes)
    }

    /// Collect top N processes by memory usage
    pub fn collect_top_by_memory(&mut self) -> Result<Vec<ProcessInfo>> {
        let mut processes = self.collect()?;
        processes.sort_by(|a, b| {
            b.memory_percent
                .partial_cmp(&a.memory_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        processes.truncate(self.top_n);
        Ok(processes)
    }

    /// Collect both top CPU and top memory processes
    pub fn collect_top_both(&mut self) -> Result<(Vec<ProcessInfo>, Vec<ProcessInfo>)> {
        let top_cpu = self.collect_top_by_cpu()?;
        let top_memory = self.collect_top_by_memory()?;
        Ok((top_cpu, top_memory))
    }

    /// Get process count
    pub fn process_count(&mut self) -> Result<usize> {
        self.refresh_if_needed();
        Ok(self.system.processes().len())
    }

    /// Convert a sysinfo Process to our ProcessInfo
    fn process_to_info(&self, process: &Process, total_memory: u64) -> ProcessInfo {
        let pid = process.pid().as_u32();
        let name = process.name().to_string_lossy().to_string();
        let memory_percent = if total_memory > 0 {
            (process.memory() as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        let cpu_percent = process.cpu_usage();

        let status = ProcessStatus::from(process.status());

        let command = process
            .exe()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string());

        // Read additional /proc data if available (Linux only)
        let (user_time, system_time, total_cpu_time, vms_bytes, shared_bytes_actual, text_bytes) =
            if is_proc_available() {
                let cpu_info = parse_proc_stat(pid)
                    .map(|s| {
                        (
                            ticks_to_seconds(s.utime_ticks),
                            ticks_to_seconds(s.stime_ticks),
                            ticks_to_seconds(s.total_ticks),
                        )
                    })
                    .unwrap_or((0, 0, 0));

                let mem_info = parse_proc_statm(pid)
                    .map(|m| (m.size_bytes, m.shared_bytes, m.text_bytes))
                    .unwrap_or((0, 0, 0));

                (
                    cpu_info.0, cpu_info.1, cpu_info.2, mem_info.0, mem_info.1, mem_info.2,
                )
            } else {
                (0, 0, 0, 0, 0, 0)
            };

        ProcessInfo::new(
            pid,
            name,
            cpu_percent,
            memory_percent,
            process.memory(),
            shared_bytes_actual,
            status,
            command,
            user_time,
            system_time,
            total_cpu_time,
            vms_bytes,
            text_bytes,
        )
    }

    /// Refresh system data if enough time has passed
    fn refresh_if_needed(&mut self) {
        if self.last_update.elapsed() >= self.refresh_interval {
            self.system
                .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            self.last_update = std::time::Instant::now();
        }
    }

    /// Get the refresh interval
    pub fn refresh_interval(&self) -> Duration {
        self.refresh_interval
    }

    /// Set a new refresh interval
    pub fn set_refresh_interval(&mut self, interval: Duration) {
        self.refresh_interval = interval;
    }

    /// Find a process by PID
    pub fn find_by_pid(&mut self, pid: u32) -> Option<ProcessInfo> {
        self.refresh_if_needed();

        let total_memory = self.system.total_memory();
        self.system
            .process(Pid::from_u32(pid))
            .map(|p| self.process_to_info(&p, total_memory))
    }

    /// Find processes by name
    pub fn find_by_name(&mut self, name: &str) -> Result<Vec<ProcessInfo>> {
        self.refresh_if_needed();

        let total_memory = self.system.total_memory();
        let mut processes = Vec::new();

        for process in self.system.processes().values() {
            if process.name() == name {
                processes.push(self.process_to_info(process, total_memory));
            }
        }

        Ok(processes)
    }
}

impl Default for ProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ProcessCollector {
    fn clone(&self) -> Self {
        Self {
            system: System::new(),
            refresh_interval: self.refresh_interval,
            last_update: std::time::Instant::now(),
            top_n: self.top_n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo::ProcessStatus as SysProcessStatus;

    #[test]
    fn test_process_collector_new() {
        let collector = ProcessCollector::new();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
        assert_eq!(collector.top_n, DEFAULT_TOP_N);
    }

    #[test]
    fn test_process_collector_with_custom_interval() {
        let interval = Duration::from_secs(10);
        let collector = ProcessCollector::with_refresh_interval(interval);
        assert_eq!(collector.refresh_interval(), interval);
    }

    #[test]
    fn test_process_collector_with_top_n() {
        let collector = ProcessCollector::new().with_top_n(20);
        assert_eq!(collector.top_n, 20);
    }

    #[test]
    fn test_process_collector_with_top_n_minimum() {
        let collector = ProcessCollector::new().with_top_n(0);
        assert_eq!(collector.top_n, 1); // Should clamp to 1
    }

    #[test]
    fn test_process_collector_default() {
        let collector = ProcessCollector::default();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_process_collector_collect() {
        let mut collector = ProcessCollector::new();
        let processes = collector.collect().expect("Failed to collect processes");

        // Should always have at least one process (the current process)
        assert!(!processes.is_empty());
    }

    #[test]
    fn test_process_collector_collect_top_by_cpu() {
        let mut collector = ProcessCollector::new();
        let top_cpu = collector
            .collect_top_by_cpu()
            .expect("Failed to collect top CPU processes");

        // Should have at most top_n processes
        assert!(top_cpu.len() <= DEFAULT_TOP_N);

        // Should be sorted by CPU usage (descending)
        for i in 1..top_cpu.len() {
            assert!(
                top_cpu[i - 1].cpu_percent >= top_cpu[i].cpu_percent,
                "Process {} should have >= CPU usage than process {}",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn test_process_collector_collect_top_by_memory() {
        let mut collector = ProcessCollector::new();
        let top_mem = collector
            .collect_top_by_memory()
            .expect("Failed to collect top memory processes");

        // Should have at most top_n processes
        assert!(top_mem.len() <= DEFAULT_TOP_N);

        // Should be sorted by memory usage (descending)
        for i in 1..top_mem.len() {
            assert!(
                top_mem[i - 1].memory_percent >= top_mem[i].memory_percent,
                "Process {} should have >= memory usage than process {}",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn test_process_collector_collect_top_both() {
        let mut collector = ProcessCollector::new();
        let (top_cpu, top_mem) = collector
            .collect_top_both()
            .expect("Failed to collect top processes");

        // Both should have at most top_n processes
        assert!(top_cpu.len() <= DEFAULT_TOP_N);
        assert!(top_mem.len() <= DEFAULT_TOP_N);
    }

    #[test]
    fn test_process_collector_process_count() {
        let mut collector = ProcessCollector::new();
        let count = collector
            .process_count()
            .expect("Failed to get process count");

        // Should have at least one process
        assert!(count > 0);
    }

    #[test]
    fn test_process_collector_set_refresh_interval() {
        let mut collector = ProcessCollector::new();
        let new_interval = Duration::from_secs(15);

        collector.set_refresh_interval(new_interval);
        assert_eq!(collector.refresh_interval(), new_interval);
    }

    #[test]
    fn test_process_info_valid_values() {
        let mut collector = ProcessCollector::new();
        let processes = collector.collect().expect("Failed to collect processes");

        for proc in processes {
            // All values should be non-negative where applicable
            assert!(proc.cpu_percent >= 0.0);
            assert!(proc.memory_percent >= 0.0);
            assert!(proc.rss_bytes >= 0);
            assert!(proc.shared_bytes >= 0);
            assert!(proc.pid > 0);
            // Note: Some kernel processes may have empty names, so we don't assert on name
        }
    }

    #[test]
    fn test_process_collector_find_by_pid() {
        let mut collector = ProcessCollector::new();
        let processes = collector.collect().expect("Failed to collect processes");

        if let Some(first_proc) = processes.first() {
            let found = collector.find_by_pid(first_proc.pid);
            assert!(found.is_some());

            let found_proc = found.unwrap();
            assert_eq!(found_proc.pid, first_proc.pid);
        }
    }

    #[test]
    fn test_process_collector_find_by_name() {
        let mut collector = ProcessCollector::new();
        let processes = collector.collect().expect("Failed to collect processes");

        if let Some(first_proc) = processes.first() {
            let found = collector
                .find_by_name(&first_proc.name)
                .expect("Failed to find by name");

            // Should find at least the process we searched for
            assert!(!found.is_empty());
            assert!(found.iter().any(|p| p.name == first_proc.name));
        }
    }

    #[test]
    fn test_process_collector_find_nonexistent_pid() {
        let mut collector = ProcessCollector::new();
        let found = collector.find_by_pid(999999);
        assert!(found.is_none());
    }

    #[test]
    fn test_process_collector_find_nonexistent_name() {
        let mut collector = ProcessCollector::new();
        let found = collector
            .find_by_name("nonexistent_process_name_xyz123")
            .expect("Failed to find by name");
        assert!(found.is_empty());
    }

    #[test]
    fn test_clone() {
        let collector1 = ProcessCollector::with_refresh_interval(Duration::from_secs(5));
        let collector2 = collector1.clone();

        assert_eq!(collector1.refresh_interval(), collector2.refresh_interval());
        assert_eq!(collector1.top_n, collector2.top_n);
    }

    #[test]
    fn test_process_status_conversion() {
        // Test all known status conversions
        assert_eq!(
            ProcessStatus::from(SysProcessStatus::Run),
            ProcessStatus::Running
        );
        assert_eq!(
            ProcessStatus::from(SysProcessStatus::Sleep),
            ProcessStatus::Sleeping
        );
        assert_eq!(
            ProcessStatus::from(SysProcessStatus::Stop),
            ProcessStatus::Stopped
        );
        assert_eq!(
            ProcessStatus::from(SysProcessStatus::Zombie),
            ProcessStatus::Zombie
        );
        assert_eq!(
            ProcessStatus::from(SysProcessStatus::Dead),
            ProcessStatus::Dead
        );
    }

    #[test]
    fn test_memory_percent_calculation() {
        let mut collector = ProcessCollector::new();
        let processes = collector.collect().expect("Failed to collect processes");

        for proc in processes {
            // Memory percent should be reasonable (0-100% typically)
            // Some processes may report > 100% due to shared memory, but
            // our calculation should keep it reasonable
            assert!(proc.memory_percent >= 0.0);
        }
    }

    #[test]
    fn test_top_processes_sorted() {
        let mut collector = ProcessCollector::new();
        let (top_cpu, top_mem) = collector
            .collect_top_both()
            .expect("Failed to collect top processes");

        // Verify CPU list is sorted
        for i in 1..top_cpu.len() {
            assert!(
                top_cpu[i - 1].cpu_percent >= top_cpu[i].cpu_percent,
                "CPU list not properly sorted at index {}",
                i
            );
        }

        // Verify memory list is sorted
        for i in 1..top_mem.len() {
            assert!(
                top_mem[i - 1].memory_percent >= top_mem[i].memory_percent,
                "Memory list not properly sorted at index {}",
                i
            );
        }
    }

    #[test]
    fn test_multiple_collections() {
        let mut collector = ProcessCollector::new();

        // First collection
        let processes1 = collector.collect().expect("Failed to collect processes");
        assert!(!processes1.is_empty());

        // Second collection
        let processes2 = collector.collect().expect("Failed to collect processes");
        assert!(!processes2.is_empty());
    }
}
