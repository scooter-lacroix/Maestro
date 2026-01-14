#!/usr/bin/env python3
"""
Maestro Pre-Tool-Use Hook: LeIndex Read

Intercepts file read operations and returns LeIndex (L1 AST + L2 Call Graph)
context instead of full file contents, providing 90%+ token savings.
Uses the consolidated TLDR + LeIndex system.
"""

import importlib
import json
import os
import sys
from pathlib import Path
from typing import Any

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))


LeIndexCache: Any = None


def _init_cache():
    """Initialize LeIndex cache module."""
    global LeIndexCache
    if LeIndexCache is None:
        try:
            # Try LeIndex cache first
            LeIndexCache = getattr(importlib.import_module("maestro.leindex.cache"), "LeIndexCache", None)
        except (ImportError, AttributeError):
            # Fall back to TLDR cache for compatibility
            try:
                LeIndexCache = getattr(importlib.import_module("maestro.tldr.cache"), "TLDRCache", None)
            except (ImportError, AttributeError):
                pass


def leindex_read_hook(input_data: dict) -> dict:
    """
    Pre-tool-use hook that provides LeIndex context for file reads.

    Args:
        input_data: Hook input data containing tool invocation info

    Returns:
        Modified input data with LeIndex context injected
    """
    _init_cache()

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
        code_extensions = {
            '.py', '.js', '.ts', '.tsx', '.jsx',
            '.java', '.go', '.rs', '.cpp', '.c', '.h'
        }
        if path_obj.suffix.lower() not in code_extensions:
            return input_data

        # Try to get LeIndex analysis
        if LeIndexCache is not None:
            try:
                cache = LeIndexCache()
                # Try 'get' method for LeIndex cache
                if hasattr(cache, 'get'):
                    tldr_data = cache.get(str(path_obj))
                else:
                    tldr_data = None

                if tldr_data:
                    # Inject LeIndex context instead of full file
                    input_data["leindex_context"] = {
                        "file_path": str(path_obj),
                        "ast_summary": tldr_data.get("ast", ""),
                        "call_graph": tldr_data.get("call_graph", ""),
                        "exports": tldr_data.get("exports", []),
                        "imports": tldr_data.get("imports", []),
                        "classes": tldr_data.get("classes", []),
                        "functions": tldr_data.get("functions", []),
                    }
                    input_data["leindex_enabled"] = True
            except Exception:
                # LeIndex not available, fall through
                pass

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = leindex_read_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
