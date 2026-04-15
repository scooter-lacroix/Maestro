//! Engine state polling and synchronization
//!
//! Monitor session.json and iterations.jsonl for background updates.

use super::model::{ActiveAgentState, AgentReason, ConductorEvent, ConductorStatus, OutputStream};
use super::pane::ConductorPane;
use chrono::TimeZone;
use leindex_core::orchestrate::model::{
    IterationLog, IterationStatus, SessionState, SessionStatus,
};
use serde_json::json;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

impl ConductorPane {
    /// Poll the orchestrate engine state for all tracks to find active ones
    pub fn poll_engine_state(&mut self) {
        // Optimization: Only poll all tracks every 4th frame (approx every 2 seconds at 500ms poll)
        // to minimize blocking I/O in the main loop.
        use std::sync::atomic::{AtomicU32, Ordering};
        static POLL_COUNTER: AtomicU32 = AtomicU32::new(0);

        let count = POLL_COUNTER.fetch_add(1, Ordering::SeqCst);
        let should_poll_all = (count % 4) == 0;

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let orchestrate_base = PathBuf::from(home).join(".maestro").join("orchestrate");

        if should_poll_all {
            self.refresh_tracks_if_needed();

            let tracks_to_poll: Vec<(usize, String)> = self
                .tracks
                .iter()
                .enumerate()
                .map(|(idx, t)| (idx, t.id.clone()))
                .collect();

            for (_idx, track_id) in tracks_to_poll {
                let orchestrate_dir = orchestrate_base.join(&track_id);
                if !orchestrate_dir.exists() {
                    continue;
                }

                // Check session status
                let session_path = orchestrate_dir.join("session.json");
                if session_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&session_path) {
                        if let Ok(session) = serde_json::from_str::<SessionState>(&content) {
                            let runtime_status = map_session_status(session.status);
                            self.state
                                .track_runtime_statuses
                                .insert(track_id.clone(), runtime_status);
                        }
                    }
                }
            }
        }

        // Always poll the selected track for detailed live updates
        let active_track_id = self
            .state
            .track_runtime_statuses
            .iter()
            .find(|(_, status)| **status == ConductorStatus::Running)
            .map(|(id, _)| id.clone());

        // 2. Poll detailed state for the selected track (for output and iteration tracking)
        let selected_track_id = match self.get_selected_track_index() {
            Some(idx) => Some(self.tracks[idx].id.clone()),
            None => active_track_id, // Fallback to active track if nothing selected
        };

        if let Some(track_id) = selected_track_id {
            self.poll_track_detailed_state(&track_id, &orchestrate_base);
        }
    }

    fn poll_track_detailed_state(&mut self, track_id: &str, orchestrate_base: &Path) {
        let track_id = track_id.to_string();
        // If track changed, reset polling state and clear output
        if self.state.current_track.as_ref() != Some(&track_id) {
            self.state.current_track = Some(track_id.clone());
            self.state.last_poll_iteration = 0;
            self.state.last_poll_offset = 0;
            self.clear_output();
            self.state.status = ConductorStatus::Ready; // Reset status until polled
            self.state.iteration_logs.clear();
        }

        let orchestrate_dir = orchestrate_base.join(&track_id);

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
                // If status changed, broadcast
                let new_status = map_session_status(session.status);
                if self.session_status != session.status {
                    match new_status {
                        ConductorStatus::Running => {
                            crate::conductor::telemetry::BUS.broadcast(ConductorEvent::Resumed)
                        }
                        ConductorStatus::Paused => {
                            crate::conductor::telemetry::BUS.broadcast(ConductorEvent::Paused)
                        }
                        _ => {}
                    }
                }

                // If task changed, emit TaskSelected to transition state machine
                if session.current_task_id != self.state.current_task {
                    if let Some(task_id) = &session.current_task_id {
                        let event = ConductorEvent::TaskSelected {
                            task_id: task_id.clone(),
                            iteration: session.current_iteration,
                        };
                        self.state.transition(&event);
                        crate::conductor::telemetry::BUS.broadcast(event);
                    }
                }

                // Update state based on session
                let new_status = map_session_status(session.status);

                // Preserve granular states (Selecting/Executing) if the session is still Running
                if new_status == ConductorStatus::Running {
                    if !matches!(
                        self.state.status,
                        ConductorStatus::Selecting
                            | ConductorStatus::Executing
                            | ConductorStatus::Pausing
                    ) {
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
                        limited_at: rl
                            .last_hit_at
                            .and_then(|ts| chrono::Utc.timestamp_opt(ts as i64, 0).single()),
                        fallback_agent: None, // Engine currently doesn't support automatic fallback agents
                        retry_count: rl.consecutive_hits,
                        backoff_until: rl
                            .backoff_until
                            .and_then(|ts| chrono::Utc.timestamp_opt(ts as i64, 0).single()),
                        last_message: if rl.is_limited {
                            Some("Rate limit hit".to_string())
                        } else {
                            None
                        },
                    });
                } else {
                    self.state.rate_limit = None;
                }

                // Active agent info
                if self.state.active_agent.is_none()
                    || self.state.active_agent.as_ref().unwrap().tool != session.agent_config.tool
                {
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

            if file
                .seek(SeekFrom::Start(self.state.last_poll_offset))
                .is_ok()
            {
                let mut reader = BufReader::new(file);
                let mut line = String::new();
                let is_initial_read = self.state.last_poll_offset == 0;

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
                        // Suppress output if we're just catching up with history
                        self.process_iteration_log(log, is_initial_read);
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

    pub fn process_iteration_log(&mut self, log: IterationLog, suppress_output: bool) {
        // Update history list
        if !self
            .state
            .iteration_logs
            .iter()
            .any(|l| l.iteration == log.iteration)
        {
            self.state.iteration_logs.push(log.clone());
            if self.state.iteration_logs.len() > 50 {
                self.state.iteration_logs.remove(0);
            }
        } else {
            // Update existing entry if status changed
            if let Some(existing) = self
                .state
                .iteration_logs
                .iter_mut()
                .find(|l| l.iteration == log.iteration)
            {
                *existing = log.clone();
            }
        }

        let summary = match log.status {
            IterationStatus::Running => format!("started {}", log.task_id),
            IterationStatus::Completed => format!("completed {}", log.task_id),
            IterationStatus::Failed => format!("failed {}", log.task_id),
            IterationStatus::Skipped => format!("skipped {}", log.task_id),
        };
        let level = match log.status {
            IterationStatus::Running => crate::conductor::model::RuntimeLogLevel::Info,
            IterationStatus::Completed => crate::conductor::model::RuntimeLogLevel::Success,
            IterationStatus::Failed => crate::conductor::model::RuntimeLogLevel::Error,
            IterationStatus::Skipped => crate::conductor::model::RuntimeLogLevel::Warning,
        };
        self.push_runtime_log(
            level,
            Some(log.iteration),
            Some(log.task_id.clone()),
            summary,
            if !log.output.is_empty() {
                Some(log.output.clone())
            } else {
                log.error.clone()
            },
        );

        // Add output to pane if not suppressed
        if !suppress_output {
            self.add_output(format!(
                "--- Iteration {} ({}) ---",
                log.iteration, log.task_id
            ));
        }

        // Use events to update state
        let started_event = ConductorEvent::IterationStarted {
            iteration: log.iteration,
            task_id: log.task_id.clone(),
        };

        // Only broadcast and transition on new events (not during history catch-up)
        if !suppress_output {
            self.state.transition(&started_event);
            crate::conductor::telemetry::BUS.broadcast(started_event);
        }

        if !log.output.is_empty() {
            let output_event = ConductorEvent::AgentOutput {
                stream: OutputStream::Stdout,
                data: log.output.clone(),
            };

            if !suppress_output {
                self.state.transition(&output_event);
                crate::conductor::telemetry::BUS.broadcast(output_event);

                for out_line in log.output.lines() {
                    self.add_output(out_line.to_string());
                }

                let summary = log
                    .output
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim().chars().take(100).collect::<String>())
                    .unwrap_or_else(|| "agent produced output".to_string());
                self.push_runtime_log(
                    crate::conductor::model::RuntimeLogLevel::Info,
                    Some(log.iteration),
                    Some(log.task_id.clone()),
                    summary,
                    Some(log.output.clone()),
                );
            }
        }

        if let Some(err) = log.error {
            let error_event = ConductorEvent::AgentOutput {
                stream: OutputStream::Stderr,
                data: err.clone(),
            };

            if !suppress_output {
                self.state.transition(&error_event);
                crate::conductor::telemetry::BUS.broadcast(error_event);
                self.add_output(format!("ERROR: {}", err));
                self.push_runtime_log(
                    crate::conductor::model::RuntimeLogLevel::Error,
                    Some(log.iteration),
                    Some(log.task_id.clone()),
                    format!("error: {}", err),
                    Some(err.clone()),
                );
            }

            let failed_event = ConductorEvent::IterationFailed {
                iteration: log.iteration,
                error: err,
            };

            if !suppress_output {
                self.state.transition(&failed_event);
                crate::conductor::telemetry::BUS.broadcast(failed_event);
            }
        } else {
            match log.status {
                IterationStatus::Completed => {
                    let comp_event = ConductorEvent::IterationCompleted {
                        iteration: log.iteration,
                        task_completed: true, // We don't know for sure from IterationLog alone if task is fully complete
                        duration_ms: 0,       // Not in IterationLog
                    };
                    if !suppress_output {
                        self.state.transition(&comp_event);
                        crate::conductor::telemetry::BUS.broadcast(comp_event);
                        // Schedule all three transitions in parallel to avoid blocking UI thread for up to 90s
                        self.schedule_cognition_transitions_parallel(
                            &[
                                ("loop", false, true),
                                ("review", true, false),
                                ("checkpoint", true, true),
                            ],
                            log.iteration,
                            &log.task_id,
                            suppress_output,
                        );
                    }
                }
                IterationStatus::Skipped => {
                    let skip_event = ConductorEvent::IterationSkipped {
                        iteration: log.iteration,
                        task_id: log.task_id,
                        reason: "Skipped in log".to_string(),
                    };
                    if !suppress_output {
                        self.state.transition(&skip_event);
                        crate::conductor::telemetry::BUS.broadcast(skip_event);
                    }
                }
                _ => {}
            }
        }
    }

    fn schedule_cognition_transition(
        &mut self,
        phase: &str,
        iteration: u64,
        task_id: &str,
        review_point_reached: bool,
        task_completed: bool,
        suppress_output: bool,
    ) {
        if suppress_output {
            return;
        }

        let project_root = self
            .current_project
            .as_ref()
            .map(|project| project.root_dir.clone())
            .or_else(|| self.tracks_dir.parent().map(|path| path.to_path_buf()))
            .unwrap_or_else(|| self.tracks_dir.clone());

        let payload = json!({
            "session_id": self.state.session_id.clone().unwrap_or_default(),
            "track_id": self.state.current_track.clone().unwrap_or_default(),
            "task_id": task_id,
            "iteration": iteration,
            "project_path": project_root.display().to_string(),
            "cwd": project_root.display().to_string(),
            "selected_cli": self
                .state
                .active_agent
                .as_ref()
                .map(|agent| agent.tool.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            "loop_mode": format!("{:?}", self.loop_mode),
            "review_point_reached": review_point_reached,
            "checkpoint_interval": self.state.max_iterations,
            "task_completed": task_completed,
        });

        if let Some(stdout) = run_hook_executor_phase(phase, &payload, &project_root) {
            let summary = format!("scheduled {} transition for iteration {}", phase, iteration);
            self.push_runtime_log(
                crate::conductor::model::RuntimeLogLevel::Info,
                Some(iteration),
                Some(task_id.to_string()),
                summary,
                Some(stdout),
            );
        }
    }

    /// Schedule multiple cognition transitions in parallel to avoid blocking UI thread
    fn schedule_cognition_transitions_parallel(
        &mut self,
        phases: &[(&str, bool, bool)], // (phase_name, review_point_reached, task_completed)
        iteration: u64,
        task_id: &str,
        suppress_output: bool,
    ) {
        if suppress_output {
            return;
        }

        let project_root = self
            .current_project
            .as_ref()
            .map(|project| project.root_dir.clone())
            .or_else(|| self.tracks_dir.parent().map(|path| path.to_path_buf()))
            .unwrap_or_else(|| self.tracks_dir.clone());

        // Build payloads for all phases
        let payloads: Vec<_> = phases
            .iter()
            .map(|(phase, review_point_reached, task_completed)| {
                let payload = json!({
                    "session_id": self.state.session_id.clone().unwrap_or_default(),
                    "track_id": self.state.current_track.clone().unwrap_or_default(),
                    "task_id": task_id,
                    "iteration": iteration,
                    "project_path": project_root.display().to_string(),
                    "cwd": project_root.display().to_string(),
                    "selected_cli": self
                        .state
                        .active_agent
                        .as_ref()
                        .map(|agent| agent.tool.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    "loop_mode": format!("{:?}", self.loop_mode),
                    "review_point_reached": review_point_reached,
                    "checkpoint_interval": self.state.max_iterations,
                    "task_completed": task_completed,
                });
                (*phase, payload)
            })
            .collect();

        // Spawn threads for each phase in parallel
        let handles: Vec<_> = payloads
            .into_iter()
            .map(|(phase, payload)| {
                let phase = phase.to_string();
                let project_root = project_root.clone();
                thread::spawn(move || {
                    (phase.clone(), run_hook_executor_phase(&phase, &payload, &project_root))
                })
            })
            .collect();

        // Collect results and log them
        for handle in handles {
            if let Ok((phase, stdout)) = handle.join() {
                if let Some(stdout) = stdout {
                    let summary = format!("scheduled {} transition for iteration {}", phase, iteration);
                    self.push_runtime_log(
                        crate::conductor::model::RuntimeLogLevel::Info,
                        Some(iteration),
                        Some(task_id.to_string()),
                        summary,
                        Some(stdout),
                    );
                }
            }
        }
    }
}

fn map_session_status(status: SessionStatus) -> ConductorStatus {
    match status {
        SessionStatus::Idle => ConductorStatus::Idle,
        SessionStatus::Running => ConductorStatus::Running,
        SessionStatus::Pausing => ConductorStatus::Pausing,
        SessionStatus::Paused => ConductorStatus::Paused,
        SessionStatus::Stopping => ConductorStatus::Stopping,
        SessionStatus::Completed => ConductorStatus::Completed,
        SessionStatus::Failed => ConductorStatus::Failed,
        SessionStatus::Interrupted => ConductorStatus::Stopping,
    }
}

fn run_hook_executor_phase(
    phase: &str,
    payload: &serde_json::Value,
    project_root: &Path,
) -> Option<String> {
    const HOOK_TIMEOUT: Duration = Duration::from_secs(30);
    const POLL_INTERVAL: Duration = Duration::from_millis(100);

    let script = r#"
import json
import sys
from maestro.hooks.executor import get_hook_executor

phase = sys.argv[1]
payload = json.loads(sys.stdin.read())
executor = get_hook_executor()
result = executor.execute_phase(phase, payload)
json.dump(result, sys.stdout)
"#;

    for python in ["python3", "python"] {
        let phase_owned = phase.to_owned();
        let project_root_owned = project_root.to_path_buf();
        let payload_owned = payload.clone();

        let (pid_tx, pid_rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
            let mut child = match Command::new(python)
                .arg("-c")
                .arg(script)
                .arg(&phase_owned)
                .current_dir(&project_root_owned)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(_) => return None,
            };

            if let Some(mut stdin) = child.stdin.take() {
                if let Ok(json) = serde_json::to_string(&payload_owned) {
                    let _ = stdin.write_all(json.as_bytes());
                }
            }

            // Send PID before blocking wait so the main thread can kill on timeout
            let _ = pid_tx.send(child.id());

            match child.wait_with_output() {
                Ok(output) if output.status.success() => {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        eprintln!(
                            "Hook executor phase '{}' failed: {}",
                            phase_owned,
                            stderr.trim()
                        );
                    }
                    None
                }
                _ => None,
            }
        });

        // Wait briefly for the spawned thread to send us the child PID
        let child_pid = pid_rx.recv_timeout(POLL_INTERVAL).ok();

        let deadline = Instant::now() + HOOK_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                eprintln!("Hook executor phase '{}' timed out after {:?}", phase, HOOK_TIMEOUT);
                // Kill orphaned child process tree to prevent resource leaks
                if let Some(pid) = child_pid {
                    // Send SIGKILL to the process group (negative PID) to kill the
                    // python process and any subprocesses it spawned.
                    let pgid = pid as i32;
                    let _ = Command::new("kill")
                        .arg(format!("-{}", pgid))
                        .output();
                    // Also kill the process directly in case setpgid wasn't called
                    let _ = Command::new("kill")
                        .arg(format!("{}", pid))
                        .output();
                }
                return None;
            }

            let wait_duration = std::cmp::min(POLL_INTERVAL, remaining);
            thread::sleep(wait_duration);

            if handle.is_finished() {
                match handle.join() {
                    Ok(Some(result)) => return Some(result),
                    Ok(None) => return None,
                    Err(_) => {
                        eprintln!("Hook executor phase '{}' thread panicked", phase);
                        return None;
                    }
                }
            }
        }
    }

    None
}
