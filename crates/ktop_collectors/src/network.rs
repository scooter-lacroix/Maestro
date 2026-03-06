//! Network metrics collector
//!
//! This module provides network interface statistics and bandwidth calculations.

use crate::error::Result;
use crate::types::{InterfaceStats, NetworkMetrics};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use sysinfo::Networks;

/// Default refresh interval for network metrics
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// Network collector
pub struct NetworkCollector {
    /// System interface for reading network stats
    networks: Networks,

    /// Previous network counters for calculating speed
    previous_recv: u64,
    previous_sent: u64,
    previous_time: Instant,

    /// Refresh interval between readings
    refresh_interval: Duration,

    /// Last update timestamp
    last_update: Instant,

    /// Auto-scaling max speed for percentage calculations
    max_observed_speed: u64,

    /// Minimum duration for speed calculation
    min_calc_duration: Duration,
}

impl NetworkCollector {
    /// Create a new network collector
    pub fn new() -> Self {
        Self::with_refresh_interval(DEFAULT_REFRESH_INTERVAL)
    }

    /// Create a new network collector with a custom refresh interval
    pub fn with_refresh_interval(interval: Duration) -> Self {
        let networks = Networks::new_with_refreshed_list();

        // Initialize with current values
        let (recv, sent) = Self::get_total_bytes(&networks);

        Self {
            networks,
            previous_recv: recv,
            previous_sent: sent,
            previous_time: Instant::now(),
            refresh_interval: interval,
            last_update: Instant::now(),
            max_observed_speed: 1, // Start with minimum to avoid division by zero
            min_calc_duration: Duration::from_millis(100),
        }
    }

    /// Collect current network metrics
    pub fn collect(&mut self) -> Result<NetworkMetrics> {
        self.refresh_if_needed();

        let (total_recv, total_sent) = Self::get_total_bytes(&self.networks);
        let now = Instant::now();

        // Calculate speeds
        let elapsed = now.duration_since(self.previous_time);
        let (download_speed, upload_speed) = if elapsed >= self.min_calc_duration {
            let elapsed_secs = elapsed.as_secs_f64();
            let dl = if elapsed_secs > 0.0 {
                ((total_recv - self.previous_recv) as f64 / elapsed_secs) as u64
            } else {
                0
            };
            let ul = if elapsed_secs > 0.0 {
                ((total_sent - self.previous_sent) as f64 / elapsed_secs) as u64
            } else {
                0
            };

            // Update max observed speed for auto-scaling
            self.max_observed_speed = self.max_observed_speed.max(dl).max(ul);

            // Store current values for next calculation
            self.previous_recv = total_recv;
            self.previous_sent = total_sent;
            self.previous_time = now;

            (dl, ul)
        } else {
            // Not enough time elapsed, return 0 speeds
            (0, 0)
        };

        // Collect per-interface stats
        let interfaces = self.collect_interfaces()?;

        Ok(NetworkMetrics::new(
            interfaces,
            total_recv,
            total_sent,
            download_speed,
            upload_speed,
        ))
    }

    /// Collect per-interface statistics
    fn collect_interfaces(&self) -> Result<HashMap<String, InterfaceStats>> {
        let mut interfaces = HashMap::new();

        for (name, data) in &self.networks {
            let stats = InterfaceStats {
                name: name.clone(),
                recv_bytes: data.received(),
                sent_bytes: data.transmitted(),
                recv_packets: data.packets_received(),
                sent_packets: data.packets_transmitted(),
                recv_errors: data.errors_on_received(),
                send_errors: data.errors_on_transmitted(),
            };
            interfaces.insert(name.clone(), stats);
        }

        Ok(interfaces)
    }

    /// Get total bytes received and sent across all interfaces
    fn get_total_bytes(networks: &Networks) -> (u64, u64) {
        let mut total_recv = 0u64;
        let mut total_sent = 0u64;

        for data in networks.iter() {
            total_recv += data.1.received();
            total_sent += data.1.transmitted();
        }

        (total_recv, total_sent)
    }

    /// Collect only interface statistics (without speed calculation)
    pub fn collect_interfaces_only(&mut self) -> Result<HashMap<String, InterfaceStats>> {
        self.refresh_if_needed();
        self.collect_interfaces()
    }

    /// Get current download speed in bytes/sec
    pub fn download_speed(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        let (total_recv, _total_sent) = Self::get_total_bytes(&self.networks);
        let now = Instant::now();

        let elapsed = now.duration_since(self.previous_time);
        if elapsed >= self.min_calc_duration {
            let elapsed_secs = elapsed.as_secs_f64();
            let speed = if elapsed_secs > 0.0 {
                ((total_recv - self.previous_recv) as f64 / elapsed_secs) as u64
            } else {
                0
            };

            // Update for next call
            self.previous_recv = total_recv;
            self.previous_time = now;
            self.max_observed_speed = self.max_observed_speed.max(speed);

            Ok(speed)
        } else {
            Ok(0)
        }
    }

    /// Get current upload speed in bytes/sec
    pub fn upload_speed(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        let (_total_recv, total_sent) = Self::get_total_bytes(&self.networks);
        let now = Instant::now();

        let elapsed = now.duration_since(self.previous_time);
        if elapsed >= self.min_calc_duration {
            let elapsed_secs = elapsed.as_secs_f64();
            let speed = if elapsed_secs > 0.0 {
                ((total_sent - self.previous_sent) as f64 / elapsed_secs) as u64
            } else {
                0
            };

            // Update for next call
            self.previous_sent = total_sent;
            self.previous_time = now;
            self.max_observed_speed = self.max_observed_speed.max(speed);

            Ok(speed)
        } else {
            Ok(0)
        }
    }

    /// Get the max observed speed (for auto-scaling)
    pub fn max_observed_speed(&self) -> u64 {
        self.max_observed_speed
    }

    /// Reset the max observed speed
    pub fn reset_max_observed_speed(&mut self) {
        self.max_observed_speed = 1;
    }

    /// Refresh system data if enough time has passed
    fn refresh_if_needed(&mut self) {
        if self.last_update.elapsed() >= self.refresh_interval {
            self.networks.refresh();
            self.last_update = Instant::now();
        }
    }

    /// Get the list of network interface names
    pub fn interface_names(&mut self) -> Result<Vec<String>> {
        self.refresh_if_needed();
        Ok(self.networks.keys().cloned().collect())
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

impl Default for NetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for NetworkCollector {
    fn clone(&self) -> Self {
        let networks = Networks::new_with_refreshed_list();

        let (recv, sent) = Self::get_total_bytes(&networks);

        Self {
            networks,
            previous_recv: recv,
            previous_sent: sent,
            previous_time: Instant::now(),
            refresh_interval: self.refresh_interval,
            last_update: Instant::now(),
            max_observed_speed: 1,
            min_calc_duration: self.min_calc_duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_collector_new() {
        let collector = NetworkCollector::new();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
        assert_eq!(collector.max_observed_speed(), 1);
    }

    #[test]
    fn test_network_collector_with_custom_interval() {
        let interval = Duration::from_secs(2);
        let collector = NetworkCollector::with_refresh_interval(interval);
        assert_eq!(collector.refresh_interval(), interval);
    }

    #[test]
    fn test_network_collector_default() {
        let collector = NetworkCollector::default();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_network_collector_collect() {
        let mut collector = NetworkCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect network metrics");

        // Values should be non-negative
        assert!(metrics.total_recv_bytes >= 0);
        assert!(metrics.total_sent_bytes >= 0);
        assert!(metrics.download_speed_bps >= 0);
        assert!(metrics.upload_speed_bps >= 0);
    }

    #[test]
    fn test_network_collector_collect_interfaces_only() {
        let mut collector = NetworkCollector::new();
        let interfaces = collector
            .collect_interfaces_only()
            .expect("Failed to collect interfaces");

        // Should have at least one network interface (typically lo)
        assert!(!interfaces.is_empty());
    }

    #[test]
    fn test_network_collector_interface_names() {
        let mut collector = NetworkCollector::new();
        let names = collector
            .interface_names()
            .expect("Failed to get interface names");

        // Should have at least one network interface
        assert!(!names.is_empty());
    }

    #[test]
    fn test_network_collector_download_speed() {
        let mut collector = NetworkCollector::new();
        let speed = collector
            .download_speed()
            .expect("Failed to get download speed");

        // Should be non-negative
        assert!(speed >= 0);
    }

    #[test]
    fn test_network_collector_upload_speed() {
        let mut collector = NetworkCollector::new();
        let speed = collector
            .upload_speed()
            .expect("Failed to get upload speed");

        // Should be non-negative
        assert!(speed >= 0);
    }

    #[test]
    fn test_network_collector_set_refresh_interval() {
        let mut collector = NetworkCollector::new();
        let new_interval = Duration::from_secs(5);

        collector.set_refresh_interval(new_interval);
        assert_eq!(collector.refresh_interval(), new_interval);
    }

    #[test]
    fn test_network_collector_reset_max_observed_speed() {
        let mut collector = NetworkCollector::new();
        collector.max_observed_speed = 1000;

        collector.reset_max_observed_speed();
        assert_eq!(collector.max_observed_speed(), 1);
    }

    #[test]
    fn test_network_metrics_timestamp() {
        let mut collector = NetworkCollector::new();
        let before = std::time::Instant::now();
        let metrics = collector
            .collect()
            .expect("Failed to collect network metrics");
        let after = std::time::Instant::now();

        assert!(metrics.timestamp >= before);
        assert!(metrics.timestamp <= after);
    }

    #[test]
    fn test_network_metrics_age() {
        let mut collector = NetworkCollector::new();
        let metrics = collector
            .collect()
            .expect("Failed to collect network metrics");

        std::thread::sleep(Duration::from_millis(10));
        assert!(metrics.age() >= Duration::from_millis(10));
    }

    #[test]
    fn test_interface_stats_valid() {
        let mut collector = NetworkCollector::new();
        let interfaces = collector
            .collect_interfaces_only()
            .expect("Failed to collect interfaces");

        for (name, stats) in interfaces {
            assert_eq!(name, stats.name);
            // Stats should be non-negative
            assert!(stats.recv_bytes >= 0);
            assert!(stats.sent_bytes >= 0);
            assert!(stats.recv_packets >= 0);
            assert!(stats.sent_packets >= 0);
            assert!(stats.recv_errors >= 0);
            assert!(stats.send_errors >= 0);
        }
    }

    #[test]
    fn test_total_bytes_consistent() {
        let mut collector = NetworkCollector::new();
        let interfaces = collector
            .collect_interfaces_only()
            .expect("Failed to collect interfaces");

        // Calculate total from interfaces
        let mut total_recv = 0u64;
        for stats in interfaces.values() {
            total_recv += stats.recv_bytes;
        }

        // Get total from collector
        let metrics = collector
            .collect()
            .expect("Failed to collect network metrics");

        // Should be approximately equal (might differ due to timing)
        assert!(
            (total_recv as i64 - metrics.total_recv_bytes as i64).abs() < 10000,
            "Total recv differs significantly: {} vs {}",
            total_recv,
            metrics.total_recv_bytes
        );
    }

    #[test]
    fn test_network_collector_multiple_collections() {
        let mut collector = NetworkCollector::new();

        // First collection
        let metrics1 = collector
            .collect()
            .expect("Failed to collect network metrics");
        assert!(metrics1.total_recv_bytes >= 0);

        // Second collection
        let metrics2 = collector
            .collect()
            .expect("Failed to collect network metrics");
        assert!(metrics2.total_recv_bytes >= 0);

        // Second collection should have >= bytes than first
        assert!(metrics2.total_recv_bytes >= metrics1.total_recv_bytes);
    }

    #[test]
    fn test_clone() {
        let collector1 = NetworkCollector::with_refresh_interval(Duration::from_secs(5));
        let collector2 = collector1.clone();

        assert_eq!(collector1.refresh_interval(), collector2.refresh_interval());
    }

    #[test]
    fn test_network_has_loopback() {
        let mut collector = NetworkCollector::new();
        let names = collector
            .interface_names()
            .expect("Failed to get interface names");

        // Most systems have a loopback interface named "lo" or similar
        let has_lo = names.iter().any(|n| n == "lo" || n.starts_with("lo"));
        assert!(
            has_lo,
            "Expected to find loopback interface, got: {:?}",
            names
        );
    }

    #[test]
    fn test_interface_stats_new() {
        let stats = InterfaceStats::new("eth0".to_string());
        assert_eq!(stats.name, "eth0");
        assert_eq!(stats.recv_bytes, 0);
        assert_eq!(stats.sent_bytes, 0);
        assert_eq!(stats.recv_packets, 0);
        assert_eq!(stats.sent_packets, 0);
        assert_eq!(stats.recv_errors, 0);
        assert_eq!(stats.send_errors, 0);
    }

    #[test]
    fn test_speed_calculation_requires_time() {
        let mut collector = NetworkCollector::new();

        // Immediate collection might return 0 speeds
        let _metrics1 = collector
            .collect()
            .expect("Failed to collect network metrics");

        // Wait for minimum calculation duration
        std::thread::sleep(Duration::from_millis(200));

        let metrics2 = collector
            .collect()
            .expect("Failed to collect network metrics");

        // Speeds should be valid (even if 0 for no activity)
        assert!(metrics2.download_speed_bps >= 0);
        assert!(metrics2.upload_speed_bps >= 0);
    }
}
