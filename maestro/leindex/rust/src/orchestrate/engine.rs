//! Orchestrate execution engine
//!
//! Core iteration loop: select → prompt → run → detect completion → update
//!
//! Bidirectional communication with Conductor TUI via control.json (commands)
//! and events.jsonl (structured events).

use crate::orchestrate::control::{ControlCommand, ControlManager, EngineEvent};
use crate::orchestrate::model::*;
use crate::orchestrate::parser::{parse_plan_md, write_plan_md};
use crate::orchestrate::prompts::PromptBuilder;
use crate::orchestrate::rate_limit_detector::{
    RateLimitDetectionInput, RateLimitDetectionResult, RateLimitDetector,
};
use crate::orchestrate::runner::{AgentRunner, RunResult};
use crate::orchestrate::state::{LockGuard, StateManager};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Orchestrate engine
pub struct OrchestrateEngine {
    config: OrchestrateConfig,
    state_manager: StateManager,
    tracks_dir: PathBuf,
    memory_service: Option<crate::memory::MemoryService>,
    rate_limit_detector: RateLimitDetector,
    rate_limit_backoff: std::sync::Arc<tokio::sync::Mutex<crate::rate_limit::RateLimitBackoff>>,
}

impl OrchestrateEngine {
    /// Create a new orchestrate engine
    pub fn new(config: OrchestrateConfig, tracks_dir: PathBuf) -> Result<Self> {
        let state_manager = StateManager::new(config.data_dir.clone())
            .context("Failed to initialize state manager")?;

        let memory_service = crate::memory::MemoryService::new(None).ok();

        let rate_limit_detector = RateLimitDetector::new();
        let rate_limit_backoff = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::rate_limit::RateLimitBackoff::new(),
        ));

        Ok(Self {
            config,
            state_manager,
            tracks_dir,
            memory_service,
            rate_limit_detector,
            rate_limit_backoff,
        })
    }

    /// Emit an event to the conductor
    fn emit_event(&self, track_id: &str, event: &EngineEvent) {
        let manager = ControlManager::new(track_id.to_string(), self.config.data_dir.clone());
        if let Err(e) = manager.emit_event(event) {
            warn!("Failed to emit event: {}", e);
        }
    }

    /// Check for pending control commands from conductor
    fn check_control_commands(&self, track_id: &str) -> Result<Option<ControlCommand>> {
        let manager = ControlManager::new(track_id.to_string(), self.config.data_dir.clone());
        manager
            .pop_command()
            .context("Failed to read control commands")
    }

    /// Start orchestrate loop for a track
    pub async fn start(
        &mut self,
        track_id: &str,
        mode: LoopMode,
        agent_config: AgentConfig,
    ) -> Result<()> {
        // Acquire lock
        let _lock = LockGuard::acquire(self.state_manager.clone(), track_id)?;

        // Load or create session state
        let mut session = self.load_or_create_session(track_id, mode, agent_config)?;

        // Load track plan
        let track_path = self.tracks_dir.join(track_id);
        let plan_path = track_path.join("plan.md");

        let mut plan = parse_plan_md(&plan_path)
            .with_context(|| format!("Failed to parse plan for track: {}", track_id))?;

        info!(
            "Starting orchestrate loop for track {} in {:?} mode",
            track_id, mode
        );

        // Truncate events file for this session (clean slate)
        let manager = ControlManager::new(track_id.to_string(), self.config.data_dir.clone());
        let _ = manager.truncate_events();

        // Emit session start event
        self.emit_event(
            track_id,
            &EngineEvent::Progress {
                iteration: session.current_iteration,
                task_id: track_id.to_string(),
                message: format!("Session started in {:?} mode", mode),
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        // Main loop
        loop {
            // Check for pausing/paused states (Ralph TUI pattern)
            if session.status == SessionStatus::Pausing {
                // Transition to paused state
                session.status = SessionStatus::Paused;
                session.updated_at = Utc::now().to_rfc3339();
                self.state_manager.save_session(&session)?;
                self.emit_event(
                    track_id,
                    &EngineEvent::Progress {
                        iteration: session.current_iteration,
                        task_id: track_id.to_string(),
                        message: "Session paused".to_string(),
                        timestamp: Utc::now().to_rfc3339(),
                    },
                );
                info!("Session transitioned to Paused state");
                continue;
            }

            if session.status == SessionStatus::Paused {
                info!("Session paused, sleeping...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Check max iterations limit (Ralph TUI pattern)
            if session.max_iterations > 0 && session.current_iteration >= session.max_iterations {
                info!("Max iterations ({}) reached", session.max_iterations);
                session.status = SessionStatus::Idle;
                session.updated_at = Utc::now().to_rfc3339();
                self.state_manager.save_session(&session)?;
                self.emit_event(
                    track_id,
                    &EngineEvent::Progress {
                        iteration: session.current_iteration,
                        task_id: track_id.to_string(),
                        message: format!("Max iterations ({}) reached", session.max_iterations),
                        timestamp: Utc::now().to_rfc3339(),
                    },
                );
                break;
            }

            // Check for control commands from conductor
            if let Ok(Some(cmd)) = self.check_control_commands(track_id) {
                match cmd {
                    ControlCommand::Abort { reason } => {
                        info!("Abort command received from conductor: {:?}", reason);
                        session.status = SessionStatus::Interrupted;
                        self.state_manager.save_session(&session)?;
                        break;
                    }
                    ControlCommand::Retry {
                        task_id: cmd_task_id,
                        iteration,
                    } => {
                        info!(
                            "Retry command from conductor for task {} iteration {}",
                            cmd_task_id, iteration
                        );
                        // Will be handled in the error handling section
                    }
                    ControlCommand::Skip {
                        task_id: cmd_task_id,
                        ..
                    } => {
                        info!("Skip command from conductor for task {}", cmd_task_id);
                        // Mark task as completed (skip)
                        let task_ref = self.find_task_mut(&mut plan.tasks, &cmd_task_id);
                        if let Ok(t) = task_ref {
                            t.status = TrackStatus::Completed;
                            t.notes = Some(format!("SKIPPED via conductor control"));
                            self.state_manager.save_session(&session)?;
                        }
                    }
                    ControlCommand::SetErrorStrategy { strategy } => {
                        info!("Error strategy changed to {:?}", strategy);
                        // Note: This would need to be stored in the session/config
                        // For now, we log it
                    }
                }
            }

            if session.status != SessionStatus::Running {
                break;
            }

            // Emit selecting event
            self.emit_event(
                track_id,
                &EngineEvent::Selecting {
                    iteration: session.current_iteration + 1,
                    timestamp: Utc::now().to_rfc3339(),
                },
            );

            // Select next actionable task
            let (task_id, task_title, task_description) = match self
                .select_next_task(&plan, &session)?
            {
                Some(t) => (t.id.clone(), t.title.clone(), t.description.clone()),
                None => {
                    // All tasks marked complete. Perform final Track Verification.
                    info!(
                        "All tasks in track {} marked as complete. Verifying integrity...",
                        track_id
                    );
                    if let Err(e) = self.verify_track_integrity(track_id, &plan, &session).await {
                        warn!(
                            "Track verification failed: {}. Re-opening relevant tasks.",
                            e
                        );
                        // logic to re-open tasks would go here
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        continue;
                    }

                    info!("Track verification successful. Track complete!");

                    // Bank track completion memory
                    if let Some(ref svc) = self.memory_service {
                        let content = format!(
                            "Track '{}' completed successfully at {}. All tasks verified and passed.",
                            track_id,
                            Utc::now().to_rfc3339()
                        );
                        let _ = svc.store_memory(
                            &content,
                            crate::memory::models::MemoryCategory::Decision,
                        );
                    }

                    session.status = SessionStatus::Completed;
                    self.state_manager.save_session(&session)?;
                    break;
                }
            };

            // Mark task as in progress
            self.mark_task_in_progress(&mut plan, &task_id)?;
            session.current_task_id = Some(task_id.clone());
            session.current_iteration += 1;
            session.updated_at = Utc::now().to_rfc3339();
            self.state_manager.save_session(&session)?;

            // Emit executing event
            self.emit_event(
                track_id,
                &EngineEvent::Executing {
                    iteration: session.current_iteration,
                    task_id: task_id.clone(),
                    task_title: task_title.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                },
            );

            // Create a temporary task for the iteration
            let temp_task = Task {
                id: task_id.clone(),
                title: task_title,
                description: task_description,
                status: TrackStatus::InProgress,
                dependencies: Vec::new(),
                subtasks: Vec::new(),
                notes: None,
                line_number: 0,
            };

            // Run iteration with rate-limit retry loop
            let iteration_result = loop {
                let result = self
                    .run_iteration(track_id, &temp_task, &session, &plan)
                    .await;

                match result {
                    Ok(iter_res) => {
                        // Check for rate limiting
                        let rate_limit_result = if self.config.enable_rate_limit_detection {
                            self.rate_limit_detector.detect(RateLimitDetectionInput {
                                stderr: iter_res.stderr.clone(),
                                stdout: iter_res.output.clone(),
                                exit_code: iter_res.exit_code,
                                agent_id: Some(session.agent_config.tool.clone()),
                            })
                        } else {
                            RateLimitDetectionResult::not_limited()
                        };

                        if self.config.enable_rate_limit_detection
                            && rate_limit_result.is_rate_limit
                        {
                            let mut backoff = self.rate_limit_backoff.lock().await;
                            let outcome = backoff.record_hit(
                                rate_limit_result.message.clone(),
                                rate_limit_result.retry_after,
                                self.config.rate_limit_max_retries,
                                self.config.rate_limit_backoff_base_secs,
                                self.config.rate_limit_backoff_max_secs,
                            );
                            let detector_state = backoff.state.clone();
                            drop(backoff);

                            // Emit rate limit event for conductor
                            self.emit_event(
                                track_id,
                                &EngineEvent::RateLimited {
                                    task_id: task_id.clone(),
                                    retry_count: detector_state.consecutive_hits,
                                    backoff_until: detector_state.backoff_until,
                                    timestamp: Utc::now().to_rfc3339(),
                                },
                            );

                            // Update session with rate limit state for TUI polling
                            session.rate_limit = Some(detector_state.clone());
                            session.updated_at = Utc::now().to_rfc3339();
                            self.state_manager.save_session(&session)?;

                            if outcome.exceeded_max {
                                error!(
                                    "Rate limit exceeded after {} hits, task {} requires manual intervention",
                                    detector_state.consecutive_hits, task_id
                                );
                                // Mark task as failed with rate limit note
                                let task_ref = self.find_task_mut(&mut plan.tasks, &task_id)?;
                                task_ref.notes = Some(format!(
                                    "RATE_LIMITED: Max retries ({}) exceeded. Manual intervention required.",
                                    self.config.rate_limit_max_retries
                                ));
                                break Err(anyhow!(
                                    "Task {} rate-limited after {} hits",
                                    task_id,
                                    detector_state.consecutive_hits
                                ));
                            }

                            let backoff_secs = outcome.delay_secs.max(1);

                            warn!(
                                "Rate limit detected on task {} (hit {}/{}), backing off {}s (retry_after={}, message={})",
                                task_id,
                                detector_state.consecutive_hits,
                                self.config.rate_limit_max_retries,
                                backoff_secs,
                                outcome.used_retry_after,
                                rate_limit_result
                                    .message
                                    .clone()
                                    .unwrap_or_else(|| "n/a".to_string())
                            );

                            // Sleep for backoff period
                            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;

                            // Re-load session in case it was modified (e.g. paused)
                            if let Some(s) = self.state_manager.load_session(track_id)? {
                                session = s;
                            }

                            // Continue loop to retry
                            continue;
                        }

                        // No rate limit, reset detector and clear rate limit state in session
                        let mut backoff = self.rate_limit_backoff.lock().await;
                        backoff.reset();
                        drop(backoff);

                        session.rate_limit = None;

                        // Exit with result
                        break Ok(iter_res);
                    }
                    Err(e) => {
                        // Non-rate-limit error, exit with error
                        break Err(e);
                    }
                }
            };

            match iteration_result {
                Ok(iteration_result) => {
                    if iteration_result.success {
                        // Clear retry counter on success
                        session.retry_counts.remove(&task_id);
                        // Only mark task complete if agent explicitly completed the task
                        if iteration_result.completed {
                            self.mark_task_complete(&mut plan, &task_id)?;
                            info!(
                                "Iteration {} completed: task {} (detected <promise>COMPLETE</promise>)",
                                session.current_iteration, task_id
                            );
                        } else {
                            info!(
                                "Iteration {} made progress but task not yet complete: {}",
                                session.current_iteration, task_id
                            );
                        }
                    } else {
                        // Handle failure based on error strategy
                        self.handle_task_failure(
                            track_id,
                            session.current_iteration,
                            &mut plan,
                            &temp_task,
                            &iteration_result.error_message,
                            &mut session,
                        )
                        .await?;
                    }
                }
                Err(e) => {
                    error!("Iteration failed: {}", e);
                    self.handle_task_failure(
                        track_id,
                        session.current_iteration,
                        &mut plan,
                        &temp_task,
                        &Some(e.to_string()),
                        &mut session,
                    )
                    .await?;
                }
            }

            // Write updated plan
            write_plan_md(&plan, &plan_path)?;

            // Update session
            session.updated_at = Utc::now().to_rfc3339();
            self.state_manager.save_session(&session)?;
        }

        Ok(())
    }

    /// Pause the orchestrate loop (sets to Pausing state, loop will transition to Paused)
    pub fn pause(&self, track_id: &str) -> Result<()> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        // Ralph TUI pattern: set to Pausing, loop transitions to Paused
        // This allows canceling a pending pause by calling resume()
        match session.status {
            SessionStatus::Running => {
                session.status = SessionStatus::Pausing;
            }
            SessionStatus::Pausing => {
                // Already pausing, this is a no-op
                return Ok(());
            }
            _ => {
                session.status = SessionStatus::Paused;
            }
        }
        session.updated_at = Utc::now().to_rfc3339();
        self.state_manager.save_session(&session)?;

        info!("Pause requested for track {}", track_id);
        Ok(())
    }

    /// Resume the orchestrate loop
    pub fn resume(&self, track_id: &str) -> Result<()> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        // Ralph TUI pattern: resume from Pausing cancels the pending pause
        match session.status {
            SessionStatus::Pausing | SessionStatus::Paused => {
                session.status = SessionStatus::Running;
            }
            _ => {
                // Not paused, no-op
                return Ok(());
            }
        }
        session.updated_at = Utc::now().to_rfc3339();
        self.state_manager.save_session(&session)?;

        info!("Resumed orchestrate loop for track {}", track_id);
        Ok(())
    }

    /// Abort the orchestrate loop
    pub fn abort(&self, track_id: &str) -> Result<()> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        session.status = SessionStatus::Interrupted;
        session.updated_at = Utc::now().to_rfc3339();
        self.state_manager.save_session(&session)?;

        info!("Aborted orchestrate loop for track {}", track_id);
        Ok(())
    }

    /// Add iterations to max_iterations at runtime (Ralph TUI pattern)
    pub fn add_iterations(&self, track_id: &str, count: u64) -> Result<bool> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        let previous_max = session.max_iterations;
        session.max_iterations += count;
        session.updated_at = Utc::now().to_rfc3339();
        self.state_manager.save_session(&session)?;

        info!(
            "Added {} iterations to track {}: {} -> {}",
            count, track_id, previous_max, session.max_iterations
        );

        // Return true if engine should be restarted (was idle after hitting max)
        let should_restart = previous_max > 0 && session.status == SessionStatus::Idle;
        Ok(should_restart)
    }

    /// Remove iterations from max_iterations at runtime (Ralph TUI pattern)
    pub fn remove_iterations(&self, track_id: &str, count: u64) -> Result<bool> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        let previous_max = session.max_iterations;
        // Don't go below current iteration
        let min_allowed = std::cmp::max(1, session.current_iteration);
        session.max_iterations =
            std::cmp::max(min_allowed, session.max_iterations.saturating_sub(count));
        session.updated_at = Utc::now().to_rfc3339();
        self.state_manager.save_session(&session)?;

        info!(
            "Removed {} iterations from track {}: {} -> {}",
            count, track_id, previous_max, session.max_iterations
        );

        Ok(session.max_iterations != previous_max)
    }

    /// Continue execution after adding more iterations (Ralph TUI pattern)
    pub async fn continue_execution(&mut self, track_id: &str) -> Result<()> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        if session.status != SessionStatus::Idle {
            return Ok(()); // Only continue from idle state
        }

        session.status = SessionStatus::Running;
        self.state_manager.save_session(&session)?;

        self.emit_event(
            track_id,
            &EngineEvent::Progress {
                iteration: session.current_iteration,
                task_id: track_id.to_string(),
                message: "Continuing execution".to_string(),
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        info!("Continuing execution for track {}", track_id);
        Ok(())
    }

    // Private methods

    fn load_or_create_session(
        &self,
        track_id: &str,
        mode: LoopMode,
        agent_config: AgentConfig,
    ) -> Result<SessionState> {
        match self.state_manager.load_session(track_id)? {
            Some(mut session) => {
                // Resume existing session
                session.status = SessionStatus::Running;
                session.mode = mode;
                session.agent_config = agent_config; // Allow updating tool/model on resume
                session.updated_at = Utc::now().to_rfc3339();
                Ok(session)
            }
            None => {
                // Create new session
                let now = Utc::now().to_rfc3339();
                let session_id = format!("{}-{}", track_id, Utc::now().timestamp());
                Ok(SessionState {
                    session_id,
                    track_id: track_id.to_string(),
                    mode,
                    agent_config,
                    current_iteration: 0,
                    current_task_id: None,
                    started_at: now.clone(),
                    updated_at: now,
                    status: SessionStatus::Running,
                    rate_limit: None,
                    retry_counts: std::collections::HashMap::new(),
                    max_iterations: 0, // 0 = unlimited
                })
            }
        }
    }

    fn select_next_task<'a>(
        &self,
        plan: &'a TrackPlan,
        session: &SessionState,
    ) -> Result<Option<&'a Task>> {
        match session.mode {
            LoopMode::Planning => {
                // In planning mode, select the first pending task
                Ok(plan
                    .all_tasks()
                    .iter()
                    .find(|t| t.status == TrackStatus::Pending)
                    .copied())
            }
            LoopMode::Building => {
                // In building mode, select the highest priority actionable task
                Ok(plan.next_actionable_task())
            }
        }
    }

    // Note: find_task_by_id and related methods are no longer needed
    // since we create temporary tasks to avoid borrow checker issues

    fn mark_task_in_progress(&self, plan: &mut TrackPlan, task_id: &str) -> Result<()> {
        self.update_task_status_recursive(&mut plan.tasks, task_id, TrackStatus::InProgress)
    }

    fn mark_task_complete(&self, plan: &mut TrackPlan, task_id: &str) -> Result<()> {
        self.update_task_status_recursive(&mut plan.tasks, task_id, TrackStatus::Completed)?;

        // Bank a task completion memory with details
        if let Some(ref svc) = self.memory_service {
            // Get the task details
            if let Ok(task) = self.find_task(&plan.tasks, task_id) {
                let content = format!(
                    "Task '{}' completed in track '{}'.\n\nTitle: {}\nDescription: {}\n\nCompleted at: {}",
                    task_id,
                    plan.track_id,
                    task.title,
                    task.description,
                    Utc::now().to_rfc3339()
                );
                let _ = svc.store_memory(&content, crate::memory::models::MemoryCategory::Decision);
            }
        }

        Ok(())
    }

    fn update_task_status_recursive(
        &self,
        tasks: &mut [Task],
        task_id: &str,
        status: TrackStatus,
    ) -> Result<()> {
        for task in tasks {
            if task.id == task_id {
                task.status = status;
                return Ok(());
            }
            if self
                .update_task_status_recursive(&mut task.subtasks, task_id, status)
                .is_ok()
            {
                return Ok(());
            }
        }
        Err(anyhow!("Task not found: {}", task_id))
    }

    async fn run_iteration(
        &self,
        track_id: &str,
        task: &Task,
        session: &SessionState,
        plan: &TrackPlan,
    ) -> Result<RunResult> {
        let start_time = Utc::now();

        // Build prompt for agent
        let prompt = self.build_prompt(task, session, plan)?;

        // Create agent runner with track-specific working directory and timeout
        let track_dir = self.tracks_dir.join(track_id);
        let runner = AgentRunner::new(
            session.agent_config.clone(),
            track_dir,
            self.config.iteration_timeout_secs,
        );

        // Run with timeout (runner also enforces timeout for safety)
        let run_result = timeout(
            Duration::from_secs(self.config.iteration_timeout_secs),
            runner.run(&prompt, task),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "Iteration timeout after {} seconds",
                self.config.iteration_timeout_secs
            )
        })??;

        // Log iteration
        let log = IterationLog {
            iteration: session.current_iteration,
            task_id: task.id.clone(),
            started_at: start_time.to_rfc3339(),
            completed_at: Some(Utc::now().to_rfc3339()),
            status: if run_result.success {
                IterationStatus::Completed
            } else {
                IterationStatus::Failed
            },
            output: run_result.output.clone(),
            error: run_result.error_message.clone(),
        };

        self.state_manager.append_iteration_log(track_id, &log)?;

        Ok(run_result)
    }

    fn build_prompt(
        &self,
        task: &Task,
        session: &SessionState,
        plan: &TrackPlan,
    ) -> Result<String> {
        let builder = PromptBuilder::new(self.config.context_budget);

        // Get recent iterations
        let recent = self
            .state_manager
            .recent_iterations(&plan.track_id, 5)
            .unwrap_or_default();

        // Get LeIndex context if enabled
        let leindex_context = if self.config.enable_leindex {
            let engine =
                crate::orchestrate::context::ContextEngine::new(self.config.context_budget);
            Some(engine.build_context(&self.tracks_dir, plan)?)
        } else {
            None
        };

        let memory_context = self.memory_service.as_ref().and_then(|svc| {
            let project_path = self.resolve_project_path();
            match svc.list_lsp_memories_for_project(&project_path, 5) {
                Ok(memories) if !memories.is_empty() => {
                    let content = memories
                        .iter()
                        .take(5)
                        .map(|m| m.content.clone())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    Some(content)
                }
                _ => None,
            }
        });

        builder.build_prompt(
            task,
            session,
            plan,
            &recent,
            leindex_context.as_deref(),
            memory_context.as_deref(),
        )
    }

    fn resolve_project_path(&self) -> String {
        let tracks_dir = &self.tracks_dir;
        if let Some(dir_name) = tracks_dir.file_name().and_then(|n| n.to_str()) {
            if dir_name == "maestro" || dir_name == ".maestro" {
                return tracks_dir
                    .parent()
                    .unwrap_or(tracks_dir)
                    .to_string_lossy()
                    .to_string();
            }
        }
        tracks_dir.to_string_lossy().to_string()
    }

    async fn handle_task_failure(
        &self,
        track_id: &str,
        iteration: u64,
        plan: &mut TrackPlan,
        task: &Task,
        error: &Option<String>,
        session: &mut SessionState,
    ) -> Result<()> {
        // Track retry count per task in session state
        let retry_count = *session.retry_counts.get(&task.id).unwrap_or(&0);

        // Emit TaskFailed event for conductor UI
        self.emit_event(
            track_id,
            &EngineEvent::TaskFailed {
                iteration,
                task_id: task.id.clone(),
                error: error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                can_retry: retry_count < self.config.max_retries,
                can_skip: matches!(self.config.error_strategy, ErrorStrategy::Skip),
                retry_count,
                max_retries: self.config.max_retries,
                timestamp: Utc::now().to_rfc3339(),
            },
        );

        match self.config.error_strategy {
            ErrorStrategy::Retry => {
                if retry_count >= self.config.max_retries {
                    error!(
                        "Task {} failed after {} retries, aborting: {:?}",
                        task.id, retry_count, error
                    );
                    return Err(anyhow!(
                        "Task {} failed after {} retries (max: {})",
                        task.id,
                        retry_count,
                        self.config.max_retries
                    ));
                }
                // Next retry attempt (1-indexed)
                let next_attempt = retry_count + 1;

                // Exponential backoff: base * 3^attempt_index (attempt_index starts at 0)
                let backoff_ms = self
                    .config
                    .retry_backoff_base_ms
                    .saturating_mul(3u64.saturating_pow(retry_count));

                warn!(
                    "Task {} failed (attempt {}/{}), retrying after {}ms: {:?}",
                    task.id, next_attempt, self.config.max_retries, backoff_ms, error
                );

                // Emit retry event for conductor (Ralph TUI pattern)
                self.emit_event(
                    track_id,
                    &EngineEvent::TaskRetrying {
                        iteration,
                        task_id: task.id.clone(),
                        retry_attempt: next_attempt,
                        max_retries: self.config.max_retries,
                        delay_ms: backoff_ms,
                        error: error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                        timestamp: Utc::now().to_rfc3339(),
                    },
                );

                // Update retry count in session state
                session.retry_counts.insert(task.id.clone(), next_attempt);
                session.updated_at = Utc::now().to_rfc3339();

                // Wait for backoff before retrying
                if backoff_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
            ErrorStrategy::Skip => {
                warn!("Task {} failed, skipping: {:?}", task.id, error);
                // Mark as completed to move on (but note it was skipped)
                let task_ref = self.find_task_mut(&mut plan.tasks, &task.id)?;
                task_ref.status = TrackStatus::Completed;
                task_ref.notes = Some(format!(
                    "SKIPPED due to error: {}",
                    error.as_deref().unwrap_or("unknown")
                ));

                // Clear retry counter when skipping
                session.retry_counts.remove(&task.id);
            }
            ErrorStrategy::Abort => {
                error!("Task {} failed, aborting: {:?}", task.id, error);
                session.retry_counts.remove(&task.id);
                return Err(anyhow!(
                    "Task {} failed, aborting track: {}",
                    task.id,
                    error.as_deref().unwrap_or("unknown")
                ));
            }
        }
        Ok(())
    }

    // Helper to find a mutable task reference by ID
    fn find_task_mut<'a>(&'a self, tasks: &'a mut [Task], task_id: &str) -> Result<&'a mut Task> {
        for task in tasks {
            if task.id == task_id {
                return Ok(task);
            }
            if let Some(subtask) = self.find_subtask_mut(&mut task.subtasks, task_id)? {
                return Ok(subtask);
            }
        }
        Err(anyhow!("Task not found: {}", task_id))
    }

    // Helper to find an immutable task reference by ID
    fn find_task<'a>(&'a self, tasks: &'a [Task], task_id: &str) -> Result<&'a Task> {
        for task in tasks {
            if task.id == task_id {
                return Ok(task);
            }
            if let Some(subtask) = self.find_subtask(&task.subtasks, task_id)? {
                return Ok(subtask);
            }
        }
        Err(anyhow!("Task not found: {}", task_id))
    }

    fn find_subtask<'a>(&'a self, subtasks: &'a [Task], task_id: &str) -> Result<Option<&'a Task>> {
        for subtask in subtasks {
            if subtask.id == task_id {
                return Ok(Some(subtask));
            }
            if let Some(found) = self.find_subtask(&subtask.subtasks, task_id)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    fn find_subtask_mut<'a>(
        &'a self,
        subtasks: &'a mut [Task],
        task_id: &str,
    ) -> Result<Option<&'a mut Task>> {
        for subtask in subtasks {
            if subtask.id == task_id {
                return Ok(Some(subtask));
            }
            if let Some(found) = self.find_subtask_mut(&mut subtask.subtasks, task_id)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    async fn verify_track_integrity(
        &self,
        track_id: &str,
        plan: &TrackPlan,
        session: &SessionState,
    ) -> Result<()> {
        info!(
            "Running final autonomous verification for track: {}",
            track_id
        );

        // Create a verification task
        let verify_task = Task {
            id: "track-verification".to_string(),
            title: "Final Track Verification".to_string(),
            description: "Verify that all implemented features are working as expected and meet the specification.".to_string(),
            status: TrackStatus::InProgress,
            dependencies: Vec::new(),
            subtasks: Vec::new(),
            notes: None,
            line_number: 0,
        };

        let result = self
            .run_iteration(track_id, &verify_task, session, plan)
            .await?;

        if result.success && result.completed {
            Ok(())
        } else {
            Err(anyhow!(
                "Verification failed: {}",
                result.error_message.unwrap_or_default()
            ))
        }
    }
}
