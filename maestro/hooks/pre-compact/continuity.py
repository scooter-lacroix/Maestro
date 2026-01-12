#!/usr/bin/env python3
"""
Maestro Pre-Compact Hook: Continuity

Captures continuity context before context compaction.
Preserves important context that should survive compaction.
"""

import json
import sys
import os
from pathlib import Path
from datetime import datetime, UTC
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


def continuity_hook(input_data: dict) -> dict:
    """
    Pre-compact hook that captures continuity context.

    Args:
        input_data: Hook input data containing compact operation info

    Returns:
        Modified input data with continuity preserved
    """
    try:
        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        current_session_id = getattr(manager, '_current_session_id', None)
        if current_session_id is None:
            return input_data

        session_id = current_session_id

        # Capture important memories before compact
        important_memories: list[dict[str, Any]] = []

        if manager.session_manager:
            # Get recent high-importance memories for this session
            recent_memories = manager.memory_manager.get_memories_by_session(
                session_id=session_id,
                limit=20,
            )

            # Filter for high importance
            for mem in recent_memories:
                importance = getattr(mem, "importance", None)
                if importance in ("high", "critical"):
                    important_memories.append({
                        "id": getattr(mem, "id", None),
                        "content": getattr(mem, "content", ""),
                        "category": getattr(mem, "category", None),
                        "summary": getattr(mem, "summary", None),
                    })

        # Create continuity ledger entry
        if important_memories:
            continuity_entry = manager.create_ledger_entry(
                entry_type="pre_compact_continuity",
                title=f"Pre-Compact Continuity: {session_id}",
                content=f"Preserving {len(important_memories)} important memories",
                metadata={
                    "session_id": session_id,
                    "preserved_count": len(important_memories),
                    "timestamp": datetime.now(UTC).isoformat(),
                },
            )

            input_data["continuity_preserved"] = {
                "session_id": session_id,
                "preserved_count": len(important_memories),
                "ledger_entry_id": continuity_entry.id if continuity_entry else None,
                "important_summaries": [m.get("summary") or m.get("content", "")[:100]
                                        for m in important_memories[:5]],
            }

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = continuity_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
