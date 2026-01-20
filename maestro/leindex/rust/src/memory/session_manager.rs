//! Session Manager logic
//!
//! Orchestrates session lifecycles, tool integration, and multiplexer control.

use anyhow::{Context, Result};
use chrono::Utc;

use super::models::{Session, SessionStatus};
#[cfg(feature = "rusqlite")]
use super::service::MemoryService;
use crate::multiplexer::{TmuxMultiplexer, TmuxSession};
use tokio::runtime::Handle;

#[cfg(feature = "rusqlite")]
use super::lsp_manager::{LspManager, LspType};

#[cfg(feature = "rusqlite")]
pub struct SessionManager {
    service: MemoryService,
    tmux: TmuxMultiplexer,
    lsp_manager: std::sync::Mutex<Option<LspManager>>,
    lsp_manager_init: std::sync::Once,
}

#[cfg(feature = "rusqlite")]
impl SessionManager {
    pub fn new(service: MemoryService) -> Result<Self> {
        Ok(Self {
            service,
            tmux: TmuxMultiplexer::new(),
            lsp_manager: std::sync::Mutex::new(None),
            lsp_manager_init: std::sync::Once::new(),
        })
    }

    /// Get or initialize the LSP manager (lazy initialization)
    ///
    /// This method lazily initializes the LspManager only when first needed,
    /// using in-memory storage to avoid blocking on SessionManager creation.
    fn get_lsp_manager(&self) -> Result<std::sync::MutexGuard<'_, Option<LspManager>>> {
        // Initialize on first access
        self.lsp_manager_init.call_once(|| {
            // Use blocking runtime for one-time initialization with in-memory storage
            // This avoids the runtime blocking issue while providing basic functionality
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                if let Ok(storage) = rt.block_on(
                    crate::memory::turso_backend::TursoStorageBackend::in_memory(None)
                ) {
                    let manager = LspManager::new(storage);
                    // Restore any auto-start LSPs from persisted state (best-effort).
                    if let Err(e) = rt.block_on(manager.restore_lsps_on_startup()) {
                        tracing::warn!("Failed to restore LSPs on startup: {}", e);
                    }
                    *self.lsp_manager.lock().unwrap() = Some(manager);
                }
            }
        });

        Ok(self.lsp_manager.lock().unwrap())
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
            None => Some(self.build_tool_command(tool, project_path, &tmux_session.name)?),
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
            sort_order: 0,
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

        // Auto-start LSPs for the session based on project language detection
        // Note: This is done in a separate task to avoid blocking session creation
        let project_path_buf = std::path::PathBuf::from(project_path);
        let session_id = session.session_id.clone();

        // Get a reference to LSP manager for the spawned task
        // We clone the Arc from inside the mutex to avoid holding the lock across await
        let lsp_manager_clone: Option<LspManager> = if let Ok(guard) = self.get_lsp_manager() {
            guard.as_ref().cloned()
        } else {
            None
        };

        // Attempt to spawn a task to auto-start LSPs in the background
        if let (Some(lsp_manager), Ok(handle)) = (lsp_manager_clone, Handle::try_current()) {
            handle.spawn(async move {
                match lsp_manager.auto_start_lsps_for_session(&session_id, &project_path_buf).await {
                    Ok(started_lsps) => {
                        tracing::info!("Auto-started {} LSPs for session '{}': {:?}",
                                      started_lsps.len(), session_id, started_lsps);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to auto-start LSPs for session '{}': {}", session_id, e);
                    }
                }
            });
        } else {
            // If not in a tokio runtime, we can't start LSPs automatically
            tracing::warn!("Cannot auto-start LSPs: not in a tokio runtime");
        }

        Ok(session)
    }

    /// Build the specific command for a tool
    fn build_tool_command(&self, tool: &str, project_path: &str, session_id: &str) -> Result<String> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let mcp_config_path = self.write_tool_search_mcp_config(session_id)?;
        let mcp_config = mcp_config_path.to_string_lossy().to_string();

        match tool.to_lowercase().as_str() {
            "claude" => Ok(format!(
                "export EDITOR={}; cd {} && claude --strict-mcp-config --mcp-config {}",
                editor, project_path, shell_escape(&mcp_config)
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
                "export EDITOR={}; cd {} && codex -c 'mcp_servers={{}}' -c 'mcp_servers.maestro_tool_search.command=\"maestro\"' -c 'mcp_servers.maestro_tool_search.args=[\"mcp\",\"tool-search\"]'",
                editor, project_path
            )),
            "shell" | _ => {
                // Default to interactive shell
                Ok(format!("export EDITOR={}; cd {}", editor, project_path))
            }
        }
    }

    fn write_tool_search_mcp_config(&self, session_id: &str) -> Result<std::path::PathBuf> {
        self.write_mcp_config_with_lsps(session_id, &[])
    }

    /// Write MCP configuration file with LSP entries
    ///
    /// Generates a .mcp.json file that includes:
    /// - Existing MCP servers (like maestro-tool-search)
    /// - LSP server entries for direct stdio exposure
    ///
    /// The LSP section follows the format defined in:
    /// maestro/leindex/docs/lsp-mcp-json-format.md
    fn write_mcp_config_with_lsps(&self, session_id: &str, lsp_types: &[LspType]) -> Result<std::path::PathBuf> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "maestro-mcp-config-{}.json",
            sanitize_filename(session_id)
        ));

        // Get session's project path from database
        let project_path = self.service
            .get_session_project_path(session_id)
            .context("Failed to get session project path")?
            .unwrap_or_else(|| std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(".".to_string()));

        // Build mcpServers section (existing MCP servers)
        let mcp_servers = serde_json::json!({
            "maestro-tool-search": {
                "command": "maestro",
                "args": ["mcp", "tool-search"],
                "type": "stdio"
            }
        });

        // Build LSP servers section from provided LSP types or detect from project
        let lsp_servers = if lsp_types.is_empty() {
            // Auto-detect LSPs from project path
            self.detect_lsps_for_project(&project_path)?
        } else {
            // Use provided LSP types
            lsp_types.iter()
                .filter_map(|lsp_type| self.build_lsp_entry(lsp_type, session_id, &project_path).ok())
                .collect()
        };

        // Combine into final config
        let config = if lsp_servers.is_empty() {
            serde_json::json!({
                "mcpServers": mcp_servers
            })
        } else {
            serde_json::json!({
                "mcpServers": mcp_servers,
                "lsp": {
                    "servers": lsp_servers
                }
            })
        };

        // Write atomically to avoid partial writes
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, serde_json::to_string_pretty(&config)?)
            .with_context(|| format!("Failed to write MCP config to {:?}", temp_path))?;
        std::fs::rename(&temp_path, &path)
            .with_context(|| format!("Failed to rename MCP config to {:?}", path))?;

        Ok(path)
    }

    /// Build an LSP entry for .mcp.json configuration
    fn build_lsp_entry(
        &self,
        lsp_type: &LspType,
        session_id: &str,
        project_path: &str,
    ) -> Result<serde_json::Value> {
        let sanitized_session = sanitize_filename(session_id);

        // Define capabilities based on LSP type
        let capabilities: Vec<&str> = match lsp_type {
            LspType::Rust => vec!["completion", "inlayHint", "definition", "hover"],
            LspType::Python => vec!["completion", "definition", "hover"],
            LspType::TypeScript => vec!["completion", "definition", "references"],
        };

        // Build args array with default additional args
        let args: Vec<String> = lsp_type.default_additional_args()
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok(serde_json::json!({
            "name": format!("{}-{}", lsp_type.display_name(), sanitized_session),
            "language": lsp_type.language(),
            "displayName": lsp_type.display_name(),
            "command": lsp_type.binary_name(),
            "args": args,
            "type": "stdio",
            "session_id": session_id,
            "project_path": project_path,
            "capabilities": capabilities,
            "transport": "stdio"
        }))
    }

    /// Detect which LSPs should be used for a project based on file extensions
    fn detect_lsps_for_project(&self, project_path: &str) -> Result<Vec<serde_json::Value>> {
        let mut lsp_entries = Vec::new();
        let mut detected_languages = std::collections::HashSet::new();

        // Walk the project directory to detect file extensions
        let path = std::path::Path::new(project_path);
        if !path.exists() {
            return Ok(lsp_entries);
        }

        // Scan for file extensions (limit depth for performance)
        let max_depth = 3;
        let mut visited = std::collections::HashSet::new();
        let mut dirs_to_visit = vec![path.to_path_buf()];

        while let Some(current_dir) = dirs_to_visit.pop() {
            if visited.contains(&current_dir) || visited.len() > 1000 {
                continue;
            }
            visited.insert(current_dir.clone());

            // Skip hidden directories and common non-source directories
            if let Some(dir_name) = current_dir.file_name() {
                let name = dir_name.to_string_lossy();
                if name.starts_with('.') || name == "node_modules" || name == "target" ||
                   name == "vendor" || name == "build" || name == "dist" {
                    continue;
                }
            }

            let entries = match std::fs::read_dir(&current_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let file_type = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_dir() {
                    if visited.len() < max_depth * 100 {
                        dirs_to_visit.push(entry.path());
                    }
                } else if file_type.is_file() {
                    if let Some(ext) = entry.path().extension() {
                        let ext_str = ext.to_string_lossy();
                        match ext_str.as_ref() {
                            "rs" => { detected_languages.insert(LspType::Rust); }
                            "py" => { detected_languages.insert(LspType::Python); }
                            "ts" | "tsx" | "js" | "jsx" => { detected_languages.insert(LspType::TypeScript); }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Build LSP entries for detected languages
        let session_id = format!("auto-detected");
        for lsp_type in detected_languages {
            if let Ok(entry) = self.build_lsp_entry(&lsp_type, &session_id, project_path) {
                lsp_entries.push(entry);
            }
        }

        Ok(lsp_entries)
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
        // Stop all LSPs for this session first
        let session_id_clone = session_id.to_string();

        // Get LSP manager reference for the spawned task
        let lsp_manager_clone: Option<LspManager> = if let Ok(guard) = self.get_lsp_manager() {
            guard.as_ref().cloned()
        } else {
            None
        };

        // Attempt to stop LSPs in a separate task
        if let (Some(lsp_manager), Ok(handle)) = (lsp_manager_clone, Handle::try_current()) {
            handle.spawn(async move {
                match lsp_manager.stop_all_session_lsps(&session_id_clone).await {
                    Ok(()) => {
                        tracing::info!("Successfully stopped all LSPs for session '{}'", session_id_clone);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to stop LSPs for session '{}': {}", session_id_clone, e);
                    }
                }
            });
        } else {
            tracing::warn!("Cannot stop LSPs: not in a tokio runtime");
        }

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

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}
