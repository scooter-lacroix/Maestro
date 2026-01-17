//! Database Models
//!
//! Pure Rust equivalents of Python SQLAlchemy models.
//! Optimized for performance with derive macros and builder patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Memory category for organizing memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    General,
    Knowledge,
    Preference,
    Specification,
    Fact,
    Pattern,
    Decision,
    Context,
    Temporary,
    Observation,
}

impl Default for MemoryCategory {
    fn default() -> Self {
        Self::Context
    }
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General => write!(f, "general"),
            Self::Knowledge => write!(f, "knowledge"),
            Self::Preference => write!(f, "preferences"),
            Self::Specification => write!(f, "specifications"),
            Self::Fact => write!(f, "fact"),
            Self::Pattern => write!(f, "pattern"),
            Self::Decision => write!(f, "decision"),
            Self::Context => write!(f, "context"),
            Self::Temporary => write!(f, "temporary"),
            Self::Observation => write!(f, "observation"),
        }
    }
}

/// Memory importance level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryImportance {
    Critical,
    High,
    Normal,
    Low,
}

impl Default for MemoryImportance {
    fn default() -> Self {
        Self::Normal
    }
}

/// Core memory record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub content: String,
    pub summary: Option<String>,
    pub category: MemoryCategory,
    pub importance: MemoryImportance,
    pub source: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<i64>,
    pub track_id: Option<i64>,
    pub command: Option<String>,
    pub command_context: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
}

impl Memory {
    /// Check if memory has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Utc::now() > exp).unwrap_or(false)
    }
}

/// Maestro project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaestroProject {
    pub id: i64,
    pub project_path: String,
    pub project_name: String,
    pub description: Option<String>,
    pub project_type: Option<String>,
    pub tech_stack: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub last_scanned_at: Option<DateTime<Utc>>,
}

impl MaestroProject {
    pub fn new(path: &str, name: &str) -> Self {
        Self {
            id: 0,
            project_path: path.to_string(),
            project_name: name.to_string(),
            description: None,
            project_type: None,
            tech_stack: Vec::new(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            last_scanned_at: None,
        }
    }
}

/// Maestro track (work unit within a project)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaestroTrack {
    pub id: i64,
    pub track_id: String,
    pub project_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: TrackStatus,
    pub total_tasks: i32,
    pub completed_tasks: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Track status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackStatus {
    New,
    InProgress,
    Completed,
    Blocked,
    Abandoned,
}

impl Default for TrackStatus {
    fn default() -> Self {
        Self::New
    }
}

impl std::fmt::Display for TrackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Blocked => write!(f, "blocked"),
            Self::Abandoned => write!(f, "abandoned"),
        }
    }
}

impl MaestroTrack {
    pub fn progress_percent(&self) -> f64 {
        if self.total_tasks == 0 {
            0.0
        } else {
            (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
        }
    }
}

/// File claim for multi-agent coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileClaim {
    pub id: i64,
    pub claim_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub file_patterns: Vec<String>,
    pub status: ClaimStatus,
    pub is_exclusive: bool,
    pub reason: Option<String>,
    pub claimed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
}

/// Claim status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimStatus {
    Active,
    Released,
    Expired,
    Revoked,
}

impl Default for ClaimStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Session for tracking agent work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub session_id: String,
    pub title: String,
    pub project_path: String,
    pub group_path: Option<String>,
    /// Stable ordering within a group (lower first). Falls back to recency when unset.
    pub sort_order: i32,
    pub parent_session_id: Option<String>,
    pub command: Option<String>,
    pub tool: Option<String>,
    pub status: SessionStatus,
    pub multiplexer_session: Option<String>, // e.g., zellij session name
    pub started_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Waiting,
    Idle,
    Error,
    Starting,
    Paused,
    Completed,
    Terminated,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Waiting => write!(f, "waiting"),
            Self::Idle => write!(f, "idle"),
            Self::Error => write!(f, "error"),
            Self::Starting => write!(f, "starting"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

/// Session group for hierarchical organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGroup {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub category: Option<String>,
    pub is_expanded: bool,
    pub sort_order: i32,
    pub parent_id: Option<i64>,
}

/// MCP Server instance in the centralized pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub transport: McpTransport,
    pub command: String,
    pub args: Vec<String>,
    pub env: serde_json::Value,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub status: McpStatus,
    pub socket_path: Option<String>,
    pub client_count: i32,
    pub last_started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Http,
}

impl Default for McpTransport {
    fn default() -> Self {
        Self::Stdio
    }
}

impl std::fmt::Display for McpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio => write!(f, "stdio"),
            Self::Http => write!(f, "http"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpStatus {
    Running,
    Stopped,
    Error,
}

/// Scan result from filesystem scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub projects_found: usize,
    pub tracks_found: usize,
    pub projects: Vec<ProjectScanInfo>,
    pub errors: Vec<String>,
    pub scan_method: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectScanInfo {
    pub path: String,
    pub name: String,
    pub description: Option<String>,
    pub project_type: Option<String>,
    pub track_count: usize,
}
