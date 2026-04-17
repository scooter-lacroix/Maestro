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
    except Exception as e:
        sys.stderr.write(f"Error getting hook manager: {e}\n")
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

        # Import MaestroMemoryService early — used by both _async_operations and _persist_operations
        from maestro.memory.service import MaestroMemoryService

        # Combined async coordinator for all async operations
        # This consolidates all asyncio.run() calls into a single event loop execution
        async def _async_operations() -> tuple[int, str]:
            """Perform all async operations in one event loop."""
            service = MaestroMemoryService()
            await service.initialize()
            try:
                # Get memory count for this session from Nexus-backed truth
                memories = await service.retrieve_session_context(session_id=session_id, limit=500)
                return len(memories), ""
            except Exception as e:
                return 0, str(e)
            finally:
                await service.close()

        memory_count = 0
        memory_error = ""
        try:
            memory_count, memory_error = asyncio.run(_async_operations())
        except Exception as e:
            input_data["outcome_memory_load_error"] = str(e)
            memory_count = 0

        if memory_error:
            input_data["outcome_memory_load_error"] = memory_error

        outcome["memory_count"] = memory_count

        # Create outcome summary
        if memory_count > 0:
            summary = f"Session ended: {agent_name} completed with {memory_count} memories"
        else:
            summary = f"Session ended: {agent_name} completed"

        if session and session.project_path:
            project_name = Path(session.project_path).name
            summary += f" in {project_name}"

        # Store outcome in memory and create DB handoff in the same async context
        current_session_id = getattr(manager, '_current_session_id', None)
        if current_session_id or session_id:
            try:
                from maestro.memory.coordination.handoffs import HandoffHandler, HandoffTemplate

                async def _persist_operations() -> None:
                    """Store outcome and create handoff in a single async operation."""
                    # Part 1: Store outcome in Nexus
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

                    # Part 2: Create DB handoff in the same event loop
                    from maestro.memory.database.models import get_session_context
                    with get_session_context() as db_session:
                        handler = HandoffHandler(db_session)
                        track_id = input_data.get("track_id")
                        project_path = (
                            session.project_path if session and session.project_path else os.getcwd()
                        )
                        context = HandoffTemplate.generic_handoff(
                            title=f"Session handoff: {session_id[:8]}",
                            description=summary,
                            current_state={
                                "track_id": track_id,
                                "current_task": input_data.get("current_task_id"),
                                "iteration": input_data.get("iteration"),
                            },
                            achievements=input_data.get("achievements", []),
                            blockers=input_data.get("blockers", []),
                            action_items=input_data.get("action_items", []),
                            remaining_work=input_data.get("remaining_work", ""),
                        )
                        handler.create_handoff(
                            title=f"Session handoff: {session_id[:8]}",
                            from_session_id=session_id,
                            from_agent_id=agent_id,
                            project_path=project_path,
                            summary=summary,
                            context_data=context,
                        )

                asyncio.run(_persist_operations())
            except Exception as e:
                input_data["outcome_storage_error"] = str(e)

        # Create final ledger entry
        if current_session_id:
            try:
                manager.create_ledger_entry(
                    entry_type="session_outcome",
                    title=f"Session Outcome: {session_id}",
                    content=summary,
                    metadata=outcome,
                )
            except Exception as e:
                input_data["outcome_ledger_error"] = str(e)

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
