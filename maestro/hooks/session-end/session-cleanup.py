#!/usr/bin/env python3
"""
Maestro Session-End Hook: Session Cleanup

Performs cleanup tasks when a session ends.
Releases file claims, flushes buffers, and finalizes state.
"""

import json
import sys
import os
from datetime import datetime, UTC
from pathlib import Path
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


def get_file_claims_handler() -> Any:
    try:
        import importlib

        module = importlib.import_module("maestro.memory.coordination.file_claims")
        return getattr(module, "FileClaimsHandler", None)
    except Exception:
        return None


def session_cleanup_hook(input_data: dict) -> dict:
    """
    Session-end hook that performs cleanup tasks.

    Args:
        input_data: Hook input data containing session info

    Returns:
        Modified input data with cleanup status
    """
    try:
        session_id = input_data.get("session_id")
        agent_id = input_data.get("agent_id")

        if not session_id:
            return input_data

        # Get hook manager
        manager = get_hook_manager()

        if manager is None:
            return input_data

        cleanup_info = {
            "session_id": session_id,
            "cleaned_at": datetime.now(UTC).isoformat(),
        }

        # Flush any buffered memories
        buffer_flushed = 0
        if hasattr(manager, 'buffer_layer') and manager.buffer_layer and hasattr(manager.buffer_layer, 'enabled') and manager.buffer_layer.enabled:
            buffer_flushed = manager.buffer_layer.flush() if hasattr(manager.buffer_layer, 'flush') else 0
            cleanup_info["buffered_memories_flushed"] = buffer_flushed

        # Release file claims for this agent
        claims_released = 0
        if agent_id and hasattr(manager, '_session') and manager._session:
            try:
                handler_cls = get_file_claims_handler()
                if handler_cls is None:
                    raise RuntimeError("FileClaimsHandler unavailable")

                file_claims = handler_cls(manager._session)
                claims = file_claims.get_active_claims(agent_id=agent_id)

                for claim in claims:
                    claim_id = getattr(claim, "claim_id", None)
                    if claim_id:
                        file_claims.release_claim(claim_id)
                        claims_released += 1
            except Exception:
                pass

        cleanup_info["claims_released"] = claims_released

        # End the session in session manager
        if manager.session_manager:
            session = manager.session_manager.get_session_by_id(session_id)
            if session and session.status != "completed":
                manager.session_manager.end_session(session_id)
                manager._session.commit()
                cleanup_info["session_ended"] = True

        input_data["session_cleanup_completed"] = cleanup_info

        return input_data

    except Exception as e:
        input_data["hook_error"] = str(e)
        return input_data


def main() -> None:
    """Main entry point for the hook."""
    input_data = json.loads(sys.stdin.read())
    result = session_cleanup_hook(input_data)
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
