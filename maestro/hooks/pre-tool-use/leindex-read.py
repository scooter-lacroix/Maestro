#!/usr/bin/env python3
"""
Maestro Pre-Tool-Use Hook: LeIndex Read

Intercepts file read operations and returns LeIndex (L1 AST + L2 Call Graph)
context instead of full file contents, providing 90%+ token savings.
Uses the LeIndex system (TLDR is deprecated and must not be referenced at runtime).
"""

import importlib
import json
import os
import sys
import shutil
import subprocess
from pathlib import Path
from typing import Any

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))


LeIndexCache: Any = None
LEGACY_READ_FALLBACK_USED = False


def _init_cache():
    """Initialize LeIndex cache module."""
    global LeIndexCache
    if LeIndexCache is None:
        try:
            # Try LeIndex cache first
            LeIndexCache = getattr(importlib.import_module("maestro.leindex.cache"), "LeIndexCache", None)
        except (ImportError, AttributeError):
            pass


def _run_leindex_read(file_path: str, working_dir: str) -> str:
    """Read a file through the standalone LeIndex CLI."""
    leindex_bin = shutil.which("leindex")
    if not leindex_bin:
        return ""

    try:
        result = subprocess.run(
            [
                leindex_bin,
                "tools",
                "run",
                "leindex_read_file",
                "--project",
                working_dir,
                "--args",
                json.dumps(
                    {
                        "file_path": file_path,
                        "project_path": working_dir,
                        "include_symbol_map": True,
                        "max_lines": 240,
                    }
                ),
            ],
            capture_output=True,
            text=True,
            timeout=30,
            cwd=working_dir,
        )
    except Exception:
        return ""

    if result.returncode != 0:
        return ""

    return result.stdout.strip()


def leindex_read_hook(input_data: dict) -> dict:
    """
    Pre-tool-use hook that provides LeIndex context for file reads.

    Args:
        input_data: Hook input data containing tool invocation info

    Returns:
        Modified input data with LeIndex context injected
    """
    _init_cache()
    global LEGACY_READ_FALLBACK_USED
    LEGACY_READ_FALLBACK_USED = False

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

        working_dir = input_data.get("cwd", os.getcwd())
        read_output = _run_leindex_read(str(path_obj), working_dir)

        if read_output:
            # Inject standalone LeIndex context instead of full file contents.
            input_data["leindex_context"] = {
                "file_path": str(path_obj),
                "source": "standalone_leindex",
                "analysis_backend": "leindex_cli",
                "raw_output": read_output,
                "ast_summary": read_output,
                "call_graph": read_output,
                "exports": [],
                "imports": [],
                "classes": [],
                "functions": [],
            }
            input_data["leindex_enabled"] = True
            return input_data

        # Compatibility fallback for environments where the standalone CLI is unavailable.
        if LeIndexCache is not None:
            try:
                cache = LeIndexCache()
                if hasattr(cache, "get"):
                    analysis_data = cache.get(str(path_obj))
                else:
                    analysis_data = None

                if analysis_data:
                    input_data["leindex_context"] = {
                        "file_path": str(path_obj),
                        "source": "compatibility_legacy_cache",
                        "ast_summary": analysis_data.get("ast", ""),
                        "call_graph": analysis_data.get("call_graph", ""),
                        "exports": analysis_data.get("exports", []),
                        "imports": analysis_data.get("imports", []),
                        "classes": analysis_data.get("classes", []),
                        "functions": analysis_data.get("functions", []),
                    }
                    LEGACY_READ_FALLBACK_USED = True
                    input_data["leindex_enabled"] = True
                    input_data["hook_warning"] = (
                        "Legacy Maestro LeIndex compatibility cache was used because the standalone CLI path was unavailable. "
                        "This cache path is compatibility-only and must not be treated as the default managed-session analysis path."
                    )
                    input_data["legacy_compatibility_path"] = True
            except Exception:
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
