#!/usr/bin/env python3
"""
Maestro Pre-Compact Hook: Continuity

Captures continuity context before context compaction.
Preserves important context that should survive compaction.
"""

import json
import sys
import os
import asyncio
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
    except Exception as e:
        import sys
        sys.stderr.write(f"Error getting hook manager: {e}\n")
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

        # Capture recent memories before compaction from Nexus-backed truth.
        important_memories: list[dict[str, Any]] = []
        try:
            from maestro.memory.service import MaestroMemoryService

            async def _load_session_memories() -> list[dict[str, Any]]:
                service = MaestroMemoryService()
                await service.initialize()
                try:
                    return await service.retrieve_session_context(session_id=session_id, limit=20)
                finally:
                    await service.close()

            recent_memories = asyncio.run(_load_session_memories())
            important_memories = recent_memories[:10]
        except Exception as e:
            input_data["continuity_memory_load_error"] = str(e)
            important_memories = []

        # Create continuity ledger entry
        # Uses "observation" entry type (valid ledger type) with pre_compact tag
        if important_memories:
            continuity_entry = manager.create_ledger_entry(
                entry_type="observation",
                title=f"[PRE-COMPACT CONTINUITY] {session_id}",
                content=f"Preserving {len(important_memories)} important memories before context compaction",
                metadata={
                    "session_id": session_id,
                    "preserved_count": len(important_memories),
                    "timestamp": datetime.now(UTC).isoformat(),
                    "hook_type": "pre_compact_continuity",
                    "important_summaries": [
                        m.get("summary") or m.get("content", "")[:200]
                        for m in important_memories[:10]
                    ],
                },
            )

            input_data["continuity_preserved"] = {
                "session_id": session_id,
                "preserved_count": len(important_memories),
                "ledger_entry_id": continuity_entry.id if continuity_entry else None,
                "important_summaries": [m.get("summary") or m.get("content", "")[:100]
                                        for m in important_memories[:5]],
            }

            try:
                from maestro.memory.service import MaestroMemoryService

                async def _store_continuity_memory() -> None:
                    service = MaestroMemoryService()
                    await service.initialize()
                    try:
                        await service.store_command_context(
                            command="hook:pre-compact",
                            project_path=getattr(
                                manager.session_manager.get_session_by_id(session_id) if manager.session_manager else None,
                                "project_path", None
                            ) or os.getcwd(),
                            context={
                                "session_id": session_id,
                                "current_task_id": input_data.get("current_task_id", ""),
                                "hook_type": "pre_compact_continuity",
                                "preserved_count": len(important_memories),
                                "important_summaries": [
                                    m.get("summary") or m.get("content", "")[:200]
                                    for m in important_memories[:10]
                                ],
                            },
                        )
                    finally:
                        await service.close()

                asyncio.run(_store_continuity_memory())
            except Exception as e:
                input_data["continuity_storage_error"] = str(e)

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
