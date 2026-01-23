#!/usr/bin/env python3
"""
Maestro Subagent-Stop Hook: Agent Report

Captures agent execution summary when a subagent completes.
Stores agent activity report in the memory system.
"""

import json
import sys
import os
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
    except Exception:
        return None
    return None


def agent_report_hook(input_data: dict) -> dict:
    """
    Subagent-stop hook that captures agent execution report.

    Args:
        input_data: Hook input data containing agent execution info

    Returns:
        Modified input data with report captured
    """
    try:
        agent_name = input_data.get("agent_name", "")
        agent_id = input_data.get("agent_id", "")
        result = input_data.get("result", {})

        if not agent_name:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        # Build agent report
        report = {
            "agent_name": agent_name,
            "agent_id": agent_id,
            "completed_at": datetime.now(UTC).isoformat(),
        }

        # Extract result info
        if isinstance(result, dict):
            report["success"] = result.get("success", True)
            report["tool_calls"] = result.get("tool_calls", [])
            report["errors"] = result.get("errors", [])

            # Count tool usage
            tool_counts: dict[str, int] = {}
            for call in result.get("tool_calls", []):
                tool_name = call.get("tool_name", "unknown")
                tool_counts[tool_name] = tool_counts.get(tool_name, 0) + 1
            report["tool_usage"] = tool_counts

        # Create summary
        tool_summary = ", ".join(
            f"{tool}:{count}" for tool, count in report.get("tool_usage", {}).items()
        ) if report.get("tool_usage") else "no tools"

        summary = f"Agent {agent_name} completed ({tool_summary})"

        # Store in memory if we have an active session
        current_session_id = getattr(manager, '_current_session_id', None)
        if current_session_id:
            manager.capture_memory(
                content=summary,
                category="agent_report",
                importance="normal",
                summary=summary,
                metadata=report,
                use_buffer=True,
            )

        # Create ledger entry
        if current_session_id:
            manager.create_ledger_entry(
                entry_type="agent_completion",
                title=f"Agent Completed: {agent_name}",
                content=summary,
                metadata=report,
            )

        input_data["agent_report_captured"] = {
            "agent_name": agent_name,
            "summary": summary,
            "captured_at": report["completed_at"],
        }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = agent_report_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
