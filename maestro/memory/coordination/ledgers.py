"""
Continuity Ledgers Handler for Session Progress Tracking

Maintains a chronological record of session activities, decisions,
and outcomes for continuity tracking and analysis.
"""

import uuid
from datetime import datetime, UTC
from typing import Optional, Dict, Any, List, Literal, cast

from sqlalchemy import select, and_, desc
from sqlalchemy.orm import Session

from maestro.memory.database.models import (
    ContinuityLedger,
    MaestroProject,
    MaestroTrack,
)

EntryType = Literal[
    "decision",
    "action",
    "outcome",
    "observation",
    "question",
    "answer",
]


class ContinuityLedgerHandler:
    """
    Handles continuity ledgers for session tracking

    Maintains chronological records of session activities,
    decisions, actions, and outcomes for continuity analysis.
    """

    VALID_ENTRY_TYPES = {
        "decision",
        "action",
        "outcome",
        "observation",
        "question",
        "answer",
    }

    def __init__(self, session: Session):
        """
        Initialize the continuity ledger handler

        Args:
            session: SQLAlchemy database session
        """
        self.session = session

    def create_entry(
        self,
        session_id: str,
        agent_id: str,
        entry_type: EntryType,
        title: str,
        content: str,
        metadata: Optional[Dict[str, Any]] = None,
        parent_entry_id: Optional[int] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        ledger_id: Optional[str] = None,
    ) -> ContinuityLedger:
        """
        Create a new ledger entry

        Args:
            session_id: Associated session ID
            agent_id: Associated agent ID
            entry_type: Type of entry (decision, action, etc.)
            title: Entry title
            content: Entry content
            metadata: Optional metadata
            parent_entry_id: Optional parent entry ID for hierarchy
            project_id: Optional Maestro project ID
            track_id: Optional Maestro track ID
            ledger_id: Optional custom ledger ID

        Returns:
            Created ContinuityLedger instance

        Raises:
            ValueError: If entry_type is invalid
        """
        if entry_type not in self.VALID_ENTRY_TYPES:
            raise ValueError(
                f"Invalid entry_type: {entry_type}. "
                f"Must be one of {self.VALID_ENTRY_TYPES}"
            )

        # Get next sequence number for the session
        sequence_number = self._get_next_sequence_number(session_id)

        entry = ContinuityLedger(
            ledger_id=ledger_id or f"ledger-{uuid.uuid4().hex[:16]}",
            session_id=session_id,
            agent_id=agent_id,
            entry_type=entry_type,
            title=title,
            content=content,
            metadata=metadata,
            parent_entry_id=parent_entry_id,
            sequence_number=sequence_number,
            project_id=project_id,
            track_id=track_id,
        )

        self.session.add(entry)
        self.session.flush()

        return entry

    def _get_next_sequence_number(self, session_id: str) -> int:
        """
        Get the next sequence number for a session

        Args:
            session_id: Session ID

        Returns:
            Next sequence number
        """
        stmt = select(ContinuityLedger).where(
            ContinuityLedger.session_id == session_id
        ).order_by(desc(ContinuityLedger.sequence_number)).limit(1)

        last_entry = self.session.execute(stmt).scalar_one_or_none()

        if not last_entry:
            return 1

        return cast(int, last_entry.sequence_number) + 1

    def get_entry(self, ledger_id: str) -> Optional[ContinuityLedger]:
        """
        Get a ledger entry by ID

        Args:
            ledger_id: Ledger entry ID

        Returns:
            ContinuityLedger instance or None
        """
        return self.session.query(ContinuityLedger).filter(
            ContinuityLedger.ledger_id == ledger_id
        ).first()

    def get_entries_by_session(
        self,
        session_id: str,
        entry_type: Optional[EntryType] = None,
        limit: int = 100,
    ) -> List[ContinuityLedger]:
        """
        Get all entries for a session

        Args:
            session_id: Session ID
            entry_type: Optional entry type filter
            limit: Maximum results

        Returns:
            List of ContinuityLedger instances
        """
        stmt = select(ContinuityLedger).where(
            ContinuityLedger.session_id == session_id
        )

        if entry_type:
            stmt = stmt.where(ContinuityLedger.entry_type == entry_type)

        stmt = stmt.order_by(ContinuityLedger.sequence_number).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_entries_by_agent(
        self,
        agent_id: str,
        entry_type: Optional[EntryType] = None,
        limit: int = 100,
    ) -> List[ContinuityLedger]:
        """
        Get all entries for an agent

        Args:
            agent_id: Agent ID
            entry_type: Optional entry type filter
            limit: Maximum results

        Returns:
            List of ContinuityLedger instances
        """
        stmt = select(ContinuityLedger).where(
            ContinuityLedger.agent_id == agent_id
        )

        if entry_type:
            stmt = stmt.where(ContinuityLedger.entry_type == entry_type)

        stmt = stmt.order_by(ContinuityLedger.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_entries_by_type(
        self,
        entry_type: EntryType,
        project_id: Optional[int] = None,
        limit: int = 100,
    ) -> List[ContinuityLedger]:
        """
        Get all entries of a specific type

        Args:
            entry_type: Entry type
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of ContinuityLedger instances
        """
        stmt = select(ContinuityLedger).where(
            ContinuityLedger.entry_type == entry_type
        )

        if project_id:
            stmt = stmt.where(ContinuityLedger.project_id == project_id)

        stmt = stmt.order_by(ContinuityLedger.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_recent_entries(
        self,
        session_id: Optional[str] = None,
        project_id: Optional[int] = None,
        limit: int = 20,
    ) -> List[ContinuityLedger]:
        """
        Get recent ledger entries

        Args:
            session_id: Optional session filter
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of recent ContinuityLedger instances
        """
        stmt = select(ContinuityLedger)

        if session_id:
            stmt = stmt.where(ContinuityLedger.session_id == session_id)
        if project_id:
            stmt = stmt.where(ContinuityLedger.project_id == project_id)

        stmt = stmt.order_by(ContinuityLedger.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_entry_chain(
        self,
        ledger_id: str,
    ) -> List[ContinuityLedger]:
        """
        Get the chain of entries from root to the specified entry

        Args:
            ledger_id: Entry ID to get chain for

        Returns:
            List of ContinuityLedger instances in order
        """
        entry = self.get_entry(ledger_id)
        if not entry:
            return []

        chain: List[ContinuityLedger] = []
        current = entry
        while current:
            chain.insert(0, current)
            current = current.parent

        return chain

    def get_child_entries(
        self,
        ledger_id: str,
        limit: int = 50,
    ) -> List[ContinuityLedger]:
        """
        Get child entries of a parent entry

        Args:
            ledger_id: Parent entry ID
            limit: Maximum results

        Returns:
            List of child ContinuityLedger instances
        """
        stmt = select(ContinuityLedger).where(
            ContinuityLedger.parent_entry_id == ledger_id
        ).order_by(ContinuityLedger.sequence_number).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def search_entries(
        self,
        query: str,
        project_id: Optional[int] = None,
        limit: int = 20,
    ) -> List[ContinuityLedger]:
        """
        Search ledger entries by title or content

        Args:
            query: Search query
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of matching ContinuityLedger instances
        """
        stmt = select(ContinuityLedger).where(
            ContinuityLedger.title.contains(query)
        )

        if project_id:
            stmt = stmt.where(ContinuityLedger.project_id == project_id)

        stmt = stmt.order_by(ContinuityLedger.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def update_entry(
        self,
        ledger_id: str,
        title: Optional[str] = None,
        content: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Optional[ContinuityLedger]:
        """
        Update a ledger entry

        Args:
            ledger_id: Entry ID to update
            title: Optional new title
            content: Optional new content
            metadata: Optional new metadata (merges with existing)

        Returns:
            Updated ContinuityLedger instance or None
        """
        entry = self.get_entry(ledger_id)
        if not entry:
            return None

        if title is not None:
            entry.title = title  # type: ignore[assignment]
        if content is not None:
            entry.content = content  # type: ignore[assignment]
        if metadata is not None:
            entry_metadata = entry.get_metadata()
            existing: Dict[str, Any] = dict(entry_metadata) if entry_metadata else {}
            entry.set_metadata({**existing, **metadata})

        self.session.flush()
        return entry

    def delete_entry(self, ledger_id: str) -> bool:
        """
        Delete a ledger entry

        Note: This does not delete child entries. They will
        have their parent_entry_id set to NULL.

        Args:
            ledger_id: Entry ID to delete

        Returns:
            True if deleted, False if not found
        """
        entry = self.get_entry(ledger_id)
        if not entry:
            return False

        # Update child entries to remove parent reference
        children = self.get_child_entries(ledger_id)
        for child in children:
            child.parent_entry_id = None  # type: ignore[assignment]

        self.session.delete(entry)
        self.session.flush()
        return True

    def get_session_summary(
        self,
        session_id: str,
    ) -> Dict[str, Any]:
        """
        Get a summary of all entries for a session

        Args:
            session_id: Session ID

        Returns:
            Summary dictionary
        """
        entries = self.get_entries_by_session(session_id, limit=10000)

        # Count by type
        by_type: Dict[str, int] = {}
        for entry in entries:
            entry_type = str(entry.entry_type)
            by_type[entry_type] = by_type.get(entry_type, 0) + 1

        # Get time range
        if entries:
            first = entries[0].created_at
            last = entries[-1].created_at
            duration = (last - first).total_seconds() if first and last else 0
        else:
            first = None
            last = None
            duration = 0

        return {
            "session_id": session_id,
            "total_entries": len(entries),
            "by_type": by_type,
            "first_entry_at": first.isoformat() if first else None,
            "last_entry_at": last.isoformat() if last else None,
            "duration_seconds": duration,
        }

    def get_timeline(
        self,
        session_id: str,
        include_types: Optional[List[EntryType]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Get a timeline of entries for a session

        Args:
            session_id: Session ID
            include_types: Optional list of entry types to include

        Returns:
            List of timeline entry dictionaries
        """
        entries = self.get_entries_by_session(session_id, limit=10000)

        if include_types:
            entries = [e for e in entries if e.entry_type in include_types]

        return [
            {
                "sequence": e.sequence_number,
                "type": e.entry_type,
                "title": e.title,
                "content": e.content,
                "timestamp": e.created_at.isoformat() if e.created_at else None,
                "agent_id": e.agent_id,
                "parent_id": e.parent_entry_id,
            }
            for e in entries
        ]

    def get_decisions(
        self,
        session_id: Optional[str] = None,
        project_id: Optional[int] = None,
        limit: int = 50,
    ) -> List[ContinuityLedger]:
        """
        Get all decision entries

        Args:
            session_id: Optional session filter
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of decision ContinuityLedger instances
        """
        return self.get_entries_by_type(
            "decision",
            project_id=project_id,
        )[:limit]

    def get_statistics(
        self,
        project_id: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Get ledger statistics

        Args:
            project_id: Optional project filter

        Returns:
            Statistics dictionary
        """
        stmt = select(ContinuityLedger)
        if project_id:
            stmt = stmt.where(ContinuityLedger.project_id == project_id)

        all_entries = list(self.session.execute(stmt).scalars().all())

        # Count by type
        by_type: Dict[str, int] = {}
        for entry in all_entries:
            entry_type = str(entry.entry_type)
            by_type[entry_type] = by_type.get(entry_type, 0) + 1

        # Count by session
        by_session: Dict[str, int] = {}
        for entry in all_entries:
            session = str(entry.session_id)
            by_session[session] = by_session.get(session, 0) + 1

        # Count by agent
        by_agent: Dict[str, int] = {}
        for entry in all_entries:
            agent = str(entry.agent_id) if entry.agent_id else "unknown"
            by_agent[agent] = by_agent.get(agent, 0) + 1

        return {
            "total_entries": len(all_entries),
            "by_type": by_type,
            "by_session": by_session,
            "by_agent": by_agent,
            "unique_sessions": len(by_session),
            "unique_agents": len(by_agent),
        }


class LedgerBuilder:
    """
    Helper class for building structured ledger entries

    Provides convenience methods for creating different types of
    ledger entries with consistent formatting.
    """

    @staticmethod
    def decision_entry(
        title: str,
        decision: str,
        rationale: str,
        alternatives_considered: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        """
        Build a decision entry

        Args:
            title: Decision title
            decision: The decision made
            rationale: Rationale for the decision
            alternatives_considered: Alternative options considered

        Returns:
            Entry content dictionary
        """
        content = f"Decision: {decision}\n\nRationale: {rationale}"

        metadata = {
            "decision": decision,
            "rationale": rationale,
        }

        if alternatives_considered:
            metadata["alternatives_considered"] = alternatives_considered  # type: ignore[assignment]

        return {
            "title": title,
            "content": content,
            "metadata": metadata,
        }

    @staticmethod
    def action_entry(
        title: str,
        action: str,
        details: Optional[str] = None,
        files_modified: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        """
        Build an action entry

        Args:
            title: Action title
            action: Action taken
            details: Optional additional details
            files_modified: Optional list of files modified

        Returns:
            Entry content dictionary
        """
        content = f"Action: {action}"
        if details:
            content += f"\n\nDetails: {details}"

        metadata: Dict[str, Any] = {"action": action}
        if files_modified:
            metadata["files_modified"] = files_modified  # type: ignore[assignment]

        return {
            "title": title,
            "content": content,
            "metadata": metadata,
        }

    @staticmethod
    def outcome_entry(
        title: str,
        outcome: str,
        success: bool,
        metrics: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        """
        Build an outcome entry

        Args:
            title: Outcome title
            outcome: Description of outcome
            success: Whether the outcome was successful
            metrics: Optional metrics

        Returns:
            Entry content dictionary
        """
        status = "Success" if success else "Failure"
        content = f"Outcome: {status}\n\n{outcome}"

        metadata = {
            "outcome": outcome,
            "success": success,
        }

        if metrics:
            metadata["metrics"] = metrics

        return {
            "title": title,
            "content": content,
            "metadata": metadata,
        }

    @staticmethod
    def observation_entry(
        title: str,
        observation: str,
        context: Optional[str] = None,
        significance: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        Build an observation entry

        Args:
            title: Observation title
            observation: The observation
            context: Optional context
            significance: Optional significance level

        Returns:
            Entry content dictionary
        """
        content = f"Observation: {observation}"
        if context:
            content += f"\n\nContext: {context}"
        if significance:
            content += f"\n\nSignificance: {significance}"

        metadata = {"observation": observation}
        if significance:
            metadata["significance"] = significance

        return {
            "title": title,
            "content": content,
            "metadata": metadata,
        }
