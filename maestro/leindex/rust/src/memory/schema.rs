//! Database Schema
//!
//! SQL schema for rusqlite. Optimized with proper indexes.

/// DDL for creating all tables
pub const CREATE_TABLES_SQL: &str = r#"
-- Maestro Projects
CREATE TABLE IF NOT EXISTS maestro_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_path TEXT NOT NULL UNIQUE,
    project_name TEXT NOT NULL,
    description TEXT,
    project_type TEXT,
    tech_stack TEXT,  -- JSON array
    is_active INTEGER DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    last_scanned_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_projects_path ON maestro_projects(project_path);
CREATE INDEX IF NOT EXISTS idx_projects_active ON maestro_projects(is_active);

-- Maestro Tracks
CREATE TABLE IF NOT EXISTS maestro_tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    project_id INTEGER NOT NULL REFERENCES maestro_projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'new',
    total_tasks INTEGER DEFAULT 0,
    completed_tasks INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT,
    UNIQUE(project_id, track_id)
);
CREATE INDEX IF NOT EXISTS idx_tracks_project ON maestro_tracks(project_id);
CREATE INDEX IF NOT EXISTS idx_tracks_status ON maestro_tracks(status);

-- Memories
CREATE TABLE IF NOT EXISTS memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    summary TEXT,
    category TEXT NOT NULL DEFAULT 'context',
    importance TEXT NOT NULL DEFAULT 'normal',
    source TEXT,
    session_id TEXT,
    project_id INTEGER REFERENCES maestro_projects(id) ON DELETE CASCADE,
    track_id INTEGER REFERENCES maestro_tracks(id) ON DELETE CASCADE,
    command TEXT,
    command_context TEXT,  -- JSON
    embedding_id INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    last_accessed TEXT,
    meta_data TEXT,  -- JSON
    tags TEXT  -- JSON array
);
CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id);
CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
CREATE INDEX IF NOT EXISTS idx_memories_expires ON memories(expires_at);

-- File Claims (multi-agent coordination)
CREATE TABLE IF NOT EXISTS file_claims (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id TEXT NOT NULL UNIQUE,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    file_patterns TEXT NOT NULL,  -- JSON array
    status TEXT NOT NULL DEFAULT 'active',
    is_exclusive INTEGER DEFAULT 1,
    reason TEXT,
    claimed_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    released_at TEXT,
    version INTEGER DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_claims_agent ON file_claims(agent_id);
CREATE INDEX IF NOT EXISTS idx_claims_status ON file_claims(status);
CREATE INDEX IF NOT EXISTS idx_claims_expires ON file_claims(expires_at);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    project_path TEXT NOT NULL,
    group_path TEXT,
    parent_session_id TEXT,
    command TEXT,
    tool TEXT,
    status TEXT NOT NULL DEFAULT 'idle',
    multiplexer_session TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_accessed_at TEXT,
    ended_at TEXT,
    meta_data TEXT  -- JSON
);
CREATE INDEX IF NOT EXISTS idx_sessions_path ON sessions(project_path);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- Session Groups
CREATE TABLE IF NOT EXISTS session_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    category TEXT,
    is_expanded INTEGER DEFAULT 1,
    sort_order INTEGER DEFAULT 0,
    parent_id INTEGER REFERENCES session_groups(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_groups_path ON session_groups(path);

-- MCP Servers (centralized pool)
CREATE TABLE IF NOT EXISTS mcp_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    command TEXT NOT NULL,
    args TEXT,  -- JSON array
    env TEXT,   -- JSON object
    status TEXT NOT NULL DEFAULT 'stopped',
    socket_path TEXT,
    client_count INTEGER DEFAULT 0,
    last_started_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_mcp_status ON mcp_servers(status);

-- Agent Namespaces
CREATE TABLE IF NOT EXISTS agent_namespaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    agent_type TEXT,
    is_public INTEGER DEFAULT 0,
    allowed_readers TEXT,  -- JSON array
    allowed_writers TEXT,  -- JSON array
    config TEXT,  -- JSON
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_namespaces_name ON agent_namespaces(name);
CREATE INDEX IF NOT EXISTS idx_namespaces_owner ON agent_namespaces(owner_type, owner_id);
"#;

/// Migrations for schema updates
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", "-- Already created in CREATE_TABLES_SQL"),
    (
        "002_add_indexes",
        r#"
        CREATE INDEX IF NOT EXISTS idx_memories_project_track ON memories(project_id, track_id);
        CREATE INDEX IF NOT EXISTS idx_memories_category_importance ON memories(category, importance);
    "#,
    ),
    (
        "003_tui_consolidation",
        r#"
        -- For existing sessions table, we need to add a few columns if they don't exist
        -- But easiest is to drop and recreate for now during dev, OR use safer ALTERs
        -- We'll use safe ALTERs for production-ready migrations
        ALTER TABLE sessions ADD COLUMN title TEXT NOT NULL DEFAULT 'Untitled';
        ALTER TABLE sessions ADD COLUMN project_path TEXT NOT NULL DEFAULT '';
        ALTER TABLE sessions ADD COLUMN group_path TEXT;
        ALTER TABLE sessions ADD COLUMN parent_session_id TEXT;
        ALTER TABLE sessions ADD COLUMN command TEXT;
        ALTER TABLE sessions ADD COLUMN tool TEXT;
        ALTER TABLE sessions ADD COLUMN multiplexer_session TEXT;
        ALTER TABLE sessions ADD COLUMN last_accessed_at TEXT;
        
        -- Create new tables
        CREATE TABLE IF NOT EXISTS session_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            is_expanded INTEGER DEFAULT 1,
            sort_order INTEGER DEFAULT 0,
            parent_id INTEGER REFERENCES session_groups(id) ON DELETE CASCADE
        );
        
        CREATE TABLE IF NOT EXISTS mcp_servers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            command TEXT NOT NULL,
            args TEXT,
            env TEXT,
            status TEXT NOT NULL DEFAULT 'stopped',
            socket_path TEXT,
            client_count INTEGER DEFAULT 0,
            last_started_at TEXT
        );
    "#,
    ),
    (
        "004_group_categorization",
        r#"
        ALTER TABLE session_groups ADD COLUMN category TEXT;
    "#,
    ),
];
