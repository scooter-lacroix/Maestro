#!/usr/bin/env python3
"""
Maestro Post-Tool-Use Hook: Edit Notify

Notifies the LeIndex daemon/index layer of file changes for cache invalidation.
Ensures LeIndex analyses stay up-to-date with code changes.
"""

import json
import sys
import os
import importlib
from pathlib import Path
from datetime import datetime, UTC
from typing import Any

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))

LeIndexCache: Any = None
LeIndexDaemonClient: Any = None
try:
    LeIndexCache = getattr(importlib.import_module("maestro.leindex.cache"), "LeIndexCache")
    LeIndexDaemonClient = getattr(importlib.import_module("maestro.leindex.daemon"), "LeIndexDaemonClient")
except (ImportError, AttributeError):
    pass


def edit_notify_hook(input_data: dict) -> dict:
    """
    Post-tool-use hook that notifies LeIndex of file changes.

    Args:
        input_data: Hook input data containing tool result

    Returns:
        Modified input data with notification status
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Only process Edit and Write operations
        if tool_name not in ("Edit", "Write"):
            return input_data

        tool_result = input_data.get("tool_result", {})

        # Check if operation was successful
        if not tool_result.get("success", False):
            return input_data

        tool_input = input_data.get("tool_input", {})
        file_path = tool_input.get("file_path", "")

        if not file_path:
            return input_data

        path_obj = Path(file_path)

        # Only process code files
        code_extensions = {'.py', '.js', '.ts', '.tsx', '.jsx', '.java', '.go', '.rs', '.cpp', '.c', '.h'}
        if path_obj.suffix.lower() not in code_extensions:
            return input_data

        notification_sent = False

        # Try to notify LeIndex daemon (if present)
        if LeIndexDaemonClient is not None:
            try:
                client = LeIndexDaemonClient()
                notification_sent = client.notify_file_change(str(path_obj))
            except Exception:
                # Daemon might not be running
                pass

        # Fallback: invalidate cache directly
        if not notification_sent and LeIndexCache is not None:
            try:
                cache = LeIndexCache()
                cache.invalidate(str(path_obj))
                notification_sent = True
            except Exception:
                pass

        if notification_sent:
            input_data["leindex_notified"] = {
                "file_path": str(path_obj),
                "notified_at": datetime.now(UTC).isoformat(),
            }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = edit_notify_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
