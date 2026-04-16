#!/usr/bin/env python3
"""
Review hook: Nexus cognition scheduling + Critical Think analysis.

Records a review transition in Nexus-backed memory so the review boundary is
preserved as part of the active session lifecycle. Also invokes the
CriticalThinkEngine for pre-review analysis to identify risks before review.
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

        # Invoke CriticalThinkEngine for pre-review analysis (runs regardless of persistence)
        try:
            from maestro.critical_think.core import CriticalThinkEngine

            engine = CriticalThinkEngine()
            review_desc = (
                input_data.get("review_description")
                or f"Review at task {input_data.get('task_id', 'unknown')}"
            )
            context_parts = []
            if input_data.get("track_id"):
                context_parts.append(f"Track: {input_data['track_id']}")
            if input_data.get("iteration"):
                context_parts.append(f"Iteration: {input_data['iteration']}")
            review_context = "; ".join(context_parts) if context_parts else "Routine review checkpoint"

            ct_result = engine.invoke_before(
                action_description=review_desc,
                context=review_context,
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

            # Persist critical think analysis to Nexus for cross-session retrieval
            try:

                async def _store_ct_result() -> None:
                    service = MaestroMemoryService()
                    await service.initialize()
                    try:
                        await service.store_command_context(
                            command="hook:review:critical_think",
                            project_path=project_path,
                            context={
                                "session_id": session_id,
                                "track_id": input_data.get("track_id"),
                                "task_id": input_data.get("task_id") or input_data.get("current_task_id"),
                                "event": "critical_think_analysis",
                                "critical_think_result": input_data["critical_think_result"],
                            },
                        )
                    finally:
                        await service.close()

                asyncio.run(_store_ct_result())
            except Exception as e:
                input_data["ct_storage_error"] = str(e)
        except Exception as e:
            # Critical think is best-effort — don't fail the review
            input_data["critical_think_error"] = str(e)

        # Persist review transition to Nexus (requires session_id and project_path)
        if session_id and project_path:
            try:
                async def _store_review_data() -> None:
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

                asyncio.run(_store_review_data())
                input_data["review_cognition_scheduled"] = True
            except Exception as e:
                input_data["review_persistence_error"] = str(e)

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
    result = review_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
