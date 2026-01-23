//! Orchestrate session state management
//!
//! Crash-safe persistence with locking.

use crate::orchestrate::model::*;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// State directory manager
#[derive(Clone)]
pub struct StateManager {
    data_dir: PathBuf,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        // Ensure directory exists and propagate errors
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create orchestrate state directory: {:?}", data_dir))?;
        Ok(Self { data_dir })
    }

    /// Get state directory for a specific track
    pub fn track_state_dir(&self, track_id: &str) -> PathBuf {
        self.data_dir.join(track_id)
    }

    /// Ensure track state directory exists
    pub fn ensure_track_dir(&self, track_id: &str) -> Result<PathBuf> {
        let dir = self.track_state_dir(track_id);
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create track state dir: {:?}", dir))?;
        Ok(dir)
    }

    /// Get lock file path for a track
    pub fn lock_file_path(&self, track_id: &str) -> PathBuf {
        self.track_state_dir(track_id).join("lock")
    }

    /// Get session state file path
    pub fn session_file_path(&self, track_id: &str) -> PathBuf {
        self.track_state_dir(track_id).join("session.json")
    }

    /// Get iteration log file path
    pub fn log_file_path(&self, track_id: &str) -> PathBuf {
        self.track_state_dir(track_id).join("iterations.jsonl")
    }

    /// Acquire a session lock
    pub fn acquire_lock(&self, track_id: &str) -> Result<SessionLock> {
        let lock_path = self.lock_file_path(track_id);
        self.ensure_track_dir(track_id)?;

        // Check for existing lock
        if lock_path.exists() {
            let existing = self.read_lock_file(&lock_path)?;
            if Self::is_lock_valid(&existing) {
                return Err(anyhow::anyhow!(
                    "Track {} is already locked by another session (started {})",
                    track_id,
                    existing.started_at
                ));
            }
            // Lock is stale, will be overwritten
        }

        // Create new lock
        let lock = SessionLock {
            track_id: track_id.to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now().to_rfc3339(),
            pid: std::process::id(),
            hostname: hostname::get()
                .ok()
<<<<<<< HEAD
                .and_then(|h: std::ffi::OsString| h.into_string().ok())
=======
                .and_then(|h| h.to_str().map(|s| s.to_string()))
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
                .unwrap_or_else(|| "unknown".to_string()),
        };

        self.write_lock_file(&lock_path, &lock)?;

        Ok(lock)
    }

    /// Release the session lock
    pub fn release_lock(&self, track_id: &str, lock: &SessionLock) -> Result<()> {
        let lock_path = self.lock_file_path(track_id);

        // Verify we own the lock
        if lock_path.exists() {
            let existing = self.read_lock_file(&lock_path)?;
            if existing.session_id == lock.session_id {
                fs::remove_file(&lock_path)?;
            }
        }

        Ok(())
    }

    /// Load session state
    pub fn load_session(&self, track_id: &str) -> Result<Option<SessionState>> {
        let session_path = self.session_file_path(track_id);

        if !session_path.exists() {
            return Ok(None);
        }

        let file = File::open(&session_path)?;
        let reader = BufReader::new(file);
        let state: SessionState = serde_json::from_reader(reader)?;

        Ok(Some(state))
    }

    /// Save session state
    pub fn save_session(&self, state: &SessionState) -> Result<()> {
        let track_id = &state.track_id;
        self.ensure_track_dir(track_id)?;

        let session_path = self.session_file_path(track_id);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&session_path)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &state)?;

        Ok(())
    }

    /// Append iteration log entry
    pub fn append_iteration_log(&self, track_id: &str, log: &IterationLog) -> Result<()> {
        self.ensure_track_dir(track_id)?;

        let log_path = self.log_file_path(track_id);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&log_path)?;

        let line = serde_json::to_string(log)?;
        writeln!(file, "{}", line)?;

        Ok(())
    }

    /// Load all iteration logs for a track
    pub fn load_iteration_logs(&self, track_id: &str) -> Result<Vec<IterationLog>> {
        let log_path = self.log_file_path(track_id);

        if !log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&log_path)?;
        let reader = BufReader::new(file);

        let mut logs = Vec::new();
        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if let Ok(log) = serde_json::from_str::<IterationLog>(&line) {
                logs.push(log);
            }
        }

        Ok(logs)
    }

    /// Get recent N iterations for context
    pub fn recent_iterations(&self, track_id: &str, n: usize) -> Result<Vec<IterationLog>> {
        let all_logs = self.load_iteration_logs(track_id)?;
        let start = if all_logs.len() > n { all_logs.len() - n } else { 0 };
        Ok(all_logs[start..].to_vec())
    }

    fn read_lock_file(&self, path: &Path) -> Result<SessionLock> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let lock: SessionLock = serde_json::from_reader(reader)?;
        Ok(lock)
    }

    fn write_lock_file(&self, path: &Path, lock: &SessionLock) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, lock)?;

        Ok(())
    }

    /// Check if a lock is still valid (process running and recent)
    fn is_lock_valid(lock: &SessionLock) -> bool {
        // First check if we're on the same hostname
        let current_hostname = hostname::get()
            .ok()
<<<<<<< HEAD
            .and_then(|h: std::ffi::OsString| h.into_string().ok())
=======
            .and_then(|h| h.to_str().map(|s| s.to_string()))
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
            .unwrap_or_else(|| "unknown".to_string());

        if current_hostname != lock.hostname {
            // Lock is from a different host - don't consider it stale
            // This prevents concurrent sessions on different machines from conflicting
            return true;
        }

        // Check if process is still running (same host only)
        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("ps")
                .arg("-p")
                .arg(lock.pid.to_string())
                .output();
            if let Ok(output) = output {
                if output.status.success() {
                    // Process is running, check if lock is recent (< 1 hour)
                    if let Ok(started) = chrono::DateTime::parse_from_rfc3339(&lock.started_at) {
                        let age = Utc::now() - started.with_timezone(&Utc);
                        return age.num_hours() < 1;
                    }
                }
            }
        }
        false
    }
}

/// Session lock file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLock {
    pub track_id: String,
    pub session_id: String,
    pub started_at: String,
    pub pid: u32,
    pub hostname: String,
}

impl SessionLock {
    /// Create a new lock
    pub fn new(track_id: String) -> Self {
        Self {
            track_id,
            session_id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now().to_rfc3339(),
            pid: std::process::id(),
            hostname: hostname::get()
                .ok()
<<<<<<< HEAD
                .and_then(|h: std::ffi::OsString| h.into_string().ok())
=======
                .and_then(|h| h.to_str().map(|s| s.to_string()))
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }
}

/// RAII guard for session locks
///
/// Automatically releases the lock when dropped.
pub struct LockGuard {
    state_manager: StateManager,
    track_id: String,
    lock: Option<SessionLock>,
}

impl LockGuard {
    /// Create a new lock guard
    pub fn acquire(state_manager: StateManager, track_id: &str) -> Result<Self> {
        let lock = state_manager.acquire_lock(track_id)?;
        Ok(Self {
            state_manager,
            track_id: track_id.to_string(),
            lock: Some(lock),
        })
    }

    /// Get the lock
    pub fn get(&self) -> Option<&SessionLock> {
        self.lock.as_ref()
    }

    /// Explicitly release the lock
    pub fn release(mut self) {
        if let Some(lock) = self.lock.take() {
            let _ = self.state_manager.release_lock(&self.track_id, &lock);
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            let _ = self.state_manager.release_lock(&self.track_id, &lock);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_state_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf());

        // Test lock acquisition
        let lock = manager.acquire_lock("test-track").unwrap();
        assert_eq!(lock.track_id, "test-track");

        // Test duplicate lock prevention
        let result = manager.acquire_lock("test-track");
        assert!(result.is_err());

        // Test lock release
        manager.release_lock("test-track", &lock).unwrap();

        // Test new lock after release
        let lock2 = manager.acquire_lock("test-track").unwrap();
        assert_ne!(lock.session_id, lock2.session_id);

        manager.release_lock("test-track", &lock2).unwrap();
    }

    #[test]
    fn test_session_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path().to_path_buf());

        let state = SessionState {
            track_id: "test-track".to_string(),
            mode: LoopMode::Building,
            agent_config: AgentConfig::default(),
            current_iteration: 5,
            current_task_id: Some("task-1".to_string()),
            started_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            status: SessionStatus::Running,
        };

        manager.save_session(&state).unwrap();

        let loaded = manager.load_session("test-track").unwrap().unwrap();
        assert_eq!(loaded.track_id, "test-track");
        assert_eq!(loaded.current_iteration, 5);
    }
}
