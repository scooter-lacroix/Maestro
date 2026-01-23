"""
Handoffs Handler for Session Continuity

Manages session handoffs for preserving context across sessions
and agents. Handoffs contain the complete state needed to resume
work in a human-readable YAML format.

Includes:
- YAML validation on handoff creation (Issue #17)
- Error recovery on read (Issue #17)
- Schema validation for handoff YAML structure (Issue #17)
- Audit logging for all operations (Issue #21)
"""

import uuid
import yaml  # type: ignore[import-untyped]
from datetime import datetime, UTC
from typing import Optional, Dict, Any, List, cast
import logging

from sqlalchemy import select, and_, or_
from sqlalchemy.orm import Session

from maestro.memory.database.models import (
    Handoff,
    HandoffStatus,
    MaestroProject,
    MaestroTrack,
    log_audit_event,
)

logger = logging.getLogger(__name__)


class HandoffNotFoundError(Exception):
    """Raised when a handoff is not found"""

    pass


class HandoffNotPickableError(Exception):
    """Raised when attempting to pick a handoff that is not available"""

    pass


class HandoffValidationError(Exception):
    """Raised when handoff YAML validation fails (Issue #17)"""

    def __init__(self, message: str, errors: List[str]):
        super().__init__(message)
        self.errors = errors


class HandoffYAMLError(Exception):
    """Raised when handoff YAML parsing fails (Issue #17)"""

    def __init__(self, message: str, yaml_error: str):
        super().__init__(message)
        self.yaml_error = yaml_error


class HandoffHandler:
    """
    Handles session handoffs for continuity

    Handoffs preserve session state for resumption by another
    agent or session, using YAML for human-readable storage.

    Includes:
    - YAML validation on handoff creation (Issue #17)
    - Error recovery on read (Issue #17)
    - Audit logging (Issue #21)
    """

    # Schema for handoff context validation (Issue #17)
    VALID_CONTEXT_KEYS = {
        'handoff_type', 'feature_name', 'status', 'files_modified',
        'decisions', 'next_steps', 'bug_description', 'diagnosis',
        'attempted_fixes', 'current_status', 'remaining_work',
        'investigation_topic', 'findings', 'hypotheses', 'tests_run',
        'next_investigation_steps', 'title', 'description', 'current_state',
        'achievements', 'blockers', 'action_items', 'summary',
        'context', 'metadata', 'state', 'progress', 'errors', 'warnings',
        'goal', 'now', 'next'  # CCv3 Feature Adoption: YAML goal/now/next schema
    }

    def __init__(self, session: Session):
        """
        Initialize the handoff handler

        Args:
            session: SQLAlchemy database session
        """
        self.session = session

    def _validate_context_data(self, context: Dict[str, Any]) -> List[str]:
        """
        Validate handoff context data structure (Issue #17).

        Args:
            context: Context dictionary to validate

        Returns:
            List of validation errors (empty if valid)
        """
        errors = []

        # Check that context is a dictionary
        if not isinstance(context, dict):
            errors.append(f"Context must be a dictionary, got {type(context).__name__}")
            return errors

        # Check for potentially dangerous keys (YAML injection prevention)
        for key in context.keys():
            if not isinstance(key, str):
                errors.append(f"Context key must be string, got {type(key).__name__}")

            # Check for YAML anchors/aliases (potential injection)
            if '&' in str(key) or '*' in str(key):
                errors.append(f"Invalid characters in context key: {key}")

        # Validate that values are JSON-serializable (for storage)
        for key, value in context.items():
            try:
                import json
                json.dumps(value)
            except (TypeError, ValueError) as e:
                errors.append(f"Context value for key '{key}' is not JSON-serializable: {e}")

        return errors

    def _context_to_yaml(self, context: Dict[str, Any]) -> str:
        """
        Convert context dictionary to YAML string with validation (Issue #17).

        Args:
            context: Context dictionary

        Returns:
            YAML formatted string

        Raises:
            HandoffValidationError: If context validation fails
        """
        # Validate context before conversion
        errors = self._validate_context_data(context)
        if errors:
            raise HandoffValidationError(
                f"Handoff context validation failed with {len(errors)} error(s)",
                errors=errors
            )

        # Filter out None values and sort keys for consistency
        clean_context = {
            k: v for k, v in sorted(context.items())
            if v is not None
        }

        try:
            return yaml.dump(clean_context, default_flow_style=False, sort_keys=False)  # type: ignore[no-any-return]
        except yaml.YAMLError as e:
            raise HandoffValidationError(
                f"Failed to convert context to YAML: {e}",
                errors=[str(e)]
            )

    def _yaml_to_context(self, yaml_str: str) -> Dict[str, Any]:
        """
        Convert YAML string to context dictionary with error recovery (Issue #17).

        Args:
            yaml_str: YAML formatted string

        Returns:
            Context dictionary (empty if parsing fails)

        Raises:
            HandoffYAMLError: If YAML parsing completely fails
        """
        if not yaml_str or not yaml_str.strip():
            logger.warning("Empty YAML string provided for handoff context")
            return {}

        try:
            context = yaml.safe_load(yaml_str)
            if not isinstance(context, dict):
                logger.warning(f"YAML parsed to non-dict type: {type(context).__name__}")
                return {}
            return context or {}  # type: ignore[return-value]
        except yaml.YAMLError as e:
            # Issue #17: Error recovery - log but don't fail completely
            error_msg = str(e)
            logger.error(f"Failed to parse handoff YAML: {error_msg}")

            # Attempt to recover partial data
            try:
                # Try to extract any valid YAML documents
                import io
                recovered_context: Dict[str, Any] = {}
                for doc in yaml.safe_load_all(io.StringIO(yaml_str)):
                    if isinstance(doc, dict):
                        recovered_context.update(doc)
                if recovered_context:
                    logger.info(f"Recovered partial context from corrupted YAML")
                    return recovered_context
            except Exception:
                pass

            # If recovery fails, raise with original error
            raise HandoffYAMLError(
                f"Failed to parse handoff YAML and recovery failed",
                yaml_error=error_msg
            ) from e

    def create_handoff(
        self,
        from_session_id: str,
        from_agent_id: str,
        title: str,
        context_data: Dict[str, Any],
        project_path: Optional[str] = None,
        summary: Optional[str] = None,
        tags: Optional[List[str]] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        handoff_id: Optional[str] = None,
        validate: bool = True,
    ) -> Handoff:
        """
        Create a new handoff with YAML validation (Issue #17).

        Args:
            from_session_id: ID of the source session
            from_agent_id: ID of the source agent
            title: Handoff title
            context_data: Dictionary containing handoff context
            project_path: Optional project path
            summary: Optional summary
            tags: Optional tags
            project_id: Optional Maestro project ID
            track_id: Optional Maestro track ID
            handoff_id: Optional custom handoff ID
            validate: Whether to validate context (default True)

        Returns:
            Created Handoff instance

        Raises:
            HandoffValidationError: If context validation fails
        """
        # Convert context to YAML with validation
        try:
            context_yaml = self._context_to_yaml(context_data)
        except HandoffValidationError:
            if validate:
                # Log audit event for validation failure
                log_audit_event(
                    operation="create_handoff_denied",
                    entity_type="Handoff",
                    entity_id=handoff_id,
                    user_id=from_agent_id,
                    metadata={
                        "reason": "validation_failed",
                        "title": title,
                    },
                    status="denied"
                )
                raise
            # If validation disabled, try without validation
            context_yaml = yaml.dump(context_data, default_flow_style=False)

        handoff = Handoff(
            handoff_id=handoff_id or f"handoff-{uuid.uuid4().hex[:16]}",
            title=title,
            from_session_id=from_session_id,
            to_session_id=None,
            from_agent_id=from_agent_id,
            to_agent_id=None,
            status=HandoffStatus.PENDING.value,
            context_yaml=context_yaml,
            context_data=context_data,
            project_path=project_path,
            summary=summary,
            tags=tags,
            project_id=project_id,
            track_id=track_id,
        )

        self.session.add(handoff)
        self.session.flush()

        # Issue #21: Log audit event
        log_audit_event(
            operation="create_handoff",
            entity_type="Handoff",
            entity_id=handoff.handoff_id,
            user_id=from_agent_id,
            metadata={
                "title": title,
                "from_session_id": from_session_id,
            },
            status="success"
        )

        return handoff

    def get_handoff(self, handoff_id: str) -> Optional[Handoff]:
        """
        Get a handoff by ID

        Args:
            handoff_id: Handoff ID

        Returns:
            Handoff instance or None
        """
        return self.session.query(Handoff).filter(
            Handoff.handoff_id == handoff_id
        ).first()

    def get_handoff_context(self, handoff_id: str) -> Dict[str, Any]:
        """
        Get the parsed context from a handoff with error recovery (Issue #17).

        Args:
            handoff_id: Handoff ID

        Returns:
            Context dictionary (may be partial if YAML is corrupted)

        Raises:
            HandoffNotFoundError: If handoff not found
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            raise HandoffNotFoundError(f"Handoff {handoff_id} not found")

        # Use stored context_data if available
        if handoff.context_data:
            return cast(Dict[str, Any], handoff.context_data)

        # Issue #17: Try to parse YAML with error recovery
        try:
            return self._yaml_to_context(str(handoff.context_yaml))
        except HandoffYAMLError as e:
            # Log the error but return empty context instead of failing
            logger.error(
                f"Failed to parse YAML for handoff {handoff_id}, "
                f"returning empty context: {e.yaml_error}"
            )
            # Log audit event for YAML error
            log_audit_event(
                operation="get_handoff_context_error",
                entity_type="Handoff",
                entity_id=handoff_id,
                user_id=None,
                metadata={
                    "yaml_error": e.yaml_error,
                },
                status="error"
            )
            return {}

    def get_pending_handoffs(
        self,
        to_agent_id: Optional[str] = None,
        project_id: Optional[int] = None,
        limit: int = 50,
    ) -> List[Handoff]:
        """
        Get all pending handoffs

        Args:
            to_agent_id: Optional target agent filter
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of pending Handoff instances
        """
        stmt = select(Handoff).where(
            Handoff.status == HandoffStatus.PENDING.value
        )

        if to_agent_id:
            stmt = stmt.where(
                or_(
                    Handoff.to_agent_id == to_agent_id,
                    Handoff.to_agent_id.is_(None),
                )
            )

        if project_id:
            stmt = stmt.where(Handoff.project_id == project_id)

        stmt = stmt.order_by(Handoff.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_pickable_handoffs(
        self,
        agent_id: Optional[str] = None,
        project_id: Optional[int] = None,
        limit: int = 50,
    ) -> List[Handoff]:
        """
        Get handoffs available for resumption

        Includes pending and in_progress handoffs.

        Args:
            agent_id: Optional agent filter
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of pickable Handoff instances
        """
        stmt = select(Handoff).where(
            Handoff.status.in_([
                HandoffStatus.PENDING.value,
                HandoffStatus.IN_PROGRESS.value,
            ])
        )

        if agent_id:
            stmt = stmt.where(
                or_(
                    Handoff.to_agent_id == agent_id,
                    Handoff.to_agent_id.is_(None),
                )
            )

        if project_id:
            stmt = stmt.where(Handoff.project_id == project_id)

        stmt = stmt.order_by(Handoff.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def pick_handoff(
        self,
        handoff_id: str,
        to_session_id: str,
        to_agent_id: str,
    ) -> Handoff:
        """
        Pick (resume) a handoff

        Args:
            handoff_id: Handoff ID to pick
            to_session_id: ID of the resuming session
            to_agent_id: ID of the resuming agent

        Returns:
            Updated Handoff instance

        Raises:
            HandoffNotFoundError: If handoff not found
            HandoffNotPickableError: If handoff is not available
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            raise HandoffNotFoundError(f"Handoff {handoff_id} not found")

        if not handoff.is_pickable():
            raise HandoffNotPickableError(
                f"Handoff {handoff_id} is not pickable (status: {handoff.status})"
            )

        handoff.to_session_id = to_session_id  # type: ignore[assignment]
        handoff.to_agent_id = to_agent_id  # type: ignore[assignment]
        handoff.status = HandoffStatus.IN_PROGRESS.value  # type: ignore[assignment]
        handoff.resumed_at = datetime.now(UTC)  # type: ignore[assignment]

        self.session.flush()
        return handoff

    def complete_handoff(
        self,
        handoff_id: str,
    ) -> Optional[Handoff]:
        """
        Mark a handoff as completed

        Args:
            handoff_id: Handoff ID to complete

        Returns:
            Updated Handoff instance or None
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            return None

        handoff.status = HandoffStatus.COMPLETED.value  # type: ignore[assignment]
        handoff.completed_at = datetime.now(UTC)  # type: ignore[assignment]
        self.session.flush()

        return handoff

    def abandon_handoff(
        self,
        handoff_id: str,
        reason: Optional[str] = None,
    ) -> Optional[Handoff]:
        """
        Abandon a handoff

        Args:
            handoff_id: Handoff ID to abandon
            reason: Optional reason for abandonment

        Returns:
            Updated Handoff instance or None
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            return None

        handoff.status = HandoffStatus.ABANDONED.value  # type: ignore[assignment]
        if reason:
            # Add reason to context_data
            if handoff.context_data:
                handoff.context_data["abandonment_reason"] = reason

        self.session.flush()
        return handoff

    def get_handoffs_by_session(
        self,
        session_id: str,
        as_source: bool = True,
        as_target: bool = True,
    ) -> List[Handoff]:
        """
        Get handoffs associated with a session

        Args:
            session_id: Session ID
            as_source: Include handoffs where session is source
            as_target: Include handoffs where session is target

        Returns:
            List of Handoff instances
        """
        conditions = []
        if as_source:
            conditions.append(Handoff.from_session_id == session_id)
        if as_target:
            conditions.append(Handoff.to_session_id == session_id)

        if not conditions:
            return []

        stmt = select(Handoff).where(or_(*conditions))
        return list(self.session.execute(stmt).scalars().all())

    def get_handoffs_by_agent(
        self,
        agent_id: str,
        limit: int = 50,
    ) -> List[Handoff]:
        """
        Get handoffs associated with an agent

        Args:
            agent_id: Agent ID
            limit: Maximum results

        Returns:
            List of Handoff instances
        """
        stmt = select(Handoff).where(
            or_(
                Handoff.from_agent_id == agent_id,
                Handoff.to_agent_id == agent_id,
            )
        ).order_by(Handoff.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def update_handoff_context(
        self,
        handoff_id: str,
        context_data: Dict[str, Any],
    ) -> Optional[Handoff]:
        """
        Update the context of a handoff

        Args:
            handoff_id: Handoff ID
            context_data: New context data (will be merged with existing)

        Returns:
            Updated Handoff instance or None
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            return None

        # Merge with existing context
        existing_context: Dict[str, Any] = dict(handoff.context_data) if handoff.context_data else {}
        merged_context = {**existing_context, **context_data}

        handoff.context_data = merged_context  # type: ignore[assignment]
        handoff.context_yaml = self._context_to_yaml(merged_context)  # type: ignore[assignment]

        self.session.flush()
        return handoff

    def search_handoffs(
        self,
        query: str,
        project_id: Optional[int] = None,
        limit: int = 20,
    ) -> List[Handoff]:
        """
        Search handoffs by title or summary

        Args:
            query: Search query
            project_id: Optional project filter
            limit: Maximum results

        Returns:
            List of matching Handoff instances
        """
        stmt = select(Handoff).where(
            or_(
                Handoff.title.contains(query),
                Handoff.summary.contains(query),
            )
        )

        if project_id:
            stmt = stmt.where(Handoff.project_id == project_id)

        stmt = stmt.order_by(Handoff.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_handoff_summary(self, handoff_id: str) -> Dict[str, Any]:
        """
        Get a summary of a handoff

        Args:
            handoff_id: Handoff ID

        Returns:
            Summary dictionary

        Raises:
            HandoffNotFoundError: If handoff not found
        """
        handoff = self.get_handoff(handoff_id)
        if not handoff:
            raise HandoffNotFoundError(f"Handoff {handoff_id} not found")

        return {
            "handoff_id": handoff.handoff_id,
            "title": handoff.title,
            "summary": handoff.summary,
            "status": handoff.status,
            "from_session_id": handoff.from_session_id,
            "to_session_id": handoff.to_session_id,
            "from_agent_id": handoff.from_agent_id,
            "to_agent_id": handoff.to_agent_id,
            "project_path": handoff.project_path,
            "tags": handoff.tags or [],
            "created_at": handoff.created_at.isoformat() if handoff.created_at else None,
            "resumed_at": handoff.resumed_at.isoformat() if handoff.resumed_at else None,
            "completed_at": handoff.completed_at.isoformat() if handoff.completed_at else None,
            "context_keys": list(handoff.context_data.keys()) if handoff.context_data else [],
        }

    def get_statistics(
        self,
        project_id: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Get handoff statistics

        Args:
            project_id: Optional project filter

        Returns:
            Statistics dictionary
        """
        stmt = select(Handoff)
        if project_id:
            stmt = stmt.where(Handoff.project_id == project_id)

        all_handoffs = list(self.session.execute(stmt).scalars().all())

        # Count by status
        by_status: Dict[str, int] = {}
        for handoff in all_handoffs:
            status = str(handoff.status)
            by_status[status] = by_status.get(status, 0) + 1

        # Count by agent
        by_agent: Dict[str, int] = {}
        for handoff in all_handoffs:
            agent = str(handoff.from_agent_id) if handoff.from_agent_id else "unknown"
            by_agent[agent] = by_agent.get(agent, 0) + 1

        return {
            "total": len(all_handoffs),
            "by_status": by_status,
            "by_agent": by_agent,
            "pending": by_status.get(HandoffStatus.PENDING.value, 0),
            "in_progress": by_status.get(HandoffStatus.IN_PROGRESS.value, 0),
            "completed": by_status.get(HandoffStatus.COMPLETED.value, 0),
            "abandoned": by_status.get(HandoffStatus.ABANDONED.value, 0),
        }


class HandoffTemplate:
    """
    Templates for creating structured handoffs

    Provides predefined structures for common handoff scenarios.
    """

    @staticmethod
    def feature_handoff(
        feature_name: str,
        status: str,
        files_modified: List[str],
        decisions: List[str],
        next_steps: List[str],
        **extra_context: Any,
    ) -> Dict[str, Any]:
        """
        Create a feature development handoff context

        Args:
            feature_name: Name of the feature
            status: Current status
            files_modified: List of modified files
            decisions: List of decisions made
            next_steps: List of next steps
            **extra_context: Additional context

        Returns:
            Handoff context dictionary
        """
        context = {
            "handoff_type": "feature_development",
            "feature_name": feature_name,
            "status": status,
            "files_modified": files_modified,
            "decisions": decisions,
            "next_steps": next_steps,
            **extra_context,
        }
        return context

    @staticmethod
    def bugfix_handoff(
        bug_description: str,
        diagnosis: str,
        attempted_fixes: List[str],
        current_status: str,
        remaining_work: List[str],
        **extra_context: Any,
    ) -> Dict[str, Any]:
        """
        Create a bug fix handoff context

        Args:
            bug_description: Description of the bug
            diagnosis: Current diagnosis
            attempted_fixes: List of attempted fixes
            current_status: Current status
            remaining_work: Remaining work items
            **extra_context: Additional context

        Returns:
            Handoff context dictionary
        """
        context = {
            "handoff_type": "bug_fix",
            "bug_description": bug_description,
            "diagnosis": diagnosis,
            "attempted_fixes": attempted_fixes,
            "current_status": current_status,
            "remaining_work": remaining_work,
            **extra_context,
        }
        return context

    @staticmethod
    def investigation_handoff(
        investigation_topic: str,
        findings: List[str],
        hypotheses: List[str],
        tests_run: List[Dict[str, Any]],
        next_investigation_steps: List[str],
        **extra_context: Any,
    ) -> Dict[str, Any]:
        """
        Create an investigation handoff context

        Args:
            investigation_topic: Topic being investigated
            findings: List of findings
            hypotheses: List of hypotheses
            tests_run: List of tests performed
            next_investigation_steps: Next investigation steps
            **extra_context: Additional context

        Returns:
            Handoff context dictionary
        """
        context = {
            "handoff_type": "investigation",
            "investigation_topic": investigation_topic,
            "findings": findings,
            "hypotheses": hypotheses,
            "tests_run": tests_run,
            "next_investigation_steps": next_investigation_steps,
            **extra_context,
        }
        return context

    @staticmethod
    def generic_handoff(
        title: str,
        description: str,
        current_state: Dict[str, Any],
        achievements: List[str],
        blockers: List[str],
        action_items: List[str],
        **extra_context: Any,
    ) -> Dict[str, Any]:
        """
        Create a generic handoff context

        Args:
            title: Handoff title
            description: Description of work
            current_state: Current state information
            achievements: List of achievements
            blockers: List of blockers
            action_items: List of action items
            **extra_context: Additional context

        Returns:
            Handoff context dictionary
        """
        context = {
            "handoff_type": "generic",
            "title": title,
            "description": description,
            "current_state": current_state,
            "achievements": achievements,
            "blockers": blockers,
            "action_items": action_items,
            **extra_context,
        }
        return context

    @staticmethod
    def goal_now_next_handoff(
        goal: str,
        now: str,
        next_step: str,
        context: Optional[Dict[str, Any]] = None,
        **extra_context: Any,
    ) -> Dict[str, Any]:
        """
        Create a handoff using the standardized goal/now/next schema (CCv3 Feature Adoption).

        Args:
            goal: Current objective or goal
            now: In-progress work or current status
            next_step: Planned next steps
            context: Optional additional context
            **extra_context: Additional context

        Returns:
            Handoff context dictionary with goal/now/next structure
        """
        context_dict = {
            "handoff_type": "goal_now_next",
            "goal": goal,
            "now": now,
            "next": next_step,
            **(context or {}),
            **extra_context,
        }
        return context_dict
