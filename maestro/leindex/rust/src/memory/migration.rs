//! Legacy Migration Logic
//! 
//! Handles importing data from the Go TUI's JSON-based storage into SQLite.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::{info, warn};

use super::models::{Session, SessionStatus, SessionGroup};
use super::service::MemoryService;

#[derive(Debug, Deserialize)]
struct StorageData {
    instances: Vec<InstanceData>,
    groups: Option<Vec<GroupData>>,
    _updated_at: String,
}

#[derive(Debug, Deserialize)]
struct InstanceData {
    id: String,
    title: String,
    project_path: String,
    group_path: String,
    parent_session_id: Option<String>,
    command: String,
    tool: String,
    status: String,
    created_at: String,
    last_accessed_at: Option<String>,
    tmux_session: String,
}

#[derive(Debug, Deserialize)]
struct GroupData {
    name: String,
    path: String,
    expanded: bool,
    order: i32,
}

pub struct LegacyMigrator;

impl LegacyMigrator {
    /// Attempt to migrate Go TUI sessions to SQLite
    pub fn migrate(service: &MemoryService) -> Result<usize> {
        let legacy_path = Self::get_legacy_storage_path()?;
        if !legacy_path.exists() {
            info!("No legacy sessions.json found at {}", legacy_path.display());
            return Ok(0);
        }

        info!("Found legacy sessions at {}, starting migration...", legacy_path.display());
        let content = fs::read_to_string(&legacy_path)?;
        let data: StorageData = serde_json::from_str(&content)
            .context("Failed to parse legacy sessions.json")?;

        let mut migrated_count = 0;

        // 1. Migrate Groups
        if let Some(groups) = data.groups {
            for g in groups {
                let group = SessionGroup {
                    id: 0,
                    name: g.name,
                    path: g.path,
                    is_expanded: g.expanded,
                    sort_order: g.order,
                    parent_id: None,
                };
                let _ = service.get_or_create_session_group(group);
            }
        }

        // 2. Migrate Sessions
        for inst in data.instances {
            // Map Go status to Rust SessionStatus
            let status = match inst.status.as_str() {
                "running" => SessionStatus::Running,
                "waiting" => SessionStatus::Waiting,
                "idle" => SessionStatus::Idle,
                "error" => SessionStatus::Error,
                "starting" => SessionStatus::Starting,
                _ => SessionStatus::Idle,
            };

            let session = Session {
                id: 0,
                session_id: inst.id,
                title: inst.title,
                project_path: inst.project_path,
                group_path: Some(inst.group_path),
                parent_session_id: inst.parent_session_id,
                command: Some(inst.command),
                tool: Some(inst.tool),
                status,
                multiplexer_session: Some(inst.tmux_session),
                started_at: Self::parse_iso8601(&inst.created_at).unwrap_or_else(|_| Utc::now()),
                last_accessed_at: inst.last_accessed_at.and_then(|s| Self::parse_iso8601(&s).ok()),
                ended_at: None,
                metadata: None,
            };

            if let Ok(_) = service.import_session(session) {
                migrated_count += 1;
            }
        }

        info!("Successfully migrated {} sessions to SQLite", migrated_count);
        
        // Optionally rename the old file to avoid re-migration
        let backup_path = legacy_path.with_extension("json.migrated");
        if let Err(e) = fs::rename(&legacy_path, &backup_path) {
            warn!("Failed to rename migrated sessions file: {}", e);
        }

        Ok(migrated_count)
    }

    fn get_legacy_storage_path() -> Result<PathBuf> {
        let mut p = dirs::home_dir().context("Failed to find home directory")?;
        p.push(".maestro");
        p.push("profiles");
        p.push("default");
        p.push("sessions.json");
        Ok(p)
    }

    fn parse_iso8601(s: &str) -> Result<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .context("Failed to parse ISO8601 date")
    }
}
