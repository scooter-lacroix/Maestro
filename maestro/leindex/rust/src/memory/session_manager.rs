//! Session Manager logic
//!
//! Orchestrates session lifecycles, tool integration, and multiplexer control.

use anyhow::{Context, Result};
use chrono::Utc;

use super::models::{Session, SessionStatus};
use super::service::MemoryService;
use crate::multiplexer::{TmuxMultiplexer, TmuxSession};

pub struct SessionManager {
    service: MemoryService,
    tmux: TmuxMultiplexer,
}

impl SessionManager {
    pub fn new(service: MemoryService) -> Self {
        Self {
            service,
            tmux: TmuxMultiplexer::new(),
        }
    }

    /// Create and start a new session
    pub fn create_session(
        &self,
        title: &str,
        project_path: &str,
        tool: &str,
        command: Option<&str>,
        group_path: Option<&str>,
    ) -> Result<Session> {
        // Create tmux session first to get the actual session name
        let mut tmux_session = TmuxSession::new(title, project_path);

        // Construct command if not provided
        let run_cmd = match command {
            Some(c) => Some(c.to_string()),
            None => Some(self.build_tool_command(tool, project_path)?),
        };

        // Start the tmux session
        self.tmux
            .start_session(&mut tmux_session, run_cmd.as_deref())
            .context("Failed to start tmux session")?;

        // Create database record with the ACTUAL tmux session name
        let session = Session {
            id: 0,
            session_id: tmux_session.name.clone(), // Use tmux session name!
            title: title.to_string(),
            project_path: project_path.to_string(),
            group_path: group_path.map(|s| s.to_string()),
            parent_session_id: None,
            command: run_cmd,
            tool: Some(tool.to_string()),
            status: SessionStatus::Running,
            multiplexer_session: Some(tmux_session.name.clone()),
            started_at: Utc::now(),
            last_accessed_at: Some(Utc::now()),
            ended_at: None,
            metadata: None,
        };

        // Save to DB
        self.service.import_session(session.clone())?;

        Ok(session)
    }

    /// Build the specific command for a tool
    fn build_tool_command(&self, tool: &str, project_path: &str) -> Result<String> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        match tool.to_lowercase().as_str() {
            "claude" => Ok(format!(
                "export EDITOR={}; cd {} && claude",
                editor, project_path
            )),
            "gemini" => Ok(format!(
                "export EDITOR={}; cd {} && gemini",
                editor, project_path
            )),
            "amp" => Ok(format!(
                "export EDITOR={}; cd {} && amp",
                editor, project_path
            )),
            "opencode" => Ok(format!(
                "export EDITOR={}; cd {} && opencode",
                editor, project_path
            )),
            "codex" => Ok(format!(
                "export EDITOR={}; cd {} && codex",
                editor, project_path
            )),
            "shell" | _ => {
                // Default to interactive shell
                Ok(format!("export EDITOR={}; cd {}", editor, project_path))
            }
        }
    }

    /// Attach to an existing session
    pub fn attach_session(&self, session_id: &str) -> Result<()> {
        // Update last accessed time in DB
        self.service.update_last_accessed(session_id)?;

        // Use tmux session name directly
        TmuxMultiplexer::attach(session_id)?;
        Ok(())
    }

    /// Check if a session exists in tmux
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.tmux.session_exists(session_id)
    }

    /// Kill a session and update DB
    pub fn kill_session(&self, session_id: &str) -> Result<()> {
        self.tmux.kill_session(session_id)?;
        self.service
            .update_session_status(session_id, SessionStatus::Terminated)?;
        Ok(())
    }

    /// Rename a session and update DB
    pub fn rename_session(&self, old_session_id: &str, new_title: &str) -> Result<String> {
        let new_session_id = self.tmux.rename_session(old_session_id, new_title)?;

        // Update DB: title and the session_id (which is the tmux name)
        self.service
            .update_session_rename(old_session_id, &new_session_id, new_title)?;

        Ok(new_session_id)
    }

    /// Fork a session and update DB
    pub fn fork_session(
        &self,
        original_session_id: &str,
        new_title: &str,
        original_session: &Session,
    ) -> Result<Session> {
        let new_session_id = self.tmux.fork_session(original_session_id, new_title)?;

        let mut new_session = original_session.clone();
        new_session.id = 0;
        new_session.session_id = new_session_id.clone();
        new_session.title = new_title.to_string();
        new_session.multiplexer_session = Some(new_session_id);
        new_session.started_at = Utc::now();
        new_session.last_accessed_at = Some(Utc::now());
        new_session.status = SessionStatus::Running;

        self.service.import_session(new_session.clone())?;

        Ok(new_session)
    }
}
