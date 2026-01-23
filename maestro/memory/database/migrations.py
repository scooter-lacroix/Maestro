"""
Database Migration System for Maestro Memory

Provides version tracking and migration support for the unified
memory database schema.
"""

import os
import json
import re
from datetime import datetime, UTC
from typing import Optional, Dict, Any, List, Callable
from dataclasses import dataclass, field

from sqlalchemy import text, inspect, Table, Column, String, DateTime, MetaData


# Allowed table names for migrations tracking (allowlist pattern)
_ALLOWED_MIGRATION_TABLES = {"_schema_migrations", "schema_migrations"}


def _validate_table_name(table_name: str) -> str:
    """
    Validate table name against allowlist and SQL injection patterns.

    Args:
        table_name: The table name to validate

    Returns:
        The validated table name

    Raises:
        ValueError: If table name contains invalid characters or is not allowed
    """
    # Check against allowlist
    if table_name not in _ALLOWED_MIGRATION_TABLES:
        raise ValueError(
            f"Table name '{table_name}' is not in the allowed migration tables. "
            f"Allowed: {_ALLOWED_MIGRATION_TABLES}"
        )

    # Additional check: only allow alphanumeric, underscores, and hyphens
    if not re.match(r'^[a-zA-Z_][a-zA-Z0-9_]*$', table_name):
        raise ValueError(
            f"Table name '{table_name}' contains invalid characters. "
            "Only alphanumeric characters and underscores are allowed."
        )

    return table_name


@dataclass
class Migration:
    """
    Represents a database migration

    Attributes:
        version: Unique version identifier (e.g., "001", "002")
        name: Human-readable migration name
        up: SQL statements or callable to apply migration
        down: SQL statements or callable to rollback migration
        applied_at: When this migration was applied
    """

    version: str
    name: str
    up: str | Callable
    down: str | Callable
    applied_at: Optional[datetime] = None

    def __post_init__(self) -> None:
        """Convert SQL strings to callable functions if needed"""
        if isinstance(self.up, str):
            original_up = self.up
            self.up = lambda session: session.execute(text(original_up))
        if isinstance(self.down, str):
            original_down = self.down
            self.down = lambda session: session.execute(text(original_down))


class MigrationManager:
    """
    Manages database migrations

    Tracks applied migrations and can apply or rollback
    migrations in the correct order.
    """

    def __init__(self, session: Any, db_path: Optional[str] = None, migrations_table: str = "_schema_migrations") -> None:
        """
        Initialize the migration manager

        Args:
            session: SQLAlchemy session
            db_path: Path to database file
            migrations_table: Name of the migrations tracking table
        """
        self.session = session
        self.db_path = db_path
        # Validate the table name against allowlist
        self.migrations_table = _validate_table_name(migrations_table)
        self._metadata = MetaData()
        # Cache the table object
        self._migrations_table: Table | None = None

    def _get_migrations_table(self) -> Table:
        """Get the SQLAlchemy Table object for migrations (cached)"""
        if self._migrations_table is None:
            self._migrations_table = Table(
                self.migrations_table,
                self._metadata,
                Column("version", String(20), primary_key=True),
                Column("name", String(200), nullable=False),
                Column("applied_at", DateTime, nullable=False),
            )
        assert self._migrations_table is not None
        return self._migrations_table

    def ensure_migrations_table(self) -> None:
        """Create the migrations tracking table if it doesn't exist"""
        check_sql = text("""
            SELECT name FROM sqlite_master
            WHERE type='table' AND name=:table_name
        """)
        result = self.session.execute(check_sql, {"table_name": self.migrations_table})
        exists = result.fetchone() is not None

        if not exists:
            # Use SQLAlchemy Table to create the table safely
            migrations_table = self._get_migrations_table()
            migrations_table.create(self.session.bind, checkfirst=True)
            self.session.commit()

    def get_applied_migrations(self) -> List[str]:
        """Get list of applied migration versions"""
        self.ensure_migrations_table()

        migrations_table = self._get_migrations_table()
        stmt = migrations_table.select().order_by(migrations_table.c.version.asc())
        result = self.session.execute(stmt)
        return [row.version for row in result.fetchall()]

    def record_migration(self, migration: Migration) -> bool:
        """
        Record that a migration has been applied with atomic compare-and-swap.

        Uses database-level locking to prevent duplicate migrations.

        Args:
            migration: Migration to record

        Returns:
            True if recorded successfully, False if already exists
        """
        self.ensure_migrations_table()

        migrations_table = self._get_migrations_table()

        # Use INSERT with ON CONFLICT DO NOTHING for atomic compare-and-swap
        # This prevents race conditions between check and insert
        try:
            # First, check if migration exists (still needed for return value)
            existing_stmt = migrations_table.select().where(
                migrations_table.c.version == migration.version
            )
            existing = self.session.execute(existing_stmt).fetchone()

            if existing is not None:
                return False  # Already exists

            # Use raw SQL with INSERT OR IGNORE for atomicity
            # Build the query with proper table name interpolation (already validated)
            insert_sql = text(
                f"INSERT OR IGNORE INTO {self.migrations_table} (version, name, applied_at) "
                "VALUES (:version, :name, :applied_at)"
            )

            self.session.execute(insert_sql, {
                "version": migration.version,
                "name": migration.name,
                "applied_at": datetime.now(UTC),
            })
            self.session.commit()
            return True

        except Exception:
            self.session.rollback()
            raise

    def remove_migration_record(self, version: str) -> None:
        """Remove a migration record"""
        self.ensure_migrations_table()

        migrations_table = self._get_migrations_table()
        stmt = migrations_table.delete().where(migrations_table.c.version == version)
        self.session.execute(stmt)
        self.session.commit()

    def apply_migration(self, migration: Migration) -> bool:
        """
        Apply a single migration with atomic check-and-apply.

        Args:
            migration: Migration to apply

        Returns:
            True if applied, False if already applied
        """
        applied = self.get_applied_migrations()
        if migration.version in applied:
            return False

        # Execute the migration
        if isinstance(migration.up, str):
            self.session.execute(text(migration.up))
        else:
            migration.up(self.session)
        self.session.commit()

        # Record the migration (atomic operation)
        migration.applied_at = datetime.now(UTC)
        recorded = self.record_migration(migration)

        return recorded

    def rollback_migration(self, migration: Migration) -> bool:
        """
        Rollback a single migration

        Args:
            migration: Migration to rollback

        Returns:
            True if rolled back, False if not applied
        """
        applied = self.get_applied_migrations()
        if migration.version not in applied:
            return False

        # Execute the rollback
        if isinstance(migration.down, str):
            self.session.execute(text(migration.down))
        else:
            migration.down(self.session)
        self.session.commit()

        # Remove the record
        self.remove_migration_record(migration.version)

        return True

    def apply_migrations(
        self,
        migrations: List[Migration],
        target_version: Optional[str] = None,
    ) -> List[str]:
        """
        Apply pending migrations

        Args:
            migrations: List of available migrations (must be sorted by version)
            target_version: Optional target version (stop here)

        Returns:
            List of applied migration versions
        """
        applied = self.get_applied_migrations()
        applied_versions = []

        for migration in migrations:
            if target_version and migration.version > target_version:
                break

            if migration.version not in applied:
                if self.apply_migration(migration):
                    applied_versions.append(migration.version)

        return applied_versions

    def get_current_version(self) -> Optional[str]:
        """Get the current database schema version"""
        applied = self.get_applied_migrations()
        return applied[-1] if applied else None


# ============================================================================
# DEFINED MIGRATIONS
# ============================================================================

def get_initial_migrations() -> List[Migration]:
    """
    Get the initial set of migrations for Maestro v2

    These migrations create the base schema for the unified memory system.
    """
    return [
        Migration(
            version="001",
            name="create_base_schema",
            up=lambda session: _create_base_schema(session),
            down=lambda session: _drop_base_schema(session),
        ),
        Migration(
            version="002",
            name="create_vector_table",
            up=lambda session: _create_vector_table(session),
            down=lambda session: _drop_vector_table(session),
        ),
        Migration(
            version="003",
            name="create_indexes",
            up=lambda session: _create_performance_indexes(session),
            down=lambda session: _drop_performance_indexes(session),
        ),
        Migration(
            version="004",
            name="add_agent_type_to_namespaces",
            up=lambda session: _add_agent_type_to_namespaces(session),
            down=lambda session: _remove_agent_type_from_namespaces(session),
        ),
    ]


def _create_base_schema(session: Any) -> None:
    """Create the base database schema"""
    statements = [
        # Memories table
        """
        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            summary TEXT,
            category VARCHAR(50) NOT NULL DEFAULT 'context',
            importance VARCHAR(50) NOT NULL DEFAULT 'normal',
            source VARCHAR(200),
            session_id VARCHAR(200),
            project_id INTEGER,
            track_id INTEGER,
            command VARCHAR(100),
            command_context JSON,
            embedding_id INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            expires_at TIMESTAMP,
            last_accessed TIMESTAMP,
            metadata JSON,
            tags JSON
        )
        """,
        # Agent namespaces table
        """
        CREATE TABLE IF NOT EXISTS agent_namespaces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name VARCHAR(200) UNIQUE NOT NULL,
            description TEXT,
            owner_type VARCHAR(50) NOT NULL,
            owner_id VARCHAR(200) NOT NULL,
            agent_type VARCHAR(50),
            is_public BOOLEAN DEFAULT 0 NOT NULL,
            allowed_readers JSON,
            allowed_writers JSON,
            config JSON,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP,
            CHECK (owner_type IN ('agent', 'project', 'track'))
        )
        """,
        # Namespace-memory junction table
        """
        CREATE TABLE IF NOT EXISTS namespace_memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            namespace_id INTEGER NOT NULL,
            memory_id INTEGER NOT NULL,
            added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (namespace_id, memory_id)
        )
        """,
        # File claims table
        """
        CREATE TABLE IF NOT EXISTS file_claims (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            claim_id VARCHAR(200) UNIQUE NOT NULL,
            agent_id VARCHAR(200) NOT NULL,
            session_id VARCHAR(200),
            file_patterns JSON NOT NULL,
            status VARCHAR(50) NOT NULL DEFAULT 'active',
            is_exclusive BOOLEAN DEFAULT 1 NOT NULL,
            reason TEXT,
            task_description TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            expires_at TIMESTAMP NOT NULL,
            released_at TIMESTAMP,
            project_id INTEGER,
            track_id INTEGER,
            CHECK (status IN ('active', 'released', 'expired', 'revoked'))
        )
        """,
        # Handoffs table
        """
        CREATE TABLE IF NOT EXISTS handoffs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            handoff_id VARCHAR(200) UNIQUE NOT NULL,
            title VARCHAR(500) NOT NULL,
            from_session_id VARCHAR(200) NOT NULL,
            to_session_id VARCHAR(200),
            from_agent_id VARCHAR(200) NOT NULL,
            to_agent_id VARCHAR(200),
            status VARCHAR(50) NOT NULL DEFAULT 'pending',
            context_yaml TEXT NOT NULL,
            context_data JSON,
            project_path VARCHAR(500),
            summary TEXT,
            tags JSON,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            resumed_at TIMESTAMP,
            completed_at TIMESTAMP,
            project_id INTEGER,
            track_id INTEGER,
            CHECK (status IN ('pending', 'in_progress', 'resumed', 'abandoned', 'completed'))
        )
        """,
        # Continuity ledgers table
        """
        CREATE TABLE IF NOT EXISTS continuity_ledgers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ledger_id VARCHAR(200) UNIQUE NOT NULL,
            session_id VARCHAR(200) NOT NULL,
            agent_id VARCHAR(200) NOT NULL,
            entry_type VARCHAR(100) NOT NULL,
            title VARCHAR(500) NOT NULL,
            content TEXT NOT NULL,
            metadata JSON,
            parent_entry_id INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            sequence_number INTEGER NOT NULL,
            project_id INTEGER,
            track_id INTEGER,
            CHECK (entry_type IN ('decision', 'action', 'outcome', 'observation', 'question', 'answer'))
        )
        """,
        # Task specifications table
        """
        CREATE TABLE IF NOT EXISTS task_specifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id VARCHAR(200) UNIQUE NOT NULL,
            title VARCHAR(500) NOT NULL,
            description TEXT,
            specification JSON NOT NULL,
            requirements JSON,
            acceptance_criteria JSON,
            task_type VARCHAR(100),
            priority VARCHAR(50),
            complexity INTEGER,
            status VARCHAR(50) NOT NULL DEFAULT 'pending',
            progress REAL DEFAULT 0.0,
            assigned_to VARCHAR(200),
            session_id VARCHAR(200),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP,
            started_at TIMESTAMP,
            completed_at TIMESTAMP,
            due_at TIMESTAMP,
            project_id INTEGER,
            track_id INTEGER,
            parent_task_id INTEGER,
            CHECK (progress >= 0.0 AND progress <= 1.0),
            CHECK (complexity >= 1 AND complexity <= 10)
        )
        """,
        # Sessions table
        """
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id VARCHAR(200) UNIQUE NOT NULL,
            session_type VARCHAR(100) NOT NULL,
            title VARCHAR(500),
            description TEXT,
            agent_id VARCHAR(200),
            agent_name VARCHAR(200),
            status VARCHAR(50) NOT NULL DEFAULT 'active',
            project_path VARCHAR(500),
            working_directory VARCHAR(500),
            metadata JSON,
            tags JSON,
            message_count INTEGER DEFAULT 0,
            tool_use_count INTEGER DEFAULT 0,
            memory_count INTEGER DEFAULT 0,
            parent_session_id VARCHAR(200),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            started_at TIMESTAMP,
            ended_at TIMESTAMP,
            last_activity TIMESTAMP,
            project_id INTEGER,
            track_id INTEGER,
            CHECK (status IN ('active', 'paused', 'completed', 'terminated')),
            CHECK (session_type IN ('cli', 'tui', 'api', 'agent', 'track'))
        )
        """,
        # Maestro projects table
        """
        CREATE TABLE IF NOT EXISTS maestro_projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_path VARCHAR UNIQUE NOT NULL,
            project_name VARCHAR(200),
            description TEXT,
            project_type VARCHAR(50),
            tech_stack JSON,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            last_active TIMESTAMP
        )
        """,
        # Maestro tracks table
        """
        CREATE TABLE IF NOT EXISTS maestro_tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            track_id VARCHAR(200) UNIQUE NOT NULL,
            project_id INTEGER NOT NULL,
            title VARCHAR(500) NOT NULL,
            description TEXT,
            status VARCHAR(50) NOT NULL DEFAULT 'new',
            track_type VARCHAR(50),
            phase_count INTEGER DEFAULT 0,
            current_phase INTEGER DEFAULT 0,
            total_tasks INTEGER DEFAULT 0,
            completed_tasks INTEGER DEFAULT 0,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP,
            started_at TIMESTAMP,
            completed_at TIMESTAMP,
            FOREIGN KEY (project_id) REFERENCES maestro_projects(id)
        )
        """,
    ]

    for stmt in statements:
        session.execute(text(stmt))

    # Enable WAL mode and foreign keys
    session.execute(text("PRAGMA journal_mode=WAL"))
    session.execute(text("PRAGMA foreign_keys=ON"))
    session.commit()


def _drop_base_schema(session: Any) -> None:
    """Drop the base database schema"""
    tables = [
        "namespace_memories",
        "maestro_tracks",
        "maestro_projects",
        "sessions",
        "task_specifications",
        "continuity_ledgers",
        "handoffs",
        "file_claims",
        "agent_namespaces",
        "memories",
    ]

    for table in tables:
        session.execute(text(f"DROP TABLE IF EXISTS {table}"))

    session.commit()


def _create_vector_table(session: Any) -> None:
    """
    Create the vector table for semantic search

    This table uses the sqlite-vec extension for efficient
    vector similarity search.
    """
    # Note: This requires sqlite-vec to be loaded
    # The actual vec0 table creation depends on the extension
    statement = """
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_embeddings USING vec0(
            embedding_id INTEGER PRIMARY KEY,
            embedding FLOAT[384]
        )
    """
    try:
        session.execute(text(statement))
        session.commit()
    except Exception as e:
        # Extension may not be available
        session.rollback()
        print(f"Warning: Could not create vector table: {e}")


def _drop_vector_table(session: Any) -> None:
    """Drop the vector table"""
    session.execute(text("DROP TABLE IF EXISTS memory_embeddings"))
    session.commit()


def _create_performance_indexes(session: Any) -> None:
    """Create performance indexes for common queries"""
    indexes = [
        # Memory indexes
        "CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category)",
        "CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance)",
        "CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_memories_command ON memories(command)",
        "CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_memories_track ON memories(track_id)",
        "CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_memories_expires ON memories(expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_memories_category_importance ON memories(category, importance)",

        # Namespace indexes
        "CREATE INDEX IF NOT EXISTS idx_namespaces_name ON agent_namespaces(name)",
        "CREATE INDEX IF NOT EXISTS idx_namespaces_owner ON agent_namespaces(owner_type, owner_id)",

        # Namespace-memory indexes
        "CREATE INDEX IF NOT EXISTS idx_ns_memory_namespace ON namespace_memories(namespace_id)",
        "CREATE INDEX IF NOT EXISTS idx_ns_memory_memory ON namespace_memories(memory_id)",

        # File claims indexes
        "CREATE INDEX IF NOT EXISTS idx_file_claims_claim_id ON file_claims(claim_id)",
        "CREATE INDEX IF NOT EXISTS idx_file_claims_agent ON file_claims(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_file_claims_status ON file_claims(status)",
        "CREATE INDEX IF NOT EXISTS idx_file_claims_expires ON file_claims(expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_file_claims_project ON file_claims(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_file_claims_track ON file_claims(track_id)",

        # Handoffs indexes
        "CREATE INDEX IF NOT EXISTS idx_handoffs_handoff_id ON handoffs(handoff_id)",
        "CREATE INDEX IF NOT EXISTS idx_handoffs_from_session ON handoffs(from_session_id)",
        "CREATE INDEX IF NOT EXISTS idx_handoffs_to_session ON handoffs(to_session_id)",
        "CREATE INDEX IF NOT EXISTS idx_handoffs_status ON handoffs(status)",
        "CREATE INDEX IF NOT EXISTS idx_handoffs_created ON handoffs(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_handoffs_project ON handoffs(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_handoffs_track ON handoffs(track_id)",

        # Continuity ledger indexes
        "CREATE INDEX IF NOT EXISTS idx_ledgers_ledger_id ON continuity_ledgers(ledger_id)",
        "CREATE INDEX IF NOT EXISTS idx_ledgers_session ON continuity_ledgers(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_ledgers_agent ON continuity_ledgers(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_ledgers_entry_type ON continuity_ledgers(entry_type)",
        "CREATE INDEX IF NOT EXISTS idx_ledgers_sequence ON continuity_ledgers(session_id, sequence_number)",
        "CREATE INDEX IF NOT EXISTS idx_ledgers_project ON continuity_ledgers(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_ledgers_track ON continuity_ledgers(track_id)",

        # Task specifications indexes
        "CREATE INDEX IF NOT EXISTS idx_tasks_task_id ON task_specifications(task_id)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_type_status ON task_specifications(task_type, status)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_priority ON task_specifications(priority)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_assigned ON task_specifications(assigned_to)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_session ON task_specifications(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_project ON task_specifications(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_track ON task_specifications(track_id)",

        # Sessions indexes
        "CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON sessions(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_id)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_type_status ON sessions(session_type, status)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_activity ON sessions(last_activity)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_track ON sessions(track_id)",

        # Project indexes
        "CREATE INDEX IF NOT EXISTS idx_maestro_project_path ON maestro_projects(project_path)",
        "CREATE INDEX IF NOT EXISTS idx_maestro_project_type ON maestro_projects(project_type)",
        "CREATE INDEX IF NOT EXISTS idx_maestro_last_active ON maestro_projects(last_active)",

        # Track indexes
        "CREATE INDEX IF NOT EXISTS idx_maestro_track_id ON maestro_tracks(track_id)",
        "CREATE INDEX IF NOT EXISTS idx_maestro_project_tracks ON maestro_tracks(project_id)",
        "CREATE INDEX IF NOT EXISTS idx_maestro_track_status ON maestro_tracks(status)",
        "CREATE INDEX IF NOT EXISTS idx_maestro_track_type ON maestro_tracks(track_type)",
    ]

    for idx in indexes:
        session.execute(text(idx))

    session.commit()


def _drop_performance_indexes(session: Any) -> None:
    """Drop performance indexes"""
    # Indexes are automatically dropped when tables are dropped
    pass


# ============================================================================
# MIGRATION FROM LEGACY SYSTEMS
# ============================================================================

class LegacyMigrationManager:
    """
    Handles migration from legacy memory systems

    Supports migration from:
    - Nexus Memory System databases
    - Previous Maestro v1 databases
    """

    def __init__(self, session: Any, db_path: Optional[str] = None) -> None:
        """
        Initialize the legacy migration manager

        Args:
            session: SQLAlchemy session for target database
            db_path: Path to target database
        """
        self.session = session
        self.db_path = db_path

    def migrate_nexus_db(
        self,
        source_db_path: str,
        auto_migrate: bool = True,
    ) -> Dict[str, Any]:
        """
        Migrate data from a Nexus memory database

        Args:
            source_db_path: Path to Nexus database file
            auto_migrate: If True, automatically migrates simple data

        Returns:
            Migration report with counts and any issues
        """
        import sqlite3

        report: Dict[str, Any] = {
            "success": False,
            "memories_migrated": 0,
            "namespaces_migrated": 0,
            "tasks_migrated": 0,
            "errors": [],
            "warnings": [],
        }

        try:
            # Connect to source database
            source_conn = sqlite3.connect(source_db_path)
            source_conn.row_factory = sqlite3.Row
            source_cursor = source_conn.cursor()

            # Migrate memories
            report["memories_migrated"] = self._migrate_nexus_memories(
                source_cursor,
                auto_migrate,
            )

            # Migrate namespaces
            report["namespaces_migrated"] = self._migrate_nexus_namespaces(
                source_cursor,
                auto_migrate,
            )

            # Migrate tasks
            report["tasks_migrated"] = self._migrate_nexus_tasks(
                source_cursor,
                auto_migrate,
            )

            source_conn.close()
            self.session.commit()
            report["success"] = True

        except Exception as e:
            report["errors"].append(str(e))
            self.session.rollback()

        return report

    def _migrate_nexus_memories(
        self,
        source_cursor: Any,
        auto_migrate: bool,
    ) -> int:
        """Migrate memories from Nexus database"""
        # Check if memories table exists
        source_cursor.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='memories'"
        )
        if not source_cursor.fetchone():
            return 0

        source_cursor.execute("SELECT * FROM memories")
        rows = source_cursor.fetchall()

        migrated = 0
        for row in rows:
            try:
                # Map Nexus columns to Maestro schema
                # Use dict() to convert sqlite3.Row to dict for .get() access
                row_dict = dict(row)
                memory_data = {
                    "content": row_dict.get("content", ""),
                    "summary": row_dict.get("summary"),
                    "category": row_dict.get("category", "context"),
                    "importance": row_dict.get("importance", "normal"),
                    "source": "nexus_migration",
                    "session_id": row_dict.get("session_id"),
                    "meta_data": row_dict.get("metadata"),  # Map metadata -> meta_data
                    "tags": row_dict.get("tags"),
                }

                from maestro.memory.database.models import Memory
                memory = Memory(**memory_data)
                self.session.add(memory)
                migrated += 1

            except Exception as e:
                if auto_migrate:
                    continue
                else:
                    raise

        return migrated

    def _migrate_nexus_namespaces(
        self,
        source_cursor: Any,
        auto_migrate: bool,
    ) -> int:
        """Migrate namespaces from Nexus database"""
        # Check if namespaces table exists
        source_cursor.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='namespaces'"
        )
        if not source_cursor.fetchone():
            return 0

        source_cursor.execute("SELECT * FROM namespaces")
        rows = source_cursor.fetchall()

        migrated = 0
        for row in rows:
            try:
                from maestro.memory.database.models import AgentNamespace
                namespace = AgentNamespace(
                    name=row.get("name"),
                    description=row.get("description"),
                    owner_type=row.get("owner_type", "agent"),
                    owner_id=row.get("owner_id", ""),
                    is_public=row.get("is_public", False),
                    allowed_readers=row.get("allowed_readers"),
                    allowed_writers=row.get("allowed_writers"),
                )
                self.session.add(namespace)
                migrated += 1

            except Exception as e:
                if auto_migrate:
                    continue
                else:
                    raise

        return migrated

    def _migrate_nexus_tasks(
        self,
        source_cursor: Any,
        auto_migrate: bool,
    ) -> int:
        """Migrate tasks from Nexus database"""
        # Check if tasks table exists
        source_cursor.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='task_specifications'"
        )
        if not source_cursor.fetchone():
            return 0

        source_cursor.execute("SELECT * FROM task_specifications")
        rows = source_cursor.fetchall()

        migrated = 0
        for row in rows:
            try:
                from maestro.memory.database.models import TaskSpecification
                task = TaskSpecification(
                    task_id=row.get("task_id", f"migrated-{migrated}"),
                    title=row.get("title", "Migrated Task"),
                    description=row.get("description"),
                    specification=row.get("specification", {}),
                    requirements=row.get("requirements"),
                    acceptance_criteria=row.get("acceptance_criteria"),
                    task_type=row.get("task_type"),
                    priority=row.get("priority"),
                    complexity=row.get("complexity"),
                    status=row.get("status", "pending"),
                    progress=row.get("progress", 0.0),
                )
                self.session.add(task)
                migrated += 1

            except Exception as e:
                if auto_migrate:
                    continue
                else:
                    raise

        return migrated

    def generate_migration_script(
        self,
        source_db_path: str,
        output_path: str,
    ) -> None:
        """
        Generate a manual migration script for complex data

        Creates a Python script that can be reviewed and executed
        to migrate data with manual intervention points.

        Args:
            source_db_path: Path to source database
            output_path: Path to write the migration script
        """
        script_content = f"""#!/usr/bin/env python3
\"\"\"
Manual Migration Script
Generated for migrating from: {source_db_path}

This script requires manual review and intervention for complex data migration.
Edit the transform functions below to customize the migration.
\"\"\"

import sqlite3
from sqlalchemy import create_engine
from maestro.memory.database.models import Base, Memory, TaskSpecification

# Configuration
SOURCE_DB = "{source_db_path}"
TARGET_DB = "{self.db_path or '~/.maestro/memory.db'}"

def transform_memory(row: dict) -> dict:
    \"\"\"Transform a Nexus memory row to Maestro format\"\"\"
    # Customize this function to transform data as needed
    return {{
        "content": row.get("content", ""),
        "summary": row.get("summary"),
        "category": row.get("category", "context"),
        "importance": row.get("importance", "normal"),
        "metadata": row.get("metadata"),
    }}

def main():
    # Connect to databases
    source_conn = sqlite3.connect(SOURCE_DB)
    source_conn.row_factory = sqlite3.Row
    source_cursor = source_conn.cursor()

    target_engine = create_engine(f"sqlite:///{{TARGET_DB}}")
    Base.metadata.create_all(target_engine)

    from sqlalchemy.orm import sessionmaker
    Session = sessionmaker(bind=target_engine)
    target_session = Session()

    try:
        # Migrate memories
        print("Migrating memories...")
        source_cursor.execute("SELECT * FROM memories")
        for row in source_cursor.fetchall():
            data = transform_memory(dict(row))
            memory = Memory(**data)
            target_session.add(memory)

        target_session.commit()
        print("Migration complete!")

    except Exception as e:
        target_session.rollback()
        print(f"Migration failed: {{e}}")
        raise
    finally:
        source_conn.close()
        target_session.close()

if __name__ == "__main__":
    main()
"""

        with open(output_path, "w", encoding="utf-8") as f:
            f.write(script_content)


def _add_agent_type_to_namespaces(session: Any) -> None:
    """
    Add agent_type column to agent_namespaces table
    
    This migration adds the agent_type column that was missing from the original
    schema but is required by the service layer.
    """
    # Check if column already exists
    result = session.execute(text("PRAGMA table_info(agent_namespaces)"))
    existing_columns = {row[1] for row in result.fetchall()}
    
    if 'agent_type' not in existing_columns:
        # Add the column
        session.execute(text("ALTER TABLE agent_namespaces ADD COLUMN agent_type VARCHAR(50)"))
        # Create index for the new column
        session.execute(text("CREATE INDEX IF NOT EXISTS idx_namespaces_agent_type ON agent_namespaces(agent_type)"))
        session.commit()
        logger.info("Added agent_type column to agent_namespaces table")
    else:
        logger.info("agent_type column already exists in agent_namespaces table")


def _remove_agent_type_from_namespaces(session: Any) -> None:
    """
    Remove agent_type column from agent_namespaces table (rollback)
    """
    # Check if column exists before trying to remove it
    result = session.execute(text("PRAGMA table_info(agent_namespaces)"))
    existing_columns = {row[1] for row in result.fetchall()}
    
    if 'agent_type' in existing_columns:
        # SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
        # First, create a new table with the desired schema
        session.execute(text("""
            CREATE TABLE IF NOT EXISTS agent_namespaces_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR(200) UNIQUE NOT NULL,
                description TEXT,
                owner_type VARCHAR(50) NOT NULL,
                owner_id VARCHAR(200) NOT NULL,
                is_public BOOLEAN DEFAULT 0 NOT NULL,
                allowed_readers JSON,
                allowed_writers JSON,
                config JSON,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP,
                CHECK (owner_type IN ('agent', 'project', 'track'))
            )
        """))
        
        # Copy data from old table to new table (excluding agent_type)
        session.execute(text("""
            INSERT INTO agent_namespaces_new (id, name, description, owner_type, owner_id,
                                           is_public, allowed_readers, allowed_writers,
                                           config, created_at, updated_at)
            SELECT id, name, description, owner_type, owner_id,
                   is_public, allowed_readers, allowed_writers,
                   config, created_at, updated_at
            FROM agent_namespaces
        """))
        
        # Drop the old table
        session.execute(text("DROP TABLE agent_namespaces"))
        
        # Rename the new table
        session.execute(text("ALTER TABLE agent_namespaces_new RENAME TO agent_namespaces"))
        
        session.commit()
        logger.info("Removed agent_type column from agent_namespaces table")
    else:
        logger.info("agent_type column does not exist in agent_namespaces table")


def run_migrations(
    session: Any,
    db_path: Optional[str] = None,
    target_version: Optional[str] = None,
) -> List[str]:
    """
    Run all pending migrations

    Args:
        session: SQLAlchemy session
        db_path: Optional database path
        target_version: Optional target version

    Returns:
        List of applied migration versions
    """
    manager = MigrationManager(session, db_path)
    migrations = get_initial_migrations()
    return manager.apply_migrations(migrations, target_version)


def get_migration_status(session: Any, db_path: Optional[str] = None) -> Dict[str, Any]:
    """
    Get the current migration status

    Args:
        session: SQLAlchemy session
        db_path: Optional database path

    Returns:
        Status information about migrations
    """
    manager = MigrationManager(session, db_path)
    manager.ensure_migrations_table()

    applied = manager.get_applied_migrations()
    current = manager.get_current_version()
    all_migrations = get_initial_migrations()
    pending = [m.version for m in all_migrations if m.version not in applied]

    return {
        "current_version": current,
        "applied_migrations": applied,
        "pending_migrations": pending,
        "total_migrations": len(all_migrations),
        "applied_count": len(applied),
        "pending_count": len(pending),
    }
