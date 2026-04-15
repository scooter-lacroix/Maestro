#!/usr/bin/env python3
"""
Review hook: Nexus cognition scheduling.

Records a review transition in Nexus-backed memory so the review boundary is
preserved as part of the active session lifecycle.
"""

import asyncio
import json
import os
import sys
from pathlib import Path
from typing import Any

maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))


def review_hook(input_data: dict) -> dict:
    try:
        from maestro.memory.service import MaestroMemoryService

        session_id = str(
            input_data.get("session_id")
            or os.environ.get("MAESTRO_SESSION_ID", "")
        ).strip()
        project_path = str(
            input_data.get("project_path")
            or input_data.get("cwd")
            or os.environ.get("MAESTRO_PROJECT_PATH", "")
        ).strip()

        if not session_id or not project_path:
            return input_data

        async def _store_review() -> None:
            service = MaestroMemoryService()
            await service.initialize()
            try:
                await service.store_command_context(
                    command="hook:review",
                    project_path=project_path,
                    context={
                        "session_id": session_id,
                        "track_id": input_data.get("track_id"),
                        "task_id": input_data.get("task_id") or input_data.get("current_task_id"),
                        "iteration": input_data.get("iteration"),
                        "event": "review_transition",
                        "review_point_reached": bool(input_data.get("review_point_reached", True)),
                        "selected_cli": input_data.get("selected_cli") or os.environ.get("MAESTRO_SELECTED_CLI", ""),
                    },
                )
            finally:
                await service.close()

        asyncio.run(_store_review())
        input_data["review_cognition_scheduled"] = True
        return input_data
    except Exception as exc:
        input_data["hook_error"] = str(exc)
        return input_data


def main() -> None:
    input_data = json.loads(sys.stdin.read())
    result = review_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
