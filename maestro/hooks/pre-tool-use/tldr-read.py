#!/usr/bin/env python3
"""
Maestro Pre-Tool-Use Hook: TLDR Read

Intercepts file read operations and returns TLDR (L1 AST + L2 Call Graph)
context instead of full file contents, providing significant token savings.
"""

import json
import sys
import os
import importlib
from pathlib import Path
from typing import Any

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))

TLDRCache: Any = None
try:
    TLDRCache = getattr(importlib.import_module("maestro.tldr.cache"), "TLDRCache")
except (ImportError, AttributeError):
    pass


def tldr_read_hook(input_data: dict) -> dict:
    """
    Pre-tool-use hook that provides TLDR context for file reads.

    Args:
        input_data: Hook input data containing tool invocation info

    Returns:
        Modified input data with TLDR context injected
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Only process Read operations
        if tool_name != "Read":
            return input_data

        tool_input = input_data.get("tool_input", {})
        file_path = tool_input.get("file_path", "")

        if not file_path:
            return input_data

        # Resolve file path
        path_obj = Path(file_path)
        if not path_obj.is_absolute():
            # Try relative to current directory
            cwd = input_data.get("cwd", os.getcwd())
            path_obj = Path(cwd) / file_path

        if not path_obj.exists():
            return input_data

        # Check if this is a code file we can analyze
        code_extensions = {'.py', '.js', '.ts', '.tsx', '.jsx', '.java', '.go', '.rs', '.cpp', '.c', '.h'}
        if path_obj.suffix.lower() not in code_extensions:
            return input_data

        # Try to get TLDR analysis
        if TLDRCache is not None:
            try:
                cache = TLDRCache()
                tldr_data = cache.get(str(path_obj))

                if tldr_data:
                    # Inject TLDR context instead of full file
                    input_data["tldr_context"] = {
                        "file_path": str(path_obj),
                        "ast_summary": tldr_data.get("ast", ""),
                        "call_graph": tldr_data.get("call_graph", ""),
                        "exports": tldr_data.get("exports", []),
                        "imports": tldr_data.get("imports", []),
                        "classes": tldr_data.get("classes", []),
                        "functions": tldr_data.get("functions", []),
                    }
                    input_data["tldr_enabled"] = True
            except Exception:
                # TLDR not available, fall through
                pass

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = tldr_read_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
