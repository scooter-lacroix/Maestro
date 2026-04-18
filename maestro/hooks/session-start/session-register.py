#!/usr/bin/env python3
"""
Maestro Session Start Hook: Session Register

Registers the current session with the memory system.
Creates a new session record and initializes tracking.
"""

import json
import sys
import os
import asyncio
from datetime import datetime, UTC
from pathlib import Path
from typing import Any

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
        import sys
        sys.stderr.write(f"Error getting hook manager: {e}\n")
        return None
    return None


def session_register_hook(input_data: dict) -> dict:
    """
    Session start hook that registers the session.

    Args:
        input_data: Hook input data containing:
            - session_id: Current session identifier
            - agent_id: Agent identifier
            - agent_name: Agent display name
            - project_path: Project directory path

    Returns:
        Modified input data with session registration info
    """
    try:
        session_id = input_data.get("session_id")
        agent_id = input_data.get("agent_id")
        agent_name = input_data.get("agent_name", agent_id)
        project_path = input_data.get("project_path")

        if not session_id or not agent_id:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        # Create session using session_manager
        project_id = None
        if project_path:
            project = manager.project_manager.get_or_create_project(project_path)
            project_id = project.id

        session = manager.session_manager.create_session(
            session_id=session_id,
            session_type="agent",
            agent_id=agent_id,
            agent_name=agent_name,
            project_path=project_path,
            project_id=project_id,
        )

        # Set as current session
        manager._current_session_id = session_id
        manager._current_agent_id = agent_id

        # Store session info in input data
        input_data["registered_session"] = {
            "session_id": session.session_id,
            "agent_id": session.agent_id,
            "project_id": project_id,
            "started_at": session.started_at.isoformat() if session.started_at else None,
        }

        # Capture session start in Nexus-backed truth.
        try:
            from maestro.memory.service import MaestroMemoryService

            async def _store_session_start() -> None:
                service = MaestroMemoryService()
                await service.initialize()
                try:
                    await service.store_command_context(
                        command="hook:session-start",
                        project_path=project_path or os.getcwd(),
                        context={
                            "session_id": session_id,
                            "agent_id": agent_id,
                            "agent_name": agent_name,
                            "event": "session_start",
                            "started_at": session.started_at.isoformat() if session.started_at else None,
                        },
                    )
                finally:
                    await service.close()

            asyncio.run(_store_session_start())
        except Exception as e:
            input_data["session_start_storage_error"] = str(e)
        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = session_register_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
