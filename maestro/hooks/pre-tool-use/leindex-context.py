#!/usr/bin/env python3
"""
Maestro Pre-Tool-Use Hook: LeIndex Context Injection

Injects LeIndex code analysis context into prompts based on intent analysis.
Adds relevant code structure context to improve Task execution using
the LeIndex (pure Rust) system for 90%+ token reduction.

Note: the default path now shells out to the standalone `leindex` binary.
Maestro-local compatibility modules are retained only as a fallback path.
"""

import json
import os
import sys
import shutil
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Optional

# Add maestro to path if needed
maestro_root = Path(__file__).parent.parent.parent
if str(maestro_root) not in sys.path:
    sys.path.insert(0, str(maestro_root))


def _optional_attr(module_name: str, attr: str) -> Any:
    """Safely get an attribute from a module."""
    try:
        import importlib
        module = importlib.import_module(module_name)
        return getattr(module, attr)
    except Exception:
        return None


def _run_leindex_tool(tool_name: str, args: Dict[str, Any], working_dir: str) -> str:
    """Run a standalone LeIndex tool and return stdout on success."""
    leindex_bin = shutil.which("leindex")
    if not leindex_bin:
        return ""

    cmd = [
        leindex_bin,
        "tools",
        "run",
        tool_name,
        "--project",
        working_dir,
        "--args",
        json.dumps(args),
    ]

    try:
        result = subprocess.run(
            cmd,
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


LEGACY_CONTEXT_FALLBACK_USED = False


def analyze_intent(prompt: str) -> str:
    """
    Analyze the intent of a Task prompt.

    Returns:
        Intent category: 'edit', 'create', 'debug', 'refactor', 'explore', 'test'
    """
    prompt_lower = prompt.lower()

    intent_keywords = {
        'edit': ['edit', 'modify', 'change', 'update', 'fix', 'alter'],
        'create': ['create', 'add', 'implement', 'write', 'new', 'build'],
        'debug': ['debug', 'error', 'bug', 'issue', 'failing', 'broken'],
        'refactor': ['refactor', 'restructure', 'reorganize', 'clean', 'simplify'],
        'explore': ['explore', 'find', 'search', 'locate', 'what', 'how', 'where'],
        'test': ['test', 'spec', 'coverage', 'mock', 'verify'],
    }

    for intent, keywords in intent_keywords.items():
        if any(kw in prompt_lower for kw in keywords):
            return intent

    return 'general'


def extract_file_references(prompt: str, working_dir: str) -> List[str]:
    """
    Extract file references from the prompt.

    Returns:
        List of potential file paths
    """
    import re

    files = []

    # Match common source file extensions across the Maestro codebase.
    file_pattern = (
        r'[\w\-./]+\.(?:py|rs|ts|tsx|js|jsx|go|java|c|cpp|h|hpp|cs|rb|php|swift|'
        r'kt|scala|lua|zig|nim|cr|ex|exs|hs|ml|fs|clj|vue|svelte|astro)[\w\-./]*'
    )
    matches = re.findall(file_pattern, prompt)

    for match in matches:
        full_path = os.path.join(working_dir, match)
        if os.path.exists(full_path):
            files.append(full_path)

    return files


def extract_symbol_references(prompt: str) -> List[str]:
    """
    Extract function/class references from the prompt.

    Returns:
        List of potential symbol names
    """
    import re

    # Match function names (lowercase_with_underscores)
    functions = re.findall(r'\b[a-z][a-z0-9_]+\b', prompt)

    # Match class names (CamelCase)
    classes = re.findall(r'\b[A-Z][a-zA-Z0-9]+\b', prompt)

    return functions + classes


def get_relevant_code_context(
    prompt: str,
    working_dir: str,
    intent: str,
) -> str:
    """
    Get relevant code context using LeIndex analysis.

    Args:
        prompt: User's prompt
        working_dir: Current working directory
        intent: Analyzed intent

    Returns:
        Formatted context string
    """
    context_parts: List[str] = []
    files = extract_file_references(prompt, working_dir)

    if files:
        for file_path in files[:3]:
            file_context = _run_leindex_tool(
                "leindex_read_file",
                {
                    "file_path": file_path,
                    "project_path": working_dir,
                    "include_symbol_map": True,
                    "max_lines": 220,
                },
                working_dir,
            )
            if file_context:
                context_parts.append(
                    f"## Standalone LeIndex File Context: {Path(file_path).name}\n{file_context}"
                )

    project_map = _run_leindex_tool(
        "leindex_project_map",
        {
            "project_path": working_dir,
            "path": ".",
            "depth": 2,
            "include_symbols": True,
            "token_budget": 1400,
        },
        working_dir,
    )
    if project_map:
        context_parts.append(f"## Standalone LeIndex Project Map\n{project_map}")

    if not context_parts:
        deep_context = _run_leindex_tool(
            "leindex_deep_analyze",
            {
                "project_path": working_dir,
                "query": prompt[:600],
                "token_budget": 2000,
            },
            working_dir,
        )
        if deep_context:
            context_parts.append(f"## Standalone LeIndex Deep Analysis\n{deep_context}")

    # Compatibility fallback only if the standalone CLI is unavailable or failed.
    global LEGACY_CONTEXT_FALLBACK_USED
    LEGACY_CONTEXT_FALLBACK_USED = False

    if not context_parts:
        try:
            context_func = _optional_attr(
                "maestro.leindex.context_extraction",
                "get_context_for_prompt",
            )
            if not callable(context_func):
                context_func = _optional_attr("maestro.leindex", "get_context_for_prompt")

            if callable(context_func):
                ctx = context_func(working_dir, prompt, max_files=3)
                if ctx and "No specific files identified" not in ctx:
                    context_parts.append(ctx)
                    LEGACY_CONTEXT_FALLBACK_USED = True
        except Exception:
            pass

    return "\n\n".join(context_parts) if context_parts else ""


def recall_relevant_memories(
    query: str,
    intent: str,
    limit: int = 3,
) -> List[Dict[str, Any]]:
    """
    Recall relevant memories using semantic search.

    Args:
        query: Search query
        intent: Intent category
        limit: Maximum results

    Returns:
        List of relevant memories
    """
    try:
        # Use LeIndex memory bridge
        bridge_factory = _optional_attr("maestro.leindex.memory_integration", "get_leindex_memory_bridge")

        bridge = bridge_factory() if callable(bridge_factory) else None
        if bridge is None:
            return []

        results: List[Dict[str, Any]] = bridge.search_code_insights(query, limit=limit)
        return results
    except Exception:
        return []


def leindex_context_hook(input_data: dict) -> dict:
    """
    Pre-tool-use hook that injects LeIndex context for Task operations.

    This hook analyzes the prompt intent and injects relevant:
    - Code structure from LeIndex analysis (90%+ token reduction)
    - Related memories from semantic search
    - File and symbol context

    Args:
        input_data: Hook input data containing tool invocation info

    Returns:
        Modified input data with injected context
    """
    try:
        tool_name = input_data.get("tool_name", "")

        # Process Task operations and potentially Edit operations
        should_process = tool_name in ("Task", "Edit")

        if not should_process:
            return input_data

        tool_input = input_data.get("tool_input", {})
        prompt = tool_input.get("prompt", "")

        if not prompt:
            return input_data

        # Get working directory from input data
        working_dir = input_data.get("working_directory", os.getcwd())

        # Analyze intent
        intent = analyze_intent(prompt)

        # Initialize context info
        context_info = {
            "intent": intent,
            "context_injected": False,
            "code_context": "",
            "memories": [],
            "source": "standalone_leindex",
            "analysis_backend": "leindex_cli",
            "compatibility_mode": False,
        }

        # Get relevant code context
        code_context = get_relevant_code_context(prompt, working_dir, intent)

        if code_context:
            context_info["code_context"] = code_context
            context_info["context_injected"] = True
        if LEGACY_CONTEXT_FALLBACK_USED:
            context_info["source"] = "compatibility_legacy_fallback"
            context_info["compatibility_mode"] = True

        # Recall relevant memories from LeIndex analysis
        search_query = f"{intent} {prompt[:100]}"
        memories = recall_relevant_memories(search_query, intent, limit=3)

        if memories:
            context_info["memories"] = [
                {
                    "id": m.get("id"),
                    "summary": m.get("summary", ""),
                    "content": m.get("content", "")[:200],
                }
                for m in memories
            ]
            context_info["context_injected"] = True
            if LEGACY_CONTEXT_FALLBACK_USED:
                context_info["compatibility_mode"] = True

        # Add context to tool input for Edit operations
        if tool_name == "Edit" and context_info["context_injected"]:
            # Prepend context to the prompt for Edit operations
            context_prefix = ""

            if code_context:
                context_prefix += f"<!-- LeIndex Code Context -->\n{code_context}\n\n"

            if memories:
                context_prefix += "<!-- Related Memories -->\n"
                for mem in memories[:2]:
                    context_prefix += f"- {mem['summary']}\n"
                context_prefix += "\n"

            if context_prefix:
                tool_input["original_prompt"] = prompt
                tool_input["prompt"] = f"{context_prefix}{prompt}"
                input_data["tool_input"] = tool_input

        if context_info.get("compatibility_mode"):
            input_data["hook_warning"] = (
                "Legacy Maestro LeIndex compatibility fallback was used because the standalone CLI path was unavailable. "
                "This path is compatibility-only and must not be treated as the default managed-session analysis path."
            )
            input_data["legacy_compatibility_path"] = True

        input_data["maestro_leindex_context"] = context_info

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = leindex_context_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
