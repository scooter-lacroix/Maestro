#!/usr/bin/env python3
"""
Maestro Post-Tool-Use Hook: Handoff Index

Indexes handoffs created via Write operations to thoughts/handoffs/**/*.md
Maintains handoff registry for continuity between sessions.
"""

import json
import sys
import os
import re
import asyncio
from pathlib import Path
from datetime import datetime, UTC
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


HANDOFF_PATTERN = re.compile(r'thoughts/handoffs/.*\.md')


def handoff_index_hook(input_data: dict) -> dict:
    """
    Post-tool-use hook that indexes handoffs.

    Args:
        input_data: Hook input data containing tool result

    Returns:
        Modified input data with handoff index info
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Only process Write operations
        if tool_name != "Write":
            return input_data

        tool_result = input_data.get("tool_result", {})

        # Check if write was successful
        if not tool_result.get("success", False):
            return input_data

        tool_input = input_data.get("tool_input", {})
        file_path = tool_input.get("file_path", "")

        if not file_path:
            return input_data

        # Check if this is a handoff file
        path_obj = Path(file_path)

        if not HANDOFF_PATTERN.search(file_path):
            # Also check for handoffs directory
            if "handoff" not in path_obj.parts:
                return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        current_session_id = getattr(manager, '_current_session_id', None)
        current_agent_id = getattr(manager, '_current_agent_id', None)

        if current_session_id is None:
            return input_data

        # Create handoff record
        handoff_title = path_obj.stem
        handoff_title = handoff_title.replace("-", " ").replace("_", " ").title()

        handoff = manager.create_handoff(
            title=handoff_title,
            context_data={
                "file_path": file_path,
                "created_at": datetime.now(UTC).isoformat(),
                "agent_id": current_agent_id,
                "session_id": current_session_id,
            },
            summary=f"Handoff created at {file_path}",
        )

        if handoff:
            input_data["handoff_indexed"] = {
                "handoff_id": handoff.id if hasattr(handoff, 'id') else None,
                "title": handoff_title,
                "file_path": file_path,
            }

            try:
                from maestro.memory.service import MaestroMemoryService

                project_path = os.getcwd()
                if hasattr(manager, "session_manager"):
                    session = manager.session_manager.get_session_by_id(current_session_id)
                    if session and getattr(session, "project_path", None):
                        project_path = session.project_path

                async def _store_handoff_memory() -> None:
                    service = MaestroMemoryService()
                    await service.initialize()
                    try:
                        await service.store_command_context(
                            command="hook:handoff-index",
                            project_path=project_path,
                            context={
                                "session_id": current_session_id,
                                "agent_id": current_agent_id,
                                "event": "handoff_indexed",
                                "handoff_id": handoff.id if hasattr(handoff, "id") else None,
                                "file_path": file_path,
                                "title": handoff_title,
                            },
                        )
                    finally:
                        await service.close()

                asyncio.run(_store_handoff_memory())
            except Exception as e:
                input_data["handoff_storage_error"] = str(e)

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = handoff_index_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
