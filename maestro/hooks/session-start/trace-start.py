#!/usr/bin/env python3
"""
Maestro Session Start Hook: Trace Start

Initializes tracing for the current session.
Sets up activity tracking and continuity ledger.
"""

import json
import sys
import os
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
    except Exception:
        return None
    return None


def trace_start_hook(input_data: dict) -> dict:
    """
    Session start hook that initializes tracing.

    Args:
        input_data: Hook input data containing session info

    Returns:
        Modified input data with trace initialization info
    """
    try:
        session_id = input_data.get("session_id")
        agent_id = input_data.get("agent_id")

        if not session_id or not agent_id:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        # Initialize continuity ledger for this session
        ledger_entry = manager.create_ledger_entry(
            entry_type="session_start",
            title=f"Session Trace: {session_id}",
            content=f"Agent {agent_id} started session at {datetime.now(UTC).isoformat()}",
            metadata={
                "session_id": session_id,
                "agent_id": agent_id,
                "trace_enabled": True,
            },
        )

        # Record initial activity
        manager.record_activity()

        # Store trace info
        input_data["trace_initialized"] = {
            "session_id": session_id,
            "ledger_entry_id": ledger_entry.id if ledger_entry else None,
            "trace_enabled": True,
        }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = trace_start_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
