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
}

impl OrchestrateEngine {
    /// Create a new orchestrate engine
    pub fn new(config: OrchestrateConfig, tracks_dir: PathBuf) -> Result<Self> {
        let state_manager = StateManager::new(config.data_dir.clone())
            .context("Failed to initialize state manager")?;
        Ok(Self {
            config,
            state_manager,
            tracks_dir,
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
                    info!("No more actionable tasks. Track complete!");
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

            let result = self
                .run_iteration(track_id, &temp_task, &session, &plan)
                .await;

            match result {
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
                session.updated_at = Utc::now().to_rfc3339();
                Ok(session)
            }
            None => {
                // Create new session
                let now = Utc::now().to_rfc3339();
                Ok(SessionState {
                    track_id: track_id.to_string(),
                    mode,
                    agent_config,
                    current_iteration: 0,
                    current_task_id: None,
                    started_at: now.clone(),
                    updated_at: now,
                    status: SessionStatus::Running,
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
        self.update_task_status_recursive(&mut plan.tasks, task_id, TrackStatus::Completed)
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
            Some(self.get_leindex_context(plan)?)
        } else {
            None
        };

        builder.build_prompt(task, session, plan, &recent, leindex_context.as_deref())
    }

    fn get_leindex_context(&self, plan: &TrackPlan) -> Result<String> {
        use crate::five_phase::{phase1_structural_scan, phase2_dependency_map, PhaseOptions};
        use crate::token_format::FormatMode;

        // Skip LeIndex if context budget is too low (< 10K tokens)
        // This prevents expensive scans that would be truncated anyway
        const MIN_BUDGET_FOR_LEINDEX: usize = 10000;
        if self.config.context_budget < MIN_BUDGET_FOR_LEINDEX {
            return Ok("// LeIndex disabled: context budget too low".to_string());
        }

        // Use the track's canonical root directory for LeIndex analysis
        // This ensures we scan the actual project code, not the tracks directory
        let track_path = self.tracks_dir.join(&plan.track_id);
        let project_root = if track_path.exists() {
            // Track directory exists - use it as the project root
            track_path.clone()
        } else {
            // Fallback to parent directory (workspace root)
            self.tracks_dir.clone()
        };

        // Determine the analysis mode based on context budget
        let mode = if self.config.context_budget > 50000 {
            FormatMode::Balanced
        } else {
            FormatMode::Ultra
        };

        // Run 5-phase analysis with appropriate token limits
        // Cap max_files based on budget to prevent excessive scans
        let max_files = std::cmp::min(15, self.config.context_budget / 3000);

        let options = PhaseOptions {
            root: project_root,
            mode,
            max_files,
            max_focus_files: std::cmp::min(3, max_files / 5),
            top_n: 10,
            max_output_chars: self.config.context_budget / 2, // Use half budget for LeIndex
        };

        // Run Phase 1 and Phase 2 analysis
        let phase1_result = phase1_structural_scan(&options);
        let phase2_result = phase2_dependency_map(&options);

        match (phase1_result, phase2_result) {
            (Ok(p1), Ok(p2)) => {
                let mut context = String::new();

                // Add phase summaries
                context.push_str(&format!("### Phase 1: Structural Scan\n\n{}\n\n", p1));
                context.push_str(&format!("### Phase 2: Dependency Map\n\n{}\n\n", p2));

                Ok(context)
            }
            (Err(e), _) | (_, Err(e)) => {
                // If LeIndex analysis fails, log but don't fail the iteration
                warn!("LeIndex analysis failed for track {}: {}", plan.track_id, e);
                Ok(format!("// LeIndex analysis failed: {}\n// Continuing with task...\n", e))
            }
        }
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
}

