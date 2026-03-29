#!/usr/bin/env python3
"""
Maestro User-Prompt-Submit Hook: Memory Recall

Recalls relevant memories based on user prompt analysis.
Provides context-aware memory retrieval for enhanced responses.
"""

import json
import sys
import os
import re
import asyncio
from pathlib import Path
from datetime import datetime, timedelta, UTC
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


def extract_key_terms(prompt: str) -> list[str]:
    """
    Extract key terms from prompt for memory search.

    Args:
        prompt: User's input prompt

    Returns:
        List of key terms
    """
    # Remove common words
    stop_words = {
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "must", "can", "to", "from", "with", "without",
        "by", "for", "of", "in", "on", "at", "as", "and", "or", "but", "not",
    }

    # Extract words (including technical terms with underscores, hyphens, dots)
    words = re.findall(r'\b[\w.-]+\b', prompt.lower())

    # Filter out stop words and short words
    key_terms = [w for w in words if w not in stop_words and len(w) > 2]

    return key_terms[:10]  # Limit to top 10 terms


def categorize_intent(prompt: str) -> str:
    """
    Categorize the user's intent.

    Returns:
        Intent category
    """
    prompt_lower = prompt.lower()

    categories = {
        "debug": ["error", "bug", "broken", "fail", "crash", "issue"],
        "feature": ["add", "create", "implement", "new", "feature"],
        "refactor": ["refactor", "clean", "restructure", "organize"],
        "question": ["how", "what", "why", "explain", "understand"],
        "fix": ["fix", "repair", "solve", "resolve"],
        "test": ["test", "spec", "coverage"],
        "review": ["review", "check", "audit"],
    }

    for category, keywords in categories.items():
        if any(kw in prompt_lower for kw in keywords):
            return category

    return "general"


def memory_recall_hook(input_data: dict) -> dict:
    """
    User-prompt-submit hook that recalls relevant memories.

    Args:
        input_data: Hook input data containing user prompt

    Returns:
        Modified input data with recalled memories
    """
    try:
        prompt = input_data.get("prompt", "")

        if not prompt:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        # Extract key terms and categorize intent
        key_terms = extract_key_terms(prompt)
        intent = categorize_intent(prompt)

        # Build search query
        if key_terms:
            query = " ".join(key_terms[:5])
        else:
            query = prompt[:100]  # Use first 100 chars as query

        # Add intent context to query
        query = f"{intent} {query}"

        recalled: list[dict[str, Any]] = []
        try:
            from maestro.memory.service import MaestroMemoryService

            async def _search_memories() -> list[dict[str, Any]]:
                service = MaestroMemoryService()
                await service.initialize()
                try:
                    project_path = os.environ.get("MAESTRO_PROJECT_PATH")
                    return await service.search_similar_commands(
                        command=query,
                        project_path=project_path,
                        limit=5,
                    )
                finally:
                    await service.close()

            memories = asyncio.run(_search_memories())
        except Exception:
            # Compatibility fallback for minimal environments.
            memories = manager.recall(
                query=query,
                category=None,
                limit=5,
            )

        for memory in memories:
            if isinstance(memory, dict):
                recalled.append({
                    "id": memory.get("id"),
                    "content": memory.get("content", ""),
                    "summary": memory.get("summary"),
                    "category": memory.get("category"),
                    "importance": memory.get("importance"),
                })
            else:
                recalled.append({
                    "id": memory.id if hasattr(memory, 'id') else None,
                    "content": memory.content if hasattr(memory, 'content') else "",
                    "summary": memory.summary if hasattr(memory, 'summary') else None,
                    "category": memory.category if hasattr(memory, 'category') else None,
                    "importance": memory.importance if hasattr(memory, 'importance') else None,
                })

        if recalled:
            input_data["recalled_memories"] = recalled
            input_data["memory_recall_count"] = len(recalled)
            input_data["recall_intent"] = intent

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = memory_recall_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
