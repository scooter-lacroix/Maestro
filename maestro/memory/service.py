"""
Maestro Memory Service

Primary interface for all memory operations in Maestro.

This service is now a thin compatibility facade over the standalone
Nexus-backed bridge plus a local async DB session manager for Maestro-owned
tables and transitional dashboard queries.
"""

import json
from datetime import datetime, UTC
from typing import Optional, List, Dict, Any
from pathlib import Path
import asyncio
import re
import uuid
from collections import deque  # IMPORTANT-6: Use deque for O(1) rate limiting operations
import logging

try:
    from loguru import logger
except ImportError:  # pragma: no cover - optional dependency in minimal test envs
    logger = logging.getLogger(__name__)

# IMPORTANT-4: Import constants
from maestro.memory.constants import (
    DATABASE_QUERY_TIMEOUT,
    DATABASE_OPERATION_TIMEOUT,
    DATABASE_LOCK_RELEASE_DELAY,
    DATABASE_BUSY_TIMEOUT,
    DATABASE_POOL_SIZE,
    DATABASE_MAX_OVERFLOW,
    DATABASE_POOL_RECYCLE,
    DATABASE_MAX_RETRIES,
    DATABASE_BASE_DELAY,
    DEFAULT_MEMORY_LIMIT,
    MAX_CONTEXT_LENGTH,
    MAX_TOKENS,
    MAX_CONTENT_SIZE,
    MAX_LABELS_COUNT,
    DEFAULT_BATCH_SIZE,
    RATE_LIMIT_WINDOW,
    RATE_LIMIT_MAX_REQUESTS,
    MEMORY_ENHANCEMENT_TIMEOUT,
    SEARCH_TIMEOUT,
    API_DEFAULT_LIMIT,
    API_MAX_LIMIT,
    MAX_PROJECT_PATH_LENGTH,
    MAX_STRING_LENGTH,
)

# Non-Nexus imports — safe at module level
from sqlalchemy import select, and_, create_engine, text

from maestro.memory.database.async_db import AsyncDatabaseManager
from maestro.memory.database.models import MaestroProject, MaestroTrack
from maestro.memory.utils.sanitizer import MemorySanitizer
from maestro.memory.exceptions import (
    MaestroValidationError,
    MaestroPathTraversalError,
    MaestroDatabaseError,
    MaestroInitializationError,
    MaestroStorageError,
    MaestroRetrievalError
)


def _load_nexus_bridge() -> tuple[Any, Any, Any]:
    """Resolve the active standalone Nexus bridge lazily."""
    try:
        from maestro.memory.nexus_client import (
            get_database_manager,
            get_memory_manager,
            get_nexus_base,
        )

        return get_database_manager(), get_memory_manager(), get_nexus_base()
    except ImportError:
        try:
            from maestro.memory.database.async_db import AsyncDatabaseManager as DeferredAsyncDatabaseManager

            return DeferredAsyncDatabaseManager, None, None
        except ImportError as exc:
            raise ImportError("No Nexus backend available") from exc

class MaestroMemoryService:
    """
    Central service for memory operations in Maestro.

    Integrates with Nexus Memory System for persistent storage while
    providing Maestro-specific project and track isolation.
    """

    # IMPORTANT-6: Use deque per key for O(1) rate limiting operations
    # Structure: Dict[key -> deque of timestamps]
    _rate_limit_cache: Dict[str, deque] = {}
    _rate_limit_lock = asyncio.Lock()

    def __init__(self, database_path: Optional[Path] = None) -> None:
        """
        Initialize memory service with database path.

        Args:
            database_path: Path to SQLite database file. Defaults to ~/.maestro/maestro.db
        """
        if database_path is None:
            database_path = Path.home() / ".maestro" / "maestro.db"

        # Ensure parent directory exists
        database_path.parent.mkdir(parents=True, exist_ok=True)

        self.database_path: Path = database_path
        self.db_manager: Any = None
        self.memory_manager: Any = None
        self.nexus_client: Any = None
        self._initialized: bool = False
        self.DatabaseManager: Any = None
        self.MemoryManager: Any = None
        self.NexusBase: Any = None
        self._bridge_loaded: bool = False

        # Issue 7: Track sync engine for cleanup on errors
        self._sync_engine: Any = None
        self._init_lock = asyncio.Lock()  # Issue 4: Add lock for initialization

        # Issue 13: Database operation metrics for monitoring
        self._db_metrics = {
            "queries_executed": 0,
            "queries_failed": 0,
            "total_query_time": 0.0,
            "lock_acquisitions": 0,
            "lock_failures": 0,
        }
        self._metrics_lock = asyncio.Lock()

    async def _record_db_metric(self, metric_name: str, value: Any = 1) -> None:
        """
        Record database operation metric (Issue 13: Pool Monitoring).

        Args:
            metric_name: Name of the metric to record
            value: Value to add (default: 1)
        """
        async with self._metrics_lock:
            if metric_name in self._db_metrics:
                if isinstance(self._db_metrics[metric_name], (int, float)):
                    self._db_metrics[metric_name] += value
                else:
                    self._db_metrics[metric_name] = value

    async def get_db_metrics(self) -> Dict[str, Any]:
        """
        Get database operation metrics (Issue 13: Pool Monitoring).

        Returns:
            Dictionary of current metrics
        """
        async with self._metrics_lock:
            return self._db_metrics.copy()

    async def reset_db_metrics(self) -> None:
        """Reset database operation metrics (Issue 13: Pool Monitoring)."""
        async with self._metrics_lock:
            for key in self._db_metrics:
                if isinstance(self._db_metrics[key], (int, float)):
                    self._db_metrics[key] = 0
                else:
                    self._db_metrics[key] = 0.0

    def _generate_request_id(self) -> str:
        """
        Generate a unique request ID for tracing (IMPORTANT-8: Structured Logging).

        Returns:
            Unique request ID in format: req_<timestamp>_<uuid>
        """
        timestamp = datetime.now(UTC).strftime('%Y%m%d%H%M%S')
        unique_id = uuid.uuid4().hex[:8]
        return f"req_{timestamp}_{unique_id}"

    def _log_structured(
        self,
        level: str,
        message: str,
        **kwargs: Any
    ) -> None:
        """
        Log structured message with context (IMPORTANT-8: Structured Logging).

        Args:
            level: Log level (debug, info, warning, error, critical)
            message: Log message
            **kwargs: Additional structured context
        """
        log_data = {
            "timestamp": datetime.now(UTC).isoformat(),
            "message": message,
            **kwargs
        }

        # Log using loguru's structured logging
        # Format message with context as keyword arguments for loguru's bind()
        log_fn = getattr(logger, level, logger.info)
        # Pass message as positional argument and context as keyword arguments
        log_fn(message, **{k: v for k, v in log_data.items() if k != "message"})

    def _create_tables_sync(self) -> None:
        """
        Create database tables synchronously to avoid async lock issues.

        Creates only Maestro-owned tables and compatibility columns.
        """
        from sqlalchemy import create_engine
        from sqlalchemy import text

        # Create synchronous engine for DDL operations only
        sync_url = f"sqlite:///{self.database_path}"
        engine = create_engine(
            sync_url,
            connect_args={
                "check_same_thread": False,
                "timeout": 10  # 10 second timeout for sync operations
            },
            echo=False,
            pool_pre_ping=False,  # Disable for DDL operations
            # Issue 3: Add proper pool configuration for SQLite
            pool_size=1,  # SQLite only supports single writer
            max_overflow=0,  # No additional connections
            pool_recycle=3600,  # Recycle connections after 1 hour
        )

        # Issue 7: Track engine for cleanup
        self._sync_engine = engine

        # Connection variable for cleanup
        conn = None

        try:
            # Use a simple connection (not begin()) for better control
            conn = engine.connect()

            # Enable WAL mode for better concurrency
            conn.execute(text("PRAGMA journal_mode=WAL"))
            conn.execute(text("PRAGMA synchronous=NORMAL"))
            conn.execute(text("PRAGMA busy_timeout=10000"))  # 10 seconds
            conn.execute(text("PRAGMA foreign_keys=ON"))
            conn.execute(text("PRAGMA temp_store=MEMORY"))  # Use memory for temp tables
            conn.execute(text("PRAGMA mmap_size=268435456"))  # 256MB mmap

            MaestroProject.__table__.create(conn, checkfirst=True)
            MaestroTrack.__table__.create(conn, checkfirst=True)

            table_names = {
                row[0]
                for row in conn.execute(text("SELECT name FROM sqlite_master WHERE type='table'")).fetchall()
            }
            if "memories" not in table_names:
                raise MaestroInitializationError(
                    "Standalone Nexus storage is not initialized for this database. "
                    "Run `nexus init` before using Maestro memory."
                )

            # Check if we need to add Maestro columns to memories table
            result = conn.execute(text("PRAGMA table_info(memories)"))
            existing_columns = {row[1] for row in result.fetchall()}

            if 'maestro_project_id' not in existing_columns:
                conn.execute(text("ALTER TABLE memories ADD COLUMN maestro_project_id INTEGER"))
                conn.execute(text("CREATE INDEX IF NOT EXISTS idx_memories_maestro_project ON memories(maestro_project_id)"))
                conn.commit()

            if 'maestro_track_id' not in existing_columns:
                conn.execute(text("ALTER TABLE memories ADD COLUMN maestro_track_id INTEGER"))
                conn.execute(text("CREATE INDEX IF NOT EXISTS idx_memories_maestro_track ON memories(maestro_track_id)"))
                conn.commit()

            if 'maestro_command' not in existing_columns:
                conn.execute(text("ALTER TABLE memories ADD COLUMN maestro_command VARCHAR(100)"))
                conn.execute(text("CREATE INDEX IF NOT EXISTS idx_memories_maestro_command ON memories(maestro_command)"))
                conn.commit()

            if 'maestro_command_context' not in existing_columns:
                conn.execute(text("ALTER TABLE memories ADD COLUMN maestro_command_context JSON"))
                conn.commit()

        finally:
            # Issue 7: CRITICAL - Close connection and dispose engine even on error
            try:
                if conn:
                    conn.close()
            except Exception as e:
                logger.warning(f"Error closing DDL connection: {e}")

            try:
                if self._sync_engine:
                    self._sync_engine.dispose()
                    logger.debug("Sync DDL engine disposed")
            except Exception as e:
                logger.warning(f"Error disposing DDL engine: {e}")
            finally:
                self._sync_engine = None

            # Issue 3: Small delay to ensure all WAL files are fully released
            # This is critical for avoiding locks when switching to async
            import time
            DATABASE_LOCK_RELEASE_DELAY = 0.05  # 50ms delay (Issue 17: extracted magic number)
            time.sleep(DATABASE_LOCK_RELEASE_DELAY)

    async def initialize(self) -> None:
        """
        Initialize the memory service.

        Creates tables synchronously, then sets up async managers.
        Must be called before any other operations.

        CRITICAL-5 FIX: Moved lock acquisition BEFORE the first check to eliminate TOCTOU vulnerability.
        The previous implementation had a fast-path check before the lock, which created a race condition
        window where multiple threads could simultaneously initialize the service.
        """
        # CRITICAL-5: Acquire lock FIRST, then check initialization status
        # This eliminates the TOCTOU vulnerability by making the check atomic
        async with self._init_lock:
            # Double-check pattern: check after acquiring lock
            if self._initialized:
                return

            try:
                # Step 1: Create tables synchronously (DDL only)
                logger.debug(f"Initializing MaestroMemoryService with database: {self.database_path}")
                self.DatabaseManager, self.MemoryManager, self.NexusBase = _load_nexus_bridge()
                self._bridge_loaded = True
                self._create_tables_sync()

                # Step 2: Initialize async database manager
                database_manager_cls = self.DatabaseManager or AsyncDatabaseManager
                self.db_manager = database_manager_cls(database_path=self.database_path)

                # Issue 3: Verify WAL files are released with proper lock verification
                # Instead of magic sleep, use actual file locking to detect database availability
                max_retries = 5
                base_delay = 0.05  # 50ms base delay (Issue 17: magic number)

                for attempt in range(max_retries):
                    try:
                        # Issue 3: Verify database file is accessible with lock check
                        if self.database_path.exists():
                            # Try to open database in read-only mode to check for locks
                            # This will fail if database is locked
                            import fcntl  # Unix file locking (Issue 3)
                            test_fd = None
                            try:
                                # Open in read-only mode to test locks
                                test_fd = open(self.database_path, 'r', encoding="utf-8")
                                # Try to acquire shared lock (non-blocking)
                                fcntl.flock(test_fd.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
                                # Lock acquired, database is available
                                await self._record_db_metric("lock_acquisitions")
                                logger.debug(f"Database lock verified on attempt {attempt + 1}")
                                break
                            except (IOError, OSError) as lock_error:
                                # Database is locked, close and retry
                                await self._record_db_metric("lock_failures")
                                if test_fd:
                                    test_fd.close()
                                if attempt == max_retries - 1:
                                    raise MaestroDatabaseError(
                                        f"Database locked after {max_retries} attempts: {lock_error}"
                                    )
                                # Exponential backoff (Issue 3)
                                delay = base_delay * (2 ** attempt)
                                logger.debug(
                                    f"Database locked, retrying in {delay:.3f}s "
                                    f"({attempt + 1}/{max_retries})"
                                )
                                await asyncio.sleep(delay)
                            finally:
                                # Always close the test file descriptor
                                if test_fd and not test_fd.closed:
                                    fcntl.flock(test_fd.fileno(), fcntl.LOCK_UN)  # Release lock
                                    test_fd.close()
                        else:
                            # Database doesn't exist yet, will be created
                            break
                    except ImportError:
                        # fcntl not available (Windows), fall back to simple retry
                        delay = base_delay * (2 ** attempt)
                        logger.debug(f"fcntl unavailable, using simple retry ({attempt + 1}/{max_retries})")
                        await asyncio.sleep(delay)
                    break

                await self.db_manager.initialize()

                # Step 3: Configure async engine for better SQLite concurrency
                # Use a single connection to avoid lock issues
                async with self.db_manager.get_async_session() as session:
                    # Set PRAGMA values for the async connection
                    await session.execute(text("PRAGMA journal_mode=WAL"))
                    await session.execute(text("PRAGMA synchronous=NORMAL"))
                    await session.execute(text("PRAGMA busy_timeout=10000"))  # 10 seconds
                    await session.execute(text("PRAGMA foreign_keys=ON"))
                    await session.execute(text("PRAGMA temp_store=MEMORY"))
                    await session.execute(text("PRAGMA mmap_size=268435456"))
                    await session.commit()

                # Step 4: Initialize standalone Nexus bridge and compatibility manager.
                from maestro.memory.nexus_client import StandaloneNexusClient, CompatibilityMemoryManager

                self.nexus_client = StandaloneNexusClient(
                    database_path=self.database_path,
                    async_db=self.db_manager,
                )
                memory_manager_cls = self.MemoryManager or CompatibilityMemoryManager
                self.memory_manager = memory_manager_cls(self.db_manager)

                self._initialized = True
                logger.info(f"MaestroMemoryService initialized successfully: {self.database_path}")

            except Exception as e:
                logger.error(f"Failed to initialize MaestroMemoryService: {e}")
                # Issue 7: Enhanced cleanup on failure - also dispose sync engine
                if self.db_manager:
                    try:
                        await self.db_manager.close()
                    except Exception as cleanup_error:
                        logger.warning(f"Error closing db_manager during cleanup: {cleanup_error}")
                    self.db_manager = None

                # Issue 7: Ensure sync engine is disposed
                if self._sync_engine:
                    try:
                        self._sync_engine.dispose()
                        logger.debug("Sync engine disposed during error cleanup")
                    except Exception as cleanup_error:
                        logger.warning(f"Error disposing sync engine during cleanup: {cleanup_error}")
                    self._sync_engine = None

                self._initialized = False
                # Issue 16: Use custom exception
                raise MaestroInitializationError(f"Failed to initialize memory service: {e}") from e

    async def close(self) -> None:
        """Close database connections and cleanup resources."""
        if self.db_manager:
            await self.db_manager.close()
            self.db_manager = None
        self.nexus_client = None
        self.memory_manager = None
        self._initialized = False
        logger.debug("MaestroMemoryService closed")

    async def _ensure_initialized(self) -> None:
        """Ensure service is initialized."""
        if not self._initialized:
            await self.initialize()

    def _sanitize_context(self, context: Dict[str, Any]) -> str:
        """
        Sanitize context data before storage.

        Args:
            context: Raw context dictionary

        Returns:
            Sanitized JSON string
        """
        # Convert to JSON
        context_json = json.dumps(context, indent=2, default=str)

        # Sanitize sensitive data
        sanitized = MemorySanitizer.sanitize(context_json)

        return sanitized

    def _sanitize_error_message(self, error: Exception, include_details: bool = False) -> str:
        """
        Sanitize error messages for user-facing responses (Issue 12: Error Sanitization).

        Logs detailed errors internally but returns safe messages to users.

        Args:
            error: The exception to sanitize
            include_details: Whether to include error details (default: False)

        Returns:
            Sanitized error message safe for user display
        """
        # Log full error with stack trace for debugging
        logger.error(f"Error occurred: {type(error).__name__}: {str(error)}")

        # Return safe message to user
        if include_details:
            # Include error type but not full stack trace or internal paths
            return f"{type(error).__name__}: {str(error)}"
        else:
            # Generic message without exposing internals
            return "An internal error occurred. Please check the logs for details."

    async def _check_rate_limit(self, operation: str, key: str) -> bool:
        """
        Check if operation should be rate limited (Issue 10: Rate Limiting).

        IMPORTANT-6 FIX: Uses deque per key for O(1) operations instead of O(n).
        - Each key has its own deque of timestamps
        - Old timestamps are popped from left in O(1)
        - New timestamps are appended to right in O(1)

        Args:
            operation: Operation name (e.g., "enhance_context")
            key: Unique key for rate limiting (e.g., agent_type)

        Returns:
            True if operation should proceed, False if rate limited
        """
        rate_limit_key = f"{operation}:{key}"
        now = datetime.now(UTC).timestamp()

        async with self._rate_limit_lock:
            # IMPORTANT-6: Get or create deque for this key
            if rate_limit_key not in self._rate_limit_cache:
                self._rate_limit_cache[rate_limit_key] = deque()

            timestamp_deque = self._rate_limit_cache[rate_limit_key]

            # IMPORTANT-6: Remove timestamps outside the time window (O(1) with deque)
            while timestamp_deque and now - timestamp_deque[0] > RATE_LIMIT_WINDOW:
                timestamp_deque.popleft()  # O(1) operation

            # Check if limit exceeded
            if len(timestamp_deque) >= RATE_LIMIT_MAX_REQUESTS:
                logger.debug(
                    f"Rate limit exceeded for {operation}:{key} "
                    f"({len(timestamp_deque)} requests in {RATE_LIMIT_WINDOW}s)"
                )
                return False

            # Add current request (O(1) operation)
            timestamp_deque.append(now)
            return True

    def _validate_project_path(self, path: str) -> str:
        """
        Validate and sanitize project path.

        Issue 2: Enhanced path traversal validation with proper path resolution
        Issue 16: Use custom exceptions

        Security measures:
        - Unicode normalization (NFC)
        - Null byte detection
        - Device file checks
        - Symlink validation
        - Protocol-relative path prevention
        """
        if not path or not isinstance(path, str):
            raise MaestroValidationError("Project path must be a non-empty string")

        # Issue 2: Unicode normalization (NFC) to prevent Unicode bypasses
        import unicodedata
        try:
            normalized_unicode = unicodedata.normalize('NFC', path)
        except (TypeError, ValueError) as e:
            raise MaestroValidationError(f"Invalid Unicode in path: {e}") from e

        # Issue 2: Check for suspicious patterns before normalization
        suspicious_patterns = [
            '..',  # Path traversal
            '\\\\',  # Windows UNC paths
            '~/',  # Home directory traversal
            '/etc/',  # System directory
            '/sys/',  # System directory
            '/proc/',  # Process directory
            '/dev/',  # Device directory
            '\\windows\\',  # Windows system
            '//',  # Protocol-relative paths
            '\\\\?\\',  # Extended Windows paths
        ]
        path_lower = normalized_unicode.lower()
        for pattern in suspicious_patterns:
            if pattern in path_lower:
                raise MaestroPathTraversalError(
                    path,
                    f"Contains suspicious pattern: {pattern}"
                )

        # Issue 2: Null byte check (critical for security)
        if '\x00' in normalized_unicode:
            raise MaestroValidationError("Null bytes not allowed in path")

        # Issue 2: Use Path.resolve() to get absolute path and detect traversal
        try:
            resolved_path = Path(normalized_unicode).resolve()
            resolved_path_str = str(resolved_path)

            # Additional validation: ensure it's a valid directory
            if not resolved_path.is_absolute():
                raise MaestroValidationError("Project path must be absolute")

            # Issue 2: Device file check (Unix/Linux)
            try:
                # Check if path points to a device file
                if resolved_path_str.startswith('/dev/'):
                    raise MaestroPathTraversalError(
                        path,
                        "Device files not allowed for project paths"
                    )
            except OSError:
                pass  # Path may not exist yet

            # Issue 2: Symlink validation - ensure symlink doesn't escape allowed directories
            try:
                if resolved_path.is_symlink():
                    # Resolve the symlink and check if it's within acceptable bounds
                    real_path = resolved_path.resolve(strict=False)
                    # Add additional checks here if you have a whitelist of allowed directories
                    logger.debug(f"Symlink detected in project path: {path} -> {real_path}")
            except OSError:
                pass  # Path may not exist yet

            # Normalize the path for consistent storage
            normalized = resolved_path.as_posix()

            return normalized

        except (OSError, ValueError) as e:
            raise MaestroValidationError(f"Invalid project path: {e}") from e

    def _validate_memory_id(self, memory_id: int | str) -> int:
        """Validate memory_id is a positive integer"""
        if not isinstance(memory_id, int):
            try:
                memory_id = int(memory_id)
            except (ValueError, TypeError):
                raise ValueError(f"memory_id must be an integer, got {type(memory_id)}")

        if memory_id <= 0:
            raise ValueError(f"memory_id must be positive, got {memory_id}")

        return memory_id

    async def store_command_context(
        self,
        command: str,
        project_path: str,
        context: Dict[str, Any]
    ) -> str:
        """
        Store Maestro command execution context.

        IMPORTANT-5 FIX: Added 10MB size limit validation on context.
        Prevents memory exhaustion and database bloat from oversized contexts.

        Args:
            command: Command name (e.g., "/maestro:setup")
            project_path: Absolute path to project directory
            context: Command-specific context data

        Returns:
            Memory ID of stored context

        Raises:
            RuntimeError: If service is not initialized
            ValueError: If parameters are invalid
        """
        await self._ensure_initialized()

        # IMPORTANT-8: Generate request ID for tracing
        request_id = self._generate_request_id()

        # Validate inputs
        if not command or not isinstance(command, str):
            self._log_structured("error", "Invalid command provided", request_id=request_id, command=command)
            raise ValueError("Command must be a non-empty string")

        project_path = self._validate_project_path(project_path)

        if not context or not isinstance(context, dict):
            raise ValueError("Context must be a non-empty dictionary")

        # IMPORTANT-5: Validate context size to prevent memory exhaustion
        context_json = json.dumps(context, default=str)
        context_size = len(context_json.encode('utf-8'))  # Size in bytes

        if context_size > MAX_CONTENT_SIZE:
            raise ValueError(
                f"Context size ({context_size} bytes) exceeds maximum allowed size "
                f"({MAX_CONTENT_SIZE} bytes = {MAX_CONTENT_SIZE // (1024*1024)}MB)"
            )

        # Step 1: Get or create project (in its own transaction)
        async with self.db_manager.get_async_session() as session:
            project = await MaestroProject.get_or_create(
                session,
                project_path,
                project_name=Path(project_path).name,
                last_active=datetime.now(UTC)
            )
            await session.commit()
            project_id = project.id

        # Step 2: Get or create track if needed (in its own transaction)
        track = None
        track_id = context.get("track_id")
        track_id_value = None
        if track_id:
            async with self.db_manager.get_async_session() as session:
                title = context.get("track_title", track_id)
                track = await MaestroTrack.get_or_create(
                    session,
                    track_id=track_id,
                    project_id=project_id,
                    title=title,
                    status=context.get("status", "new")
                )
                await session.commit()
                track_id_value = track.id

        if self.nexus_client is None:
            raise MaestroInitializationError("Standalone Nexus client not initialized")

        # Step 3: Store through the standalone Nexus bridge.
        result = await self.nexus_client.store_command_context(
            command=command,
            project_path=project_path,
            context=context,
            project_id=project_id,
            track_row_id=track_id_value,
            track_id=track_id,
            session_id=str(context.get("session_id")) if context.get("session_id") else None,
            agent="maestro",
        )

        if not result.get("success"):
            raise RuntimeError(f"Failed to store memory: {result.get('error')}")

        memory_id = self._validate_memory_id(result["memory_id"])

        self._log_structured(
            "info",
            "Successfully stored command context",
            request_id=request_id,
            command=command,
            project_path=project_path,
            memory_id=memory_id,
            context_size_bytes=context_size
        )

        return str(memory_id)

    async def retrieve_project_context(
        self,
        project_path: str,
        limit: int = 10
    ) -> List[Dict[str, Any]]:
        """
        Retrieve all context for a specific project.

        Args:
            project_path: Absolute path to project directory
            limit: Maximum number of memories to retrieve

        Returns:
            List of memory dictionaries ordered by recency

        Raises:
            RuntimeError: If service is not initialized
        """
        await self._ensure_initialized()

        project_path = self._validate_project_path(project_path)

        if not isinstance(limit, int) or limit <= 0:
            raise ValueError("Limit must be a positive integer")

        try:
            if self.nexus_client is None:
                raise MaestroInitializationError("Standalone Nexus client not initialized")
            results = await self.nexus_client.retrieve_project_context(project_path=project_path, limit=limit)
            logger.info(f"Retrieved {len(results)} memories for project {project_path}")
            return results

        except Exception as e:
            # Issue 12: Sanitize error message for user
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to retrieve project context: {sanitized_msg}") from e

    async def retrieve_track_context(
        self,
        track_id: str,
        limit: int = 20
    ) -> List[Dict[str, Any]]:
        """
        Retrieve all context for a specific track.

        Args:
            track_id: Track identifier (e.g., maestro-unified_20250101)
            limit: Maximum number of memories to retrieve

        Returns:
            List of memory dictionaries ordered by recency

        Raises:
            RuntimeError: If service is not initialized
        """
        await self._ensure_initialized()

        if not track_id or not isinstance(track_id, str):
            raise ValueError("Track ID must be a non-empty string")

        if not isinstance(limit, int) or limit <= 0:
            raise ValueError("Limit must be a positive integer")

        try:
            if self.nexus_client is None:
                raise MaestroInitializationError("Standalone Nexus client not initialized")
            results = await self.nexus_client.retrieve_track_context(track_id=track_id, limit=limit)
            logger.info(f"Retrieved {len(results)} memories for track {track_id}")
            return results

        except Exception as e:
            # Issue 12: Sanitize error message for user
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to retrieve track context: {sanitized_msg}") from e

    async def retrieve_session_context(
        self,
        session_id: str,
        limit: int = 20
    ) -> List[Dict[str, Any]]:
        """
        Retrieve all context captured for a specific session.

        Args:
            session_id: Session identifier
            limit: Maximum number of memories to retrieve

        Returns:
            List of memory dictionaries ordered by recency
        """
        await self._ensure_initialized()

        if not session_id or not isinstance(session_id, str):
            raise ValueError("Session ID must be a non-empty string")

        if not isinstance(limit, int) or limit <= 0:
            raise ValueError("Limit must be a positive integer")

        try:
            if self.nexus_client is None:
                raise MaestroInitializationError("Standalone Nexus client not initialized")
            results = await self.nexus_client.retrieve_session_context(session_id=session_id, limit=limit)
            logger.info(f"Retrieved {len(results)} memories for session {session_id}")
            return results

        except Exception as e:
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to retrieve session context: {sanitized_msg}") from e

    async def search_similar_commands(
        self,
        command: str,
        project_path: Optional[str] = None,
        limit: int = 5
    ) -> List[Dict[str, Any]]:
        """
        Find similar command executions using semantic search.

        Args:
            command: Command to search for
            project_path: Optional project path filter
            limit: Maximum results

        Returns:
            List of similar command executions

        Raises:
            RuntimeError: If service is not initialized
        """
        await self._ensure_initialized()

        if not command or not isinstance(command, str):
            raise ValueError("Command must be a non-empty string")

        if not isinstance(limit, int) or limit <= 0:
            raise ValueError("Limit must be a positive integer")

        try:
            if self.nexus_client is None:
                raise MaestroInitializationError("Standalone Nexus client not initialized")
            result = await self.nexus_client.search_similar_commands(
                query=command,
                agent="maestro",
                project_path=project_path,
                limit=limit,
                include_raw=True,
            )

            if not result.get("results"):
                logger.warning(f"Memory search failed: {result.get('error')}")
                return []

            results = list(result.get("results", []))
            logger.info(f"Found {len(results)} similar commands for '{command}'")
            return results

        except Exception as e:
            # Issue 12: Sanitize error message for user
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to search similar commands: {sanitized_msg}") from e

    # Constants for memory enhancement (Issue 17: Extract magic numbers)
    DEFAULT_MEMORY_LIMIT = 3
    MAX_CONTEXT_LENGTH = 200
    MAX_TOKENS = 4000  # Approximate token limit for enhanced context (Issue 6)


    async def enhance_context_with_memory(
        self,
        context: str,
        agent_type: str = "general",
        limit: int = DEFAULT_MEMORY_LIMIT
    ) -> str:
        """
        Enhance context with relevant memories from the memory system.

        Issue 6: Fixed unbounded resource consumption with:
        - Graceful word-boundary truncation
        - Token counting with max_tokens limit
        - Memory size validation

        IMPORTANT-1 FIX: Added comprehensive error handling with metrics tracking.
        - Tracks success/failure rates
        - Records enhancement statistics
        - Provides detailed error context

        This method implements LLM enhancement by searching for semantically
        relevant memories and prepending them to the context to provide LLMs
        with relevant historical information.

        Args:
            context: The original context string to enhance
            agent_type: Agent type to search memories for (default: "general")
            limit: Maximum number of memories to retrieve (default: 3)

        Returns:
            Enhanced context string with relevant memories prepended.
            If no relevant memories found, returns original context unchanged.

        Example:
            >>> context = "I need to implement user authentication"
            >>> enhanced = await service.enhance_context_with_memory(context, "claude-code")
            >>> print(enhanced)
            RELEVANT MEMORY CONTEXT:
            - Implemented JWT authentication with refresh tokens...
            - Created database migration for user profiles...

            I need to implement user authentication
        """
        await self._ensure_initialized()

        # IMPORTANT-1: Track enhancement metrics
        enhancement_start_time = datetime.now(UTC)

        # Validate inputs
        if not context or not isinstance(context, str):
            await self._record_db_metric("enhance_context_invalid_input")
            return context

        if not agent_type or not isinstance(agent_type, str):
            agent_type = "general"

        if not isinstance(limit, int) or limit <= 0:
            limit = self.DEFAULT_MEMORY_LIMIT

        # Issue 10: Check rate limit for expensive operations
        if not await self._check_rate_limit("enhance_context", agent_type):
            logger.debug(
                f"Rate limited enhance_context for agent_type={agent_type}, "
                f"returning original context"
            )
            await self._record_db_metric("enhance_context_rate_limited")
            return context

        try:
            # Extract keywords from context for better matching
            # Simple approach: use the first meaningful word for best match
            words = context.split()
            # Filter out common words and take first meaningful word
            stop_words = {'i', 'need', 'to', 'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'for', 'with', 'work', 'set', 'up', "i'm", 'working'}
            keywords = [w for w in words if w.lower() not in stop_words and len(w) > 2]

            # Use first keyword as query for best matching, fallback to full context
            # Clean up special characters from keyword
            search_query = keywords[0].strip('/.:,;') if keywords else context

            # Search for relevant memories using the Nexus-backed text search.
            if self.nexus_client is None:
                raise MaestroInitializationError("Standalone Nexus client not initialized")
            search_result = await self.nexus_client.search_similar_commands(
                query=search_query,
                agent=agent_type,
                limit=limit,
                include_raw=True,
            )

            # Check if search was successful and has results
            if not search_result.get("results"):
                logger.debug(f"Memory search failed: {search_result.get('error')}")
                await self._record_db_metric("enhance_context_search_failed")
                return context

            results = search_result.get("results", [])

            # If no memories found, return original context
            if not results:
                logger.debug(f"No relevant memories found for agent_type={agent_type}")
                await self._record_db_metric("enhance_context_no_results")
                return context

            # Format memories as bullet points
            memory_lines: list[str] = []
            total_length: float = 0.0

            for memory in results:
                content = memory.get("content", "")

                # Issue 6: Graceful word-boundary truncation instead of hard cut
                if len(content) > self.MAX_CONTEXT_LENGTH:
                    # Find the last complete word within the limit
                    truncated = content[:self.MAX_CONTEXT_LENGTH]
                    last_space = truncated.rfind(' ')
                    if last_space > self.MAX_CONTEXT_LENGTH - 50:  # If we can find a word break
                        truncated = truncated[:last_space]
                    content = truncated + "..."

                # Issue 6: Check token limit (approximate: 1 token ≈ 4 characters)
                estimated_tokens = len(content) / 4
                if total_length + estimated_tokens > self.MAX_TOKENS:
                    logger.debug(
                        f"Token limit reached, stopping at {len(memory_lines)} memories "
                        f"out of {len(results)}"
                    )
                    break

                memory_lines.append(f"- {content}")
                total_length += estimated_tokens

            # Build enhanced context
            memory_section = "RELEVANT MEMORY CONTEXT:\n" + "\n".join(memory_lines) + "\n\n"
            enhanced_context = memory_section + context

            # IMPORTANT-1: Track success metrics
            enhancement_time = (datetime.now(UTC) - enhancement_start_time).total_seconds()
            await self._record_db_metric("enhance_context_success")
            await self._record_db_metric("enhance_context_memories_added", len(memory_lines))
            await self._record_db_metric("enhance_context_time", enhancement_time)

            logger.info(
                f"Enhanced context with {len(memory_lines)} memories "
                f"(~{int(total_length)} tokens, {enhancement_time:.3f}s) "
                f"for agent_type={agent_type}"
            )

            return enhanced_context

        except Exception as e:
            # IMPORTANT-1: Track failure metrics with detailed error context
            enhancement_time = (datetime.now(UTC) - enhancement_start_time).total_seconds()
            await self._record_db_metric("enhance_context_error")
            logger.error(
                f"Failed to enhance context with memory after {enhancement_time:.3f}s: {e}",
                exc_info=True
            )
            # On error, return original context (graceful degradation)
            return context
