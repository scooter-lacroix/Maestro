#!/usr/bin/env python3
"""
Session-start hook: Maestro Docs Reader Enforcement

At the start of every implement/orchestrate session, injects a message
requiring the agent to read all maestro docs before proceeding.
"""

import json
import os
import sys
from pathlib import Path


def _find_maestro_docs(project_path: str) -> list[str]:
    """Find all maestro docs that should be read."""
    docs = []
    maestro_dir = Path(project_path) / "maestro"
    if not maestro_dir.exists():
        return docs

    # Core workflow docs
    for name in ["workflow.md", "workflow-config.json"]:
        p = maestro_dir / name
        if p.exists():
            docs.append(str(p))

    # Style guides
    style_dir = maestro_dir / "code_styleguides"
    if style_dir.exists():
        for f in style_dir.glob("*.md"):
            docs.append(str(f))

    # LeIndex tools reference
    leindex_docs = maestro_dir / "docs" / "leindex-tools.md"
    if leindex_docs.exists():
        docs.append(str(leindex_docs))

    return docs


def docs_reader_hook(input_data: dict) -> dict:
    """Inject docs reading requirement at session start."""
    try:
        session_type = input_data.get("session_type") or ""
        if session_type not in ("implement", "orchestrate", ""):
            return input_data

        project_path = input_data.get("project_path", os.getcwd())
        docs = _find_maestro_docs(project_path)

        if docs:
            input_data["maestro_docs_to_read"] = docs
            input_data["maestro_docs_instruction"] = (
                "MANDATORY: Before starting any implementation work, you MUST read "
                "the following maestro project docs to understand the workflow, "
                "style guides, and available tools:\n\n"
                + "\n".join(f"  - {d}" for d in docs)
                + "\n\nDo NOT skip this step. Workflow adherence depends on it."
            )
    except Exception as e:
        input_data["hook_error"] = str(e)

    return input_data


def main() -> None:
    input_data = json.loads(sys.stdin.read())
    result = docs_reader_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
