#!/usr/bin/env python3
"""
Maestro Pre-Tool-Use Hook: File Claims

Automatically creates file claims when Edit/Write tools are used.
Implements coordination pattern for multi-agent scenarios.
"""

import json
import sys
import os
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


def file_claims_hook(input_data: dict) -> dict:
    """
    Pre-tool-use hook that creates file claims for edits/writes.

    Args:
        input_data: Hook input data containing tool invocation info

    Returns:
        Modified input data with claim info
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Only process Edit and Write operations
        if tool_name not in ("Edit", "Write"):
            return input_data

        tool_input = input_data.get("tool_input", {})
        file_path = tool_input.get("file_path", "")

        if not file_path:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        current_agent_id = getattr(manager, '_current_agent_id', None)
        if current_agent_id is None:
            return input_data

        # Create file claim
        claim = manager.create_file_claim(
            file_patterns=[file_path],
            reason=f"Pre-tool-use claim for {tool_name}",
            ttl_seconds=3600,  # 1 hour
        )

        if claim:
            input_data["file_claim"] = {
                "claim_id": claim.id if hasattr(claim, 'id') else None,
                "file_pattern": file_path,
                "agent_id": current_agent_id,
                "claimed_at": claim.created_at.isoformat() if hasattr(claim, 'created_at') else None,
            }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = file_claims_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
