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

        # Invoke CriticalThinkEngine for post-checkpoint analysis (best-effort)
        # Run before persistence guard so analysis is available even when
        # session_id or project_path is missing.
        try:
            from maestro.critical_think.core import CriticalThinkEngine

            engine = CriticalThinkEngine()

            def _as_text(value: Any) -> str:
                return value if isinstance(value, str) else "" if value is None else str(value)

            def _as_bool(value: Any) -> bool:
                if isinstance(value, bool):
                    return value
                if value is None:
                    return False
                if isinstance(value, (int, float)):
                    return value != 0
                if isinstance(value, str):
                    return value.strip().lower() in {"1", "true", "yes", "y", "on"}
                return bool(value)

            task_desc = (
                _as_text(input_data.get("task_description"))
                or _as_text(input_data.get("current_task"))
                or f"Checkpoint at task {input_data.get('task_id', 'unknown')}"
            )
            original_plan = _as_text(input_data.get("original_plan"))
            actual_result = _as_text(input_data.get("actual_result"))
            if not actual_result:
                completed = _as_bool(input_data.get("task_completed"))
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
        except Exception as e:
            # Critical think is best-effort — don't fail the checkpoint
            input_data["critical_think_error"] = str(e)

        if not session_id or not project_path:
            return input_data

        # Single event loop for all persistence operations
        async def _store_all() -> None:
            service = MaestroMemoryService()
            await service.initialize()
            try:
                # Part 1: Store checkpoint
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
                        "task_completed": _as_bool(input_data.get("task_completed")),
                        "selected_cli": input_data.get("selected_cli") or os.environ.get("MAESTRO_SELECTED_CLI", ""),
                    },
                )

                # Part 2: Store critical think result (if available)
                if "critical_think_result" in input_data:
                    try:
                        await service.store_command_context(
                            command="hook:checkpoint:critical_think",
                            project_path=project_path,
                            context={
                                "session_id": session_id,
                                "track_id": input_data.get("track_id"),
                                "task_id": input_data.get("task_id") or input_data.get("current_task_id"),
                                "event": "critical_think_analysis",
                                "critical_think_result": input_data["critical_think_result"],
                            },
                        )
                    except Exception as e:
                        input_data["ct_storage_error"] = str(e)
            finally:
                await service.close()

        try:
            asyncio.run(_store_all())
            input_data["checkpoint_cognition_scheduled"] = True
        except Exception as e:
            input_data["checkpoint_storage_error"] = str(e)

        return input_data
    except Exception as exc:
        input_data["hook_error"] = str(exc)
        return input_data


def main() -> None:
    try:
        raw_input = sys.stdin.read()
        input_data = json.loads(raw_input) if raw_input.strip() else {}
    except json.JSONDecodeError:
        input_data = {}
    if not isinstance(input_data, dict):
        input_data = {}
    result = checkpoint_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
