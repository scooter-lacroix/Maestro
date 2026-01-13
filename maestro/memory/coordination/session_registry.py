"""
Session registry for cross-terminal coordination.
Manages the active_sessions.json file with proper locking.
"""

import os
import json
import logging
from typing import Dict, Any, List, Optional
from datetime import datetime, UTC
from maestro.memory.coordination.file_locks import get_maestro_lock

logger = logging.getLogger(__name__)

REGISTRY_PATH = os.path.expanduser("~/.maestro/sessions/active_sessions.json")

class SessionRegistry:
    """
    Manages the registry of active Maestro sessions.
    """

    def __init__(self, registry_path: str = REGISTRY_PATH):
        self.registry_path = registry_path
        self._ensure_directories()
        self.lock = get_maestro_lock("session_registry")
        self._migrate_old_registry()

    def _ensure_directories(self) -> None:
        """Ensure necessary directories exist."""
        os.makedirs(os.path.dirname(self.registry_path), exist_ok=True)

    def _migrate_old_registry(self) -> None:
        """Migrate from old coordination directory if it exists."""
        old_path = os.path.expanduser("~/.maestro/coordination/active_sessions.json")
        if os.path.exists(old_path) and not os.path.exists(self.registry_path):
            try:
                # Ensure directory exists (redundant but safe)
                os.makedirs(os.path.dirname(self.registry_path), exist_ok=True)
                os.rename(old_path, self.registry_path)
                logger.info(f"Migrated session registry from {old_path} to {self.registry_path}")

                # Try to remove old directory if empty
                old_dir = os.path.dirname(old_path)
                try:
                    os.rmdir(old_dir)
                except OSError:
                    pass # Directory not empty or other error
            except OSError as e:
                logger.warning(f"Failed to migrate session registry: {e}")

    def _read_registry(self) -> Dict[str, Any]:
        """Read registry from disk. Assumes lock is held."""
        if not os.path.exists(self.registry_path):
            return {"sessions": {}, "last_updated": datetime.now(UTC).isoformat()}

        try:
            with open(self.registry_path, 'r') as f:
                return json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            logger.error(f"Error reading session registry: {e}")
            return {"sessions": {}, "last_updated": datetime.now(UTC).isoformat()}

    def _write_registry(self, data: Dict[str, Any]) -> None:
        """Write registry to disk. Assumes lock is held."""
        data["last_updated"] = datetime.now(UTC).isoformat()
        try:
            with open(self.registry_path, 'w') as f:
                json.dump(data, f, indent=2)
        except OSError as e:
            logger.error(f"Error writing session registry: {e}")

    def register_session(self, session_id: str, metadata: Dict[str, Any]) -> None:
        """Register a new active session."""
        with self.lock.lock():
            registry = self._read_registry()
            registry["sessions"][session_id] = {
                **metadata,
                "registered_at": datetime.now(UTC).isoformat(),
                "pid": os.getpid()
            }
            self._write_registry(registry)

    def unregister_session(self, session_id: str) -> None:
        """Unregister a session (e.g., on exit)."""
        with self.lock.lock():
            registry = self._read_registry()
            if session_id in registry["sessions"]:
                del registry["sessions"][session_id]
                self._write_registry(registry)

    def get_active_sessions(self) -> Dict[str, Any]:
        """Get all registered active sessions."""
        with self.lock.lock():
            return self._read_registry()["sessions"]

    def cleanup_stale_sessions(self) -> int:
        """
        Remove sessions whose PIDs are no longer active.
        This is a simple way to handle sessions that didn't unregister gracefully.
        """
        import psutil

        cleaned = 0
        with self.lock.lock():
            registry = self._read_registry()
            to_remove = []
            for session_id, info in registry["sessions"].items():
                pid = info.get("pid")
                if pid:
                    try:
                        if not psutil.pid_exists(pid):
                            to_remove.append(session_id)
                    except Exception:
                        pass

            for session_id in to_remove:
                del registry["sessions"][session_id]
                cleaned += 1

            if cleaned > 0:
                self._write_registry(registry)

        return cleaned
