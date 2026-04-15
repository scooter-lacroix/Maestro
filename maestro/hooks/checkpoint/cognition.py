#!/usr/bin/env python3
"""
Checkpoint hook: Nexus cognition scheduling + Critical Think analysis.

Records a checkpoint transition in Nexus-backed memory so major loop milestones
are persisted as part of the active session lifecycle. Also invokes the
CriticalThinkEngine for post-action analysis at each checkpoint.
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


def checkpoint_hook(input_data: dict) -> dict:
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

        async def _store_checkpoint() -> None:
            service = MaestroMemoryService()
            await service.initialize()
            try:
                await service.store_command_context(
                    command="hook:checkpoint",
                    project_path=project_path,
                    context={
                        "session_id": session_id,
                        "track_id": input_data.get("track_id"),
                        "task_id": input_data.get("task_id") or input_data.get("current_task_id"),
                        "iteration": input_data.get("iteration"),
                        "event": "checkpoint_transition",
                        "checkpoint_interval": input_data.get("checkpoint_interval"),
                        "task_completed": bool(input_data.get("task_completed", False)),
                        "selected_cli": input_data.get("selected_cli") or os.environ.get("MAESTRO_SELECTED_CLI", ""),
                    },
                )
            finally:
                await service.close()

        asyncio.run(_store_checkpoint())
        input_data["checkpoint_cognition_scheduled"] = True

        # Invoke CriticalThinkEngine for post-checkpoint analysis
        try:
            from maestro.critical_think.core import CriticalThinkEngine

            engine = CriticalThinkEngine()
            task_desc = (
                input_data.get("task_description")
                or input_data.get("current_task")
                or f"Checkpoint at task {input_data.get('task_id', 'unknown')}"
            )
            original_plan = input_data.get("original_plan", "")
            actual_result = input_data.get("actual_result", "")
            if not actual_result:
                completed = input_data.get("task_completed", False)
                actual_result = "Task completed" if completed else "Task in progress"

            ct_result = engine.invoke_after(
                action_description=task_desc,
                original_plan=original_plan,
                actual_result=actual_result,
                action_type="implementation",
            )
            input_data["critical_think_result"] = {
                "confidence_score": ct_result.confidence_score,
                "revised_confidence": ct_result.revised_confidence,
                "pitfalls": ct_result.pitfalls,
                "risks": ct_result.risks,
                "synthesis": ct_result.synthesis,
                "next_steps": ct_result.next_steps,
            }
        except Exception:
            # Critical think is best-effort — don't fail the checkpoint
            pass

        return input_data
    except Exception as exc:
        input_data["hook_error"] = str(exc)
        return input_data


def main() -> None:
    input_data = json.loads(sys.stdin.read())
    result = checkpoint_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
