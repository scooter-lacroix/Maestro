"""
Maestro Track Models

Defines the track management classes and enums for coordinating
development tracks with the memory system.
"""

from datetime import datetime, UTC
from typing import Optional, Dict, Any, List, TYPE_CHECKING
from enum import Enum
from dataclasses import dataclass, field

if TYPE_CHECKING:
    from maestro.memory.database.models import Memory


class TrackStatus(str, Enum):
    """Status of a development track"""

    NEW = "new"
    PLANNING = "planning"
    IN_PROGRESS = "in_progress"
    PAUSED = "paused"
    REVIEW = "review"
    COMPLETED = "completed"
    CANCELLED = "cancelled"
    BLOCKED = "blocked"


class TrackType(str, Enum):
    """Type of development track"""

    FEATURE = "feature"
    BUGFIX = "bugfix"
    CHORE = "chore"
    REFACTOR = "refactor"
    DOCUMENTATION = "documentation"
    TEST = "test"
    SECURITY = "security"
    PERFORMANCE = "performance"


@dataclass
class TrackSpec:
    """
    Track specification document

    Contains the formal specification for a track, defining
    what needs to be built and the acceptance criteria.
    """

    track_id: str
    title: str
    description: str
    track_type: TrackType
    status: TrackStatus = TrackStatus.NEW

    # Specification content
    functional_requirements: List[str] = field(default_factory=list)
    non_functional_requirements: List[str] = field(default_factory=list)
    acceptance_criteria: List[str] = field(default_factory=list)
    out_of_scope: List[str] = field(default_factory=list)

    # Track metadata
    priority: str = "normal"  # critical, high, normal, low
    complexity: int = 5  # 1-10 scale
    estimated_hours: Optional[float] = None

    # Maestro memory references
    maestro_project_id: Optional[int] = None
    maestro_track_id: Optional[int] = None

    # Timestamps
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))
    updated_at: datetime = field(default_factory=lambda: datetime.now(UTC))
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert track spec to dictionary"""
        return {
            "track_id": self.track_id,
            "title": self.title,
            "description": self.description,
            "track_type": self.track_type.value if isinstance(self.track_type, TrackType) else self.track_type,
            "status": self.status.value if isinstance(self.status, TrackStatus) else self.status,
            "functional_requirements": self.functional_requirements,
            "non_functional_requirements": self.non_functional_requirements,
            "acceptance_criteria": self.acceptance_criteria,
            "out_of_scope": self.out_of_scope,
            "priority": self.priority,
            "complexity": self.complexity,
            "estimated_hours": self.estimated_hours,
            "maestro_project_id": self.maestro_project_id,
            "maestro_track_id": self.maestro_track_id,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "updated_at": self.updated_at.isoformat() if self.updated_at else None,
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "completed_at": self.completed_at.isoformat() if self.completed_at else None,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TrackSpec":
        """Create track spec from dictionary"""
        track_type = data.get("track_type", TrackType.FEATURE)
        if isinstance(track_type, str):
            try:
                track_type = TrackType(track_type)
            except ValueError:
                track_type = TrackType.FEATURE

        status = data.get("status", TrackStatus.NEW)
        if isinstance(status, str):
            try:
                status = TrackStatus(status)
            except ValueError:
                status = TrackStatus.NEW

        return cls(
            track_id=data["track_id"],
            title=data["title"],
            description=data["description"],
            track_type=track_type,
            status=status,
            functional_requirements=data.get("functional_requirements", []),
            non_functional_requirements=data.get("non_functional_requirements", []),
            acceptance_criteria=data.get("acceptance_criteria", []),
            out_of_scope=data.get("out_of_scope", []),
            priority=data.get("priority", "normal"),
            complexity=data.get("complexity", 5),
            estimated_hours=data.get("estimated_hours"),
            maestro_project_id=data.get("maestro_project_id"),
            maestro_track_id=data.get("maestro_track_id"),
        )


@dataclass
class TrackPlan:
    """
    Track implementation plan

    Contains the structured plan for implementing a track,
    organized by phases and tasks.
    """

    track_id: str
    phases: List[Dict[str, Any]] = field(default_factory=list)

    # Plan metadata
    total_phases: int = 0
    total_tasks: int = 0
    completed_tasks: int = 0
    current_phase: int = 0

    def to_dict(self) -> Dict[str, Any]:
        """Convert track plan to dictionary"""
        return {
            "track_id": self.track_id,
            "phases": self.phases,
            "total_phases": self.total_phases,
            "total_tasks": self.total_tasks,
            "completed_tasks": self.completed_tasks,
            "current_phase": self.current_phase,
        }

    def calculate_progress(self) -> float:
        """Calculate progress percentage"""
        if self.total_tasks == 0:
            return 0.0
        return (self.completed_tasks / self.total_tasks) * 100


@dataclass
class TrackMetadata:
    """
    Track metadata for storage and tracking

    Contains the metadata stored in the track's metadata.json file.
    """

    track_id: str
    type: str
    status: str
    description: str
    created_at: str
    updated_at: str

    # Maestro memory references
    maestro_project_id: Optional[int] = None
    maestro_track_id: Optional[int] = None

    # Handoff reference
    current_handoff_id: Optional[str] = None

    # TLDR analysis reference
    tldr_analysis_id: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert metadata to dictionary"""
        return {
            "track_id": self.track_id,
            "type": self.type,
            "status": self.status,
            "description": self.description,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "maestro_project_id": self.maestro_project_id,
            "maestro_track_id": self.maestro_track_id,
            "current_handoff_id": self.current_handoff_id,
            "tldr_analysis_id": self.tldr_analysis_id,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TrackMetadata":
        """Create metadata from dictionary"""
        return cls(
            track_id=data["track_id"],
            type=data["type"],
            status=data["status"],
            description=data["description"],
            created_at=data["created_at"],
            updated_at=data["updated_at"],
            maestro_project_id=data.get("maestro_project_id"),
            maestro_track_id=data.get("maestro_track_id"),
            current_handoff_id=data.get("current_handoff_id"),
            tldr_analysis_id=data.get("tldr_analysis_id"),
        )


class TrackManager:
    """
    Manager for track operations

    Coordinates track lifecycle with memory system integration.
    """

    def __init__(self, db_session: Any, project_path: str) -> None:
        """
        Initialize track manager

        Args:
            db_session: SQLAlchemy database session
            project_path: Path to the project directory
        """
        self.db_session = db_session
        self.project_path = project_path
        self._project_id = None
        self._track_ids: Dict[str, int] = {}

    def get_or_create_project(self) -> int:
        """
        Get or create Maestro project record

        Returns:
            Project ID
        """
        from maestro.memory.database.models import MaestroProject

        project = self.db_session.query(MaestroProject).filter(
            MaestroProject.project_path == self.project_path
        ).first()

        if not project:
            import os
            project_name = os.path.basename(self.project_path)
            project = MaestroProject(
                project_path=self.project_path,
                project_name=project_name,
                project_type="brownfield",
            )
            self.db_session.add(project)
            self.db_session.flush()

        self._project_id = project.id
        return int(project.id)

    def get_or_create_track(self, track_id: str, title: str) -> int:
        """
        Get or create Maestro track record

        Args:
            track_id: Track identifier (e.g., "feature_20260110")
            title: Track title

        Returns:
            Track database ID
        """
        from maestro.memory.database.models import MaestroTrack

        if track_id in self._track_ids:
            return self._track_ids[track_id]

        project_id = self.get_or_create_project()

        track = self.db_session.query(MaestroTrack).filter(
            MaestroTrack.track_id == track_id
        ).first()

        if not track:
            track = MaestroTrack(
                track_id=track_id,
                project_id=project_id,
                title=title,
                status="new",
                track_type="feature",
            )
            self.db_session.add(track)
            self.db_session.flush()

        self._track_ids[track_id] = track.id
        return int(track.id)

    def update_track_status(
        self,
        track_id: str,
        status: str,
        completed_tasks: Optional[int] = None,
        total_tasks: Optional[int] = None,
    ) -> None:
        """
        Update track status in database

        Args:
            track_id: Track identifier
            status: New status
            completed_tasks: Optional completed task count
            total_tasks: Optional total task count
        """
        from maestro.memory.database.models import MaestroTrack

        track = self.db_session.query(MaestroTrack).filter(
            MaestroTrack.track_id == track_id
        ).first()

        if track:
            track.status = status
            if completed_tasks is not None:
                track.completed_tasks = completed_tasks
            if total_tasks is not None:
                track.total_tasks = total_tasks

            if status == TrackStatus.COMPLETED.value and not track.completed_at:
                from datetime import datetime, UTC
                track.completed_at = datetime.now(UTC)

            self.db_session.flush()

    def store_track_memory(
        self,
        track_id: str,
        content: str,
        category: str = "context",
        importance: str = "normal",
        summary: Optional[str] = None,
    ) -> None:
        """
        Store a memory associated with a track

        Args:
            track_id: Track identifier
            content: Memory content
            category: Memory category
            importance: Memory importance
            summary: Optional summary
        """
        from maestro.memory.database.models import Memory

        maestro_track_id = self.get_or_create_track(track_id, track_id)
        project_id = self.get_or_create_project()

        memory = Memory(
            content=content,
            summary=summary,
            category=category,
            importance=importance,
            source="track_manager",
            project_id=project_id,
            track_id=maestro_track_id,
            command="maestro:newTrack",
        )

        self.db_session.add(memory)
        self.db_session.flush()

    def get_track_memories(
        self,
        track_id: str,
        limit: int = 50,
    ) -> List["Memory"]:
        """
        Retrieve memories associated with a track

        Args:
            track_id: Track identifier
            limit: Maximum results

        Returns:
            List of Memory objects
        """
        from maestro.memory.database.models import Memory

        maestro_track_id = self.get_or_create_track(track_id, track_id)

        memories = self.db_session.query(Memory).filter(
            Memory.track_id == maestro_track_id
        ).order_by(Memory.created_at.desc()).limit(limit).all()

        return list(memories)

    def get_track_summary(self, track_id: str) -> Dict[str, Any]:
        """
        Get summary of track with memory context

        Args:
            track_id: Track identifier

        Returns:
            Track summary dictionary
        """
        from maestro.memory.database.models import MaestroTrack

        track = self.db_session.query(MaestroTrack).filter(
            MaestroTrack.track_id == track_id
        ).first()

        if not track:
            return {
                "track_id": track_id,
                "found": False,
            }

        memories = self.get_track_memories(track_id, limit=10)

        return {
            "track_id": track_id,
            "found": True,
            "title": track.title,
            "status": track.status,
            "total_tasks": track.total_tasks,
            "completed_tasks": track.completed_tasks,
            "progress": (track.completed_tasks / track.total_tasks * 100) if track.total_tasks > 0 else 0,
            "created_at": track.created_at.isoformat() if track.created_at else None,
            "updated_at": track.updated_at.isoformat() if track.updated_at else None,
            "recent_memories": [
                {
                    "content": m.content[:200] + "..." if len(m.content) > 200 else m.content,
                    "category": m.category,
                    "created_at": m.created_at.isoformat() if m.created_at else None,
                }
                for m in memories
            ],
        }
