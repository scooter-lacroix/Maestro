#!/usr/bin/env python3
"""
Pre-compact hook: Generate/update track-specific compaction handoff document.

On each compaction event, this hook:
1. Reads the compaction handoff template
2. Fills it with current execution state
3. Writes/updates maestro/tracks/{track_id}/compaction-handoff.md
4. Increments the compaction count
"""

import json
import os
import subprocess
import sys
import asyncio
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


TEMPLATE_PATH = Path(__file__).parent.parent.parent / "templates" / "compaction-handoff-template.md"


def _safe_get(data: dict, *keys: str, default: Any = "") -> Any:
    """Safely traverse nested dict keys."""
    current: Any = data
    for key in keys:
        if isinstance(current, dict):
            current = current.get(key, default)
        else:
            return default
    return current if current is not None else default


def _detect_git_info(project_path: str) -> dict:
    """Detect git branch and dirty status."""
    info: dict[str, Any] = {"branch": "unknown", "dirty": False}
    try:
        result = subprocess.run(
            ["git", "branch", "--show-current"],
            capture_output=True, text=True, timeout=5,
            cwd=project_path,
        )
        if result.returncode == 0:
            info["branch"] = result.stdout.strip()
    except Exception:
        pass
    return info


def _read_template() -> str:
    """Read the compaction handoff template."""
    if TEMPLATE_PATH.exists():
        return TEMPLATE_PATH.read_text()
    return "# Compaction Handoff\n\n## Status\n{{status_summary}}\n\n## Remaining Work\n{{remaining_work}}"


def _count_compactions(handoff_path: Path) -> int:
    """Count how many times this handoff has been updated."""
    if not handoff_path.exists():
        return 0
    content = handoff_path.read_text()
    for line in content.split("\n"):
        if line.startswith("- Compaction Count:"):
            try:
                return int(line.split(":")[-1].strip())
            except ValueError:
                return 0
    return 0


def generate_handoff(input_data: dict) -> dict:
    """Generate or update the compaction handoff document."""
    project_path = input_data.get("project_path", os.getcwd())
    track_id = input_data.get("track_id", input_data.get("current_track", "unknown"))

    # Sanitize track_id to prevent path traversal
    if ".." in track_id or (len(track_id) > 0 and track_id[0] == "/"):
        return {"hook_error": f"Invalid track_id: {track_id}"}

    tracks_dir = Path(project_path) / "maestro" / "tracks"
    track_dir = tracks_dir / track_id

    if not track_dir.exists() and tracks_dir.exists():
        for d in tracks_dir.iterdir():
            if d.is_dir() and track_id in d.name:
                track_dir = d
                break

    track_dir.mkdir(parents=True, exist_ok=True)
    handoff_path = track_dir / "compaction-handoff.md"

    compaction_count = _count_compactions(handoff_path) + 1
    git_info = _detect_git_info(project_path)
    timestamp = datetime.now(timezone.utc).isoformat()

    template = _read_template()

    replacements = {
        "{{project_name}}": Path(project_path).name,
        "{{repo_path}}": project_path,
        "{{git_branch}}": git_info["branch"],
        "{{timestamp}}": timestamp,
        "{{track_id}}": track_id,
        "{{current_task_id}}": str(_safe_get(input_data, "current_task", default="N/A")),
        "{{iteration_number}}": str(_safe_get(input_data, "iteration", default="0")),
        "{{loop_mode}}": str(_safe_get(input_data, "loop_mode", default="building")),
        "{{agent_tool}}": str(_safe_get(input_data, "agent_tool", default="unknown")),
        "{{compaction_count}}": str(compaction_count),
        "{{track_objective}}": str(_safe_get(input_data, "objective", default="See track spec")),
        "{{status}}": str(_safe_get(input_data, "status", default="In progress")),
        "{{status_summary}}": str(_safe_get(input_data, "status_summary", default="Compaction triggered during active implementation.")),
        "{{completed_work}}": str(_safe_get(input_data, "completed_work", default="- See iteration history")),
        "{{key_findings}}": str(_safe_get(input_data, "key_findings", default="- None recorded")),
        "{{locked_decisions}}": str(_safe_get(input_data, "locked_decisions", default="- None recorded")),
        "{{changed_files}}": str(_safe_get(input_data, "changed_files", default="- See git diff")),
        "{{investigated_files}}": str(_safe_get(input_data, "investigated_files", default="- N/A")),
        "{{planned_files}}": str(_safe_get(input_data, "planned_files", default="- See plan.md")),
        "{{remaining_work}}": str(_safe_get(input_data, "remaining_work", default="- Continue from current task")),
        "{{active_style_guide}}": str(_safe_get(input_data, "style_guide", default="N/A")),
        "{{workflow_phase}}": str(_safe_get(input_data, "workflow_phase", default="implementation")),
        "{{review_agent}}": str(_safe_get(input_data, "review_agent", default="N/A")),
        "{{checkpoint_interval}}": str(_safe_get(input_data, "checkpoint_interval", default="N/A")),
        "{{model_name}}": str(_safe_get(input_data, "model", default="unknown")),
        "{{tasks_completed_count}}": str(_safe_get(input_data, "tasks_completed", default="0")),
        "{{total_tasks_count}}": str(_safe_get(input_data, "total_tasks", default="0")),
        "{{blockers_list}}": str(_safe_get(input_data, "blockers", default="[]")),
        "{{modified_files_list}}": str(_safe_get(input_data, "modified_files", default="[]")),
        "{{errors_list}}": str(_safe_get(input_data, "errors", default="[]")),
        "{{subtrack_statuses}}": str(_safe_get(input_data, "subtrack_statuses", default="{}")),
        "{{validation_commands}}": str(_safe_get(input_data, "validation_commands", default="- Run test suite\n- Check build")),
        "{{learnings}}": str(_safe_get(input_data, "learnings", default="N/A")),
        "{{issues}}": str(_safe_get(input_data, "issues", default="N/A")),
        "{{decisions}}": str(_safe_get(input_data, "decisions", default="N/A")),
        "{{self_evaluation}}": str(_safe_get(input_data, "self_evaluation", default="N/A")),
        "{{goal_line}}": str(_safe_get(input_data, "objective", default="Continue track implementation")),
        "{{done_line}}": str(_safe_get(input_data, "completed_work", default="See history")),
        "{{locked_line}}": str(_safe_get(input_data, "locked_decisions", default="None")),
        "{{next_line}}": str(_safe_get(input_data, "remaining_work", default="Continue from current task")),
        "{{verify_line}}": str(_safe_get(input_data, "validation_commands", default="Run tests")),
        "{{risks_line}}": str(_safe_get(input_data, "errors", default="None identified")),
    }

    content = template
    for placeholder, value in replacements.items():
        content = content.replace(placeholder, value)

    handoff_path.write_text(content)

    input_data["compaction_handoff_path"] = str(handoff_path)
    input_data["compaction_count"] = compaction_count

    try:
        from maestro.memory.service import MaestroMemoryService

        async def _store_compaction_handoff() -> None:
            service = MaestroMemoryService()
            await service.initialize()
            try:
                    await service.store_command_context(
                        command="hook:pre-compact-handoff",
                        project_path=project_path,
                        context={
                            "session_id": _safe_get(input_data, "session_id", default=""),
                            "track_id": track_id,
                            "current_task_id": _safe_get(input_data, "current_task_id", default=""),
                            "event": "compaction_handoff_generated",
                            "compaction_count": compaction_count,
                            "handoff_path": str(handoff_path),
                        },
                    )
            finally:
                await service.close()

        asyncio.run(_store_compaction_handoff())
    except Exception:
        pass

    return input_data


def main() -> None:
    input_data = json.loads(sys.stdin.read())
    result = generate_handoff(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
