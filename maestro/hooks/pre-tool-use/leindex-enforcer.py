#!/usr/bin/env python3
"""
Pre-tool-use hook: LeIndex Usage Enforcer

When LeIndex is available, this hook:
1. Detects when the agent uses Read/Grep/Glob on source code files
2. BLOCKS the operation with a REPRIMAND message
3. Instructs the agent to use the LeIndex equivalent
"""

import json
import os
import shutil
import sys
from pathlib import Path

SOURCE_EXTENSIONS = {
    '.py', '.rs', '.ts', '.tsx', '.js', '.jsx', '.go', '.java', '.c', '.cpp',
    '.h', '.hpp', '.cs', '.rb', '.php', '.swift', '.kt', '.scala', '.lua',
    '.zig', '.nim', '.cr', '.ex', '.exs', '.hs', '.ml', '.fs', '.clj',
    '.vue', '.svelte', '.astro',
}

TOOL_SUBSTITUTIONS = {
    "Read": "leindex_read_file (or leindex_read_symbol for specific functions/classes)",
    "Grep": "leindex_text_search (for text patterns) or leindex_grep_symbols (for symbol names)",
    "Glob": "leindex_project_map (for project structure exploration)",
}

REPRIMAND = (
    "\n[WARNING] WORKFLOW VIOLATION: LeIndex is available but you used a standard tool.\n\n"
    "YOU MUST use LeIndex tools for source code analysis. This is NOT optional.\n"
    "LeIndex provides structural awareness, token efficiency, and better results.\n\n"
)


def _is_source_file(path: str) -> bool:
    return Path(path).suffix.lower() in SOURCE_EXTENSIONS


def _leindex_available() -> bool:
    if os.environ.get("MAESTRO_LEINDEX_AVAILABLE") == "1":
        return True
    return shutil.which("leindex") is not None


def enforce_leindex(input_data: dict) -> dict:
    """Enforce LeIndex usage for source code operations."""
    if not _leindex_available():
        return input_data

    tool_name = input_data.get("tool_name", "")
    if tool_name not in TOOL_SUBSTITUTIONS:
        return input_data

    tool_input = input_data.get("tool_input", {})
    replacement = TOOL_SUBSTITUTIONS[tool_name]

    if tool_name == "Read":
        target = tool_input.get("path", "") or tool_input.get("file_path", "")
        if target and _is_source_file(target):
            input_data["hook_block"] = True
            input_data["hook_message"] = (
                f"{REPRIMAND}"
                f"INSTEAD OF: Read '{target}'\n"
                f"USE: {replacement}\n\n"
                f"leindex_read_file returns file contents WITH symbol context, imports, "
                f"and dependents — giving you much richer analysis context.\n\n"
                f"RETRY your operation using the correct LeIndex tool."
            )
    elif tool_name == "Grep":
        search_path = tool_input.get("path", "")
        if search_path:
            input_data["hook_block"] = True
            input_data["hook_message"] = (
                f"{REPRIMAND}"
                f"INSTEAD OF: Grep\n"
                f"USE: {replacement}\n\n"
                f"LeIndex search provides owning symbol context, file:line locations, "
                f"and structural awareness that Grep cannot provide.\n\n"
                f"RETRY your operation using the correct LeIndex tool."
            )
    elif tool_name == "Glob":
        input_data["hook_block"] = True
        input_data["hook_message"] = (
            f"{REPRIMAND}"
            f"INSTEAD OF: Glob\n"
            f"USE: {replacement}\n\n"
            f"leindex_project_map shows files with symbol counts, complexity hotspots, "
            f"and inter-module dependency arrows — far more useful than a flat file list.\n\n"
            f"RETRY your operation using the correct LeIndex tool."
        )

    return input_data


def main() -> None:
    try:
        raw_input = sys.stdin.read()
        input_data = json.loads(raw_input) if raw_input.strip() else {}
    except json.JSONDecodeError as e:
        sys.stderr.write(f"Failed to parse JSON input: {e}\n")
        sys.stdout.write("{}")
        sys.exit(1)
    except Exception as e:
        sys.stderr.write(f"Failed to read hook stdin: {e}\n")
        sys.stdout.write("{}")
        sys.exit(1)
    result = enforce_leindex(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
