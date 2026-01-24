//! Engine state polling and synchronization
//!
//! Monitor session.json and iterations.jsonl for background updates.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use chrono::TimeZone;
use leindex_core::orchestrate::model::{SessionState, IterationLog, SessionStatus, IterationStatus};
use super::pane::ConductorPane;
use super::model::{ConductorStatus, ConductorEvent, OutputStream, ActiveAgentState, AgentReason};

impl ConductorPane {
    /// Poll the orchestrate engine state for the currently selected track
    pub fn poll_engine_state(&mut self) {
        let track_idx = match self.get_selected_track_index() {
            Some(idx) => idx,
            None => return,
        };
        let track_id = self.tracks[track_idx].id.clone();

        // If track changed, reset polling state and clear output
        if self.state.current_track.as_ref() != Some(&track_id) {
            self.state.current_track = Some(track_id.clone());
            self.state.last_poll_iteration = 0;
            self.state.last_poll_offset = 0;
            self.clear_output();
            self.state.status = ConductorStatus::Ready; // Reset status until polled
        }

        // Find orchestrate data dir. Default is ~/.maestro/orchestrate
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let orchestrate_dir = PathBuf::from(home).join(".maestro").join("orchestrate").join(&track_id);
        
        if !orchestrate_dir.exists() {
            // Reset status to Ready if no session exists for this track
            if self.state.status != ConductorStatus::Ready {
                self.state.status = ConductorStatus::Ready;
            }
            return;
        }

        self.poll_session_json(&orchestrate_dir);
        self.poll_iterations_jsonl(&orchestrate_dir);

        // Update Git info using project root
        let git_dir = if let Some(project) = &self.current_project {
            &project.root_dir
        } else {
            &self.tracks_dir
        };

        if let Some(git_status) = crate::conductor::git::get_git_status(git_dir) {
            self.state.git_info = Some(crate::conductor::model::GitInfo {
                repo_name: None,
                branch: Some(git_status.branch),
                is_dirty: git_status.is_dirty,
                commit_hash: None,
            });
        }
    }

    fn poll_session_json(&mut self, dir: &Path) {
        let session_path = dir.join("session.json");
        if !session_path.exists() {
            return;
        }

        // Read to string first to minimize file handle open time and handle partial writes
        if let Ok(content) = std::fs::read_to_string(&session_path) {
            if let Ok(session) = serde_json::from_str::<SessionState>(&content) {
                // If task changed, emit TaskSelected to transition state machine
                if session.current_task_id != self.state.current_task {
                    if let Some(task_id) = &session.current_task_id {
                        self.state.transition(&ConductorEvent::TaskSelected {
                            task_id: task_id.clone(),
                            iteration: session.current_iteration,
                        });
                    }
                }

                // Update state based on session
                let new_status = map_session_status(session.status);
                
                // Preserve granular states (Selecting/Executing) if the session is still Running
                if new_status == ConductorStatus::Running {
                    if !matches!(self.state.status, ConductorStatus::Selecting | ConductorStatus::Executing | ConductorStatus::Pausing) {
                        self.state.status = new_status;
                    }
                } else {
                    self.state.status = new_status;
                }
                
                self.state.current_iteration = session.current_iteration;
                self.state.session_id = Some(session.session_id.clone());
                self.state.current_task = session.current_task_id.clone();
                self.state.loop_mode = session.mode;
                self.state.sandbox_enabled = session.agent_config.sandbox;
                self.state.dangerous_mode = session.agent_config.dangerous_mode;
                
                // Map RateLimitState if present
                if let Some(rl) = session.rate_limit {
                    self.state.rate_limit = Some(crate::conductor::model::RateLimitState {
                        primary_agent: session.agent_config.tool.clone(),
                        limited_at: rl.last_hit_at.and_then(|ts| {
                            chrono::Utc.timestamp_opt(ts as i64, 0).single()
                        }),
                        fallback_agent: None, // Engine currently doesn't support automatic fallback agents
                        retry_count: rl.consecutive_hits,
                        backoff_until: rl.backoff_until.and_then(|ts| {
                            chrono::Utc.timestamp_opt(ts as i64, 0).single()
                        }),
                        last_message: if rl.is_limited { Some("Rate limit hit".to_string()) } else { None },
                    });
                } else {
                    self.state.rate_limit = None;
                }

                // Active agent info
                if self.state.active_agent.is_none() || self.state.active_agent.as_ref().unwrap().tool != session.agent_config.tool {
                    self.state.active_agent = Some(ActiveAgentState {
                        tool: session.agent_config.tool.clone(),
                        model: session.agent_config.model.clone(),
                        reason: AgentReason::Primary,
                        since: chrono::Utc::now(),
                    });
                }
                
                // Synchronize legacy pane fields
                self.session_status = session.status;
                self.loop_mode = session.mode;
                self.current_iteration = session.current_iteration;
            }
        }
    }

    fn poll_iterations_jsonl(&mut self, dir: &Path) {
        let log_path = dir.join("iterations.jsonl");
        if !log_path.exists() {
            self.state.last_poll_offset = 0;
            self.state.last_poll_iteration = 0;
            return;
        }

        if let Ok(mut file) = File::open(&log_path) {
            let metadata = file.metadata().ok();
            let file_len = metadata.map(|m| m.len()).unwrap_or(0);

            // Handle file truncation (e.g. new session started)
            if self.state.last_poll_offset > file_len {
                self.state.last_poll_offset = 0;
                self.state.last_poll_iteration = 0;
                self.clear_output();
            }

            if file.seek(SeekFrom::Start(self.state.last_poll_offset)).is_ok() {
                let mut reader = BufReader::new(file);
                let mut line = String::new();
                
                while let Ok(bytes_read) = reader.read_line(&mut line) {
                    if bytes_read == 0 {
                        break;
                    }
                    
                    // If the line doesn't end with a newline, it might be a partial write
                    if !line.ends_with('\n') {
                        break;
                    }

                    if let Ok(log) = serde_json::from_str::<IterationLog>(&line) {
                        // Process this new log entry
                        self.process_iteration_log(log);
                        self.state.last_poll_iteration += 1;
                        self.state.last_poll_offset += bytes_read as u64;
                    } else {
                        // Advance offset anyway if it's a complete line but invalid JSON
                        // to avoid getting stuck on malformed data
                        self.state.last_poll_offset += bytes_read as u64;
                    }
                    line.clear();
                }
            }
        }
    }

    fn process_iteration_log(&mut self, log: IterationLog) {
        // Add output to pane
        self.add_output(format!("--- Iteration {} ({}) ---", log.iteration, log.task_id));
        
        // Use events to update state
        self.state.transition(&ConductorEvent::IterationStarted { 
            iteration: log.iteration, 
            task_id: log.task_id.clone() 
        });

        if !log.output.is_empty() {
            self.state.transition(&ConductorEvent::AgentOutput { 
                stream: OutputStream::Stdout, 
                data: log.output.clone() 
            });
            
            for out_line in log.output.lines() {
                self.add_output(out_line.to_string());
            }
        }

        if let Some(err) = log.error {
            self.state.transition(&ConductorEvent::AgentOutput { 
                stream: OutputStream::Stderr, 
                data: err.clone() 
            });
            self.add_output(format!("ERROR: {}", err));
            
            self.state.transition(&ConductorEvent::IterationFailed {
                iteration: log.iteration,
                error: err,
            });
        } else {
            match log.status {
                IterationStatus::Completed => {
                    self.state.transition(&ConductorEvent::IterationCompleted {
                        iteration: log.iteration,
                        task_completed: true, // We don't know for sure from IterationLog alone if task is fully complete
                        duration_ms: 0, // Not in IterationLog
                    });
                }
                IterationStatus::Skipped => {
                    self.state.transition(&ConductorEvent::IterationSkipped {
                        iteration: log.iteration,
                        task_id: log.task_id,
                        reason: "Skipped in log".to_string(),
                    });
                }
                _ => {}
            }
        }
    }
}

fn map_session_status(status: SessionStatus) -> ConductorStatus {
    match status {
        SessionStatus::Idle => ConductorStatus::Idle,
        SessionStatus::Running => ConductorStatus::Running,
        SessionStatus::Paused => ConductorStatus::Paused,
        SessionStatus::Completed => ConductorStatus::Completed,
        SessionStatus::Failed => ConductorStatus::Failed,
        SessionStatus::Interrupted => ConductorStatus::Stopping,
    }
}
