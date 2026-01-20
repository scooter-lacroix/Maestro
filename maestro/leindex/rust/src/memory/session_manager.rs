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

/// Mode for restoring a session
#[cfg(feature = "rusqlite")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRestoreMode {
    /// Resume the session if it exists, otherwise recreate it
    Resume,
    /// Force restart the session (kill existing if running, then recreate)
    Restart,
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
    pub(crate) fn detect_lsps_for_project(&self, project_path: &str) -> Result<Vec<serde_json::Value>> {
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
        let root_path = path.to_path_buf();

        while let Some(current_dir) = dirs_to_visit.pop() {
            if visited.contains(&current_dir) || visited.len() > 1000 {
                continue;
            }
            visited.insert(current_dir.clone());

            // Skip hidden directories and common non-source directories
            // IMPORTANT: Don't skip the root project path, even if it starts with '.'
            if current_dir != root_path {
                if let Some(dir_name) = current_dir.file_name() {
                    let name = dir_name.to_string_lossy();
                    if name.starts_with('.') || name == "node_modules" || name == "target" ||
                       name == "vendor" || name == "build" || name == "dist" {
                        continue;
                    }
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

    /// Restore a session (resume existing or recreate it)
    ///
    /// ## Arguments
    ///
    /// - `session`: The session to restore
    /// - `mode`: Restore mode (Resume or Restart)
    ///
    /// ## Behavior
    ///
    /// - `Resume`: If tmux session exists, does nothing. If not, recreates it.
    /// - `Restart`: Kills existing tmux session if running, then recreates it.
    pub fn restore_session(&self, session: &Session, mode: SessionRestoreMode) -> Result<()> {
        let session_id = &session.session_id;
        let title = &session.title;
        let project_path = &session.project_path;

        match mode {
            SessionRestoreMode::Resume => {
                // If session already exists in tmux, nothing to do
                if self.tmux.session_exists(session_id) {
                    tracing::debug!("Session '{}' already running, resuming", session_id);
                    // Update status to Running in case it was stale
                    self.service
                        .update_session_status(session_id, SessionStatus::Running)?;
                    return Ok(());
                }
                // Fall through to recreate
            }
            SessionRestoreMode::Restart => {
                // Kill existing session if it's running
                if self.tmux.session_exists(session_id) {
                    tracing::debug!("Restarting session '{}': killing existing", session_id);
                    let _ = self.tmux.kill_session(session_id);
                }
            }
        }

        // Recreate the tmux session
        let mut tmux_session = TmuxSession::new(title, project_path);

        // Use the stored command if available, otherwise build from tool
        let run_cmd = session.command.clone().or_else(|| {
            session.tool.as_ref().map(|tool| {
                self.build_tool_command(tool, project_path, &tmux_session.name)
                    .unwrap_or_else(|_| format!("cd {}", project_path))
            })
        });

        // Start the tmux session
        self.tmux
            .start_session(&mut tmux_session, run_cmd.as_deref())
            .context("Failed to start tmux session during restore")?;

        // Update status to Running
        self.service
            .update_session_status(session_id, SessionStatus::Running)?;

        tracing::info!("Session '{}' restored successfully", session_id);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test-session-123"), "test-session-123");
        assert_eq!(sanitize_filename("session@test/123"), "session_test_123");
        assert_eq!(sanitize_filename("session.with.dots"), "session_with_dots");
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("simple"), "'simple'");
        // The function wraps in single quotes, replacing single quotes with '"'"'
        // For "with 'quotes'", the single quote gets replaced with '"'"', resulting in:
        // 'with '"' quotes' (the single quote in the middle is replaced)
        let result = shell_escape("with 'quotes'");
        assert!(result.starts_with("'") && result.ends_with("'"));
        assert!(result.contains("with"));
        assert!(result.contains("quotes"));
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_mcp_config_with_no_lsps() {
        // Test that MCP config without LSPs works correctly
        let service = MemoryService::new(None).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();

        // Create a test session
        let session_id = "test-session-no-lsp";
        let project_path = "/tmp/test";

        session_manager.service.import_session(Session {
            id: 0,
            session_id: session_id.to_string(),
            title: "Test Session".to_string(),
            project_path: project_path.to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Running,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).unwrap();

        // Generate MCP config
        let config_path = session_manager.write_mcp_config_with_lsps(session_id, &[]).unwrap();

        // Verify file exists and is valid JSON
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify mcpServers exists
        assert!(json.get("mcpServers").is_some());

        // Verify lsp section does NOT exist when no LSPs
        assert!(json.get("lsp").is_none());

        // Clean up
        std::fs::remove_file(&config_path).ok();
    }

    #[test]
    fn test_mcp_config_with_lsps() {
        // Test that MCP config with LSPs includes lsp section
        let service = MemoryService::new(None).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();

        let session_id = "test-session-with-lsp";
        let project_path = "/tmp/test";

        session_manager.service.import_session(Session {
            id: 0,
            session_id: session_id.to_string(),
            title: "Test Session".to_string(),
            project_path: project_path.to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Running,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).unwrap();

        // Generate MCP config with Rust LSP
        let lsp_types = vec![LspType::Rust];
        let config_path = session_manager.write_mcp_config_with_lsps(session_id, &lsp_types).unwrap();

        // Verify file exists and is valid JSON
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify mcpServers exists
        assert!(json.get("mcpServers").is_some());

        // Verify lsp section exists when LSPs are provided
        let lsp = json.get("lsp").unwrap();
        let servers = lsp.get("servers").unwrap().as_array().unwrap();

        // Verify Rust LSP entry
        assert!(!servers.is_empty());
        let rust_lsp = &servers[0];
        assert_eq!(rust_lsp["language"], "rust");
        assert_eq!(rust_lsp["displayName"], "rust-analyzer");
        assert_eq!(rust_lsp["command"], "rust-analyzer");

        // Verify capabilities
        let capabilities = rust_lsp["capabilities"].as_array().unwrap();
        assert!(capabilities.iter().any(|c| c == "completion"));
        assert!(capabilities.iter().any(|c| c == "inlayHint"));

        // Clean up
        std::fs::remove_file(&config_path).ok();
    }

    #[test]
    fn test_detect_lsps_for_rust_project() {
        let service = MemoryService::new(None).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with Rust files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create some Rust files
        std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp_dir.path().join("lib.rs"), "pub fn test() {}").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager.detect_lsps_for_project(&project_path).unwrap();

        // Should detect Rust LSP
        assert!(!lsp_entries.is_empty(), "No LSP entries detected for Rust project");
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "rust"), "Rust LSP not found in entries");
    }

    #[test]
    fn test_detect_lsps_for_python_project() {
        let service = MemoryService::new(None).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with Python files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create some Python files
        std::fs::write(temp_dir.path().join("main.py"), "print('hello')").unwrap();
        std::fs::write(temp_dir.path().join("lib.py"), "def test(): pass").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager.detect_lsps_for_project(&project_path).unwrap();

        // Should detect Python LSP
        assert!(!lsp_entries.is_empty());
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "python"));
    }

    #[test]
    fn test_detect_lsps_for_typescript_project() {
        let service = MemoryService::new(None).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with TypeScript files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create some TypeScript files
        std::fs::write(temp_dir.path().join("main.ts"), "console.log('hello')").unwrap();
        std::fs::write(temp_dir.path().join("app.tsx"), "export default function App() {}").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager.detect_lsps_for_project(&project_path).unwrap();

        // Should detect TypeScript LSP
        assert!(!lsp_entries.is_empty());
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "typescript"));
    }

    #[test]
    fn test_detect_lsps_for_mixed_project() {
        let service = MemoryService::new(None).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with mixed language files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create files from different languages
        std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp_dir.path().join("script.py"), "print('hello')").unwrap();
        std::fs::write(temp_dir.path().join("app.tsx"), "export default function App() {}").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager.detect_lsps_for_project(&project_path).unwrap();

        // Should detect all three LSPs
        assert!(!lsp_entries.is_empty());
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "rust"));
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "python"));
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "typescript"));
    }

    #[test]
    fn test_detect_lsps_for_empty_project() {
        let service = MemoryService::new(None).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create an empty temporary directory
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Detect LSPs
        let lsp_entries = session_manager.detect_lsps_for_project(&project_path).unwrap();

        // Should return empty list
        assert!(lsp_entries.is_empty());
    }

    #[test]
    fn test_detect_lsps_skips_non_source_dirs() {
        let service = MemoryService::new(None).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create non-source directories
        let node_modules = temp_dir.path().join("node_modules");
        std::fs::create_dir(&node_modules).unwrap();
        std::fs::write(node_modules.join("package.json"), "{}").unwrap();

        let target = temp_dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("lib.rs"), "fn main() {}").unwrap();

        // Create actual source file
        std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager.detect_lsps_for_project(&project_path).unwrap();

        // Should detect Rust LSP from main.rs but not from target directory
        assert!(!lsp_entries.is_empty());
        assert_eq!(lsp_entries.len(), 1);
    }

    #[test]
    fn test_get_session_project_path() {
        let service = MemoryService::new(None).unwrap();
        service.initialize().unwrap();

        // Import a test session
        let session_id = "test-session-path";
        let project_path = "/tmp/test-project";

        service.import_session(Session {
            id: 0,
            session_id: session_id.to_string(),
            title: "Test".to_string(),
            project_path: project_path.to_string(),
            group_path: None,
            sort_order: 0,
            parent_session_id: None,
            command: None,
            tool: None,
            status: SessionStatus::Running,
            multiplexer_session: None,
            started_at: Utc::now(),
            last_accessed_at: None,
            ended_at: None,
            metadata: None,
        }).unwrap();

        // Query project path
        let result = service.get_session_project_path(session_id).unwrap();

        // Should return the correct project path
        assert_eq!(result, Some(project_path.to_string()));
    }

    #[test]
    fn test_get_session_project_path_not_found() {
        let service = MemoryService::new(None).unwrap();
        service.initialize().unwrap();

        // Query non-existent session
        let result = service.get_session_project_path("non-existent-session").unwrap();

        // Should return None
        assert!(result.is_none());
    }
}
