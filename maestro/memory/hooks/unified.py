"""
Unified Hook Manager for Maestro Memory System

Provides a multi-layer hook system for capturing context throughout
the development workflow. Layers include native hooks, process monitoring,
inactivity detection, and persistent buffering.

Issue #25: Uses thread pool executor for non-blocking database I/O.
"""

import os
import time
import psutil
import logging
from abc import ABC, abstractmethod
from datetime import datetime, timedelta, UTC
from typing import Optional, Dict, Any, List, Iterator, cast, TYPE_CHECKING
from contextlib import contextmanager
from threading import Thread, Lock, Event
from collections import deque
from concurrent.futures import ThreadPoolExecutor, Future

from sqlalchemy import create_engine
from sqlalchemy.orm import Session as OrmSession, sessionmaker

if not TYPE_CHECKING:
    from maestro.memory.database.models import (
        Memory,
        MemoryCategory,
        MemoryImportance,
    )
    from maestro.memory.database.managers import (
        MemoryManager,
        SessionManager,
        ProjectManager,
    )
    from maestro.memory.coordination import (
        FileClaimsHandler,
        HandoffHandler,
        ContinuityLedgerHandler,
        EntryType,
    )
else:
    Memory = Any
    MemoryCategory = Any
    MemoryImportance = Any
    MemoryManager = Any
    SessionManager = Any
    ProjectManager = Any
    FileClaimsHandler = Any
    HandoffHandler = Any
    ContinuityLedgerHandler = Any
    EntryType = Any

logger = logging.getLogger(__name__)


# ============================================================================
# BASE HOOK CLASSES
# ============================================================================

class Hook(ABC):
    """
    Abstract base class for all hooks

    Hooks capture context at specific points in the workflow
    and store it in the memory system.
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None) -> None:
        """
        Initialize the hook

        Args:
            config: Optional hook configuration
        """
        self.config: Dict[str, Any] = config or {}
        self.enabled: bool = bool(self.config.get("enabled", True))

    @abstractmethod
    def execute(self, context: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """
        Execute the hook

        Args:
            context: Hook execution context

        Returns:
            Optional result data
        """
        pass

    def is_enabled(self) -> bool:
        """Check if hook is enabled"""
        return self.enabled


class HookLayer(ABC):
    """
    Abstract base class for hook layers

    A hook layer manages a collection of related hooks and
    provides lifecycle management.
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None) -> None:
        """
        Initialize the hook layer

        Args:
            config: Optional layer configuration
        """
        self.config: Dict[str, Any] = config or {}
        self.enabled: bool = bool(self.config.get("enabled", True))
        self.hooks: List[Hook] = []

    def register_hook(self, hook: Hook) -> None:
        """Register a hook with this layer"""
        self.hooks.append(hook)

    @abstractmethod
    def start(self) -> None:
        """Start the hook layer"""
        pass

    @abstractmethod
    def stop(self) -> None:
        """Stop the hook layer"""
        pass

    def execute_hooks(
        self,
        hook_type: str,
        context: Dict[str, Any],
    ) -> List[Dict[str, Any]]:
        """
        Execute all hooks of a specific type

        Args:
            hook_type: Type of hooks to execute
            context: Execution context

        Returns:
            List of results from executed hooks
        """
        if not self.enabled:
            return []

        results = []
        for hook in self.hooks:
            if hook.is_enabled():
                try:
                    result = hook.execute(context)
                    if result is not None:
                        results.append(result)
                except Exception as e:
                    # Log error but continue
                    results.append({"error": str(e)})

        return results


# ============================================================================
# NATIVE HOOK LAYER
# ============================================================================

class NativeHookLayer(HookLayer):
    """
    Native hook layer for direct event capture

    Captures events directly from the application workflow
    without instrumentation or monitoring.
    """

    def __init__(
        self,
        config: Optional[Dict[str, Any]] = None,
        memory_manager: Optional[MemoryManager] = None,
    ) -> None:
        super().__init__(config)
        self.memory_manager: Optional[MemoryManager] = memory_manager

    def start(self) -> None:
        """Start the native hook layer"""
        # Native hooks don't need startup
        pass

    def stop(self) -> None:
        """Stop the native hook layer"""
        # Native hooks don't need shutdown
        pass

    def capture_memory(
        self,
        content: str,
        category: str = MemoryCategory.CONTEXT.value,
        importance: str = MemoryImportance.NORMAL.value,
        **kwargs: Any,
    ) -> Optional[Memory]:
        """
        Capture a memory directly

        Args:
            content: Memory content
            category: Memory category
            importance: Memory importance
            **kwargs: Additional memory attributes

        Returns:
            Created Memory instance
        """
        if not self.enabled or not self.memory_manager:
            return None

        return self.memory_manager.create_memory(
            content=content,
            category=category,
            importance=importance,
            **kwargs,
        )


# ============================================================================
# PROCESS MONITOR LAYER
# ============================================================================

class ProcessMonitorLayer(HookLayer):
    """
    Process monitoring hook layer

    Monitors resource usage and captures memories at
    strategic points (e.g., high memory usage).
    """

    def __init__(
        self,
        config: Optional[Dict[str, Any]] = None,
        memory_manager: Optional[MemoryManager] = None,
    ) -> None:
        super().__init__(config)
        self.memory_manager: Optional[MemoryManager] = memory_manager
        self.sampling_interval: float = float(self.config.get("sampling_interval", 1.0))
        self.memory_threshold: float = float(self.config.get("memory_threshold", 0.85))
        self._running: bool = False
        self._thread: Optional[Thread] = None
        self._lock = Lock()

    def start(self) -> None:
        """Start the process monitoring thread"""
        if not self.enabled:
            return

        self._running = True
        self._thread = Thread(target=self._monitor_loop, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        """Stop the process monitoring thread"""
        self._running = False
        if self._thread:
            self._thread.join(timeout=5)
            self._thread = None

    def _monitor_loop(self) -> None:
        """Main monitoring loop"""
        process = psutil.Process()

        while self._running:
            try:
                # Check memory usage
                memory_percent = process.memory_percent() / 100

                if memory_percent >= self.memory_threshold:
                    self._capture_memory_threshold(memory_percent)

                time.sleep(self.sampling_interval)

            except Exception:
                # Process may have ended
                break

    def _capture_memory_threshold(self, memory_percent: float) -> None:
        """Capture memory when threshold is reached"""
        if self.memory_manager:
            self.memory_manager.create_memory(
                content=f"Memory usage threshold reached: {memory_percent:.1%}",
                category=MemoryCategory.OBSERVATION.value,
                importance=MemoryImportance.NORMAL.value,
                metadata={"memory_percent": memory_percent},
            )

    def get_process_info(self) -> Dict[str, Any]:
        """
        Get current process information

        Returns:
            Process information dictionary
        """
        try:
            process = psutil.Process()
            return {
                "pid": process.pid,
                "memory_percent": process.memory_percent(),
                "cpu_percent": process.cpu_percent(),
                "num_threads": process.num_threads(),
                "open_files": len(process.open_files()),
                "connections": len(process.connections()),
            }
        except Exception:
            return {}


# ============================================================================
# INACTIVITY DETECTOR LAYER
# ============================================================================

class InactivityDetectorLayer(HookLayer):
    """
    Inactivity detection hook layer

    Detects periods of inactivity and can trigger
    context capture or other actions.
    """

    def __init__(
        self,
        config: Optional[Dict[str, Any]] = None,
        memory_manager: Optional[MemoryManager] = None,
    ) -> None:
        super().__init__(config)
        self.memory_manager: Optional[MemoryManager] = memory_manager
        self.threshold_seconds: float = float(self.config.get("threshold_seconds", 30))
        self._last_activity = datetime.now(UTC)
        self._lock = Lock()

    def start(self) -> None:
        """Start the inactivity detector"""
        # Activity is tracked via record_activity calls
        pass

    def stop(self) -> None:
        """Stop the inactivity detector"""
        pass

    def record_activity(self) -> None:
        """Record that activity occurred"""
        with self._lock:
            self._last_activity = datetime.now(UTC)

    def check_inactive(self) -> bool:
        """
        Check if the session has been inactive

        Returns:
            True if inactive beyond threshold
        """
        with self._lock:
            inactive_duration = (datetime.now(UTC) - self._last_activity).total_seconds()
            return inactive_duration >= self.threshold_seconds

    def get_inactive_duration(self) -> float:
        """
        Get the duration of inactivity

        Returns:
            Inactive duration in seconds
        """
        with self._lock:
            return (datetime.now(UTC) - self._last_activity).total_seconds()


# ============================================================================
# PERSISTENT BUFFER LAYER
# ============================================================================

class PersistentBufferLayer(HookLayer):
    """
    Persistent buffer hook layer

    Issue #25: Uses thread pool executor for non-blocking database I/O.

    Buffers memories in memory and periodically flushes
    them to persistent storage. Provides durability for
    captured context.
    """

    def __init__(
        self,
        config: Optional[Dict[str, Any]] = None,
        memory_manager: Optional[MemoryManager] = None,
    ) -> None:
        super().__init__(config)
        self.memory_manager: Optional[MemoryManager] = memory_manager
        self.buffer_size: int = int(self.config.get("buffer_size", 1000))
        self.flush_interval: float = float(self.config.get("flush_interval", 5))
        self._buffer: deque[Dict[str, Any]] = deque(maxlen=self.buffer_size)
        self._lock = Lock()
        self._running: bool = False
        self._thread: Optional[Thread] = None
        self._flush_event = Event()
        # Issue #25: Thread pool for non-blocking database operations
        self._executor: Optional[ThreadPoolExecutor] = None
        self._max_workers: int = int(self.config.get("max_workers", 2))

    def start(self) -> None:
        """Start the background flush thread"""
        if not self.enabled:
            return

        self._running = True
        # Issue #25: Initialize thread pool for database operations
        self._executor = ThreadPoolExecutor(
            max_workers=self._max_workers,
            thread_name_prefix="buffer_flush"
        )
        self._thread = Thread(target=self._flush_loop, daemon=True)
        self._thread.start()

    def stop(self) -> None:
        """Stop the background flush thread"""
        self._running = False
        self._flush_event.set()
        if self._thread:
            self._thread.join(timeout=5)
            self._thread = None

        # Issue #25: Shutdown thread pool
        if self._executor:
            self._executor.shutdown(wait=True)
            self._executor = None

        # Final flush
        self.flush()

    def _flush_loop(self) -> None:
        """Background flush loop"""
        while self._running:
            self._flush_event.wait(self.flush_interval)
            self._flush_event.clear()
            if self._running:
                self.flush()

    def _submit_db_write(self, item: Dict[str, Any]) -> Optional[Future[Optional[Memory]]]:
        """
        Submit a database write operation to the thread pool.

        Issue #25: Non-blocking database I/O using thread pool executor.

        Args:
            item: Memory data to write

        Returns:
            Future for the write operation (or None if executed synchronously)
        """
        memory_manager = self.memory_manager
        if memory_manager is None:
            return None

        if not self._executor:
            # Fallback to synchronous if executor not available
            try:
                memory_manager.create_memory(**item)
            except Exception:
                pass
            return None

        def _write() -> Optional[Memory]:
            try:
                return memory_manager.create_memory(**item)
            except Exception as e:
                logger.warning(f"Failed to write buffered memory: {e}")
                return None

        return self._executor.submit(_write)

    def add_to_buffer(
        self,
        content: str,
        category: str = MemoryCategory.CONTEXT.value,
        **kwargs: Any,
    ) -> None:
        """
        Add a memory to the buffer

        Args:
            content: Memory content
            category: Memory category
            **kwargs: Additional memory attributes
        """
        with self._lock:
            self._buffer.append({
                "content": content,
                "category": category,
                **kwargs,
            })

    def flush(self) -> int:
        """
        Flush buffered memories to storage

        Issue #25: Uses thread pool for non-blocking writes.

        Returns:
            Number of memories flushed
        """
        if not self.memory_manager:
            return 0

        with self._lock:
            to_flush = list(self._buffer)
            self._buffer.clear()

        flushed = 0
        futures = []

        # Issue #25: Submit all writes to thread pool
        for item in to_flush:
            future = self._submit_db_write(item)
            if future:
                futures.append(future)

        # Wait for all pending writes
        for future in futures:
            try:
                if future.result(timeout=1.0):
                    flushed += 1
            except Exception:
                pass  # Already logged in _write

        return flushed

    def get_buffer_size(self) -> int:
        """Get current buffer size"""
        with self._lock:
            return len(self._buffer)


# ============================================================================
# UNIFIED HOOK MANAGER
# ============================================================================

class UnifiedHookManager:
    """
    Unified manager for all hook layers

    Coordinates multiple hook layers and provides a single
    interface for context capture throughout the workflow.
    """

    def __init__(
        self,
        db_path: Optional[str] = None,
        config: Optional[Dict[str, Any]] = None,
    ) -> None:
        """
        Initialize the unified hook manager

        Args:
            db_path: Path to the database
            config: Optional configuration dictionary
        """
        self.config = config or {}
        self.db_path = db_path or os.path.expanduser("~/.maestro/memory.db")

        # Ensure database exists
        self._ensure_database()

        # Create managers
        self._session = self._get_session()
        self.memory_manager = MemoryManager(self._session)
        self.session_manager = SessionManager(self._session)
        self.project_manager = ProjectManager(self._session)

        # Coordination handlers
        self.file_claims = FileClaimsHandler(self._session)
        self.handoffs = HandoffHandler(self._session)
        self.ledgers = ContinuityLedgerHandler(self._session)

        # Hook layers
        hook_config = self.config.get("hooks", {})
        native_config = hook_config.get("native", {})
        process_config = hook_config.get("process_monitor", {})
        inactivity_config = hook_config.get("inactivity_detector", {})
        buffer_config = hook_config.get("persistent_buffer", {})

        self.native_layer = NativeHookLayer(
            native_config,
            self.memory_manager,
        )
        self.process_layer = ProcessMonitorLayer(
            process_config,
            self.memory_manager,
        )
        self.inactivity_layer = InactivityDetectorLayer(
            inactivity_config,
            self.memory_manager,
        )
        self.buffer_layer = PersistentBufferLayer(
            buffer_config,
            self.memory_manager,
        )

        self._current_session_id: Optional[str] = None
        self._current_agent_id: Optional[str] = None

    def _ensure_database(self) -> None:
        """Ensure the database exists and has the schema"""
        from maestro.memory.database.models import create_tables
        create_tables(db_path=self.db_path)

    def _get_session(self) -> OrmSession:
        """Get a database session"""
        engine = create_engine(f"sqlite:///{self.db_path}")
        SessionLocal = sessionmaker(bind=engine)
        return SessionLocal()

    def start(self) -> None:
        """Start all hook layers"""
        self.process_layer.start()
        self.buffer_layer.start()
        self.inactivity_layer.start()
        self.native_layer.start()

    def stop(self) -> None:
        """Stop all hook layers"""
        self.process_layer.stop()
        self.buffer_layer.stop()
        self.inactivity_layer.stop()
        self.native_layer.stop()

        self._session.close()

    @contextmanager
    def session_context(
        self,
        session_id: str,
        agent_id: str,
        agent_name: Optional[str] = None,
        project_path: Optional[str] = None,
    ) -> Iterator["UnifiedHookManager"]:
        """
        Context manager for a session

        Args:
            session_id: Session identifier
            agent_id: Agent identifier
            agent_name: Optional agent name
            project_path: Optional project path

        Yields:
            The UnifiedHookManager instance
        """
        # Create session record
        project_id: Optional[int] = None
        if project_path:
            project = self.project_manager.get_or_create_project(project_path)
            project_id = cast(int, project.id)

        session = self.session_manager.create_session(
            session_id=session_id,
            session_type="agent",
            agent_id=agent_id,
            agent_name=agent_name,
            project_path=project_path,
            project_id=project_id,
        )
        self._session.commit()

        self._current_session_id = session_id
        self._current_agent_id = agent_id

        # Start hooks
        self.start()

        try:
            self.record_activity()
            yield self
        finally:
            # End session
            self.session_manager.end_session(session_id)
            self._session.commit()

            # Stop hooks
            self.stop()

            self._current_session_id = None
            self._current_agent_id = None

    def record_activity(self) -> None:
        """Record that activity occurred"""
        self.inactivity_layer.record_activity()

    def capture_memory(
        self,
        content: str,
        category: str = MemoryCategory.CONTEXT.value,
        importance: str = MemoryImportance.NORMAL.value,
        summary: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
        use_buffer: bool = True,
    ) -> Optional[Memory]:
        """
        Capture a memory

        Args:
            content: Memory content
            category: Memory category
            importance: Memory importance
            summary: Optional summary
            metadata: Optional metadata
            use_buffer: Whether to use the buffer layer

        Returns:
            Created Memory instance
        """
        self.record_activity()

        if use_buffer and self.buffer_layer.enabled:
            self.buffer_layer.add_to_buffer(
                content=content,
                category=category,
                importance=importance,
                summary=summary,
                session_id=self._current_session_id,
                source=self._current_agent_id,
                metadata=metadata,
            )
            return None

        return self.memory_manager.create_memory(
            content=content,
            category=category,
            importance=importance,
            summary=summary,
            session_id=self._current_session_id,
            source=self._current_agent_id,
            metadata=metadata,
        )

    def create_file_claim(
        self,
        file_patterns: List[str],
        reason: Optional[str] = None,
        ttl_seconds: Optional[int] = None,
    ) -> Optional[Any]:
        """
        Create a file claim for the current agent

        Args:
            file_patterns: File patterns to claim
            reason: Optional reason
            ttl_seconds: Optional TTL

        Returns:
            Created FileClaim instance
        """
        if not self._current_agent_id:
            return None

        self.record_activity()

        return self.file_claims.create_claim(
            agent_id=self._current_agent_id,
            file_patterns=file_patterns,
            session_id=self._current_session_id,
            reason=reason,
            ttl_seconds=ttl_seconds,
        )

    def create_handoff(
        self,
        title: str,
        context_data: Dict[str, Any],
        summary: Optional[str] = None,
    ) -> Optional[Any]:
        """
        Create a handoff from the current session

        Args:
            title: Handoff title
            context_data: Handoff context
            summary: Optional summary

        Returns:
            Created Handoff instance
        """
        if not self._current_session_id or not self._current_agent_id:
            return None

        self.record_activity()

        return self.handoffs.create_handoff(
            from_session_id=self._current_session_id,
            from_agent_id=self._current_agent_id,
            title=title,
            context_data=context_data,
            summary=summary,
        )

    def create_ledger_entry(
        self,
        entry_type: EntryType,
        title: str,
        content: str,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Optional[Any]:
        """
        Create a continuity ledger entry

        Args:
            entry_type: Entry type
            title: Entry title
            content: Entry content
            metadata: Optional metadata

        Returns:
            Created ContinuityLedger instance
        """
        if not self._current_session_id or not self._current_agent_id:
            return None

        self.record_activity()

        return self.ledgers.create_entry(
            session_id=self._current_session_id,
            agent_id=self._current_agent_id,
            entry_type=entry_type,
            title=title,
            content=content,
            metadata=metadata,
        )

    def recall(
        self,
        query: str,
        category: Optional[str] = None,
        limit: int = 10,
    ) -> List[Memory]:
        """
        Recall memories matching a query

        Args:
            query: Search query
            category: Optional category filter
            limit: Maximum results

        Returns:
            List of matching Memory instances
        """
        results = self.memory_manager.search_memories(
            query=query,
            category=category,
            limit=limit,
        )
        return cast(List[Memory], results)

    def get_status(self) -> Dict[str, Any]:
        """
        Get the current status of the hook manager

        Returns:
            Status dictionary
        """
        return {
            "current_session_id": self._current_session_id,
            "current_agent_id": self._current_agent_id,
            "native_enabled": self.native_layer.enabled,
            "process_monitoring": self.process_layer.enabled,
            "inactivity_detection": self.inactivity_layer.enabled,
            "buffer_enabled": self.buffer_layer.enabled,
            "buffer_size": self.buffer_layer.get_buffer_size(),
            "inactive_duration": self.inactivity_layer.get_inactive_duration(),
            "is_inactive": self.inactivity_layer.check_inactive(),
            "process_info": self.process_layer.get_process_info(),
        }


# Global instance with thread-safe initialization
_global_hook_manager: Optional[UnifiedHookManager] = None
_manager_lock = Lock()
_manager_initialized = False


def get_hook_manager(
    db_path: Optional[str] = None,
    config: Optional[Dict[str, Any]] = None,
) -> UnifiedHookManager:
    """
    Get the global hook manager instance.

    This function is thread-safe and uses proper locking to ensure
    only one UnifiedHookManager instance is created. The db_path and
    config are only used on first initialization; subsequent calls
    ignore them to prevent multiple instances.

    Args:
        db_path: Optional database path (only used on first call)
        config: Optional configuration (only used on first call)

    Returns:
        UnifiedHookManager instance
    """
    global _global_hook_manager, _manager_initialized

    # Fast path: return existing instance if already initialized
    if _manager_initialized and _global_hook_manager is not None:
        return _global_hook_manager

    # Slow path: initialize with lock
    with _manager_lock:
        # Double-check after acquiring lock
        if _global_hook_manager is None:
            _global_hook_manager = UnifiedHookManager(
                db_path=db_path,
                config=config,
            )
            _manager_initialized = True
        else:
            # Ensure flag is set if instance already exists
            _manager_initialized = True

        return _global_hook_manager


def shutdown_hook_manager() -> None:
    """Shutdown the global hook manager.

    This function is thread-safe. It stops the hook manager and
    resets the initialization flag so a subsequent call to
    get_hook_manager will create a new instance.
    """
    global _global_hook_manager, _manager_initialized

    with _manager_lock:
        if _global_hook_manager:
            try:
                _global_hook_manager.stop()
            except Exception:
                # Ignore errors during shutdown
                pass
            _global_hook_manager = None
            _manager_initialized = False
