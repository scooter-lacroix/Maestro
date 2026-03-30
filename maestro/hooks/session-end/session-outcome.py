#!/usr/bin/env python3
"""
Maestro Session-End Hook: Session Outcome

Captures session outcome summary when a session ends.
Stores final results and creates a session summary memory.
"""

import json
import sys
import os
import asyncio
from datetime import datetime, UTC
from pathlib import Path
from typing import Any, Optional

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))

def get_hook_manager(**kwargs: Any) -> Any:
    try:
        import importlib

        module = importlib.import_module("maestro.memory.hooks.unified")
        func = getattr(module, "get_hook_manager", None)
        if callable(func):
            return func(**kwargs)
    except Exception:
        return None
    return None


def session_outcome_hook(input_data: dict) -> dict:
    """
    Session-end hook that captures session outcome.

    Args:
        input_data: Hook input data containing session info

    Returns:
        Modified input data with outcome captured
    """
    try:
        session_id = input_data.get("session_id")
        agent_id = input_data.get("agent_id")
        agent_name = input_data.get("agent_name", agent_id)

        if not session_id:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        outcome = {
            "session_id": session_id,
            "agent_id": agent_id,
            "agent_name": agent_name,
            "captured_at": datetime.now(UTC).isoformat(),
        }

        # Get session details
        session = None
        if manager.session_manager:
            session = manager.session_manager.get_session_by_id(session_id)

        if session:
            outcome["started_at"] = session.started_at.isoformat() if session.started_at else None
            outcome["ended_at"] = session.ended_at.isoformat() if session.ended_at else None
            outcome["project_path"] = session.project_path

            # Calculate duration
            if session.started_at and session.ended_at:
                duration = (session.ended_at - session.started_at).total_seconds()
                outcome["duration_seconds"] = duration

        # Get memory count for this session from Nexus-backed truth.
        memory_count = 0
        try:
            from maestro.memory.service import MaestroMemoryService

            async def _load_session_memories() -> list[dict[str, Any]]:
                service = MaestroMemoryService()
                await service.initialize()
                try:
                    return await service.retrieve_session_context(session_id=session_id, limit=500)
                finally:
                    await service.close()

            memories = asyncio.run(_load_session_memories())
            memory_count = len(memories)
        except Exception:
            memory_count = 0

        outcome["memory_count"] = memory_count

        # Create outcome summary
        if memory_count > 0:
            summary = f"Session ended: {agent_name} completed with {memory_count} memories"
        else:
            summary = f"Session ended: {agent_name} completed"

        if session and session.project_path:
            project_name = Path(session.project_path).name
            summary += f" in {project_name}"

        # Store outcome in memory
        current_session_id = getattr(manager, '_current_session_id', None)
        if current_session_id or session_id:
            try:
                from maestro.memory.service import MaestroMemoryService

                async def _store_session_outcome() -> None:
                    service = MaestroMemoryService()
                    await service.initialize()
                    try:
                        await service.store_command_context(
                            command="hook:session-end",
                            project_path=session.project_path if session and session.project_path else os.getcwd(),
                            context={
                                "session_id": session_id,
                                "agent_id": agent_id,
                                "agent_name": agent_name,
                                "current_task_id": input_data.get("current_task_id", ""),
                                "event": "session_end",
                                "summary": summary,
                                "memory_count": memory_count,
                                "outcome": outcome,
                            },
                        )
                    finally:
                        await service.close()

                asyncio.run(_store_session_outcome())
            except Exception:
                # Session might already be closed
                pass

        # Create final ledger entry
        if current_session_id:
            try:
                manager.create_ledger_entry(
                    entry_type="session_outcome",
                    title=f"Session Outcome: {session_id}",
                    content=summary,
                    metadata=outcome,
                )
            except Exception:
                pass

        input_data["session_outcome_captured"] = outcome
        input_data["outcome_summary"] = summary

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = session_outcome_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
