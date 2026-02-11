//! CPU metrics collector
//!
//! This module provides CPU usage, load average, and frequency metrics.

use crate::error::Result;
use crate::types::CpuMetrics;
use sysinfo::{System, CpuRefreshKind, RefreshKind};
use std::time::Duration;

/// Default refresh interval for CPU metrics
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

/// CPU collector
pub struct CpuCollector {
    /// System interface for reading CPU stats
    system: System,

    /// Refresh interval between readings
    refresh_interval: Duration,

    /// Last update timestamp
    last_update: std::time::Instant,
}

impl CpuCollector {
    /// Create a new CPU collector
    pub fn new() -> Self {
        Self::with_refresh_interval(DEFAULT_REFRESH_INTERVAL)
    }

    /// Create a new CPU collector with a custom refresh interval
    pub fn with_refresh_interval(interval: Duration) -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything())
        );
        system.refresh_cpu_all(); // Initial refresh

        Self {
            system,
            refresh_interval: interval,
            last_update: std::time::Instant::now(),
        }
    }

    /// Collect current CPU metrics
    pub fn collect(&mut self) -> Result<CpuMetrics> {
        self.refresh_if_needed();

        let usage_percent = self.system.global_cpu_usage();
        let core_count = self.system.cpus().len();
        let per_core_usage: Vec<f32> = self.system.cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect();

        // Get CPU frequency if available
        let frequency_mhz = self.system.cpus().first().and_then(|cpu| {
            let freq = cpu.frequency();
            if freq > 0 {
                Some(freq as f32)
            } else {
                None
            }
        });

        // Load averages (sysinfo doesn't provide this directly on all platforms)
        // We'll use a simple calculation based on CPU usage
        let load_avg = self.calculate_load_average(usage_percent, core_count);

        Ok(CpuMetrics::new(
            usage_percent,
            core_count,
            per_core_usage,
            frequency_mhz,
            load_avg,
        ))
    }

    /// Collect only CPU usage (lightweight operation)
    pub fn collect_usage(&mut self) -> Result<f32> {
        self.refresh_if_needed();
        Ok(self.system.global_cpu_usage())
    }

    /// Get per-core CPU usage
    pub fn collect_per_core(&mut self) -> Result<Vec<f32>> {
        self.refresh_if_needed();
        Ok(self.system.cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect())
    }

    /// Refresh system data if enough time has passed
    fn refresh_if_needed(&mut self) {
        if self.last_update.elapsed() >= self.refresh_interval {
            self.system.refresh_cpu_all();
            self.last_update = std::time::Instant::now();
        }
    }

    /// Calculate a simple load average estimate
    ///
    /// Note: This is an approximation since sysinfo doesn't provide
    /// load averages on all platforms. For accurate load averages,
    /// consider using the `libc` getloadavg() function directly.
    fn calculate_load_average(&self, usage_percent: f32, core_count: usize) -> (f32, f32, f32) {
        // Convert usage percentage to an estimated load
        let load = (usage_percent / 100.0) * core_count as f32;

        // Simulate 1, 5, 15 minute averages with decay
        let load_1min = load;
        let load_5min = load * 0.9; // Assume slight decay
        let load_15min = load * 0.8; // Assume more decay

        (load_1min, load_5min, load_15min)
    }

    /// Get the number of CPU cores
    pub fn core_count(&self) -> usize {
        self.system.cpus().len()
    }

    /// Get the refresh interval
    pub fn refresh_interval(&self) -> Duration {
        self.refresh_interval
    }

    /// Set a new refresh interval
    pub fn set_refresh_interval(&mut self, interval: Duration) {
        self.refresh_interval = interval;
    }
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Prevent serde serialization for Instant
impl Clone for CpuCollector {
    fn clone(&self) -> Self {
        Self {
            system: System::new(),
            refresh_interval: self.refresh_interval,
            last_update: std::time::Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_collector_new() {
        let collector = CpuCollector::new();
        assert!(collector.core_count() > 0);
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_cpu_collector_with_custom_interval() {
        let interval = Duration::from_millis(1000);
        let collector = CpuCollector::with_refresh_interval(interval);
        assert_eq!(collector.refresh_interval(), interval);
    }

    #[test]
    fn test_cpu_collector_default() {
        let collector = CpuCollector::default();
        assert!(collector.core_count() > 0);
    }

    #[test]
    fn test_cpu_collector_collect() {
        let mut collector = CpuCollector::new();
        let metrics = collector.collect().expect("Failed to collect CPU metrics");

        assert!(metrics.usage_percent >= 0.0 && metrics.usage_percent <= 100.0);
        assert!(metrics.core_count > 0);
        assert_eq!(metrics.per_core_usage.len(), metrics.core_count);
    }

    #[test]
    fn test_cpu_collector_collect_usage() {
        let mut collector = CpuCollector::new();
        let usage = collector.collect_usage().expect("Failed to collect CPU usage");

        assert!(usage >= 0.0 && usage <= 100.0);
    }

    #[test]
    fn test_cpu_collector_collect_per_core() {
        let mut collector = CpuCollector::new();
        let per_core = collector.collect_per_core().expect("Failed to collect per-core usage");

        assert!(!per_core.is_empty());
        assert!(per_core.iter().all(|&u| u >= 0.0 && u <= 100.0));
    }

    #[test]
    fn test_cpu_collector_set_refresh_interval() {
        let mut collector = CpuCollector::new();
        let new_interval = Duration::from_millis(2000);

        collector.set_refresh_interval(new_interval);
        assert_eq!(collector.refresh_interval(), new_interval);
    }

    #[test]
    fn test_cpu_metrics_timestamp() {
        let mut collector = CpuCollector::new();
        let before = std::time::Instant::now();
        let metrics = collector.collect().expect("Failed to collect CPU metrics");
        let after = std::time::Instant::now();

        assert!(metrics.timestamp >= before);
        assert!(metrics.timestamp <= after);
    }

    #[test]
    fn test_cpu_metrics_age() {
        let mut collector = CpuCollector::new();
        let metrics = collector.collect().expect("Failed to collect CPU metrics");

        std::thread::sleep(Duration::from_millis(10));
        assert!(metrics.age() >= Duration::from_millis(10));
    }

    #[test]
    fn test_calculate_load_average() {
        let collector = CpuCollector::new();
        let (load1, load5, load15) = collector.calculate_load_average(50.0, 8);

        // Load should be proportional to usage and core count
        assert!(load1 > 0.0);
        assert!(load5 <= load1); // Should decay over time
        assert!(load15 <= load5); // Should decay more over time
    }

    #[test]
    fn test_calculate_load_average_zero_usage() {
        let collector = CpuCollector::new();
        let (load1, load5, load15) = collector.calculate_load_average(0.0, 4);

        assert_eq!(load1, 0.0);
        assert_eq!(load5, 0.0);
        assert_eq!(load15, 0.0);
    }

    #[test]
    fn test_calculate_load_average_full_usage() {
        let collector = CpuCollector::new();
        let (load1, load5, load15) = collector.calculate_load_average(100.0, 4);

        assert_eq!(load1, 4.0); // 100% usage on 4 cores = load of 4
        assert!(load5 < load1);
        assert!(load15 < load5);
    }

    #[test]
    fn test_cpu_collector_multiple_collections() {
        let mut collector = CpuCollector::new();

        // First collection
        let metrics1 = collector.collect().expect("Failed to collect CPU metrics");
        assert!(metrics1.usage_percent >= 0.0);

        // Wait to ensure refresh interval passes
        std::thread::sleep(Duration::from_millis(600));

        // Second collection
        let metrics2 = collector.collect().expect("Failed to collect CPU metrics");
        assert!(metrics2.usage_percent >= 0.0);
    }

    #[test]
    fn test_cpu_collector_frequency_present() {
        let mut collector = CpuCollector::new();
        let metrics = collector.collect().expect("Failed to collect CPU metrics");

        // Frequency may be present or None depending on platform
        // Just verify it's either Some positive value or None
        if let Some(freq) = metrics.frequency_mhz {
            assert!(freq > 0.0);
        }
    }

    #[test]
    fn test_per_core_usage_matches_count() {
        let mut collector = CpuCollector::new();
        let metrics = collector.collect().expect("Failed to collect CPU metrics");

        assert_eq!(metrics.per_core_usage.len(), metrics.core_count);
    }

    #[test]
    fn test_clone() {
        let collector1 = CpuCollector::with_refresh_interval(Duration::from_millis(500));
        let collector2 = collector1.clone();

        assert_eq!(collector1.refresh_interval(), collector2.refresh_interval());
        // Note: system state and last_update will differ after clone
    }

    // Unit tests for error handling
    #[test]
    fn test_error_display() {
        let err = crate::error::Error::CpuReadFailed("test error".to_string());
        assert_eq!(format!("{}", err), "Failed to read CPU metrics: test error");
    }
}
