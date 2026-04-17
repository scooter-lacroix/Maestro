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
import re
import subprocess
import sys
from loguru import logger
import asyncio
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


TEMPLATE_PATH = Path(__file__).parent.parent.parent / "templates" / "compaction-handoff-template.md"

_TRACK_ID_RE = re.compile(r'^[a-zA-Z0-9_-]+$')


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
        logger.opt(exception=True).debug("Could not determine git branch info")
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
    # Resolve track_id: explicit None falls back to current_track, then "unknown"
    raw_track_id = input_data.get("track_id")
    if raw_track_id is None:
        raw_track_id = input_data.get("current_track", "unknown")

    # Validate track_id: allow only alphanumeric, hyphens, underscores
    # Ensure track_id is a string before regex match (could be null/number)
    track_id_str = str(raw_track_id) if raw_track_id is not None else "unknown"
    if not _TRACK_ID_RE.match(track_id_str):
        raise ValueError(f"Invalid track_id: must match [a-zA-Z0-9_-]: {track_id_str}")
    track_id = track_id_str  # Use validated string

    tracks_dir = Path(project_path) / "maestro" / "tracks"
    track_dir = tracks_dir / track_id

    if not track_dir.exists() and tracks_dir.exists():
        # Try fuzzy matching: prioritize directories that start with track_id
        # followed by common archive/suffix patterns
        candidates = []
        for d in tracks_dir.iterdir():
            if not d.is_dir():
                continue
            # Exact match (shouldn't reach here due to the exists() check above,
            # but kept for completeness)
            if d.name == track_id:
                candidates.append((0, d))  # Priority 0: exact match
            # Starts with track_id and has a common separator/suffix
            elif d.name.startswith(track_id):
                suffix = d.name[len(track_id):]
                # Common patterns: -archived, -backup, -old, .bak, _v2, etc.
                if suffix.startswith(('-', '_', '.')) or suffix.startswith('-archived') or suffix.startswith('-backup'):
                    candidates.append((1, d))  # Priority 1: starts with track_id + separator
            # Contains track_id as a word boundary (less preferred, catches non-prefix names)
            elif re.search(rf'\b{re.escape(track_id)}\b', d.name):
                candidates.append((2, d))  # Priority 2: word boundary match

        if candidates:
            # Sort by priority and use the best match
            candidates.sort(key=lambda x: x[0])
            track_dir = candidates[0][1]

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

    # Create DB handoff record and store in memory in a single async operation
    try:
        from maestro.memory.coordination.handoffs import HandoffHandler
        from maestro.memory.service import MaestroMemoryService

        async def _persist_compaction_handoff() -> None:
            """Create DB handoff and store in memory in one event loop."""
            # Part 1: Create DB handoff record (sync SQLAlchemy)
            from maestro.memory.database.models import get_session_context
            with get_session_context() as db_session:
                handler = HandoffHandler(db_session)
                context = {
                    "compaction_count": compaction_count,
                    "current_task": str(_safe_get(input_data, "current_task", default="N/A")),
                    "iteration": str(_safe_get(input_data, "iteration", default="0")),
                    "workflow_phase": str(_safe_get(input_data, "workflow_phase", default="implementation")),
                    "completed_work": str(_safe_get(input_data, "completed_work", default="")),
                    "remaining_work": str(_safe_get(input_data, "remaining_work", default="")),
                    "changed_files": str(_safe_get(input_data, "changed_files", default="")),
                    "handoff_path": str(handoff_path),
                }
                handler.create_handoff(
                    title=f"Compaction #{compaction_count}: {track_id}",
                    from_session_id=_safe_get(input_data, "session_id", default=""),
                    from_agent_id=_safe_get(input_data, "agent_id", default=""),
                    project_path=project_path,
                    summary=f"Compaction handoff #{compaction_count} for track {track_id}",
                    context_data=context,
                )

            # Part 2: Store in Nexus memory (async)
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

        asyncio.run(_persist_compaction_handoff())
    except Exception as e:
        message = f"Failed to persist compaction handoff: {e}"
        logger.opt(exception=True).warning(message)
        input_data["compaction_memory_error"] = str(e)
        input_data["hook_error"] = message

    return input_data


def main() -> None:
    try:
        input_data = json.loads(sys.stdin.read())
    except json.JSONDecodeError as e:
        json.dump({"hook_error": f"Invalid JSON input: {e}"}, sys.stdout)
        sys.exit(1)
    try:
        result = generate_handoff(input_data)
        json.dump(result, sys.stdout)
    except Exception as e:
        json.dump({"hook_error": f"generate_handoff failed: {e}"}, sys.stdout)
        sys.exit(1)


if __name__ == "__main__":
    main()
