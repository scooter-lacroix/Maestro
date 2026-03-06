//! Disk metrics collector
//!
//! This module provides disk usage and I/O statistics.

use crate::error::Result;
use crate::types::{DiskMetrics, DiskMount};
use std::time::{Duration, Instant};
use sysinfo::{DiskKind, Disks};

/// Default refresh interval for disk metrics
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// Minimum duration for I/O speed calculation
const MIN_IO_DURATION: Duration = Duration::from_secs(1);

/// Disk collector
pub struct DiskCollector {
    /// Disks interface for reading disk stats
    disks: Disks,

    /// Previous I/O counters for calculating speed
    previous_read_bytes: u64,
    previous_write_bytes: u64,
    previous_time: Instant,

    /// Refresh interval between readings
    refresh_interval: Duration,

    /// Last update timestamp
    last_update: Instant,

    /// Last time I/O was calculated
    last_io_time: Instant,
}

impl DiskCollector {
    /// Create a new disk collector
    pub fn new() -> Self {
        Self::with_refresh_interval(DEFAULT_REFRESH_INTERVAL)
    }

    /// Create a new disk collector with a custom refresh interval
    pub fn with_refresh_interval(interval: Duration) -> Self {
        let disks = Disks::new_with_refreshed_list();

        // Initialize with current I/O values
        let (read_bytes, write_bytes) = Self::get_total_io(&disks);

        Self {
            disks,
            previous_read_bytes: read_bytes,
            previous_write_bytes: write_bytes,
            previous_time: Instant::now(),
            refresh_interval: interval,
            last_update: Instant::now(),
            last_io_time: Instant::now(),
        }
    }

    /// Collect current disk metrics
    pub fn collect(&mut self) -> Result<DiskMetrics> {
        self.refresh_if_needed();

        // Collect mount information
        let mounts = self.collect_mounts()?;

        // Get I/O statistics
        let (read_bytes, write_bytes) = Self::get_total_io(&self.disks);
        let now = Instant::now();

        // Calculate I/O speeds if enough time has passed
        let (read_speed, write_speed) = if now.duration_since(self.last_io_time) >= MIN_IO_DURATION
        {
            let elapsed = now.duration_since(self.previous_time);
            if elapsed.as_secs_f64() > 0.0 {
                let read_speed = ((read_bytes.saturating_sub(self.previous_read_bytes)) as f64
                    / elapsed.as_secs_f64()) as u64;
                let write_speed = ((write_bytes.saturating_sub(self.previous_write_bytes)) as f64
                    / elapsed.as_secs_f64()) as u64;

                // Update previous values
                self.previous_read_bytes = read_bytes;
                self.previous_write_bytes = write_bytes;
                self.previous_time = now;
                self.last_io_time = now;

                (read_speed, write_speed)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        Ok(DiskMetrics::new(
            mounts,
            read_bytes,
            write_bytes,
            read_speed,
            write_speed,
        ))
    }

    /// Collect mount point information
    fn collect_mounts(&self) -> Result<Vec<DiskMount>> {
        let mut mounts = Vec::new();

        for disk in &self.disks {
            let mount_point = disk.mount_point().to_string_lossy().to_string();
            let device = disk.name().to_string_lossy().to_string();
            let fs_type = Self::disk_type_to_string(&disk.kind());

            let total_bytes = disk.total_space();
            let available_bytes = disk.available_space();
            let used_bytes = total_bytes.saturating_sub(available_bytes);

            let is_read_only = Self::is_read_only(&disk.kind());

            mounts.push(DiskMount::new(
                mount_point,
                device,
                fs_type,
                total_bytes,
                available_bytes,
                used_bytes,
                is_read_only,
            ));
        }

        Ok(mounts)
    }

    /// Convert DiskKind to filesystem type string
    fn disk_type_to_string(kind: &DiskKind) -> String {
        match kind {
            DiskKind::HDD => "HDD".to_string(),
            DiskKind::SSD => "SSD".to_string(),
            DiskKind::Unknown(_) => "Unknown".to_string(),
        }
    }

    /// Check if a disk type is typically read-only
    fn is_read_only(_kind: &DiskKind) -> bool {
        // Most disks are not read-only by default
        false
    }

    /// Get total bytes read and written across all disks
    ///
    /// NOTE: sysinfo crate doesn't provide disk I/O counters.
    /// This returns zeros since implementing /proc/diskstats parsing
    /// would require significant additional code and platform-specific handling.
    /// The disk usage (space used/available) is still correctly reported via mounts.
    fn get_total_io(_disks: &Disks) -> (u64, u64) {
        // sysinfo doesn't provide I/O counters per disk
        // To get actual I/O stats, we would need to:
        // 1. Parse /proc/diskstats on Linux
        // 2. Use platform-specific APIs on macOS/Windows
        // For now, return 0 to avoid reporting incorrect data
        (0, 0)
    }

    /// Collect only mount information (without I/O stats)
    pub fn collect_mounts_only(&mut self) -> Result<Vec<DiskMount>> {
        self.refresh_if_needed();
        self.collect_mounts()
    }

    /// Get total disk space across all mounts
    pub fn total_space(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        let mut total = 0u64;
        for disk in self.disks.iter() {
            total = total.saturating_add(disk.total_space());
        }
        Ok(total)
    }

    /// Get total available space across all mounts
    pub fn total_available(&mut self) -> Result<u64> {
        self.refresh_if_needed();
        let mut total = 0u64;
        for disk in self.disks.iter() {
            total = total.saturating_add(disk.available_space());
        }
        Ok(total)
    }

    /// Refresh system data if enough time has passed
    fn refresh_if_needed(&mut self) {
        if self.last_update.elapsed() >= self.refresh_interval {
            self.disks.refresh();
            self.last_update = Instant::now();
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

    /// Get the number of disks/mounts
    pub fn disk_count(&mut self) -> Result<usize> {
        self.refresh_if_needed();
        Ok(self.disks.iter().count())
    }

    /// Find a mount by path
    pub fn find_mount(&mut self, path: &str) -> Option<DiskMount> {
        self.refresh_if_needed();
        for disk in self.disks.iter() {
            let mount_point = disk.mount_point().to_string_lossy();
            if mount_point == path || path.starts_with(mount_point.as_ref()) {
                let total_bytes = disk.total_space();
                let available_bytes = disk.available_space();
                let used_bytes = total_bytes.saturating_sub(available_bytes);

                return Some(DiskMount::new(
                    mount_point.to_string(),
                    disk.name().to_string_lossy().to_string(),
                    Self::disk_type_to_string(&disk.kind()),
                    total_bytes,
                    available_bytes,
                    used_bytes,
                    Self::is_read_only(&disk.kind()),
                ));
            }
        }
        None
    }
}

impl Default for DiskCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DiskCollector {
    fn clone(&self) -> Self {
        let disks = Disks::new_with_refreshed_list();

        let (read_bytes, write_bytes) = Self::get_total_io(&disks);

        Self {
            disks,
            previous_read_bytes: read_bytes,
            previous_write_bytes: write_bytes,
            previous_time: Instant::now(),
            refresh_interval: self.refresh_interval,
            last_update: Instant::now(),
            last_io_time: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_collector_new() {
        let collector = DiskCollector::new();
        assert_eq!(collector.refresh_interval(), DEFAULT_REFRESH_INTERVAL);
    }

    #[test]
    fn test_disk_collector_collect() {
        let mut collector = DiskCollector::new();
        let metrics = collector.collect().expect("Failed to collect disk metrics");
        assert!(!metrics.mounts.is_empty());
    }

    #[test]
    fn test_get_total_io_returns_zeros() {
        let disks = Disks::new_with_refreshed_list();
        let (read, write) = DiskCollector::get_total_io(&disks);
        // Should return 0 since sysinfo doesn't provide I/O counters
        assert_eq!(read, 0);
        assert_eq!(write, 0);
    }
}
