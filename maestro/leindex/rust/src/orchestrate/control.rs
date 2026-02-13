//! Bidirectional control channel between conductor and orchestrate engine
//!
//! This module provides a file-based communication mechanism:
//! - `control.json`: Conductor writes commands, engine reads them
//! - `events.jsonl`: Engine writes structured events, conductor reads them

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

/// Orchestrate execution mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Auto-pilot mode (agent runs autonomously)
    Auto,
    /// Interactive mode (requires user confirmation for major actions)
    Interactive,
    /// Dry run mode (show what would happen without executing)
    DryRun,
}

/// Control command from conductor to engine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlCommand {
    /// Retry the current task
    Retry { task_id: String, iteration: u64 },
    /// Skip the current task
    Skip { task_id: String, iteration: u64 },
    /// Abort the orchestrate session
    Abort { reason: Option<String> },
    /// Override error strategy for a task
    SetErrorStrategy { strategy: ErrorStrategyValue },
    /// Inject steering text into the next prompt
    Steer(String),
    /// Update the maximum iteration limit
    SetMaxIterations(usize),
    /// Toggle between execution modes (Auto, Interactive, DryRun)
    SetMode(Mode),
    /// Change the active agent by name
    SwitchAgent(String),
    /// Enable/disable sandbox mode
    ToggleSandbox,
    /// Enable/disable dangerous mode (bypass safety)
    ToggleDangerous,
}

/// Error strategy values (subset of full ErrorStrategy for JSON)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategyValue {
    Retry,
    Skip,
    Abort,
}

/// Control file state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFile {
    /// Commands from conductor
    pub commands: Vec<ControlCommand>,
    /// Last updated timestamp
    pub updated_at: String,
    /// Conductor session ID (for validation)
    pub session_id: Option<String>,
}

impl Default for ControlFile {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            updated_at: Utc::now().to_rfc3339(),
            session_id: None,
        }
    }
}

/// Structured event from engine to conductor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// Task selection started
    Selecting { iteration: u64, timestamp: String },
    /// Task execution started
    Executing {
        iteration: u64,
        task_id: String,
        task_title: String,
        timestamp: String,
    },
    /// Task failed with retry/skip/abort decision point
    TaskFailed {
        iteration: u64,
        task_id: String,
        error: String,
        can_retry: bool,
        can_skip: bool,
        retry_count: u32,
        max_retries: u32,
        timestamp: String,
    },
    /// Task is being retried with backoff
    TaskRetrying {
        iteration: u64,
        task_id: String,
        retry_attempt: u32,
        max_retries: u32,
        delay_ms: u64,
        error: String,
        timestamp: String,
    },
    /// Rate limit detected
    RateLimited {
        task_id: String,
        retry_count: u32,
        backoff_until: Option<u64>,
        timestamp: String,
    },
    /// Progress update
    Progress {
        iteration: u64,
        task_id: String,
        message: String,
        timestamp: String,
    },
}

/// Control file manager
#[derive(Clone)]
pub struct ControlManager {
    pub track_id: String,
    state_dir: PathBuf,
}

impl ControlManager {
    /// Create a new control manager for a track
    pub fn new(track_id: String, state_dir: PathBuf) -> Self {
        Self {
            track_id,
            state_dir,
        }
    }

    /// Get control file path
    fn control_file_path(&self) -> PathBuf {
        self.state_dir
            .join(self.track_id.clone())
            .join("control.json")
    }

    /// Get events file path
    fn events_file_path(&self) -> PathBuf {
        self.state_dir
            .join(self.track_id.clone())
            .join("events.jsonl")
    }

    /// Ensure track directory exists
    fn ensure_dir(&self) -> Result<()> {
        let dir = self.state_dir.join(self.track_id.clone());
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create control dir: {:?}", dir))?;
        Ok(())
    }

    /// Read control file (returns default if doesn't exist)
    pub fn read_control(&self) -> Result<ControlFile> {
        let path = self.control_file_path();
        if !path.exists() {
            return Ok(ControlFile::default());
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let control: ControlFile = serde_json::from_reader(reader)?;
        Ok(control)
    }

    /// Write control file (atomic)
    pub fn write_control(&self, control: &ControlFile) -> Result<()> {
        self.ensure_dir()?;

        let path = self.control_file_path();
        let temp_path = path.with_extension("tmp");

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, control)?;

        // Atomic rename
        fs::rename(&temp_path, &path)?;
        Ok(())
    }

    /// Pop next pending command (removes it from file)
    pub fn pop_command(&self) -> Result<Option<ControlCommand>> {
        let mut control = self.read_control()?;
        if control.commands.is_empty() {
            return Ok(None);
        }

        let cmd = control.commands.remove(0);
        control.updated_at = Utc::now().to_rfc3339();
        self.write_control(&control)?;

        Ok(Some(cmd))
    }

    /// Add a command to the control file
    pub fn add_command(&self, cmd: ControlCommand) -> Result<()> {
        let mut control = self.read_control();
        if let Ok(ref mut c) = control {
            c.commands.push(cmd);
            c.updated_at = Utc::now().to_rfc3339();
            self.write_control(c)?;
        }
        Ok(())
    }

    /// Clear all commands
    pub fn clear_commands(&self) -> Result<()> {
        let mut control = self.read_control()?;
        control.commands.clear();
        control.updated_at = Utc::now().to_rfc3339();
        self.write_control(&control)?;
        Ok(())
    }

    /// Append an event to the event log
    pub fn emit_event(&self, event: &EngineEvent) -> Result<()> {
        self.ensure_dir()?;

        let path = self.events_file_path();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&path)?;

        let line = serde_json::to_string(event)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Read recent events from the log
    pub fn read_events(&self, limit: usize) -> Result<Vec<EngineEvent>> {
        let path = self.events_file_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        let mut events = Vec::new();
        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if let Ok(event) = serde_json::from_str::<EngineEvent>(&line) {
                events.push(event);
            }
        }

        // Keep only the most recent events
        if events.len() > limit {
            events = events.into_iter().rev().take(limit).collect();
        }

        Ok(events)
    }

    /// Truncate events file (call on session start)
    pub fn truncate_events(&self) -> Result<()> {
        let path = self.events_file_path();
        if path.exists() {
            fs::write(&path, "")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_control_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ControlManager::new("test-track".to_string(), temp_dir.path().to_path_buf());

        // Test empty read
        let control = manager.read_control().unwrap();
        assert!(control.commands.is_empty());

        // Test add command
        manager
            .add_command(ControlCommand::Retry {
                task_id: "task-1".to_string(),
                iteration: 1,
            })
            .unwrap();

        // Test read command
        let control = manager.read_control().unwrap();
        assert_eq!(control.commands.len(), 1);

        // Test pop command
        let cmd = manager.pop_command().unwrap().unwrap();
        match cmd {
            ControlCommand::Retry { task_id, iteration } => {
                assert_eq!(task_id, "task-1");
                assert_eq!(iteration, 1);
            }
            _ => panic!("Expected Retry command"),
        }

        // Test empty after pop
        let cmd = manager.pop_command().unwrap();
        assert!(cmd.is_none());
    }

    #[test]
    fn test_events() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ControlManager::new("test-track".to_string(), temp_dir.path().to_path_buf());

        // Test emit event
        manager
            .emit_event(&EngineEvent::Selecting {
                iteration: 1,
                timestamp: Utc::now().to_rfc3339(),
            })
            .unwrap();

        // Test read events
        let events = manager.read_events(10).unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            EngineEvent::Selecting { iteration, .. } => {
                assert_eq!(*iteration, 1);
            }
            _ => panic!("Expected Selecting event"),
        }
    }
}
