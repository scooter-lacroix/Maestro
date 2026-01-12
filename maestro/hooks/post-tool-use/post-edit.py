#!/usr/bin/env python3
"""
Maestro Post-Tool-Use Hook: Post-Edit

Captures context after successful Edit operations.
Updates continuity ledger and memory system.
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


def post_edit_hook(input_data: dict) -> dict:
    """
    Post-tool-use hook that captures edit context.

    Args:
        input_data: Hook input data containing tool result

    Returns:
        Modified input data with captured context
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Only process Edit operations
        if tool_name != "Edit":
            return input_data

        tool_result = input_data.get("tool_result", {})

        # Check if edit was successful
        if not tool_result.get("success", False):
            return input_data

        tool_input = input_data.get("tool_input", {})
        file_path = tool_input.get("file_path", "")

        if not file_path:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        current_session_id = getattr(manager, '_current_session_id', None)
        if current_session_id is None:
            return input_data

        # Capture edit in memory
        old_string = tool_input.get("old_string", "")
        new_string = tool_input.get("new_string", "")

        summary = f"Edited {Path(file_path).name}"
        if old_string and new_string:
            # Create brief diff summary
            old_preview = old_string[:50] + "..." if len(old_string) > 50 else old_string
            summary = f"Replaced '{old_preview}' in {Path(file_path).name}"

        manager.capture_memory(
            content=summary,
            category="edit",
            importance="normal",
            summary=summary,
            metadata={
                "file_path": file_path,
                "old_length": len(old_string),
                "new_length": len(new_string),
                "timestamp": datetime.now(UTC).isoformat(),
            },
            use_buffer=True,
        )

        # Update ledger
        manager.create_ledger_entry(
            entry_type="edit",
            title=f"Edit: {Path(file_path).name}",
            content=summary,
            metadata={
                "file_path": file_path,
                "edit_size": len(new_string) - len(old_string),
            },
        )

        input_data["post_edit_captured"] = {
            "file_path": file_path,
            "summary": summary,
            "captured_at": datetime.now(UTC).isoformat(),
        }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = post_edit_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
