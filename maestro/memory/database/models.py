"""
Maestro-specific database models

Extensions to Nexus memory schema for Maestro project and track management.
"""

from datetime import datetime, UTC
from typing import Optional, Dict, Any, List
from sqlalchemy import Column, Integer, String, Text, DateTime, ForeignKey, JSON, Index
from sqlalchemy.orm import declarative_base, relationship, Session
from sqlalchemy.sql import func
import sys
import os

# Issue 10: Use environment variable for Nexus path instead of hardcoded path
# Import Nexus Base to extend the existing schema
# Ensure nexus package is importable but don't trigger __init__.py server imports
nexus_path = os.environ.get('NEXUS_MEMORY_PATH', '/home/stan/Prod/work_resources/nexus-memory-system')
if nexus_path not in sys.path:
    sys.path.insert(0, nexus_path)

# Import only the database models, not the full package
from nexus.database.models import Base as NexusBase, Memory

# Combined Base for all models
Base = NexusBase


class MaestroProject(Base):
    """
    Maestro project registry

    Tracks all projects managed by Maestro with their metadata.
    """
    __tablename__ = "maestro_projects"

    id = Column(Integer, primary_key=True, index=True)
    project_path = Column(String, unique=True, nullable=False, index=True)
    project_name = Column(String(200), nullable=True)
    description = Column(Text, nullable=True)
    project_type = Column(String(50), nullable=True)  # e.g., "greenfield", "brownfield"
    tech_stack = Column(JSON, nullable=True)  # Store as JSON dict
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)
    last_active = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)

    # Relationships
    tracks = relationship("MaestroTrack", back_populates="project", cascade="all, delete-orphan")

    # Indexes
    __table_args__ = (
        Index('idx_maestro_project_path', 'project_path'),
        Index('idx_maestro_project_type', 'project_type'),
        Index('idx_maestro_last_active', 'last_active'),
    )

    def __repr__(self):
        return f"<MaestroProject(id={self.id}, name='{self.project_name}', path='{self.project_path}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert project to dictionary"""
        return {
            "id": self.id,
            "project_path": self.project_path,
            "project_name": self.project_name,
            "description": self.description,
            "project_type": self.project_type,
            "tech_stack": self.tech_stack or {},
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "last_active": self.last_active.isoformat() if self.last_active else None,
        }

    @classmethod
    async def get_or_create(cls, session, project_path: str, **kwargs) -> 'MaestroProject':
        """
        Get or create a Maestro project

        Args:
            session: SQLAlchemy async session
            project_path: Unique project path
            **kwargs: Additional project attributes

        Returns:
            MaestroProject instance
        """
        from sqlalchemy import select

        # Async session query
        stmt = select(cls).filter_by(project_path=project_path)
        result = await session.execute(stmt)
        project = result.scalars().first()

        if not project:
            project = cls(project_path=project_path, **kwargs)
            session.add(project)
            # Flush to get the ID
            await session.flush()

        return project


class MaestroTrack(Base):
    """
    Maestro track registry

    Tracks all development tracks within projects.
    """
    __tablename__ = "maestro_tracks"

    id = Column(Integer, primary_key=True, index=True)
    track_id = Column(String(200), unique=True, nullable=False, index=True)
    project_id = Column(Integer, ForeignKey("maestro_projects.id"), nullable=False, index=True)
    title = Column(String(500), nullable=False)
    description = Column(Text, nullable=True)
    status = Column(String(50), nullable=False, default="new")  # new, in_progress, completed, blocked
    track_type = Column(String(50), nullable=True)  # feature, bugfix, refactor, etc.

    # Track metadata
    phase_count = Column(Integer, default=0)
    current_phase = Column(Integer, default=0)
    total_tasks = Column(Integer, default=0)
    completed_tasks = Column(Integer, default=0)

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)
    updated_at = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)
    started_at = Column(DateTime(timezone=True), nullable=True)
    completed_at = Column(DateTime(timezone=True), nullable=True)

    # Relationships
    project = relationship("MaestroProject", back_populates="tracks")

    # Indexes
    __table_args__ = (
        Index('idx_maestro_track_id', 'track_id'),
        Index('idx_maestro_project_tracks', 'project_id'),
        Index('idx_maestro_track_status', 'status'),
        Index('idx_maestro_track_type', 'track_type'),
    )

    def __repr__(self):
        return f"<MaestroTrack(id={self.id}, track_id='{self.track_id}', title='{self.title}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert track to dictionary"""
        return {
            "id": self.id,
            "track_id": self.track_id,
            "project_id": self.project_id,
            "title": self.title,
            "description": self.description,
            "status": self.status,
            "track_type": self.track_type,
            "phase_count": self.phase_count,
            "current_phase": self.current_phase,
            "total_tasks": self.total_tasks,
            "completed_tasks": self.completed_tasks,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "updated_at": self.updated_at.isoformat() if self.updated_at else None,
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "completed_at": self.completed_at.isoformat() if self.completed_at else None,
        }

    @classmethod
    async def get_or_create(cls, session, track_id: str, project_id: int, **kwargs) -> 'MaestroTrack':
        """
        Get or create a Maestro track

        Args:
            session: SQLAlchemy async session
            track_id: Unique track identifier
            project_id: Parent project ID
            **kwargs: Additional track attributes

        Returns:
            MaestroTrack instance
        """
        from sqlalchemy import select

        # Async session query
        stmt = select(cls).filter_by(track_id=track_id)
        result = await session.execute(stmt)
        track = result.scalars().first()

        if not track:
            track = cls(track_id=track_id, project_id=project_id, **kwargs)
            session.add(track)
            # Flush to get the ID
            await session.flush()

        return track


# Extend Memory model with Maestro-specific columns via migration
# These columns are added to the existing memories table:
# - maestro_project_id: INTEGER (FK to maestro_projects.id)
# - maestro_track_id: INTEGER (FK to maestro_tracks.id)
# - maestro_command: VARCHAR(100) - The command that created this memory
# - maestro_command_context: JSON - Additional command-specific context


def create_maestro_extension_tables(engine):
    """
    Create Maestro-specific database tables

    Args:
        engine: SQLAlchemy engine (can be sync or async)
    """
    # For async engines, we need to get the sync engine
    from sqlalchemy.ext.asyncio import AsyncEngine
    if isinstance(engine, AsyncEngine):
        sync_engine = engine.sync_engine
    else:
        sync_engine = engine

    Base.metadata.create_all(sync_engine, tables=[MaestroProject.__table__, MaestroTrack.__table__])


def add_maestro_columns_to_memories(engine):
    """
    Add Maestro-specific columns to existing memories table

    Args:
        engine: SQLAlchemy engine
    """
    from sqlalchemy import text

    with engine.connect() as conn:
        # Check if columns already exist
        inspector_query = text("PRAGMA table_info(memories)")
        result = conn.execute(inspector_query)
        existing_columns = {row[1] for row in result.fetchall()}

        # Add columns if they don't exist
        if 'maestro_project_id' not in existing_columns:
            conn.execute(text(
                "ALTER TABLE memories ADD COLUMN maestro_project_id INTEGER"
            ))
            conn.execute(text(
                "CREATE INDEX IF NOT EXISTS idx_memories_maestro_project ON memories(maestro_project_id)"
            ))

        if 'maestro_track_id' not in existing_columns:
            conn.execute(text(
                "ALTER TABLE memories ADD COLUMN maestro_track_id INTEGER"
            ))
            conn.execute(text(
                "CREATE INDEX IF NOT EXISTS idx_memories_maestro_track ON memories(maestro_track_id)"
            ))

        if 'maestro_command' not in existing_columns:
            conn.execute(text(
                "ALTER TABLE memories ADD COLUMN maestro_command VARCHAR(100)"
            ))
            conn.execute(text(
                "CREATE INDEX IF NOT EXISTS idx_memories_maestro_command ON memories(maestro_command)"
            ))

        if 'maestro_command_context' not in existing_columns:
            conn.execute(text(
                "ALTER TABLE memories ADD COLUMN maestro_command_context JSON"
            ))

        conn.commit()
