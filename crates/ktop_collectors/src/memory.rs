//! Memory metrics collector
//!
//! This module provides RAM and swap memory metrics.

use crate::error::Result;
use crate::types::MemoryMetrics;
use std::time::Duration;
use sysinfo::System;

/// Default refresh interval for memory metrics
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

/// Memory collector
pub struct MemoryCollector {
    /// System interface for reading memory stats
    system: System,

    /// Refresh interval between readings
    refresh_interval: Duration,

    /// Last update timestamp
    last_update: std::time::Instant,
}

impl MemoryCollector {
    /// Create a new memory collector
    pub fn new() -> Self {
        Self::with_refresh_interval(DEFAULT_REFRESH_INTERVAL)
    }

    /// Create a new memory collector with a custom refresh interval
    pub fn with_refresh_interval(interval: Duration) -> Self {
        let mut system = System::new();
        system.refresh_memory(); // Initial refresh

        Self {
            system,
            refresh_interval: interval,
            last_update: std::time::Instant::now(),
        }
    }

    /// Collect current memory metrics
    pub fn collect(&mut self) -> Result<MemoryMetrics> {
        self.refresh_if_needed();

        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let available_memory = self.system.available_memory();

        // sysinfo doesn't directly provide buffers/cache
        // We'll calculate available as an approximation
        let buffers_bytes = 0;
        let cached_bytes = 0;

        let swap_total = self.system.total_swap();
        let swap_used = self.system.used_swap();

        Ok(MemoryMetrics::new(
            total_memory,
            used_memory,
            available_memory,
            buffers_bytes,
            cached_bytes,
            swap_total,
            swap_used,
        ))
    }

    /// Collect only RAM usage percentage
    pub fn collect_usage_percent(&mut self) -> Result<f32> {
        self.refresh_if_needed();
        let total = self.system.total_memory();
        if total == 0 {
            return Ok(0.0);
        }
        Ok((self.system.used_memory() as f32 / total as f32) * 100.0)
    }

    /// Collect only swap usage percentage
    pub fn collect_swap_usage_percent(&mut self) -> Result<f32> {
        self.refresh_if_needed();
        let total = self.system.total_swap();
        if total == 0 {
            return Ok(0.0);
        }
        Ok((self.system.used_swap() as f32 / total as f32) * 100.0)
    }

    /// Get total memory in bytes
    pub fn total_memory(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        Ok(self.system.total_memory())
    }

    /// Get available memory in bytes
    pub fn available_memory(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        Ok(self.system.available_memory())
    }

    /// Get total swap in bytes
    pub fn total_swap(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        Ok(self.system.total_swap())
    }

    /// Get used swap in bytes
    pub fn used_swap(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        Ok(self.system.used_swap())
    }

    /// Refresh system data if enough time has passed
    fn refresh_if_needed(&mut self) {
        if self.last_update.elapsed() >= self.refresh_interval {
            self.system.refresh_memory();
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
}

impl Default for MemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemoryCollector {
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
    fn test_memory_collector_new() {
        let collector = MemoryCollector::new();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_memory_collector_with_custom_interval() {
        let interval = Duration::from_secs(5);
        let collector = MemoryCollector::with_refresh_interval(interval);
        assert_eq!(collector.refresh_interval(), interval);
    }

    #[test]
    fn test_memory_collector_default() {
        let collector = MemoryCollector::default();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_memory_collector_collect() {
        let mut collector = MemoryCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");

        // Total memory should be positive on any real system
        assert!(metrics.total_bytes > 0);
        // Used + available should be approximately total
        assert!(metrics.used_bytes <= metrics.total_bytes);
        assert!(metrics.available_bytes <= metrics.total_bytes);
    }

    #[test]
    fn test_memory_collector_collect_usage_percent() {
        let mut collector = MemoryCollector::new();
        let usage = collector
            .collect_usage_percent()
            .expect("Failed to collect memory usage");

        assert!((0.0..=100.0).contains(&usage));
    }

    #[test]
    fn test_memory_collector_collect_swap_usage_percent() {
        let mut collector = MemoryCollector::new();
        let swap_usage = collector
            .collect_swap_usage_percent()
            .expect("Failed to collect swap usage");

        assert!((0.0..=100.0).contains(&swap_usage));
    }

    #[test]
    fn test_memory_collector_total_memory() {
        let mut collector = MemoryCollector::new();
        let total = collector
            .total_memory()
            .expect("Failed to get total memory");

        assert!(total > 0);
    }

    #[test]
    fn test_memory_collector_available_memory() {
        let mut collector = MemoryCollector::new();
        let available = collector
            .available_memory()
            .expect("Failed to get available memory");

        assert!(available > 0);
    }

    #[test]
    fn test_memory_collector_total_swap() {
        let mut collector = MemoryCollector::new();
        let total_swap = collector.total_swap().expect("Failed to get total swap");

        // Swap may be 0 if not configured
        assert!(total_swap >= 0);
    }

    #[test]
    fn test_memory_collector_used_swap() {
        let mut collector = MemoryCollector::new();
        let used_swap = collector.used_swap().expect("Failed to get used swap");

        // Used swap should be <= total swap (if any)
        let total_swap = collector.total_swap().expect("Failed to get total swap");
        assert!(used_swap <= total_swap);
    }

    #[test]
    fn test_memory_collector_set_refresh_interval() {
        let mut collector = MemoryCollector::new();
        let new_interval = Duration::from_secs(10);

        collector.set_refresh_interval(new_interval);
        assert_eq!(collector.refresh_interval(), new_interval);
    }

    #[test]
    fn test_memory_metrics_timestamp() {
        let mut collector = MemoryCollector::new();
        let before = std::time::Instant::now();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");
        let after = std::time::Instant::now();

        assert!(metrics.timestamp >= before);
        assert!(metrics.timestamp <= after);
    }

    #[test]
    fn test_memory_metrics_age() {
        let mut collector = MemoryCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");

        std::thread::sleep(Duration::from_millis(10));
        assert!(metrics.age() >= Duration::from_millis(10));
    }

    #[test]
    fn test_memory_metrics_usage_percent() {
        let mut collector = MemoryCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");

        let usage = metrics.usage_percent();
        assert!((0.0..=100.0).contains(&usage));
    }

    #[test]
    fn test_memory_metrics_swap_usage_percent() {
        let mut collector = MemoryCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");

        let swap_usage = metrics.swap_usage_percent();
        assert!((0.0..=100.0).contains(&swap_usage));
    }

    #[test]
    fn test_memory_metrics_zero_total() {
        let metrics = MemoryMetrics::new(0, 0, 0, 0, 0, 0, 0);
        assert_eq!(metrics.usage_percent(), 0.0);
        assert_eq!(metrics.swap_usage_percent(), 0.0);
    }

    #[test]
    fn test_memory_collector_multiple_collections() {
        let mut collector = MemoryCollector::new();

        // First collection
        let metrics1 = collector
            .collect()
            .expect("Failed to collect memory metrics");
        assert!(metrics1.total_bytes > 0);

        // Wait to ensure refresh interval passes
        std::thread::sleep(Duration::from_secs(3));

        // Second collection
        let metrics2 = collector
            .collect()
            .expect("Failed to collect memory metrics");
        assert!(metrics2.total_bytes > 0);

        // Total memory should be consistent
        assert_eq!(metrics1.total_bytes, metrics2.total_bytes);
    }

    #[test]
    fn test_clone() {
        let collector1 = MemoryCollector::with_refresh_interval(Duration::from_secs(5));
        let collector2 = collector1.clone();

        assert_eq!(collector1.refresh_interval(), collector2.refresh_interval());
    }

    #[test]
    fn test_memory_consistency() {
        let mut collector = MemoryCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");

        // Used + Available should be approximately equal to total
        let used_plus_available = metrics.used_bytes + metrics.available_bytes;
        // Allow for some rounding differences
        assert!(
            used_plus_available <= metrics.total_bytes * 105 / 100
                && used_plus_available >= metrics.total_bytes * 95 / 100,
            "Used + Available ({}) should be approximately equal to Total ({})",
            used_plus_available,
            metrics.total_bytes
        );
    }

    #[test]
    fn test_swap_less_than_total() {
        let mut collector = MemoryCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect memory metrics");

        // Used swap should not exceed total swap
        assert!(metrics.swap_used_bytes <= metrics.swap_total_bytes);
    }
}
