"""
File-based advisory locking for cross-terminal coordination.
Provides a simple way to synchronize operations across multiple processes.
Cross-platform: Uses fcntl on Unix, msvcrt on Windows.
"""

import os
import sys
import time
import logging
import json
from contextlib import contextmanager
from typing import Optional, Dict, Any

# Cross-platform locking support
if sys.platform == 'win32':
    import msvcrt
    WINDOWS = True
else:
    import fcntl
    WINDOWS = False

logger = logging.getLogger(__name__)

class FileLockError(Exception):
    """Raised when a file lock cannot be acquired."""
    pass

class AdvisoryLock:
    """
    Implements file-based advisory locking using fcntl.lockf.
    Useful for cross-terminal/cross-process coordination.
    """

    def __init__(self, lock_file: str):
        self.lock_file = lock_file
        self.fd = None

    def acquire(self, timeout: float = 10.0, poll_interval: float = 0.1) -> bool:
        """
        Acquire the lock.

        Args:
            timeout: Maximum time to wait for the lock in seconds.
            poll_interval: Interval between poll attempts.

        Returns:
            True if lock acquired, False if timeout exceeded (and no exception raised if timeout=0).
        """
        # Ensure directory exists
        os.makedirs(os.path.dirname(os.path.abspath(self.lock_file)), exist_ok=True)

        try:
            # Use 0o600 for secure permissions (owner read/write only)
            self.fd = os.open(self.lock_file, os.O_RDWR | os.O_CREAT, 0o600)
        except OSError as e:
            logger.error(f"Could not open lock file {self.lock_file}: {e}")
            return False

        start_time = time.time()
        while True:
            try:
                # Non-blocking lock attempt - cross-platform
                if WINDOWS:
                    msvcrt.locking(self.fd, msvcrt.LK_NBLCK, 1)
                else:
                    fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                return True
            except (IOError, OSError):
                if timeout > 0 and (time.time() - start_time >= timeout):
                    os.close(self.fd)
                    self.fd = None
                    return False
                if timeout <= 0:
                    os.close(self.fd)
                    self.fd = None
                    return False
                time.sleep(poll_interval)

    def release(self) -> None:
        """Release the lock."""
        if self.fd is not None:
            try:
                if WINDOWS:
                    msvcrt.locking(self.fd, msvcrt.LK_UNLCK, 1)
                else:
                    fcntl.flock(self.fd, fcntl.LOCK_UN)
                os.close(self.fd)
            except Exception as e:
                logger.error(f"Error releasing lock {self.lock_file}: {e}")
            finally:
                self.fd = None

    @contextmanager
    def lock(self, timeout: float = 10.0):
        """Context manager for easy lock handling."""
        acquired = self.acquire(timeout=timeout)
        if not acquired:
            raise FileLockError(f"Timeout acquiring lock on {self.lock_file}")
        try:
            yield
        finally:
            self.release()

class SessionRegistry:
    """
    Registry for active sessions stored in a JSON file.
    Uses AdvisoryLock to ensure atomic updates.
    """

    def __init__(self, registry_path: str):
        self.registry_path = registry_path
        self.lock_path = registry_path + ".lock"
        self._lock = AdvisoryLock(self.lock_path)

    def _read_registry(self) -> Dict[str, Any]:
        """Read the registry file. Must be called while holding the lock."""
        if not os.path.exists(self.registry_path):
            return {}
        try:
            with open(self.registry_path, 'r') as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            return {}

    def _write_registry(self, data: Dict[str, Any]) -> None:
        """Write to the registry file. Must be called while holding the lock."""
        os.makedirs(os.path.dirname(os.path.abspath(self.registry_path)), exist_ok=True)
        with open(self.registry_path, 'w') as f:
            json.dump(data, f, indent=2)

    def register(self, session_id: str, metadata: Dict[str, Any]) -> bool:
        """Register a new active session."""
        try:
            with self._lock.lock(timeout=5.0):
                data = self._read_registry()
                data[session_id] = {
                    "registered_at": int(time.time() * 1000), # ms timestamp
                    **metadata
                }
                self._write_registry(data)
                return True
        except FileLockError:
            return False

    def unregister(self, session_id: str) -> bool:
        """Unregister a session."""
        try:
            with self._lock.lock(timeout=5.0):
                data = self._read_registry()
                if session_id in data:
                    del data[session_id]
                    self._write_registry(data)
                    return True
                return False
        except FileLockError:
            return False

    def get_active_sessions(self) -> Dict[str, Any]:
        """Get all currently registered active sessions."""
        try:
            with self._lock.lock(timeout=5.0):
                return self._read_registry()
        except FileLockError:
            logger.warning(f"Could not acquire lock for registry {self.registry_path}")
            return {}

def get_maestro_lock(name: str) -> AdvisoryLock:
    """Helper to get a standard Maestro lock by name."""
    lock_path = os.path.expanduser(f"~/.maestro/locks/{name}.lock")
    return AdvisoryLock(lock_path)
