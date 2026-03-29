//! Session Manager logic
//!
//! Orchestrates session lifecycles, tool integration, and multiplexer control.
//!
//! ## LSP Proxy Integration
//!
//! The LSP stdio proxy (src/lsp/stdio_proxy.rs) is currently NOT integrated.
//! This is intentional design - proxy requires:
//! - Session lifecycle management
//! - Configuration option to enable/disable
//! - Production testing
//!
//! TODO: Add use_proxy: bool parameter to build_lsp_entry() when ready

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::models::{McpTransport, MemoryCategory, Session, SessionStatus};
#[cfg(feature = "rusqlite")]
use super::service::MemoryService;
use crate::multiplexer::{TmuxMultiplexer, TmuxSession};
use crate::provider_boundary::{
    managed_cli_overlap_matrix_for, managed_cli_overlap_profile, AnalysisProviderKind,
    LaunchOrigin, MemoryProvider, MemoryProviderKind, PooledMcpServerRef, ProviderDiagnostic,
    ProviderMcpConfig, ProviderStatus, SessionProviderProfile,
};
use crate::providers::{StandaloneLeIndexProvider, StandaloneNexusProvider};
use tokio::runtime::Handle;

#[cfg(feature = "rusqlite")]
use super::lsp_manager::{LspManager, LspType};

#[cfg(feature = "rusqlite")]
use tempfile::NamedTempFile;

#[cfg(feature = "rusqlite")]
pub struct SessionManager {
    service: MemoryService,
    tmux: TmuxMultiplexer,
    lsp_manager: std::sync::OnceLock<LspManager>,
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
            lsp_manager: std::sync::OnceLock::new(),
        })
    }

    /// Inject an existing LspManager into the SessionManager
    ///
    /// This method allows pre-initializing the LspManager outside of a tokio runtime context,
    /// preventing the "Cannot start a runtime from within a runtime" panic when the
    /// SessionManager is used from within async contexts like the TUI.
    pub fn with_lsp_manager(self, manager: LspManager) -> Self {
        let _ = self.lsp_manager.set(manager);
        self
    }

    /// Get or initialize the LSP manager (lazy initialization)
    ///
    /// This method lazily initializes the LspManager only when first needed.
    /// It now checks if we're already in a tokio runtime and avoids creating a nested one.
    fn get_lsp_manager(&self) -> Option<&LspManager> {
        // Fast path: already initialized
        if let Some(mgr) = self.lsp_manager.get() {
            return Some(mgr);
        }

        // Check if we're already in a tokio runtime — can't create a nested one
        if Handle::try_current().is_ok() {
            tracing::debug!(
                "Skipping lazy LSP manager init in async context to avoid nested runtime panic"
            );
            tracing::debug!("LSP features will be unavailable. Use SessionManager::with_lsp_manager() to pre-initialize.");
            return None;
        }

        // Build outside the lock, then set atomically via OnceLock
        let rt = tokio::runtime::Runtime::new().ok()?;
        let storage = rt
            .block_on(crate::memory::turso_backend::TursoStorageBackend::in_memory(None))
            .ok()?;
        let manager = LspManager::new(storage);
        if let Err(e) = rt.block_on(manager.restore_lsps_on_startup()) {
            tracing::warn!("Failed to restore LSPs on startup: {}", e);
        }

        // OnceLock::set races safely — if another thread beat us, we discard ours
        let _ = self.lsp_manager.set(manager);
        self.lsp_manager.get()
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

        // Bank memory for session start (best-effort)
        let memory_content = format!(
            "Session '{}' started with tool '{}' at project '{}'. Session ID: {}. Started at: {}",
            title,
            tool,
            project_path,
            session.session_id,
            session.started_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = self
            .service
            .store_memory(&memory_content, MemoryCategory::Observation);
        let _ = self.notify_nexus_session_started(&session.session_id, project_path);

        // Auto-start LSPs for the session based on project language detection
        // Note: This is done in a separate task to avoid blocking session creation
        let project_path_buf = std::path::PathBuf::from(project_path);
        let session_id = session.session_id.clone();

        // Get a reference to LSP manager for the spawned task
        // We clone the Arc from inside the mutex to avoid holding the lock across await
        let lsp_manager_clone = self.get_lsp_manager().cloned();

        // Attempt to spawn a task to auto-start LSPs in the background
        if let (Some(lsp_manager), Ok(handle)) = (lsp_manager_clone, Handle::try_current()) {
            let session_id_clone = session_id.clone();
            handle.spawn(async move {
                match lsp_manager
                    .auto_start_lsps_for_session(&session_id_clone, &project_path_buf)
                    .await
                {
                    Ok(started_lsps) => {
                        tracing::info!(
                            "Auto-started {} LSPs for session '{}': {:?}",
                            started_lsps.len(),
                            session_id_clone,
                            started_lsps
                        );

                        // Regenerate MCP config with proxy-enabled entries after LSPs start
                        // Note: We need the SessionManager reference, which we don't have here.
                        // The config will be regenerated on the next attach/restore or when explicitly requested.
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to auto-start LSPs for session '{}': {}",
                            session_id_clone,
                            e
                        );
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
    ///
    /// All user inputs are properly escaped to prevent shell injection.
    ///
    /// Sets the following environment variables for all CLI tools:
    /// - MAESTRO_SESSION_ID: Session identifier for memory banking
    /// - MAESTRO_PROJECT_PATH: Project root path for context
    /// - MAESTRO_MCP_CONFIG: Path to .mcp.json with pooled MCP and LSP entries
    fn build_tool_command(
        &self,
        tool: &str,
        project_path: &str,
        session_id: &str,
    ) -> Result<String> {
        let editor = shell_escape(&std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string()));
        let escaped_project = shell_escape(project_path);
        let escaped_session_id = shell_escape(session_id);
        let mcp_config_path = self.write_session_mcp_config(session_id)?;
        let mcp_config = shell_escape(mcp_config_path.to_string_lossy().as_ref());
        let profile = self.build_session_provider_profile(session_id, project_path)?;
        let provider_profile = shell_escape("maestro_runtime");
        let analysis_provider = shell_escape("standalone_leindex");
        let memory_provider = shell_escape("standalone_nexus");
        let suppression_policy = shell_escape(&profile.suppression_policy.to_json_string()?);
        let overlap_profile = shell_escape(&serde_json::to_string(&managed_cli_overlap_profile(
            &profile.selected_cli,
        ))?);
        let nexus_bin = StandaloneNexusProvider::discover()
            .map(|provider| {
                shell_escape(
                    provider
                        .installation()
                        .executable
                        .to_string_lossy()
                        .as_ref(),
                )
            })
            .unwrap_or_else(|| shell_escape("nexus"));
        let nexus_home = StandaloneNexusProvider::discover()
            .and_then(|provider| {
                provider
                    .state_root
                    .map(|root| shell_escape(root.to_string_lossy().as_ref()))
            })
            .unwrap_or_else(|| shell_escape(""));

        // Common environment variables for all tools
        let env_vars = format!(
            "export EDITOR={} MAESTRO_SESSION_ID={} MAESTRO_PROJECT_PATH={} MAESTRO_SELECTED_CLI={} MAESTRO_MCP_CONFIG={} MAESTRO_PROVIDER_PROFILE={} MAESTRO_ANALYSIS_PROVIDER={} MAESTRO_MEMORY_PROVIDER={} MAESTRO_TOOL_SUPPRESSION_POLICY={} MAESTRO_CLI_OVERLAP_PROFILE={} MAESTRO_NEXUS_PROVIDER=standalone NEXUS_BIN={} NEXUS_HOME={}",
            editor,
            escaped_session_id,
            escaped_project,
            shell_escape(&profile.selected_cli),
            mcp_config,
            provider_profile,
            analysis_provider,
            memory_provider,
            suppression_policy,
            overlap_profile,
            nexus_bin,
            nexus_home
        );

        match tool.to_lowercase().as_str() {
            "claude" => Ok(format!(
                "{}; cd {} && claude --strict-mcp-config --mcp-config {}",
                env_vars, escaped_project, mcp_config
            )),
            "gemini" => {
                let settings_path =
                    self.write_tool_system_settings_file(session_id, "gemini", false)?;
                Ok(format!(
                    "{}; export GEMINI_CLI_SYSTEM_SETTINGS_PATH={}; cd {} && gemini",
                    env_vars,
                    shell_escape(&settings_path.to_string_lossy()),
                    escaped_project
                ))
            }
            "qwen" => {
                let settings_path =
                    self.write_tool_system_settings_file(session_id, "qwen", false)?;
                Ok(format!(
                    "{}; export QWEN_CODE_SYSTEM_SETTINGS_PATH={}; cd {} && qwen",
                    env_vars,
                    shell_escape(&settings_path.to_string_lossy()),
                    escaped_project
                ))
            }
            "iflow" => {
                let settings_path =
                    self.write_tool_system_settings_file(session_id, "iflow", false)?;
                Ok(format!(
                    "{}; export IFLOW_CLI_SYSTEM_SETTINGS_PATH={}; cd {} && iflow",
                    env_vars,
                    shell_escape(&settings_path.to_string_lossy()),
                    escaped_project
                ))
            }
            "amp" => {
                let amp_mcp_config_path = self.write_amp_mcp_config_file(session_id)?;
                Ok(format!(
                    "{}; cd {} && amp --mcp-config {}",
                    env_vars,
                    escaped_project,
                    shell_escape(&amp_mcp_config_path.to_string_lossy())
                ))
            }
            "opencode" => {
                let opencode_config_path = self.write_opencode_config_file(session_id)?;
                Ok(format!(
                    "{}; export OPENCODE_CONFIG={}; cd {} && opencode",
                    env_vars,
                    shell_escape(&opencode_config_path.to_string_lossy()),
                    escaped_project
                ))
            }
            "codex" => {
                let mut command = format!(
                    "{}; cd {} && codex -c {}",
                    env_vars,
                    escaped_project,
                    shell_escape("mcp_servers={}")
                );
                for override_arg in self.build_codex_mcp_overrides(session_id, project_path)? {
                    command.push_str(" -c ");
                    command.push_str(&shell_escape(&override_arg));
                }
                Ok(command)
            }
            "droid" => {
                let home_root = self.write_standard_home_tool_settings(
                    session_id,
                    "droid",
                    ".factory",
                    "mcp.json",
                    &[],
                    true,
                )?;
                Ok(format!(
                    "{}; export HOME={}; cd {} && droid",
                    env_vars,
                    shell_escape(&home_root.to_string_lossy()),
                    escaped_project
                ))
            }
            _ => {
                // Default to interactive shell with environment variables
                Ok(format!("{}; cd {}", env_vars, escaped_project))
            }
        }
    }

    fn write_session_mcp_config(&self, session_id: &str) -> Result<std::path::PathBuf> {
        // For the synchronous call (during session creation), we use direct stdio mode
        // Proxy-enabled entries require async access to LspManager
        self.write_mcp_config_with_lsps(session_id, &[])
    }

    fn build_mcp_servers_config_with_stdio_type(
        &self,
        session_id: &str,
        project_path: &str,
        include_stdio_type: bool,
    ) -> Result<BTreeMap<String, serde_json::Value>> {
        let provider_config =
            self.build_provider_mcp_config(session_id, project_path, include_stdio_type)?;
        let mut servers = provider_config.direct_servers;
        servers.extend(provider_config.pooled_servers);
        Ok(servers)
    }

    fn build_provider_mcp_config(
        &self,
        _session_id: &str,
        _project_path: &str,
        include_stdio_type: bool,
    ) -> Result<ProviderMcpConfig> {
        let provider = StandaloneLeIndexProvider::detect()?
            .context("Standalone LeIndex provider not found. Install LeIndex before launching managed Maestro sessions.")?;
        let direct_servers = BTreeMap::from([(
            "leindex".to_string(),
            provider.direct_stdio_config(include_stdio_type),
        )]);

        let mut pooled_servers = BTreeMap::new();
        if let Ok(pool_servers) = self.service.list_mcp_servers() {
            for server in pool_servers {
                let name = server.name.clone();
                if name == "maestro-tool-search" || name.eq_ignore_ascii_case("leindex") {
                    continue;
                }

                let config = match server.transport {
                    McpTransport::Stdio => stdio_mcp_config(
                        "maestro",
                        vec!["mcp".to_string(), "proxy".to_string(), name.clone()],
                        include_stdio_type,
                    ),
                    McpTransport::Http => {
                        let Some(url) = server.url.clone() else {
                            continue;
                        };
                        let mut value = serde_json::json!({
                            "type": "http",
                            "url": url
                        });
                        if let Some(headers) = server.headers.clone() {
                            value["headers"] = headers;
                        }
                        value
                    }
                };

                pooled_servers.insert(name, config);
            }
        }

        Ok(ProviderMcpConfig {
            direct_servers,
            pooled_servers,
        })
    }

    fn build_session_provider_profile(
        &self,
        session_id: &str,
        project_path: &str,
    ) -> Result<SessionProviderProfile> {
        let leindex_provider = StandaloneLeIndexProvider::detect()?
            .context("Standalone LeIndex provider not found. Install LeIndex before launching managed Maestro sessions.")?;
        let nexus_provider = StandaloneNexusProvider::discover()
            .context("Standalone Nexus provider not found. Install and initialize Nexus before launching managed Maestro sessions.")?;
        let selected_cli = self
            .service
            .list_sessions()
            .ok()
            .and_then(|sessions| {
                sessions
                    .into_iter()
                    .find(|session| session.session_id == session_id)
                    .and_then(|session| session.tool)
            })
            .unwrap_or_else(|| "unknown".to_string());
        let overlap_profile = managed_cli_overlap_profile(&selected_cli);

        let pooled_shared_servers = self
            .service
            .list_mcp_servers()
            .map(|servers| {
                servers
                    .into_iter()
                    .filter(|server| {
                        server.name != "maestro-tool-search"
                            && !server.name.eq_ignore_ascii_case("leindex")
                    })
                    .map(|server| PooledMcpServerRef {
                        name: server.name,
                        transport: match server.transport {
                            McpTransport::Stdio => "stdio".to_string(),
                            McpTransport::Http => "http".to_string(),
                        },
                        source: "maestro_pool".to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SessionProviderProfile {
            profile_name: "maestro_runtime".to_string(),
            launch_origin: LaunchOrigin::Sessions,
            selected_cli,
            project_root: std::path::PathBuf::from(project_path),
            session_id: session_id.to_string(),
            track_id: None,
            analysis_provider: AnalysisProviderKind::StandaloneLeindex,
            memory_provider: MemoryProviderKind::StandaloneNexus,
            pooled_shared_servers,
            suppression_policy: overlap_profile.suppression_policy(),
            overlap_matrix: managed_cli_overlap_matrix_for(&overlap_profile.cli),
            diagnostics: vec![
                leindex_provider
                    .diagnostic_snapshot(std::path::Path::new(project_path))
                    .ok()
                    .unwrap_or_else(|| ProviderDiagnostic {
                        provider_name: "leindex".to_string(),
                        provider_kind: "analysis".to_string(),
                        status: ProviderStatus::Degraded,
                        executable: Some("leindex".to_string()),
                        version: None,
                        source: Some("standalone".to_string()),
                        detail: "Standalone LeIndex provider detected but diagnostics could not be collected"
                            .to_string(),
                        capabilities: ["mcp"].into_iter().map(str::to_string).collect(),
                        checked_at: Utc::now(),
                    }),
                nexus_provider
                    .health_report_sync(std::path::Path::new(project_path))
                    .ok()
                    .and_then(|report| report.diagnostics.into_iter().next())
                    .unwrap_or_else(|| ProviderDiagnostic {
                        provider_name: "nexus".to_string(),
                        provider_kind: "memory".to_string(),
                        status: ProviderStatus::Degraded,
                        executable: Some("nexus".to_string()),
                        version: None,
                        source: Some("standalone".to_string()),
                        detail: "Standalone Nexus provider detected but diagnostics could not be collected"
                            .to_string(),
                        capabilities: ["memory", "runtime", "digests"]
                            .into_iter()
                            .map(str::to_string)
                            .collect(),
                        checked_at: Utc::now(),
                    }),
            ],
        })
    }

    fn build_provider_diagnostics_json(
        &self,
        profile: &SessionProviderProfile,
        mcp_config: &ProviderMcpConfig,
    ) -> serde_json::Value {
        let overlap_profile = managed_cli_overlap_profile(&profile.selected_cli);
        serde_json::json!({
            "managedSession": {
                "profileName": profile.profile_name,
                "launchOrigin": profile.launch_origin,
                "selectedCli": profile.selected_cli,
                "analysisProvider": profile.analysis_provider,
                "memoryProvider": profile.memory_provider,
                "suppressionPolicy": &profile.suppression_policy,
                "cliOverlapProfile": overlap_profile,
                "directServers": mcp_config.direct_servers.keys().cloned().collect::<Vec<_>>(),
                "pooledServers": mcp_config.pooled_servers.keys().cloned().collect::<Vec<_>>(),
                "pooledSharedServers": profile.pooled_shared_servers.iter().map(|server| {
                    serde_json::json!({
                        "name": server.name,
                        "transport": server.transport,
                        "source": server.source,
                    })
                }).collect::<Vec<_>>(),
            }
        })
    }

    fn build_standard_settings_json(
        &self,
        session_id: &str,
        project_path: &str,
        existing: serde_json::Value,
        include_stdio_type: bool,
    ) -> Result<serde_json::Value> {
        let profile = self.build_session_provider_profile(session_id, project_path)?;
        let provider_mcp =
            self.build_provider_mcp_config(session_id, project_path, include_stdio_type)?;
        let mut settings = ensure_json_object(existing);
        settings.as_object_mut().unwrap().insert(
            "mcpServers".to_string(),
            serde_json::to_value(self.build_mcp_servers_config_with_stdio_type(
                session_id,
                project_path,
                include_stdio_type,
            )?)?,
        );
        settings.as_object_mut().unwrap().insert(
            "maestro".to_string(),
            self.build_provider_diagnostics_json(&profile, &provider_mcp),
        );
        Ok(settings)
    }

    fn build_opencode_settings_json(
        &self,
        session_id: &str,
        project_path: &str,
        existing: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let profile = self.build_session_provider_profile(session_id, project_path)?;
        let provider_mcp = self.build_provider_mcp_config(session_id, project_path, false)?;
        let mut settings = ensure_json_object(existing);
        settings.as_object_mut().unwrap().insert(
            "mcp".to_string(),
            serde_json::to_value(
                self.build_opencode_mcp_servers_config(session_id, project_path)?,
            )?,
        );
        settings.as_object_mut().unwrap().insert(
            "maestro".to_string(),
            self.build_provider_diagnostics_json(&profile, &provider_mcp),
        );
        Ok(settings)
    }

    fn build_opencode_mcp_servers_config(
        &self,
        session_id: &str,
        project_path: &str,
    ) -> Result<BTreeMap<String, serde_json::Value>> {
        let mut servers = BTreeMap::new();

        for (name, config) in
            self.build_mcp_servers_config_with_stdio_type(session_id, project_path, false)?
        {
            let opencode_config = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
                let mut value = serde_json::json!({
                    "type": "http",
                    "url": url
                });
                if let Some(headers) = config.get("headers") {
                    value["headers"] = headers.clone();
                }
                value
            } else {
                let command = config
                    .get("command")
                    .and_then(|value| value.as_str())
                    .unwrap_or("maestro")
                    .to_string();
                let mut command_parts = vec![serde_json::Value::String(command)];
                if let Some(args) = config.get("args").and_then(|value| value.as_array()) {
                    command_parts.extend(args.iter().cloned());
                }
                serde_json::json!({
                    "type": "local",
                    "command": command_parts,
                    "environment": config
                        .get("env")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({}))
                })
            };

            servers.insert(name, opencode_config);
        }

        Ok(servers)
    }

    fn build_codex_mcp_overrides(
        &self,
        session_id: &str,
        project_path: &str,
    ) -> Result<Vec<String>> {
        let mut overrides = Vec::new();

        for (name, config) in
            self.build_mcp_servers_config_with_stdio_type(session_id, project_path, true)?
        {
            let key = sanitize_codex_key(&name);

            if let Some(command) = config.get("command").and_then(|v| v.as_str()) {
                overrides.push(format!(
                    "mcp_servers.{}.command={}",
                    key,
                    toml_string_literal(command)
                ));
            }

            if let Some(args) = config.get("args") {
                if let Some(args_literal) = toml_literal(args) {
                    overrides.push(format!("mcp_servers.{}.args={}", key, args_literal));
                }
            }

            if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
                overrides.push(format!(
                    "mcp_servers.{}.url={}",
                    key,
                    toml_string_literal(url)
                ));
            }

            if let Some(headers) = config.get("headers") {
                if let Some(headers_literal) = toml_literal(headers) {
                    overrides.push(format!("mcp_servers.{}.headers={}", key, headers_literal));
                }
            }
        }

        Ok(overrides)
    }

    fn write_standard_home_tool_settings(
        &self,
        session_id: &str,
        tool_name: &str,
        config_dir_rel: &str,
        config_file_name: &str,
        skip_dirs: &[&str],
        include_stdio_type: bool,
    ) -> Result<std::path::PathBuf> {
        let home_root =
            self.prepare_session_home_overlay(session_id, tool_name, config_dir_rel, skip_dirs)?;
        let config_path = home_root.join(config_dir_rel).join(config_file_name);
        let existing = read_json_value_or_empty(&config_path)?;
        let project_path = self
            .service
            .get_session_project_path(session_id)?
            .unwrap_or_else(|| ".".to_string());
        let updated = self.build_standard_settings_json(
            session_id,
            &project_path,
            existing,
            include_stdio_type,
        )?;
        write_json_value_atomic(&config_path, &updated)?;
        Ok(home_root)
    }

    fn write_tool_system_settings_file(
        &self,
        session_id: &str,
        tool_name: &str,
        include_stdio_type: bool,
    ) -> Result<std::path::PathBuf> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "maestro-{}-settings-{}.json",
            tool_name,
            sanitize_filename(session_id)
        ));

        let project_path = self
            .service
            .get_session_project_path(session_id)?
            .unwrap_or_else(|| ".".to_string());
        let updated = self.build_standard_settings_json(
            session_id,
            &project_path,
            serde_json::json!({}),
            include_stdio_type,
        )?;
        write_json_value_atomic(&path, &updated)?;
        Ok(path)
    }

    fn write_amp_mcp_config_file(&self, session_id: &str) -> Result<std::path::PathBuf> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "maestro-amp-mcp-config-{}.json",
            sanitize_filename(session_id)
        ));

        let project_path = self
            .service
            .get_session_project_path(session_id)?
            .unwrap_or_else(|| ".".to_string());
        let config = serde_json::to_value(self.build_mcp_servers_config_with_stdio_type(
            session_id,
            &project_path,
            false,
        )?)?;
        write_json_value_atomic(&path, &config)?;
        Ok(path)
    }

    fn write_opencode_config_file(&self, session_id: &str) -> Result<std::path::PathBuf> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "maestro-opencode-config-{}.json",
            sanitize_filename(session_id)
        ));

        let existing_path = dirs::home_dir()
            .map(|home| home.join(".config").join("opencode").join("opencode.json"));
        let existing = match existing_path {
            Some(path) => read_json_value_or_empty(&path)?,
            None => serde_json::json!({}),
        };
        let project_path = self
            .service
            .get_session_project_path(session_id)?
            .unwrap_or_else(|| ".".to_string());
        let updated = self.build_opencode_settings_json(session_id, &project_path, existing)?;
        write_json_value_atomic(&path, &updated)?;
        Ok(path)
    }

    fn prepare_session_home_overlay(
        &self,
        session_id: &str,
        tool_name: &str,
        config_dir_rel: &str,
        skip_dirs: &[&str],
    ) -> Result<std::path::PathBuf> {
        let mut home_root = std::env::temp_dir();
        home_root.push(format!(
            "maestro-{}-home-{}",
            tool_name,
            sanitize_filename(session_id)
        ));

        if home_root.exists() {
            std::fs::remove_dir_all(&home_root)
                .with_context(|| format!("Failed to reset {:?}", home_root))?;
        }
        std::fs::create_dir_all(&home_root)
            .with_context(|| format!("Failed to create {:?}", home_root))?;

        if let Some(real_home) = dirs::home_dir() {
            let source_dir = real_home.join(config_dir_rel);
            if source_dir.exists() {
                let skip = skip_dirs.iter().copied().collect::<BTreeSet<_>>();
                copy_dir_recursive_filtered(&source_dir, &home_root.join(config_dir_rel), &skip)?;
            }
        }

        Ok(home_root)
    }

    /// Write MCP configuration with LSP entries including proxy support (async version)
    ///
    /// This is the preferred method for generating MCP config as it can include
    /// proxy-enabled LSP entries when available.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_manager`: Optional reference to LspManager for proxy information
    pub async fn write_mcp_config_with_proxy(
        &self,
        session_id: &str,
        lsp_manager: Option<&LspManager>,
    ) -> Result<std::path::PathBuf> {
        // Get the list of LSPs running for this session
        let lsp_types = if let Some(manager) = lsp_manager {
            manager
                .session_lsps(session_id)
                .await
                .into_iter()
                .map(|(lsp_type, _)| lsp_type)
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        self.write_mcp_config_with_lsps_async(session_id, &lsp_types, lsp_manager)
            .await
    }

    /// Write MCP configuration file with LSP entries
    ///
    /// Generates a .mcp.json file that includes:
    /// - Direct provider entries (for example standalone LeIndex)
    /// - Pooled shared MCP servers from the Maestro pool
    /// - LSP server entries for direct stdio exposure
    ///
    /// The LSP section follows the format defined in:
    /// maestro/leindex/docs/lsp-mcp-json-format.md
    fn write_mcp_config_with_lsps(
        &self,
        session_id: &str,
        lsp_types: &[LspType],
    ) -> Result<std::path::PathBuf> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "maestro-mcp-config-{}.json",
            sanitize_filename(session_id)
        ));

        // Get session's project path from database
        let project_path = self
            .service
            .get_session_project_path(session_id)
            .context("Failed to get session project path")?
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or(".".to_string())
            });

        let profile = self.build_session_provider_profile(session_id, &project_path)?;
        let provider_mcp = self.build_provider_mcp_config(session_id, &project_path, true)?;
        let mcp_servers =
            self.build_mcp_servers_config_with_stdio_type(session_id, &project_path, true)?;

        // Build LSP servers section from provided LSP types or detect from project
        let lsp_servers = if lsp_types.is_empty() {
            // Auto-detect LSPs from project path (pass session_id for correct association)
            // Note: Auto-detected LSPs do not use proxy (no LspManager context available)
            self.detect_lsps_for_project(&project_path, session_id)?
        } else {
            // Use provided LSP types
            // Note: We don't have access to LspManager proxy info here, so direct stdio only
            // Proxy-enabled entries must be generated through a different path
            lsp_types
                .iter()
                .filter_map(|lsp_type| {
                    self.build_lsp_entry(lsp_type, session_id, &project_path, false, None)
                        .ok()
                })
                .collect()
        };

        // Combine into final config
        let config = if lsp_servers.is_empty() {
            serde_json::json!({
                "mcpServers": mcp_servers,
                "maestro": self.build_provider_diagnostics_json(&profile, &provider_mcp)
            })
        } else {
            serde_json::json!({
                "mcpServers": mcp_servers,
                "maestro": self.build_provider_diagnostics_json(&profile, &provider_mcp),
                "lsp": {
                    "servers": lsp_servers
                }
            })
        };

        // Write atomically using secure temp file with O_EXCL
        // This prevents symlink attacks and ensures atomic writes
        let temp_file =
            NamedTempFile::new().with_context(|| "Failed to create secure temp file")?;

        std::fs::write(temp_file.path(), serde_json::to_string_pretty(&config)?)
            .with_context(|| format!("Failed to write MCP config to {:?}", temp_file.path()))?;

        temp_file
            .persist(&path)
            .with_context(|| format!("Failed to persist MCP config to {:?}", path))?;

        Ok(path)
    }

    /// Write MCP configuration file with LSP entries including stdio-proxy if enabled
    ///
    /// This is an async version that can query the LspManager for proxy socket paths.
    /// Use this when you have access to LspManager and want to include proxy-enabled entries.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    /// - `lsp_types`: LSP types to include in the configuration
    /// - `lsp_manager`: Reference to LspManager for proxy information
    async fn write_mcp_config_with_lsps_async(
        &self,
        session_id: &str,
        lsp_types: &[LspType],
        lsp_manager: Option<&LspManager>,
    ) -> Result<std::path::PathBuf> {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "maestro-mcp-config-{}.json",
            sanitize_filename(session_id)
        ));

        // Get session's project path from database
        let project_path = self
            .service
            .get_session_project_path(session_id)
            .context("Failed to get session project path")?
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or(".".to_string())
            });

        let profile = self.build_session_provider_profile(session_id, &project_path)?;
        let provider_mcp = self.build_provider_mcp_config(session_id, &project_path, true)?;
        let mcp_servers =
            self.build_mcp_servers_config_with_stdio_type(session_id, &project_path, true)?;

        // Build LSP servers section from provided LSP types or detect from project
        let lsp_servers = if lsp_types.is_empty() {
            // Auto-detect LSPs from project path (direct stdio only, no proxy context)
            self.detect_lsps_for_project(&project_path, session_id)?
        } else {
            // Build LSP entries with proxy information if available
            let mut entries = Vec::new();
            for lsp_type in lsp_types {
                // Get proxy socket path and use_proxy flag from LspManager
                let (use_proxy, proxy_socket_path) = if let Some(manager) = lsp_manager {
                    let socket_path = manager.get_proxy_socket_path(session_id, *lsp_type).await;
                    (
                        socket_path.is_some(),
                        socket_path.map(|p| p.to_string_lossy().to_string()),
                    )
                } else {
                    // No LspManager context - use direct stdio
                    (false, None)
                };

                if let Ok(entry) = self.build_lsp_entry(
                    lsp_type,
                    session_id,
                    &project_path,
                    use_proxy,
                    proxy_socket_path.as_deref(),
                ) {
                    entries.push(entry);
                }
            }
            entries
        };

        // Combine into final config
        let config = if lsp_servers.is_empty() {
            serde_json::json!({
                "mcpServers": mcp_servers,
                "maestro": self.build_provider_diagnostics_json(&profile, &provider_mcp)
            })
        } else {
            serde_json::json!({
                "mcpServers": mcp_servers,
                "maestro": self.build_provider_diagnostics_json(&profile, &provider_mcp),
                "lsp": {
                    "servers": lsp_servers
                }
            })
        };

        // Write atomically using secure temp file with O_EXCL
        let temp_file =
            NamedTempFile::new().with_context(|| "Failed to create secure temp file")?;

        std::fs::write(temp_file.path(), serde_json::to_string_pretty(&config)?)
            .with_context(|| format!("Failed to write MCP config to {:?}", temp_file.path()))?;

        temp_file
            .persist(&path)
            .with_context(|| format!("Failed to persist MCP config to {:?}", path))?;

        Ok(path)
    }

    /// Build an LSP entry for .mcp.json configuration
    ///
    /// ## Arguments
    ///
    /// - `lsp_type`: The LSP type to build an entry for
    /// - `session_id`: Session identifier
    /// - `project_path`: Project root path
    /// - `use_proxy`: Whether to use stdio-proxy transport
    /// - `proxy_socket_path`: Optional Unix socket path if proxy is enabled
    fn build_lsp_entry(
        &self,
        lsp_type: &LspType,
        session_id: &str,
        project_path: &str,
        use_proxy: bool,
        proxy_socket_path: Option<&str>,
    ) -> Result<serde_json::Value> {
        let sanitized_session = sanitize_filename(session_id);

        // Define capabilities based on LSP type
        let capabilities: Vec<&str> = match lsp_type {
            LspType::Rust => vec!["completion", "inlayHint", "definition", "hover"],
            LspType::Python => vec!["completion", "definition", "hover"],
            LspType::TypeScript => vec!["completion", "definition", "references"],
        };

        // Build args array with default additional args
        let args: Vec<String> = lsp_type
            .default_additional_args()
            .iter()
            .map(|s| s.to_string())
            .collect();

        // Build base LSP entry
        let mut entry = serde_json::json!({
            "name": format!("{}-{}", lsp_type.display_name(), sanitized_session),
            "language": lsp_type.language(),
            "displayName": lsp_type.display_name(),
            "command": lsp_type.binary_name(),
            "args": args,
            "type": "stdio",
            "session_id": session_id,
            "project_path": project_path,
            "capabilities": capabilities,
        });

        // Add transport and proxy configuration based on use_proxy flag
        if use_proxy {
            if let Some(socket_path) = proxy_socket_path {
                // Use stdio-proxy transport with socket configuration
                entry["transport"] = serde_json::json!("stdio-proxy");
                entry["stdio_proxy"] = serde_json::json!({
                    "socket_path": socket_path,
                    "enabled": true
                });
            } else {
                // use_proxy=true but no socket path provided - log warning and fall back to direct stdio
                tracing::warn!(
                    "LSP {} requested proxy mode but no socket path provided, using direct stdio",
                    lsp_type.display_name()
                );
                entry["transport"] = serde_json::json!("stdio");
            }
        } else {
            // Direct stdio transport (default)
            entry["transport"] = serde_json::json!("stdio");
        }

        Ok(entry)
    }

    /// Detect which LSPs should be used for a project based on file extensions
    ///
    /// ## Arguments
    ///
    /// - `project_path`: Path to the project directory
    /// - `session_id`: Session ID to associate with detected LSPs
    pub(crate) fn detect_lsps_for_project(
        &self,
        project_path: &str,
        session_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let mut lsp_entries = Vec::new();
        // Use BTreeSet for deterministic JSON ordering
        let mut detected_languages = BTreeSet::new();

        // Walk the project directory to detect file extensions
        let path = std::path::Path::new(project_path);
        if !path.exists() {
            return Ok(lsp_entries);
        }

        // Scan for file extensions (limit depth for performance)
        let max_depth = 3;
        let mut visited = std::collections::HashSet::new();
        // Track (path, depth) to correctly measure depth
        let mut dirs_to_visit: Vec<(std::path::PathBuf, usize)> = vec![(path.to_path_buf(), 0)];
        let root_path = path.to_path_buf();

        while let Some((current_dir, depth)) = dirs_to_visit.pop() {
            if visited.contains(&current_dir) || visited.len() > 1000 {
                continue;
            }
            visited.insert(current_dir.clone());

            // Check actual depth, not visited count
            if depth > max_depth {
                continue;
            }

            // Skip hidden directories and common non-source directories
            // IMPORTANT: Don't skip the root project path, even if it starts with '.'
            if current_dir != root_path {
                if let Some(dir_name) = current_dir.file_name() {
                    let name = dir_name.to_string_lossy();
                    if name.starts_with('.')
                        || name == "node_modules"
                        || name == "target"
                        || name == "vendor"
                        || name == "build"
                        || name == "dist"
                    {
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
                    // Only recurse if we haven't exceeded max_depth
                    if depth < max_depth {
                        dirs_to_visit.push((entry.path(), depth + 1));
                    }
                } else if file_type.is_file() {
                    if let Some(ext) = entry.path().extension() {
                        let ext_str = ext.to_string_lossy();
                        match ext_str.as_ref() {
                            "rs" => {
                                detected_languages.insert(LspType::Rust);
                            }
                            "py" => {
                                detected_languages.insert(LspType::Python);
                            }
                            "ts" | "tsx" | "js" | "jsx" => {
                                detected_languages.insert(LspType::TypeScript);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Build LSP entries for detected languages using the actual session_id
        // Auto-detected LSPs use direct stdio mode (no proxy)
        for lsp_type in detected_languages {
            if let Ok(entry) =
                self.build_lsp_entry(&lsp_type, session_id, project_path, false, None)
            {
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

        // Recreate the tmux session using the ORIGINAL session name (not a new one)
        let mut tmux_session = TmuxSession::with_name(session_id.clone(), title, project_path);

        // Use the stored command if available, otherwise build from tool
        let run_cmd = session.command.clone().or_else(|| {
            session.tool.as_ref().map(|tool| {
                self.build_tool_command(tool, project_path, session_id)
                    .unwrap_or_else(|_| format!("cd {}", shell_escape(project_path)))
            })
        });

        // Start the tmux session
        self.tmux
            .start_session(&mut tmux_session, run_cmd.as_deref())
            .context("Failed to start tmux session during restore")?;

        // Update status to Running
        self.service
            .update_session_status(session_id, SessionStatus::Running)?;

        // Restart LSPs for this session
        let lsp_manager_clone = self.get_lsp_manager().cloned();

        let session_id_clone = session_id.clone();
        if let (Some(lsp_manager), Ok(handle)) = (lsp_manager_clone, Handle::try_current()) {
            handle.spawn(async move {
                let lsps = lsp_manager.session_lsps(&session_id_clone).await;
                tracing::info!(
                    "Restoring {} LSPs for session '{}'",
                    lsps.len(),
                    session_id_clone
                );
                for (lsp_type, _) in lsps {
                    if let Err(e) = lsp_manager.restart_lsp(&session_id_clone, lsp_type).await {
                        tracing::warn!(
                            "Failed to restart LSP {:?} for session '{}': {}",
                            lsp_type,
                            session_id_clone,
                            e
                        );
                    }
                }
            });
        }

        tracing::info!("Session '{}' restored successfully", session_id);

        Ok(())
    }

    /// Check if a session exists in tmux
    pub fn session_exists(&self, session_id: &str) -> bool {
        self.tmux.session_exists(session_id)
    }

    /// Regenerate MCP configuration for an existing session (async version)
    ///
    /// This regenerates the .mcp.json file with the current LSP states,
    /// including proxy-enabled entries when LSPs have proxy sockets available.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    pub async fn regenerate_mcp_config_async(
        &self,
        session_id: &str,
    ) -> Result<std::path::PathBuf> {
        let lsp_manager = self.get_lsp_manager().cloned();

        self.write_mcp_config_with_proxy(session_id, lsp_manager.as_ref())
            .await
    }

    /// Regenerate MCP configuration for an existing session (blocking version)
    ///
    /// This is a blocking wrapper around `regenerate_mcp_config_async` for use in
    /// synchronous contexts. It will block the current thread until the config is regenerated.
    ///
    /// ## Arguments
    ///
    /// - `session_id`: Session identifier
    ///
    /// ## Returns
    ///
    /// Returns the path to the regenerated MCP config file
    pub fn regenerate_mcp_config_blocking(&self, session_id: &str) -> Result<std::path::PathBuf> {
        // Create a new runtime for blocking execution
        let rt = tokio::runtime::Runtime::new()
            .context("Failed to create tokio runtime for MCP config regeneration")?;

        rt.block_on(self.regenerate_mcp_config_async(session_id))
    }

    /// Kill a session and update DB
    pub fn kill_session(&self, session_id: &str) -> Result<()> {
        let project_path = self
            .service
            .get_session_project_path(session_id)?
            .unwrap_or_else(|| ".".to_string());

        // Stop all LSPs for this session first
        let session_id_clone = session_id.to_string();

        // Get LSP manager reference for the spawned task
        let lsp_manager_clone = self.get_lsp_manager().cloned();

        // Attempt to stop LSPs in a separate task
        if let (Some(lsp_manager), Ok(handle)) = (lsp_manager_clone, Handle::try_current()) {
            handle.spawn(async move {
                match lsp_manager.stop_all_session_lsps(&session_id_clone).await {
                    Ok(()) => {
                        tracing::info!(
                            "Successfully stopped all LSPs for session '{}'",
                            session_id_clone
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to stop LSPs for session '{}': {}",
                            session_id_clone,
                            e
                        );
                    }
                }
            });
        } else {
            tracing::warn!("Cannot stop LSPs: not in a tokio runtime");
        }

        self.tmux.kill_session(session_id)?;
        self.service
            .update_session_status(session_id, SessionStatus::Terminated)?;

        // Bank memory for session end (best-effort)
        let memory_content = format!(
            "Session '{}' terminated. Session ID: {}. Ended at: {}",
            session_id,
            session_id,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        let _ = self
            .service
            .store_memory(&memory_content, MemoryCategory::Observation);
        let _ = self.notify_nexus_session_stopped(session_id, &project_path);

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

    fn notify_nexus_session_started(&self, session_id: &str, project_path: &str) -> Result<()> {
        let provider = match StandaloneNexusProvider::discover() {
            Some(provider) => provider,
            None => return Ok(()),
        };
        let profile = self.build_session_provider_profile(session_id, project_path)?;
        self.run_memory_provider_future(provider.session_started(&profile))
    }

    fn notify_nexus_session_stopped(&self, session_id: &str, project_path: &str) -> Result<()> {
        let provider = match StandaloneNexusProvider::discover() {
            Some(provider) => provider,
            None => return Ok(()),
        };
        let profile = self.build_session_provider_profile(session_id, project_path)?;
        self.run_memory_provider_future(provider.session_stopped(&profile))
    }

    fn run_memory_provider_future<F>(&self, future: F) -> Result<()>
    where
        F: std::future::Future<Output = Result<()>>,
    {
        if let Ok(handle) = Handle::try_current() {
            handle.block_on(future)
        } else {
            tokio::runtime::Runtime::new()?.block_on(future)
        }
    }
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize_codex_key(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

fn stdio_mcp_config(
    command: &str,
    args: Vec<impl Into<String>>,
    include_stdio_type: bool,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "command": command,
        "args": args.into_iter().map(Into::into).collect::<Vec<_>>()
    });
    if include_stdio_type {
        value["type"] = serde_json::Value::String("stdio".to_string());
    }
    value
}

fn toml_string_literal(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn toml_literal(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(v) => Some(v.to_string()),
        serde_json::Value::Number(v) => Some(v.to_string()),
        serde_json::Value::String(v) => Some(toml_string_literal(v)),
        serde_json::Value::Array(values) => {
            let mut rendered = Vec::with_capacity(values.len());
            for value in values {
                rendered.push(toml_literal(value)?);
            }
            Some(format!("[{}]", rendered.join(", ")))
        }
        serde_json::Value::Object(map) => {
            let mut rendered = Vec::with_capacity(map.len());
            for (key, value) in map {
                rendered.push(format!("{}={}", key, toml_literal(value)?));
            }
            Some(format!("{{{}}}", rendered.join(", ")))
        }
    }
}

fn ensure_json_object(value: serde_json::Value) -> serde_json::Value {
    if value.is_object() {
        value
    } else {
        serde_json::json!({})
    }
}

fn read_json_value_or_empty(path: &Path) -> Result<serde_json::Value> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }

    let contents =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;
    serde_json::from_str(&contents).with_context(|| format!("Invalid JSON in {:?}", path))
}

fn write_json_value_atomic(path: &Path, value: &serde_json::Value) -> Result<()> {
    let parent = path
        .parent()
        .context("Cannot write JSON config without a parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create parent directory {:?}", parent))?;

    let temp_file = NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create secure temp file in {:?}", parent))?;
    std::fs::write(temp_file.path(), serde_json::to_string_pretty(value)?)
        .with_context(|| format!("Failed to write JSON config to {:?}", temp_file.path()))?;
    temp_file
        .persist(path)
        .with_context(|| format!("Failed to persist JSON config to {:?}", path))?;

    Ok(())
}

fn copy_dir_recursive_filtered(
    source: &Path,
    target: &Path,
    skip_names: &BTreeSet<&str>,
) -> Result<()> {
    std::fs::create_dir_all(target)
        .with_context(|| format!("Failed to create target directory {:?}", target))?;

    for entry in
        std::fs::read_dir(source).with_context(|| format!("Failed to read {:?}", source))?
    {
        let entry = entry.with_context(|| format!("Failed to read entry in {:?}", source))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if skip_names.contains(name.as_ref()) {
            continue;
        }

        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry
            .file_type()
            .with_context(|| format!("Failed to inspect {:?}", source_path))?;

        if file_type.is_dir() {
            copy_dir_recursive_filtered(&source_path, &target_path, skip_names)?;
        } else if file_type.is_file() {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create {:?}", parent))?;
            }
            std::fs::copy(&source_path, &target_path).with_context(|| {
                format!("Failed to copy {:?} to {:?}", source_path, target_path)
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn insert_test_session(session_manager: &SessionManager, session_id: &str, project_path: &str) {
        session_manager
            .service
            .import_session(Session {
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
            })
            .unwrap();
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test-session-123"), "test-session-123");
        assert_eq!(sanitize_filename("session@test/123"), "session_test_123");
        assert_eq!(sanitize_filename("session.with.dots"), "session_with_dots");
    }

    #[test]
    fn test_sanitize_codex_key() {
        assert_eq!(
            sanitize_codex_key("maestro-tool-search"),
            "maestro_tool_search"
        );
        assert_eq!(sanitize_codex_key("agent-browser"), "agent_browser");
        assert_eq!(sanitize_codex_key("already_ok"), "already_ok");
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
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();

        // Create a test session
        let session_id = "test-session-no-lsp";
        let project_path = "/tmp/test";

        session_manager
            .service
            .import_session(Session {
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
            })
            .unwrap();

        session_manager
            .service
            .update_mcp_server(super::super::models::McpServer {
                id: 0,
                name: "agent-browser".to_string(),
                transport: super::super::models::McpTransport::Stdio,
                command: "agent-browser".to_string(),
                args: vec!["server".to_string()],
                env: serde_json::json!({}),
                cwd: None,
                url: None,
                headers: None,
                status: super::super::models::McpStatus::Stopped,
                socket_path: None,
                client_count: 0,
                last_started_at: None,
                managed: false,
                install_type: super::super::models::McpInstallKind::Unmanaged,
                install_state: super::super::models::McpInstallState::Unmanaged,
                install_root: None,
                install_recipe: None,
                install_message: None,
                install_log_path: None,
                last_install_at: None,
            })
            .unwrap();

        // Generate MCP config
        let config_path = session_manager
            .write_mcp_config_with_lsps(session_id, &[])
            .unwrap();

        // Verify file exists and is valid JSON
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify mcpServers exists
        assert!(json.get("mcpServers").is_some());
        let mcp_servers = json.get("mcpServers").unwrap();
        assert!(mcp_servers.get("maestro-tool-search").is_none());
        let direct = mcp_servers.get("leindex").unwrap();
        assert_eq!(direct["command"], "leindex");
        assert_eq!(direct["args"], serde_json::json!(["mcp"]));
        let pooled = mcp_servers.get("agent-browser").unwrap();
        assert_eq!(pooled["command"], "maestro");
        assert_eq!(
            pooled["args"],
            serde_json::json!(["mcp", "proxy", "agent-browser"])
        );
        assert_eq!(
            json["maestro"]["managedSession"]["directServers"],
            serde_json::json!(["leindex"])
        );
        assert_eq!(
            json["maestro"]["managedSession"]["pooledServers"],
            serde_json::json!(["agent-browser"])
        );

        // Verify lsp section does NOT exist when no LSPs
        assert!(json.get("lsp").is_none());

        // Clean up
        std::fs::remove_file(&config_path).ok();
    }

    #[test]
    fn test_mcp_config_with_lsps() {
        // Test that MCP config with LSPs includes lsp section
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();

        let session_id = "test-session-with-lsp";
        let project_path = "/tmp/test";

        session_manager
            .service
            .import_session(Session {
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
            })
            .unwrap();

        // Generate MCP config with Rust LSP
        let lsp_types = vec![LspType::Rust];
        let config_path = session_manager
            .write_mcp_config_with_lsps(session_id, &lsp_types)
            .unwrap();

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
    fn test_codex_mcp_overrides_include_pooled_servers() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();
        session_manager
            .service
            .update_mcp_server(super::super::models::McpServer {
                id: 0,
                name: "agent-browser".to_string(),
                transport: super::super::models::McpTransport::Stdio,
                command: "agent-browser".to_string(),
                args: vec!["server".to_string()],
                env: serde_json::json!({}),
                cwd: None,
                url: None,
                headers: None,
                status: super::super::models::McpStatus::Stopped,
                socket_path: None,
                client_count: 0,
                last_started_at: None,
                managed: false,
                install_type: super::super::models::McpInstallKind::Unmanaged,
                install_state: super::super::models::McpInstallState::Unmanaged,
                install_root: None,
                install_recipe: None,
                install_message: None,
                install_log_path: None,
                last_install_at: None,
            })
            .unwrap();

        let overrides = session_manager
            .build_codex_mcp_overrides("session-codex", "/tmp/project")
            .unwrap();

        assert!(overrides
            .iter()
            .any(|entry| { entry == "mcp_servers.leindex.command=\"leindex\"" }));
        assert!(overrides
            .iter()
            .any(|entry| { entry == "mcp_servers.leindex.args=[\"mcp\"]" }));
        assert!(overrides
            .iter()
            .any(|entry| { entry == "mcp_servers.agent_browser.command=\"maestro\"" }));
        assert!(overrides.iter().any(|entry| {
            entry == "mcp_servers.agent_browser.args=[\"mcp\", \"proxy\", \"agent-browser\"]"
        }));
    }

    #[test]
    fn test_amp_mcp_config_uses_plain_server_map() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();
        session_manager
            .service
            .update_mcp_server(super::super::models::McpServer {
                id: 0,
                name: "agent-browser".to_string(),
                transport: super::super::models::McpTransport::Stdio,
                command: "agent-browser".to_string(),
                args: vec!["server".to_string()],
                env: serde_json::json!({}),
                cwd: None,
                url: None,
                headers: None,
                status: super::super::models::McpStatus::Stopped,
                socket_path: None,
                client_count: 0,
                last_started_at: None,
                managed: false,
                install_type: super::super::models::McpInstallKind::Unmanaged,
                install_state: super::super::models::McpInstallState::Unmanaged,
                install_root: None,
                install_recipe: None,
                install_message: None,
                install_log_path: None,
                last_install_at: None,
            })
            .unwrap();

        let config = session_manager
            .write_amp_mcp_config_file("amp-session")
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();

        assert!(json.get("mcpServers").is_none());
        assert_eq!(json["agent-browser"]["command"], "maestro");
        assert_eq!(
            json["agent-browser"]["args"],
            serde_json::json!(["mcp", "proxy", "agent-browser"])
        );
    }

    #[test]
    fn test_opencode_config_preserves_existing_commands() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();
        let updated = session_manager
            .build_opencode_settings_json(
                "opencode-session",
                "/tmp/project",
                serde_json::json!({
                    "command": {
                        "custom": {
                            "template": "do custom work"
                        }
                    }
                }),
            )
            .unwrap();

        assert_eq!(updated["command"]["custom"]["template"], "do custom work");
        assert_eq!(
            updated["mcp"]["leindex"]["command"],
            serde_json::json!(["leindex", "mcp"])
        );
        assert_eq!(
            updated["mcp"]["leindex"]["type"],
            serde_json::json!("local")
        );
    }

    #[test]
    fn test_build_tool_command_uses_tool_specific_overrides() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        let session_manager = SessionManager::new(service).unwrap();
        insert_test_session(&session_manager, "session-gemini", "/tmp/project-gemini");
        insert_test_session(&session_manager, "session-qwen", "/tmp/project-qwen");
        insert_test_session(&session_manager, "session-iflow", "/tmp/project-iflow");
        insert_test_session(&session_manager, "session-amp", "/tmp/project-amp");
        insert_test_session(
            &session_manager,
            "session-opencode",
            "/tmp/project-opencode",
        );
        insert_test_session(&session_manager, "session-droid", "/tmp/project-droid");

        let gemini = session_manager
            .build_tool_command("gemini", "/tmp/project-gemini", "session-gemini")
            .unwrap();
        assert!(gemini.contains("GEMINI_CLI_SYSTEM_SETTINGS_PATH="));

        let qwen = session_manager
            .build_tool_command("qwen", "/tmp/project-qwen", "session-qwen")
            .unwrap();
        assert!(qwen.contains("QWEN_CODE_SYSTEM_SETTINGS_PATH="));

        let iflow = session_manager
            .build_tool_command("iflow", "/tmp/project-iflow", "session-iflow")
            .unwrap();
        assert!(iflow.contains("IFLOW_CLI_SYSTEM_SETTINGS_PATH="));

        let amp = session_manager
            .build_tool_command("amp", "/tmp/project-amp", "session-amp")
            .unwrap();
        assert!(amp.contains("amp --mcp-config "));

        let opencode = session_manager
            .build_tool_command("opencode", "/tmp/project-opencode", "session-opencode")
            .unwrap();
        assert!(opencode.contains("OPENCODE_CONFIG="));

        let droid = session_manager
            .build_tool_command("droid", "/tmp/project-droid", "session-droid")
            .unwrap();
        assert!(droid.contains("export HOME="));
        assert!(droid.contains("&& droid"));
    }

    #[test]
    fn test_detect_lsps_for_rust_project() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with Rust files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create some Rust files
        std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp_dir.path().join("lib.rs"), "pub fn test() {}").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager
            .detect_lsps_for_project(&project_path, "test-session")
            .unwrap();

        // Should detect Rust LSP
        assert!(
            !lsp_entries.is_empty(),
            "No LSP entries detected for Rust project"
        );
        assert!(
            lsp_entries.iter().any(|lsp| lsp["language"] == "rust"),
            "Rust LSP not found in entries"
        );
    }

    #[test]
    fn test_detect_lsps_for_python_project() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with Python files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create some Python files
        std::fs::write(temp_dir.path().join("main.py"), "print('hello')").unwrap();
        std::fs::write(temp_dir.path().join("lib.py"), "def test(): pass").unwrap();

        // Detect LSPs
        let lsp_entries = session_manager
            .detect_lsps_for_project(&project_path, "test-session")
            .unwrap();

        // Should detect Python LSP
        assert!(!lsp_entries.is_empty());
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "python"));
    }

    #[test]
    fn test_detect_lsps_for_typescript_project() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with TypeScript files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create some TypeScript files
        std::fs::write(temp_dir.path().join("main.ts"), "console.log('hello')").unwrap();
        std::fs::write(
            temp_dir.path().join("app.tsx"),
            "export default function App() {}",
        )
        .unwrap();

        // Detect LSPs
        let lsp_entries = session_manager
            .detect_lsps_for_project(&project_path, "test-session")
            .unwrap();

        // Should detect TypeScript LSP
        assert!(!lsp_entries.is_empty());
        assert!(lsp_entries
            .iter()
            .any(|lsp| lsp["language"] == "typescript"));
    }

    #[test]
    fn test_detect_lsps_for_mixed_project() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create a temporary directory with mixed language files
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Create files from different languages
        std::fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp_dir.path().join("script.py"), "print('hello')").unwrap();
        std::fs::write(
            temp_dir.path().join("app.tsx"),
            "export default function App() {}",
        )
        .unwrap();

        // Detect LSPs
        let lsp_entries = session_manager
            .detect_lsps_for_project(&project_path, "test-session")
            .unwrap();

        // Should detect all three LSPs
        assert!(!lsp_entries.is_empty());
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "rust"));
        assert!(lsp_entries.iter().any(|lsp| lsp["language"] == "python"));
        assert!(lsp_entries
            .iter()
            .any(|lsp| lsp["language"] == "typescript"));
    }

    #[test]
    fn test_detect_lsps_for_empty_project() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        let session_manager = SessionManager::new(service).unwrap();

        // Create an empty temporary directory
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Detect LSPs
        let lsp_entries = session_manager
            .detect_lsps_for_project(&project_path, "test-session")
            .unwrap();

        // Should return empty list
        assert!(lsp_entries.is_empty());
    }

    #[test]
    fn test_detect_lsps_skips_non_source_dirs() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
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
        let lsp_entries = session_manager
            .detect_lsps_for_project(&project_path, "test-session")
            .unwrap();

        // Should detect Rust LSP from main.rs but not from target directory
        assert!(!lsp_entries.is_empty());
        assert_eq!(lsp_entries.len(), 1);
    }

    #[test]
    fn test_get_session_project_path() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        // Import a test session
        let session_id = "test-session-path";
        let project_path = "/tmp/test-project";

        service
            .import_session(Session {
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
            })
            .unwrap();

        // Query project path
        let result = service.get_session_project_path(session_id).unwrap();

        // Should return the correct project path
        assert_eq!(result, Some(project_path.to_string()));
    }

    #[test]
    fn test_get_session_project_path_not_found() {
        let service = MemoryService::new(Some(std::path::PathBuf::from(":memory:"))).unwrap();
        service.initialize().unwrap();

        // Query non-existent session
        let result = service
            .get_session_project_path("non-existent-session")
            .unwrap();

        // Should return None
        assert!(result.is_none());
    }
}
