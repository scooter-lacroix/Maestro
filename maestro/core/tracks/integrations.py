"""
Maestro Track Integrations

Integrates tracks with memory system components including
handoffs and TLDR code analysis.
"""

from datetime import datetime, UTC
from typing import Optional, Dict, Any, List
import uuid

from maestro.memory.coordination.handoffs import (
    HandoffHandler,
    HandoffTemplate,
)
from maestro.core.tracks.repository import TrackRepository
from maestro.core.tracks.models import TrackManager, TrackStatus


class TrackHandoffIntegration:
    """
    Integrates tracks with handoff system

    Creates handoffs when tracks are paused, resumes from
    handoffs when continuing work, and maintains handoff
    context in track metadata.
    """

    def __init__(
        self,
        track_repository: TrackRepository,
        track_manager: TrackManager,
        db_session: Any,
    ) -> None:
        """
        Initialize track handoff integration

        Args:
            track_repository: Track file repository
            track_manager: Track database manager
            db_session: SQLAlchemy database session
        """
        self.repository = track_repository
        self.manager = track_manager
        self.db_session = db_session
        self.handler = HandoffHandler(db_session)

    def create_pause_handoff(
        self,
        track_id: str,
        session_id: str,
        agent_id: str,
        current_task: str,
        completed_tasks: List[str],
        next_steps: List[str],
        files_modified: List[str],
        notes: Optional[str] = None,
    ) -> str:
        """
        Create a handoff when pausing track work

        Args:
            track_id: Track identifier
            session_id: Current session ID
            agent_id: Current agent ID
            current_task: Task being worked on
            completed_tasks: List of completed tasks
            next_steps: List of next steps to take
            files_modified: List of modified files
            notes: Optional additional notes

        Returns:
            Created handoff ID
        """
        # Load track metadata
        metadata = self.repository.load_metadata(track_id)

        # Get database IDs
        project_id = metadata.maestro_project_id or self.manager.get_or_create_project()
        track_db_id = metadata.maestro_track_id or self.manager.get_or_create_track(
            track_id, track_id
        )

        # Load spec for context
        try:
            spec_content = self.repository.load_spec(track_id)
        except FileNotFoundError:
            spec_content = ""

        # Create handoff context
        handoff_context = HandoffTemplate.feature_handoff(
            feature_name=track_id,
            status="in_progress",
            files_modified=files_modified,
            decisions=[],
            next_steps=next_steps,
            current_task=current_task,
            completed_tasks=completed_tasks,
            notes=notes,
            spec_summary=spec_content[:500] if spec_content else "",
            paused_at=datetime.now(UTC).isoformat(),
        )

        # Create handoff
        handoff = self.handler.create_handoff(
            from_session_id=session_id,
            from_agent_id=agent_id,
            title=f"Paused Track: {track_id}",
            context_data=handoff_context,
            project_path=self.manager.project_path,
            summary=f"Track {track_id} paused during: {current_task}",
            tags=["track", "paused", track_id],
            project_id=project_id,
            track_id=track_db_id,
        )

        # Update track metadata with handoff reference
        handoff_id_str: str = handoff.handoff_id  # type: ignore[assignment]
        self.repository.update_metadata(
            track_id,
            status="paused",
            current_handoff_id=handoff_id_str,
        )

        # Update track status in database
        self.manager.update_track_status(track_id, TrackStatus.PAUSED.value)

        return str(handoff.handoff_id)

    def resume_from_handoff(
        self,
        track_id: str,
        handoff_id: str,
        session_id: str,
        agent_id: str,
    ) -> Dict[str, Any]:
        """
        Resume track work from a handoff

        Args:
            track_id: Track identifier
            handoff_id: Handoff ID to resume from
            session_id: New session ID
            agent_id: Resuming agent ID

        Returns:
            Handoff context dictionary

        Raises:
            Exception: If handoff cannot be resumed
        """
        # Pick the handoff
        handoff = self.handler.pick_handoff(handoff_id, session_id, agent_id)

        # Update track metadata
        self.repository.update_metadata(
            track_id,
            status="in_progress",
            current_handoff_id=None,
        )

        # Update track status in database
        self.manager.update_track_status(track_id, TrackStatus.IN_PROGRESS.value)

        # Store memory about resumption
        self.manager.store_track_memory(
            track_id,
            f"Resumed track from handoff {handoff_id}. Agent: {agent_id}",
            category="context",
            importance="high",
            summary=f"Track {track_id} resumed by {agent_id}",
        )

        return self.handler.get_handoff_context(handoff_id)

    def get_pending_handoffs(
        self,
        track_id: str,
        limit: int = 10,
    ) -> List[Dict[str, Any]]:
        """
        Get pending handoffs for a track

        Args:
            track_id: Track identifier
            limit: Maximum results

        Returns:
            List of handoff summary dictionaries
        """
        metadata = self.repository.load_metadata(track_id)
        track_db_id = metadata.maestro_track_id

        if not track_db_id:
            return []

        handoffs = self.handler.get_pickable_handoffs(
            project_id=metadata.maestro_project_id,
            limit=limit,
        )

        # Filter to this track
        track_handoffs = [
            h for h in handoffs
            if h.track_id == track_db_id
        ]

        return [
            {
                "handoff_id": h.handoff_id,
                "title": h.title,
                "summary": h.summary,
                "status": h.status,
                "created_at": h.created_at.isoformat() if h.created_at else None,
            }
            for h in track_handoffs
        ]

    def complete_track_with_handoff(
        self,
        track_id: str,
        session_id: str,
        agent_id: str,
        completion_summary: str,
        achievements: List[str],
        files_modified: List[str],
    ) -> str:
        """
        Complete a track and create a completion handoff

        Args:
            track_id: Track identifier
            session_id: Session ID
            agent_id: Agent ID
            completion_summary: Summary of completed work
            achievements: List of achievements
            files_modified: List of modified files

        Returns:
            Created handoff ID
        """
        metadata = self.repository.load_metadata(track_id)

        project_id = metadata.maestro_project_id or self.manager.get_or_create_project()
        track_db_id = metadata.maestro_track_id or self.manager.get_or_create_track(
            track_id, track_id
        )

        # Create completion handoff
        handoff_context = HandoffTemplate.generic_handoff(
            title=f"Completed Track: {track_id}",
            description=completion_summary,
            current_state={"status": "completed"},
            achievements=achievements,
            blockers=[],
            action_items=[],
            files_modified=files_modified,
            completed_at=datetime.now(UTC).isoformat(),
        )

        handoff = self.handler.create_handoff(
            from_session_id=session_id,
            from_agent_id=agent_id,
            title=f"Completed Track: {track_id}",
            context_data=handoff_context,
            project_path=self.manager.project_path,
            summary=completion_summary,
            tags=["track", "completed", track_id],
            project_id=project_id,
            track_id=track_db_id,
        )

        # Mark handoff as completed
        handoff_id_str: str = handoff.handoff_id  # type: ignore[assignment]
        self.handler.complete_handoff(handoff_id_str)

        # Update track metadata
        self.repository.update_metadata(
            track_id,
            status="completed",
        )

        # Update track status in database
        self.manager.update_track_status(track_id, TrackStatus.COMPLETED.value)

        # Store completion memory
        self.manager.store_track_memory(
            track_id,
            completion_summary,
            category="context",
            importance="high",
            summary=f"Track {track_id} completed. {len(achievements)} achievements.",
        )

        return str(handoff.handoff_id)


class TrackTldrIntegration:
    """
    Integrates tracks with TLDR code analysis

    Stores TLDR analysis results with tracks and enables
    code context retrieval for track work.
    """

    def __init__(
        self,
        track_repository: TrackRepository,
        track_manager: TrackManager,
        db_session: Any,
    ) -> None:
        """
        Initialize track TLDR integration

        Args:
            track_repository: Track file repository
            track_manager: Track database manager
            db_session: SQLAlchemy database session
        """
        self.repository = track_repository
        self.manager = track_manager
        self.db_session = db_session

    def store_tldr_analysis(
        self,
        track_id: str,
        analysis_id: str,
        files_analyzed: List[str],
        findings: Dict[str, Any],
    ) -> None:
        """
        Store TLDR analysis results with a track

        Args:
            track_id: Track identifier
            analysis_id: Unique analysis identifier
            files_analyzed: List of analyzed files
            findings: Analysis findings dictionary
        """
        # Update track metadata with TLDR reference
        self.repository.update_metadata(
            track_id,
            tldr_analysis_id=analysis_id,
        )

        # Store as memory
        findings_summary = self._format_findings_summary(findings, files_analyzed)

        self.manager.store_track_memory(
            track_id,
            findings_summary,
            category="pattern",
            importance="normal",
            summary=f"TLDR analysis {analysis_id}: {len(files_analyzed)} files analyzed",
        )

    def get_code_context(
        self,
        track_id: str,
        file_path: str,
        context_type: str = "structure",
    ) -> Dict[str, Any]:
        """
        Retrieve code context for a track

        Args:
            track_id: Track identifier
            file_path: Path to file
            context_type: Type of context (structure, callgraph, cfg, dfg)

        Returns:
            Context dictionary
        """
        # Get track memories related to code analysis
        memories = self.manager.get_track_memories(track_id, limit=100)

        # Filter for TLDR-related memories
        code_memories = [
            m for m in memories
            if m.category in ("pattern", "context") and
            ("analysis" in m.content.lower() or "tldr" in m.content.lower())
        ]

        return {
            "track_id": track_id,
            "file_path": file_path,
            "context_type": context_type,
            "available_analyses": len(code_memories),
            "recent_findings": [
                {
                    "content": m.content[:300],
                    "category": m.category,
                    "created_at": m.created_at.isoformat() if m.created_at else None,
                }
                for m in code_memories[:5]
            ],
        }

    def _format_findings_summary(
        self,
        findings: Dict[str, Any],
        files_analyzed: List[str],
    ) -> str:
        """
        Format findings into a readable summary

        Args:
            findings: Analysis findings
            files_analyzed: List of analyzed files

        Returns:
            Formatted summary string
        """
        summary_parts = [
            f"TLDR Code Analysis Summary",
            f"Files analyzed: {len(files_analyzed)}",
            "",
        ]

        if "structures" in findings:
            summary_parts.append("Structures:")
            summary_parts.append(f"- {findings['structures']}")

        if "patterns" in findings:
            summary_parts.append("Patterns:")
            for pattern in findings["patterns"][:10]:
                summary_parts.append(f"- {pattern}")

        if "issues" in findings:
            summary_parts.append("Issues:")
            for issue in findings["issues"][:10]:
                summary_parts.append(f"- {issue}")

        return "\n".join(summary_parts)

    def create_analysis_memory(
        self,
        track_id: str,
        file_path: str,
        analysis_type: str,
        analysis_result: Dict[str, Any],
    ) -> None:
        """
        Create a memory entry for code analysis

        Args:
            track_id: Track identifier
            file_path: Path to analyzed file
            analysis_type: Type of analysis (ast, cfg, dfg, etc.)
            analysis_result: Analysis result dictionary
        """
        content = self._format_analysis_result(
            file_path,
            analysis_type,
            analysis_result,
        )

        self.manager.store_track_memory(
            track_id,
            content,
            category="pattern",
            importance="normal",
            summary=f"{analysis_type.upper()} analysis for {file_path}",
        )

    def _format_analysis_result(
        self,
        file_path: str,
        analysis_type: str,
        result: Dict[str, Any],
    ) -> str:
        """
        Format analysis result into readable content

        Args:
            file_path: Path to analyzed file
            analysis_type: Type of analysis
            result: Analysis result

        Returns:
            Formatted content string
        """
        lines = [
            f"Code Analysis: {analysis_type.upper()}",
            f"File: {file_path}",
            "",
        ]

        if "functions" in result:
            lines.append("Functions:")
            for func in result["functions"][:20]:
                lines.append(f"  - {func}")

        if "classes" in result:
            lines.append("Classes:")
            for cls in result["classes"][:20]:
                lines.append(f"  - {cls}")

        if "complexity" in result:
            lines.append(f"Complexity: {result['complexity']}")

        if "issues" in result:
            lines.append("Issues:")
            for issue in result["issues"][:10]:
                lines.append(f"  - {issue}")

        return "\n".join(lines)

    def get_track_code_insights(
        self,
        track_id: str,
        limit: int = 50,
    ) -> Dict[str, Any]:
        """
        Get all code insights for a track

        Args:
            track_id: Track identifier
            limit: Maximum results

        Returns:
            Dictionary of code insights
        """
        memories = self.manager.get_track_memories(track_id, limit=limit)

        # Group by category
        by_category: Dict[str, List[Dict[str, Any]]] = {}
        for memory in memories:
            category = str(memory.category)
            if category not in by_category:
                by_category[category] = []
            by_category[category].append({
                "content": str(memory.content)[:500],
                "summary": str(memory.summary) if memory.summary else None,
                "created_at": memory.created_at.isoformat() if memory.created_at else None,
            })

        return {
            "track_id": track_id,
            "total_insights": len(memories),
            "by_category": by_category,
        }
