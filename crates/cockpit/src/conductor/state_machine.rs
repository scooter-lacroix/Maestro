//! Conductor state machine and transitions
//!
//! Handles the logic for valid state transitions and engine guards.

use super::model::{ConductorState, ConductorStatus, ConductorEvent, StopReason, OutputStream};

impl ConductorState {
    /// Check if the engine can transition to Started
    pub fn can_start(&self) -> bool {
        matches!(self.status, ConductorStatus::Ready | ConductorStatus::Idle | ConductorStatus::Completed | ConductorStatus::Failed)
    }
    
    /// Check if the engine can be paused
    pub fn can_pause(&self) -> bool {
        matches!(self.status, ConductorStatus::Running | ConductorStatus::Selecting | ConductorStatus::Executing)
    }
    
    /// Check if the engine can be resumed
    pub fn can_resume(&self) -> bool {
        matches!(self.status, ConductorStatus::Paused)
    }

    /// Check if the engine can be stopped
    pub fn can_stop(&self) -> bool {
        !matches!(self.status, ConductorStatus::Stopping | ConductorStatus::Ready)
    }

    /// Apply an event to the state machine to transition the status
    pub fn transition(&mut self, event: &ConductorEvent) {
        match (&self.status, event) {
            (ConductorStatus::Ready | ConductorStatus::Idle | ConductorStatus::Completed | ConductorStatus::Failed, ConductorEvent::Started { total_tasks, .. }) => {
                self.status = ConductorStatus::Running;
                self.total_tasks = *total_tasks;
                self.tasks_completed = 0;
            }
            (ConductorStatus::Running | ConductorStatus::Selecting | ConductorStatus::Executing, ConductorEvent::TaskSelected { task_id, .. }) => {
                // Only transition to Selecting if we're not already executing this task
                if self.current_task.as_ref() != Some(task_id) || !matches!(self.status, ConductorStatus::Executing) {
                    self.status = ConductorStatus::Selecting;
                }
                self.current_task = Some(task_id.clone());
            }
            (ConductorStatus::Running | ConductorStatus::Selecting | ConductorStatus::Executing, ConductorEvent::IterationStarted { iteration, .. }) => {
                self.status = ConductorStatus::Executing;
                self.current_iteration = *iteration;
                self.current_output.clear();
                self.current_stderr.clear();
            }
            (ConductorStatus::Executing, ConductorEvent::IterationCompleted { .. }) => {
                self.status = ConductorStatus::Running;
            }
            (ConductorStatus::Executing, ConductorEvent::IterationFailed { error, .. }) => {
                self.status = ConductorStatus::Running;
                self.current_stderr.push_str(&format!("Error: {}\n", error));
            }
            (_, ConductorEvent::Paused) => {
                self.status = ConductorStatus::Paused;
            }
            (ConductorStatus::Paused, ConductorEvent::Resumed) => {
                self.status = ConductorStatus::Running;
            }
            (_, ConductorEvent::Stopped { reason, .. }) => {
                match reason {
                    StopReason::Completed => self.status = ConductorStatus::Completed,
                    StopReason::Error => self.status = ConductorStatus::Failed,
                    StopReason::NoTasks => self.status = ConductorStatus::Idle,
                    StopReason::Interrupted => self.status = ConductorStatus::Ready,
                    StopReason::MaxIterations => self.status = ConductorStatus::Completed,
                }
            }
            (_, ConductorEvent::AllComplete { total_completed, .. }) => {
                self.status = ConductorStatus::Completed;
                self.tasks_completed = *total_completed;
            }
            (_, ConductorEvent::TaskCompleted { .. }) => {
                self.status = ConductorStatus::Running;
            }
            (_, ConductorEvent::AgentOutput { stream, data }) => {
                match stream {
                    OutputStream::Stdout => self.current_output.push_str(data),
                    OutputStream::Stderr => self.current_stderr.push_str(data),
                }
            }
            _ => {}
        }
    }
}
