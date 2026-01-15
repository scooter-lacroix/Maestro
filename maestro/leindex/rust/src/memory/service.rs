//! Memory Service
//!
//! Primary interface for all memory operations.
//! Provides CRUD operations for projects, tracks, and memories.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

use super::db::DatabaseManager;
use super::models::*;
use super::scanner::Scanner;

#[derive(Clone)]
pub struct MemoryService {
    db: DatabaseManager,
    scanner: Scanner,
    search_index: Arc<super::search::MemorySearchIndex>,
}

impl MemoryService {
    /// Create new memory service
    pub fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let db = DatabaseManager::new(db_path.clone())?;
        let search_path = db_path.map(|p| p.parent().unwrap().join("search_index"));
        let search_index = Arc::new(super::search::MemorySearchIndex::new(search_path)?);

        Ok(Self {
            db,
            scanner: Scanner::new(),
            search_index,
        })
    }

    /// Initialize the service (create tables)
    pub fn initialize(&self) -> Result<()> {
        self.db.initialize()?;

        // Trigger legacy migration
        if let Err(e) = super::migration::LegacyMigrator::migrate(self) {
            warn!("Legacy migration failed: {}", e);
        }

        Ok(())
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<super::db::DbStats> {
        self.db.stats()
    }

    // ========================================================================
    // Project Operations
    // ========================================================================

    /// Get or create a project
    pub fn get_or_create_project(&self, path: &str, name: &str) -> Result<MaestroProject> {
        self.db.with_connection(|conn| {
            let project = conn
                .query_row(
                    "SELECT id, project_path, project_name, description, project_type, tech_stack, 
                        is_active, created_at, updated_at, last_scanned_at
                 FROM maestro_projects WHERE project_path = ?",
                    [path],
                    |row| {
                        Ok(MaestroProject {
                            id: row.get(0)?,
                            project_path: row.get(1)?,
                            project_name: row.get(2)?,
                            description: row.get(3)?,
                            project_type: row.get(4)?,
                            tech_stack: row
                                .get::<_, Option<String>>(5)?
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_default(),
                            is_active: row.get::<_, i32>(6)? == 1,
                            created_at: parse_datetime(row.get::<_, String>(7)?),
                            updated_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                            last_scanned_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
                        })
                    },
                )
                .optional()?;

            if let Some(p) = project {
                return Ok(p);
            }

            // Create new
            conn.execute(
                "INSERT INTO maestro_projects (project_path, project_name) VALUES (?, ?)",
                params![path, name],
            )?;

            let id = conn.last_insert_rowid();
            Ok(MaestroProject {
                id,
                project_path: path.to_string(),
                project_name: name.to_string(),
                description: None,
                project_type: None,
                tech_stack: Vec::new(),
                is_active: true,
                created_at: Utc::now(),
                updated_at: None,
                last_scanned_at: None,
            })
        })
    }

    /// List all projects
    pub fn list_projects(&self) -> Result<Vec<MaestroProject>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, project_path, project_name, description, project_type, tech_stack,
                        is_active, created_at, updated_at, last_scanned_at
                 FROM maestro_projects WHERE is_active = 1 ORDER BY project_name",
            )?;

            let projects = stmt
                .query_map([], |row| {
                    Ok(MaestroProject {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        project_name: row.get(2)?,
                        description: row.get(3)?,
                        project_type: row.get(4)?,
                        tech_stack: row
                            .get::<_, Option<String>>(5)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                        is_active: row.get::<_, i32>(6)? == 1,
                        created_at: parse_datetime(row.get::<_, String>(7)?),
                        updated_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                        last_scanned_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("Failed to map projects")?;

            Ok(projects)
        })
    }

    /// Get project by ID
    pub fn get_project(&self, id: i64) -> Result<Option<MaestroProject>> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, project_path, project_name, description, project_type, tech_stack,
                        is_active, created_at, updated_at, last_scanned_at
                 FROM maestro_projects WHERE id = ?",
                [id],
                |row| {
                    Ok(MaestroProject {
                        id: row.get(0)?,
                        project_path: row.get(1)?,
                        project_name: row.get(2)?,
                        description: row.get(3)?,
                        project_type: row.get(4)?,
                        tech_stack: row
                            .get::<_, Option<String>>(5)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                        is_active: row.get::<_, i32>(6)? == 1,
                        created_at: parse_datetime(row.get::<_, String>(7)?),
                        updated_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                        last_scanned_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
                    })
                },
            )
            .optional()
            .context("Failed to get project")
        })
    }

    // ========================================================================
    // Track Operations
    // ========================================================================

    /// Get or create a track
    pub fn get_or_create_track(
        &self,
        project_id: i64,
        track_id: &str,
        title: &str,
    ) -> Result<MaestroTrack> {
        self.db.with_connection(|conn| {
            let track = conn
                .query_row(
                    "SELECT id, track_id, project_id, title, description, status,
                        total_tasks, completed_tasks, created_at, updated_at
                 FROM maestro_tracks WHERE project_id = ? AND track_id = ?",
                    params![project_id, track_id],
                    |row| {
                        Ok(MaestroTrack {
                            id: row.get(0)?,
                            track_id: row.get(1)?,
                            project_id: row.get(2)?,
                            title: row.get(3)?,
                            description: row.get(4)?,
                            status: parse_track_status(row.get::<_, String>(5)?),
                            total_tasks: row.get(6)?,
                            completed_tasks: row.get(7)?,
                            created_at: parse_datetime(row.get::<_, String>(8)?),
                            updated_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
                        })
                    },
                )
                .optional()?;

            if let Some(t) = track {
                return Ok(t);
            }

            // Create new
            conn.execute(
                "INSERT INTO maestro_tracks (project_id, track_id, title) VALUES (?, ?, ?)",
                params![project_id, track_id, title],
            )?;

            let id = conn.last_insert_rowid();
            Ok(MaestroTrack {
                id,
                track_id: track_id.to_string(),
                project_id,
                title: title.to_string(),
                description: None,
                status: TrackStatus::New,
                total_tasks: 0,
                completed_tasks: 0,
                created_at: Utc::now(),
                updated_at: None,
            })
        })
    }

    /// List tracks for a project
    pub fn list_tracks(&self, project_id: i64) -> Result<Vec<MaestroTrack>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, track_id, project_id, title, description, status,
                        total_tasks, completed_tasks, created_at, updated_at
                 FROM maestro_tracks WHERE project_id = ? ORDER BY created_at",
            )?;

            let tracks = stmt
                .query_map([project_id], |row| {
                    Ok(MaestroTrack {
                        id: row.get(0)?,
                        track_id: row.get(1)?,
                        project_id: row.get(2)?,
                        title: row.get(3)?,
                        description: row.get(4)?,
                        status: parse_track_status(row.get::<_, String>(5)?),
                        total_tasks: row.get(6)?,
                        completed_tasks: row.get(7)?,
                        created_at: parse_datetime(row.get::<_, String>(8)?),
                        updated_at: row.get::<_, Option<String>>(9)?.map(parse_datetime),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(tracks)
        })
    }

    // ========================================================================
    // Memory Operations
    // ========================================================================

    /// Store a memory
    pub fn store_memory(&self, content: &str, category: MemoryCategory) -> Result<i64> {
        let id = self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO memories (content, category, importance) VALUES (?, ?, ?)",
                params![content, category.to_string(), "normal"],
            )?;
            Ok(conn.last_insert_rowid())
        })?;

        // Index in Tantivy
        if let Err(e) = self
            .search_index
            .index_memory(id, content, &category.to_string(), None)
        {
            warn!("Failed to index memory in Tantivy: {}", e);
        }

        Ok(id)
    }

    /// Search memories by content (Hybrid Search)
    pub fn search_memories(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        // Try Tantivy search for ranked full-text
        let ids = match self.search_index.search(query, limit) {
            Ok(ids) if !ids.is_empty() => Some(ids),
            Ok(_) => None,
            Err(e) => {
                warn!("Search index error: {}. Falling back to SQLite.", e);
                None
            }
        };

        self.db.with_connection(|conn| {
            let memories = if let Some(ids) = ids {
                // Fetch by IDs from search index, preserving order
                let id_list = ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let query_sql = format!(
                    "SELECT id, content, summary, category, importance, source, session_id,
                            project_id, track_id, command, command_context, created_at,
                            expires_at, last_accessed, meta_data, tags
                     FROM memories 
                     WHERE id IN ({})
                     ORDER BY CASE id {} END",
                    id_list,
                    ids.iter()
                        .enumerate()
                        .map(|(i, id)| format!("WHEN {} THEN {}", id, i))
                        .collect::<Vec<_>>()
                        .join(" ")
                );

                let mut stmt = conn.prepare(&query_sql)?;
                let rows = stmt.query_map([], |row| self.map_memory(row))?;
                let results: std::result::Result<Vec<_>, _> = rows.collect();
                results?
            } else {
                // Fallback to SQLite LIKE
                let pattern = format!("%{}%", query);
                let mut stmt = conn.prepare(
                    "SELECT id, content, summary, category, importance, source, session_id,
                            project_id, track_id, command, command_context, created_at,
                            expires_at, last_accessed, meta_data, tags
                     FROM memories 
                     WHERE content LIKE ?
                     ORDER BY created_at DESC
                     LIMIT ?",
                )?;

                let rows =
                    stmt.query_map(params![pattern, limit as i32], |row| self.map_memory(row))?;
                let results: std::result::Result<Vec<_>, _> = rows.collect();
                results?
            };

            Ok(memories)
        })
    }

    /// Helper to map memory row
    fn map_memory(&self, row: &rusqlite::Row) -> rusqlite::Result<Memory> {
        Ok(Memory {
            id: row.get(0)?,
            content: row.get(1)?,
            summary: row.get(2)?,
            category: parse_category(row.get::<_, String>(3)?),
            importance: parse_importance(row.get::<_, String>(4)?),
            source: row.get(5)?,
            session_id: row.get(6)?,
            project_id: row.get(7)?,
            track_id: row.get(8)?,
            command: row.get(9)?,
            command_context: row
                .get::<_, Option<String>>(10)?
                .and_then(|s| serde_json::from_str(&s).ok()),
            created_at: parse_datetime(row.get::<_, String>(11)?),
            expires_at: row.get::<_, Option<String>>(12)?.map(parse_datetime),
            last_accessed: row.get::<_, Option<String>>(13)?.map(parse_datetime),
            metadata: row
                .get::<_, Option<String>>(14)?
                .and_then(|s| serde_json::from_str(&s).ok()),
            tags: row
                .get::<_, Option<String>>(15)?
                .and_then(|s| serde_json::from_str(&s).ok()),
        })
    }

    /// List latest memories
    pub fn list_memories(&self, limit: usize) -> Result<Vec<Memory>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, content, summary, category, importance, source, session_id,
                        project_id, track_id, command, command_context, created_at,
                        expires_at, last_accessed, meta_data, tags
                 FROM memories 
                 ORDER BY created_at DESC
                 LIMIT ?",
            )?;

            let memories: Vec<Memory> = stmt
                .query_map([limit], |row| self.map_memory(row))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(memories)
        })
    }

    // ========================================================================
    // Scanning Operations
    // ========================================================================

    pub fn scan_directories(&self, dirs: &[PathBuf], max_depth: usize) -> Result<ScanResult> {
        let result = self.scanner.scan(dirs, max_depth);

        // Import discovered projects to database
        for project_info in &result.projects {
            if let Err(e) = self.get_or_create_project(&project_info.path, &project_info.name) {
                warn!("Failed to import project {}: {}", project_info.name, e);
            }
        }

        info!("Imported {} projects to database", result.projects_found);
        Ok(result)
    }

    // ========================================================================
    // Session & Group Operations
    // ========================================================================

    /// Import a session (used by migration)
    pub fn import_session(&self, session: Session) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO sessions (
                    session_id, title, project_path, group_path, parent_session_id,
                    command, tool, status, multiplexer_session, started_at,
                    last_accessed_at, ended_at, meta_data
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    session.session_id,
                    session.title,
                    session.project_path,
                    session.group_path,
                    session.parent_session_id,
                    session.command,
                    session.tool,
                    session.status.to_string(),
                    session.multiplexer_session,
                    session.started_at.to_rfc3339(),
                    session.last_accessed_at.map(|dt| dt.to_rfc3339()),
                    session.ended_at.map(|dt| dt.to_rfc3339()),
                    session.metadata.map(|v| serde_json::to_string(&v).unwrap()),
                ],
            )?;
            Ok(())
        })
    }

    /// Update a session's title in the database
    pub fn update_session_title(&self, session_id: &str, new_title: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE session_id = ?",
                params![new_title, session_id],
            )?;
            Ok(())
        })
    }

    /// Update a session's status in the database
    pub fn update_session_status(&self, session_id: &str, status: SessionStatus) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE session_id = ?",
                params![status.to_string(), session_id],
            )?;
            Ok(())
        })
    }

    /// Delete a session from the database
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM sessions WHERE session_id = ?", [session_id])?;
            Ok(())
        })
    }

    /// Update last accessed timestamp
    pub fn update_last_accessed(&self, session_id: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET last_accessed_at = CURRENT_TIMESTAMP WHERE session_id = ?",
                [session_id],
            )?;
            Ok(())
        })
    }

    /// Update session after rename (affects title and session_id)
    pub fn update_session_rename(&self, old_id: &str, new_id: &str, new_title: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET title = ?, session_id = ?, multiplexer_session = ?, updated_at = CURRENT_TIMESTAMP WHERE session_id = ?",
                params![new_title, new_id, new_id, old_id],
            )?;
            Ok(())
        })
    }

    /// Get or create a session group
    pub fn get_or_create_session_group(&self, group: SessionGroup) -> Result<SessionGroup> {
        self.db.with_connection(|conn| {
            let existing = conn
                .query_row(
                    "SELECT id, name, path, category, is_expanded, sort_order, parent_id 
                 FROM session_groups WHERE path = ?",
                    [&group.path],
                    |row| {
                        Ok(SessionGroup {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            path: row.get(2)?,
                            category: row.get(3)?,
                            is_expanded: row.get::<_, i32>(4)? == 1,
                            sort_order: row.get(5)?,
                            parent_id: row.get(6)?,
                        })
                    },
                )
                .optional()?;

            if let Some(g) = existing {
                return Ok(g);
            }

            conn.execute(
                "INSERT INTO session_groups (name, path, category, is_expanded, sort_order, parent_id)
                 VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    group.name,
                    group.path,
                    group.category,
                    if group.is_expanded { 1 } else { 0 },
                    group.sort_order,
                    group.parent_id,
                ],
            )?;

            let id = conn.last_insert_rowid();
            Ok(SessionGroup { id, ..group })
        })
    }
    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, title, project_path, group_path, parent_session_id,
                        command, tool, status, multiplexer_session, started_at,
                        last_accessed_at, ended_at, meta_data
                 FROM sessions ORDER BY last_accessed_at DESC, started_at DESC",
            )?;

            let sessions = stmt
                .query_map([], |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        title: row.get(2)?,
                        project_path: row.get(3)?,
                        group_path: row.get(4)?,
                        parent_session_id: row.get(5)?,
                        command: row.get(6)?,
                        tool: row.get(7)?,
                        status: parse_session_status(row.get(8)?),
                        multiplexer_session: row.get(9)?,
                        started_at: parse_datetime(row.get(10)?),
                        last_accessed_at: row.get::<_, Option<String>>(11)?.map(parse_datetime),
                        ended_at: row.get::<_, Option<String>>(12)?.map(parse_datetime),
                        metadata: row
                            .get::<_, Option<String>>(13)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(sessions)
        })
    }

    /// List all session groups
    pub fn list_session_groups(&self) -> Result<Vec<SessionGroup>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, path, category, is_expanded, sort_order, parent_id 
                 FROM session_groups ORDER BY sort_order, name",
            )?;

            let groups = stmt
                .query_map([], |row| {
                    Ok(SessionGroup {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: row.get(2)?,
                        category: row.get(3)?,
                        is_expanded: row.get::<_, i32>(4)? == 1,
                        sort_order: row.get(5)?,
                        parent_id: row.get(6)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(groups)
        })
    }

    /// Update a session group's name
    pub fn update_group_name(&self, group_path: &str, new_name: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE session_groups SET name = ? WHERE path = ?",
                params![new_name, group_path],
            )?;
            Ok(())
        })
    }

    /// Update a session group's expansion state
    pub fn update_group_expansion(&self, group_path: &str, is_expanded: bool) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE session_groups SET is_expanded = ? WHERE path = ?",
                params![if is_expanded { 1 } else { 0 }, group_path],
            )?;
            Ok(())
        })
    }

    /// Update a session group's category
    pub fn update_group_category(&self, group_path: &str, category: Option<String>) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE session_groups SET category = ? WHERE path = ?",
                params![category, group_path],
            )?;
            Ok(())
        })
    }

    /// Move a session to a group
    pub fn update_session_group(&self, session_id: &str, group_path: Option<String>) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET group_path = ? WHERE session_id = ?",
                params![group_path, session_id],
            )?;
            Ok(())
        })
    }

    /// Delete a session group and disconnect its sessions from the group
    pub fn delete_group(&self, group_path: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            // First clear the group_path from all sessions in this group
            conn.execute(
                "UPDATE sessions SET group_path = NULL WHERE group_path = ?",
                [group_path],
            )?;
            // Then delete the group entry
            conn.execute("DELETE FROM session_groups WHERE path = ?", [group_path])?;
            Ok(())
        })
    }

    // ========================================================================
    /// Create a new session group
    pub fn create_session_group(
        &self,
        name: &str,
        path: &str,
        category: Option<String>,
    ) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO session_groups (name, path, category, is_expanded, sort_order) VALUES (?, ?, ?, 1, 0)",
                params![name, path, category],
            )?;
            Ok(())
        })
    }

    /// Update or create MCP server state
    pub fn update_mcp_server(&self, server: McpServer) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO mcp_servers (
                    name, command, args, env, status, socket_path, 
                    client_count, last_started_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    server.name,
                    server.command,
                    serde_json::to_string(&server.args).unwrap(),
                    serde_json::to_string(&server.env).unwrap(),
                    server.status.to_string(),
                    server.socket_path,
                    server.client_count,
                    server.last_started_at.map(|dt| dt.to_rfc3339()),
                ],
            )?;
            Ok(())
        })
    }

    /// List all pooled MCP servers
    pub fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, command, args, env, status, socket_path, 
                        client_count, last_started_at 
                 FROM mcp_servers ORDER BY name",
            )?;

            let servers = stmt
                .query_map([], |row| {
                    Ok(McpServer {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        command: row.get(2)?,
                        args: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                        env: serde_json::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                        status: parse_mcp_status(row.get(5)?),
                        socket_path: row.get(6)?,
                        client_count: row.get(7)?,
                        last_started_at: row.get::<_, Option<String>>(8)?.map(parse_datetime),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(servers)
        })
    }

    /// Delete an MCP server from pool
    pub fn delete_mcp_server(&self, name: &str) -> Result<()> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM mcp_servers WHERE name = ?", [name])?;
            Ok(())
        })
    }
}

// Helper functions (existing + new)

fn parse_mcp_status(s: String) -> McpStatus {
    match s.as_str() {
        "running" => McpStatus::Running,
        "stopped" => McpStatus::Stopped,
        _ => McpStatus::Error,
    }
}

impl std::fmt::Display for McpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Error => write!(f, "error"),
        }
    }
}

fn parse_session_status(s: String) -> SessionStatus {
    match s.as_str() {
        "running" => SessionStatus::Running,
        "waiting" => SessionStatus::Waiting,
        "idle" => SessionStatus::Idle,
        "error" => SessionStatus::Error,
        "starting" => SessionStatus::Starting,
        "paused" => SessionStatus::Paused,
        "completed" => SessionStatus::Completed,
        "terminated" => SessionStatus::Terminated,
        _ => SessionStatus::Idle,
    }
}

fn parse_datetime(s: String) -> DateTime<Utc> {
    // Try primary ISO8601 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return dt.with_timezone(&Utc);
    }

    // Try SQLite default format: YYYY-MM-DD HH:MM:SS
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
        return DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
    }

    // Try other variants or fallback to now
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        warn!("Failed to parse datetime: '{}', using Utc::now()", s);
        Utc::now()
    })
}

fn parse_category(s: String) -> MemoryCategory {
    match s.as_str() {
        "fact" => MemoryCategory::Fact,
        "pattern" => MemoryCategory::Pattern,
        "decision" => MemoryCategory::Decision,
        "context" => MemoryCategory::Context,
        "temporary" => MemoryCategory::Temporary,
        "observation" => MemoryCategory::Observation,
        _ => MemoryCategory::Context,
    }
}

fn parse_importance(s: String) -> MemoryImportance {
    match s.as_str() {
        "critical" => MemoryImportance::Critical,
        "high" => MemoryImportance::High,
        "normal" => MemoryImportance::Normal,
        "low" => MemoryImportance::Low,
        _ => MemoryImportance::Normal,
    }
}

fn parse_track_status(s: String) -> TrackStatus {
    match s.as_str() {
        "new" => TrackStatus::New,
        "in_progress" => TrackStatus::InProgress,
        "completed" => TrackStatus::Completed,
        "blocked" => TrackStatus::Blocked,
        "abandoned" => TrackStatus::Abandoned,
        _ => TrackStatus::New,
    }
}

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_service_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let service = MemoryService::new(Some(db_path)).unwrap();
        service.initialize().unwrap();

        let stats = service.stats().unwrap();
        assert_eq!(stats.project_count, 0);
    }

    #[test]
    fn test_project_crud() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let service = MemoryService::new(Some(db_path)).unwrap();
        service.initialize().unwrap();

        // Create project
        let project = service
            .get_or_create_project("/test/path", "TestProject")
            .unwrap();
        assert_eq!(project.project_name, "TestProject");

        // List projects
        let projects = service.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
    }
}
