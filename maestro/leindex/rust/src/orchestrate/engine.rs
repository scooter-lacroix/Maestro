//! Orchestrate execution engine
//!
//! Core iteration loop: select → prompt → run → detect completion → update

use crate::orchestrate::model::*;
use crate::orchestrate::parser::{parse_plan_md, write_plan_md};
use crate::orchestrate::runner::{AgentRunner, RunResult};
use crate::orchestrate::state::{LockGuard, StateManager};
use crate::orchestrate::prompts::PromptBuilder;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn, error};

/// Orchestrate engine
pub struct OrchestrateEngine {
    config: OrchestrateConfig,
    state_manager: StateManager,
    tracks_dir: PathBuf,
    memory_service: Option<crate::memory::MemoryService>,
    rate_limit_detector: std::sync::Arc<tokio::sync::Mutex<crate::rate_limit::RateLimitDetector>>,
}

impl OrchestrateEngine {
    /// Create a new orchestrate engine
    pub fn new(config: OrchestrateConfig, tracks_dir: PathBuf) -> Result<Self> {
        let state_manager = StateManager::new(config.data_dir.clone())
            .context("Failed to initialize state manager")?;
        
        let memory_service = crate::memory::MemoryService::new(None).ok();

        let rate_limit_detector = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::rate_limit::RateLimitDetector::new(
                config.rate_limit_max_retries,
                config.rate_limit_backoff_base_secs,
            )
        ));
        Ok(Self {
            config,
            state_manager,
            tracks_dir,
            memory_service,
            rate_limit_detector,
        })
    }

    /// Start orchestrate loop for a track
    pub async fn start(
        &self,
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

        // Main loop
        loop {
            // Check for pause/interrupt
            if session.status == SessionStatus::Paused {
                info!("Session paused, sleeping...");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            if session.status != SessionStatus::Running {
                break;
            }

            // Select next actionable task
            let (task_id, task_title, task_description) = match self.select_next_task(&plan, &session)? {
                Some(t) => (t.id.clone(), t.title.clone(), t.description.clone()),
                None => {
                    // All tasks marked complete. Perform final Track Verification.
                    info!("All tasks in track {} marked as complete. Verifying integrity...", track_id);
                    if let Err(e) = self.verify_track_integrity(track_id, &plan, &session).await {
                        warn!("Track verification failed: {}. Re-opening relevant tasks.", e);
                        // logic to re-open tasks would go here
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        continue;
                    }

                    info!("Track verification successful. Track complete!");
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
                        if iter_res.rate_limited && self.config.enable_rate_limit_detection {
                            let mut detector = self.rate_limit_detector.lock().await;
                            detector.record_hit();
                            let detector_state = detector.state.clone();
                            drop(detector);

                            // Update session with rate limit state for TUI polling
                            session.rate_limit = Some(detector_state.clone());
                            session.updated_at = Utc::now().to_rfc3339();
                            self.state_manager.save_session(&session)?;

                            if detector_state.consecutive_hits > self.config.rate_limit_max_retries {
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
                                    task_id, detector_state.consecutive_hits
                                ));
                            }

                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            
                            let backoff_secs = detector_state.backoff_until.unwrap_or(now) - now;

                            warn!(
                                "Rate limit detected on task {} (hit {}/{}), backing off {}s",
                                task_id,
                                detector_state.consecutive_hits,
                                self.config.rate_limit_max_retries,
                                backoff_secs
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
                        let mut detector = self.rate_limit_detector.lock().await;
                        detector.reset();
                        drop(detector);
                        
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
                    // Only mark task complete if agent explicitly completed the task
                    // (success + completed flag means the agent signaled completion)
                    if iteration_result.success && iteration_result.completed {
                        // Mark task complete
                        self.mark_task_complete(&mut plan, &task_id)?;
                        info!(
                            "Iteration {} completed: task {} (detected <promise>COMPLETE</promise>)",
                            session.current_iteration, task_id
                        );
                    } else if iteration_result.success {
                        // Process succeeded but task wasn't completed (partial progress)
                        info!(
                            "Iteration {} made progress but task not yet complete: {}",
                            session.current_iteration, task_id
                        );
                    } else {
                        // Handle failure based on error strategy
                        self.handle_task_failure(&mut plan, &temp_task, &iteration_result.error_message)?;
                    }
                }
                Err(e) => {
                    error!("Iteration failed: {}", e);
                    self.handle_task_failure(&mut plan, &temp_task, &Some(e.to_string()))?;
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

    /// Pause the orchestrate loop
    pub fn pause(&self, track_id: &str) -> Result<()> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        session.status = SessionStatus::Paused;
        session.updated_at = Utc::now().to_rfc3339();
        self.state_manager.save_session(&session)?;

        info!("Paused orchestrate loop for track {}", track_id);
        Ok(())
    }

    /// Resume the orchestrate loop
    pub fn resume(&self, track_id: &str) -> Result<()> {
        let mut session = self
            .state_manager
            .load_session(track_id)?
            .ok_or_else(|| anyhow!("No active session for track: {}", track_id))?;

        session.status = SessionStatus::Running;
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
                Ok(plan.all_tasks().iter().find(|t| t.status == TrackStatus::Pending).copied())
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
        
        // Bank a task completion memory
        if let Some(ref svc) = self.memory_service {
            let content = format!("Completed task '{}' in track '{}'", task_id, plan.track_id);
            let _ = svc.store_memory(&content, crate::memory::models::MemoryCategory::Decision);
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
            if self.update_task_status_recursive(&mut task.subtasks, task_id, status).is_ok() {
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
        .map_err(|_| anyhow!("Iteration timeout after {} seconds", self.config.iteration_timeout_secs))??;

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

    fn build_prompt(&self, task: &Task, session: &SessionState, plan: &TrackPlan) -> Result<String> {
        let builder = PromptBuilder::new(self.config.context_budget);

        // Get recent iterations
        let recent = self
            .state_manager
            .recent_iterations(&plan.track_id, 5)
            .unwrap_or_default();

        // Get LeIndex context if enabled
        let leindex_context = if self.config.enable_leindex {
            let engine = crate::orchestrate::context::ContextEngine::new(self.config.context_budget);
            Some(engine.build_context(&self.tracks_dir, plan)?)
        } else {
            None
        };

        builder.build_prompt(task, session, plan, &recent, leindex_context.as_deref())
    }

    fn handle_task_failure(
        &self,
        plan: &mut TrackPlan,
        task: &Task,
        error: &Option<String>,
    ) -> Result<()> {
        match self.config.error_strategy {
            ErrorStrategy::Retry => {
                // Parse retry count from task notes (format: "retries: N")
                let retry_count = task.notes
                    .as_ref()
                    .and_then(|n| n.split("retries:").nth(1))
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(0);

                if retry_count >= self.config.max_retries {
                    error!(
                        "Task {} failed after {} retries, aborting: {:?}",
                        task.id, retry_count, error
                    );
                    return Err(anyhow!(
                        "Task {} failed after {} retries (max: {})",
                        task.id, retry_count, self.config.max_retries
                    ));
                }

                // Increment retry counter
                warn!(
                    "Task {} failed (attempt {}/{}), will retry: {:?}",
                    task.id, retry_count + 1, self.config.max_retries, error
                );

                // Update task notes with incremented retry count
                let task_ref = self.find_task_mut(&mut plan.tasks, &task.id)?;
                task_ref.notes = Some(format!("retries: {}", retry_count + 1));
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
            }
            ErrorStrategy::Abort => {
                error!("Task {} failed, aborting: {:?}", task.id, error);
                return Err(anyhow!("Task {} failed, aborting track: {}", task.id,
                    error.as_deref().unwrap_or("unknown")));
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

    fn find_subtask_mut<'a>(&'a self, subtasks: &'a mut [Task], task_id: &str) -> Result<Option<&'a mut Task>> {
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

    async fn verify_track_integrity(&self, track_id: &str, plan: &TrackPlan, session: &SessionState) -> Result<()> {
        info!("Running final autonomous verification for track: {}", track_id);
        
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

        let result = self.run_iteration(track_id, &verify_task, session, plan).await?;
        
        if result.success && result.completed {
            Ok(())
        } else {
            Err(anyhow!("Verification failed: {}", result.error_message.unwrap_or_default()))
        }
    }
}

