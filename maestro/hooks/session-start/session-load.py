#!/usr/bin/env python3
"""
Maestro Session Start Hook: Session Load

Loads and restores session context from the memory system.
Integrates with the UnifiedHookManager to restore previous session state.
"""

import json
import sys
import os
import asyncio
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
    except Exception:
        return None
    return None


def session_load_hook(input_data: dict) -> dict:
    """
    Session start hook that loads and restores session context.

    Args:
        input_data: Hook input data containing:
            - session_id: Current session identifier
            - agent_id: Agent identifier
            - project_path: Project directory path

    Returns:
        Modified input data with restored context
    """
    try:
        session_id = input_data.get("session_id")
        agent_id = input_data.get("agent_id")
        project_path = input_data.get("project_path")

        if not session_id or not agent_id:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        memories = []
        try:
            from maestro.memory.service import MaestroMemoryService

            async def _load_session_memories() -> list[dict[str, Any]]:
                service = MaestroMemoryService()
                await service.initialize()
                try:
                    session_memories = await service.retrieve_session_context(session_id=session_id, limit=10)
                    if project_path:
                        project_memories = await service.retrieve_project_context(project_path=project_path, limit=5)
                        session_memories.extend(project_memories)
                    return session_memories
                finally:
                    await service.close()

            memories = asyncio.run(_load_session_memories())
        except Exception:
            # Fallback to compatibility recall if the direct Nexus bridge is unavailable.
            memories = manager.recall(
                query=f"session {session_id} agent {agent_id}",
                category="context",
                limit=10,
            )
            if project_path:
                memories.extend(
                    manager.recall(
                        query=f"project {project_path} recent context",
                        category="context",
                        limit=5,
                    )
                )

        # Inject context into input
        if memories:
            context_summaries = []
            for m in memories:
                if isinstance(m, dict):
                    context_summaries.append(m.get("content", ""))
                elif hasattr(m, 'content'):
                    context_summaries.append(m.content)
            input_data["restored_context"] = context_summaries
            input_data["context_loaded"] = True

        return input_data

    except Exception as e:
        # Log error but don't fail the hook
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    # Read stdin
    input_data = json.loads(sys.stdin.read())

    # Execute hook
    result = session_load_hook(input_data)

    # Write output
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
