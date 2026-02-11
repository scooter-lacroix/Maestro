//! Linux /proc filesystem parser for detailed process metrics
//!
//! This module provides parsers for reading additional process information
//! from the /proc filesystem that sysinfo doesn't provide directly.

use crate::error::{Error, Result};
use std::fs;
use std::path::Path;

/// Clock ticks per second (typically 100 on Linux)
const CLOCK_TICKS_PER_SEC: u64 = 100;

/// Detailed memory information from /proc/[pid]/statm
#[derive(Debug, Clone, PartialEq)]
pub struct ProcStatm {
    /// Total virtual memory size in bytes
    pub size_bytes: u64,
    /// Resident set size in bytes
    pub resident_bytes: u64,
    /// Shared memory in bytes
    pub shared_bytes: u64,
    /// Text (code) segment size in bytes
    pub text_bytes: u64,
    /// Data + stack segment size in bytes
    pub data_bytes: u64,
}

/// Detailed CPU information from /proc/[pid]/stat
#[derive(Debug, Clone, PartialEq)]
pub struct ProcStat {
    /// User CPU time in clock ticks
    pub utime_ticks: u64,
    /// System CPU time in clock ticks
    pub stime_ticks: u64,
    /// Total CPU time in clock ticks (utime + stime)
    pub total_ticks: u64,
}

/// Get the system page size in bytes
fn page_size() -> u64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 }
}

/// Parse /proc/[pid]/statm for detailed memory info
pub fn parse_proc_statm(pid: u32) -> Result<ProcStatm> {
    let path = format!("/proc/{}/statm", pid);
    let content = fs::read_to_string(&path)
        .map_err(|e| Error::CollectionFailed(format!("Failed to read {}: {}", path, e)))?;

    let fields: Vec<u64> = content
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();

    if fields.len() < 6 {
        return Err(Error::CollectionFailed(
            "Invalid /proc/[pid]/statm format".to_string(),
        ));
    }

    let page = page_size();
    Ok(ProcStatm {
        size_bytes: fields[0] * page,
        resident_bytes: fields[1] * page,
        shared_bytes: fields[2] * page,
        text_bytes: fields[3] * page,
        data_bytes: fields[5] * page,
    })
}

/// Parse /proc/[pid]/stat for detailed CPU info
pub fn parse_proc_stat(pid: u32) -> Result<ProcStat> {
    let path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&path)
        .map_err(|e| Error::CollectionFailed(format!("Failed to read {}: {}", path, e)))?;

    // Handle process names with parentheses and spaces
    // Format: pid (comm) state utime stime ...
    let last_paren = content.rfind(')').ok_or_else(|| {
        Error::CollectionFailed("Invalid /proc/[pid]/stat format: no closing paren".to_string())
    })?;

    let after_paren = &content[last_paren + 1..];
    let fields: Vec<&str> = after_paren.split_whitespace().collect();

    // utime is field 14 (index 11 after paren), stime is field 15 (index 12)
    if fields.len() < 13 {
        return Err(Error::CollectionFailed(
            "Invalid /proc/[pid]/stat format: not enough fields".to_string(),
        ));
    }

    let utime: u64 = fields[11]
        .parse()
        .map_err(|_| Error::CollectionFailed("Failed to parse utime".to_string()))?;
    let stime: u64 = fields[12]
        .parse()
        .map_err(|_| Error::CollectionFailed("Failed to parse stime".to_string()))?;
    let total_time = utime + stime;

    Ok(ProcStat {
        utime_ticks: utime,
        stime_ticks: stime,
        total_ticks: total_time,
    })
}

/// Check if /proc filesystem is available
pub fn is_proc_available() -> bool {
    Path::new("/proc").exists()
}

/// Convert clock ticks to seconds
pub fn ticks_to_seconds(ticks: u64) -> u64 {
    ticks / CLOCK_TICKS_PER_SEC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticks_to_seconds() {
        assert_eq!(ticks_to_seconds(0), 0);
        assert_eq!(ticks_to_seconds(100), 1);
        assert_eq!(ticks_to_seconds(200), 2);
        assert_eq!(ticks_to_seconds(150), 1);
    }

    #[test]
    fn test_is_proc_available() {
        // On Linux, /proc should be available
        #[cfg(target_os = "linux")]
        assert!(is_proc_available());
    }

    #[test]
    fn test_parse_proc_statm_current_process() {
        // Test parsing our own process
        let pid = std::process::id();
        if is_proc_available() {
            let result = parse_proc_statm(pid);
            assert!(result.is_ok());
            let statm = result.unwrap();
            assert!(statm.resident_bytes > 0);
            assert!(statm.size_bytes >= statm.resident_bytes);
        }
    }

    #[test]
    fn test_parse_proc_stat_current_process() {
        // Test parsing our own process
        let pid = std::process::id();
        if is_proc_available() {
            let result = parse_proc_stat(pid);
            assert!(result.is_ok());
            let stat = result.unwrap();
            assert!(stat.total_ticks >= stat.utime_ticks);
            assert!(stat.total_ticks >= stat.stime_ticks);
        }
    }

    #[test]
    fn test_parse_proc_statm_invalid_pid() {
        let result = parse_proc_statm(99999999);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_proc_stat_invalid_pid() {
        let result = parse_proc_stat(99999999);
        assert!(result.is_err());
    }
}
