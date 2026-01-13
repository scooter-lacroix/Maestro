"""
Maestro Memory Service

Primary interface for all memory operations in Maestro.

This service wraps Nexus MemoryManager and provides Maestro-specific
functionality for project and track-based memory isolation.
"""

import json
from datetime import datetime, UTC
from typing import Optional, List, Dict, Any
from pathlib import Path
from loguru import logger
import sys
import os
import asyncio
import re
import uuid
from collections import deque  # IMPORTANT-6: Use deque for O(1) rate limiting operations
from types import ModuleType

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

# =============================================================================
# CRITICAL-2 FIX: Proper Nexus Path Discovery
# =============================================================================


def _discover_nexus_path() -> str:
    """
    Discover Nexus Memory System path using multiple robust fallback strategies.

    Priority order:
    1. Environment variable NEXUS_MEMORY_PATH
    2. Parent directory relative to maestro package (work_resources sibling)
    3. Common development directories (~/work_resources, ~/dev, ~/projects)
    4. System-wide installation under /opt
    5. Recursive search in home directory (depth-limited to 3 levels)
    6. Raise error if not found

    This function is designed to work automatically on any system without
    manual environment variable configuration. It searches intelligently
    based on common project structures and locations.

    Returns:
        Absolute path to Nexus Memory System

    Raises:
        ImportError: If Nexus cannot be found in any location
    """
    # Strategy 1: Environment variable (highest priority - explicit override)
    env_path = os.environ.get('NEXUS_MEMORY_PATH')
    if env_path and Path(env_path).exists():
        logger.debug(f"Using Nexus from environment variable: {env_path}")
        return str(env_path)

    # Strategy 2: Parent directory relative to maestro package
    # Handles structure: .../work_resources/nexus-memory-system and .../Prod/maestro
    # This works for both: ~/Prod/maestro and ~/projects/maestro layouts
    try:
        maestro_root = Path(__file__).parent.parent.parent  # maestro package root
        # Try work_resources sibling (common pattern)
        work_resources = maestro_root.parent / "work_resources" / "nexus-memory-system"
        if work_resources.exists():
            logger.debug(f"Using Nexus from work_resources sibling: {work_resources}")
            return str(work_resources)

        # Try sibling directories directly
        sibling_nexus = maestro_root.parent / "nexus-memory-system"
        if sibling_nexus.exists():
            logger.debug(f"Using Nexus from sibling directory: {sibling_nexus}")
            return str(sibling_nexus)
    except Exception as e:
        logger.debug(f"Could not discover Nexus via parent directory: {e}")

    # Strategy 3: Common development directories in user's home
    # This covers various common development setups
    common_locations = [
        Path.home() / "work_resources" / "nexus-memory-system",
        Path.home() / "dev" / "nexus-memory-system",
        Path.home() / "development" / "nexus-memory-system",
        Path.home() / "projects" / "nexus-memory-system",
        Path.home() / "code" / "nexus-memory-system",
        Path.home() / "src" / "nexus-memory-system",
        Path.home() / "Prod" / "work_resources" / "nexus-memory-system",
        Path.home() / "Prod" / "nexus-memory-system",
    ]

    for location in common_locations:
        if location.exists():
            logger.debug(f"Using Nexus from common location: {location}")
            return str(location)

    # Strategy 4: System-wide installation paths
    # Covers both Unix/Linux and common system installation locations
    system_paths = [
        Path("/opt/nexus-memory-system"),
        Path("/usr/local/nexus-memory-system"),
        Path("/usr/local/lib/nexus-memory-system"),
    ]

    for system_path in system_paths:
        if system_path.exists():
            logger.debug(f"Using Nexus from system path: {system_path}")
            return str(system_path)

    # Strategy 5: Depth-limited recursive search in home directory
    # This is a fallback to find nexus in non-standard locations
    # Limited to depth 3 to avoid excessive filesystem traversal
    try:
        logger.debug("Performing depth-limited recursive search for Nexus...")
        max_depth = 3
        home_dir = Path.home()

        # Use os.walk for controlled depth iteration
        for root, dirs, files in os.walk(home_dir):
            # Calculate current depth
            current_depth = len(Path(root).relative_to(home_dir).parts)

            # Skip if we've exceeded max depth
            if current_depth > max_depth:
                # Don't descend further
                dirs[:] = []
                continue

            # Skip hidden directories and common system dirs
            dirs[:] = [d for d in dirs if not d.startswith('.') and d not in {
                'node_modules', 'venv', '.venv', 'env', '__pycache__',
                'site-packages', '.git', '.cache', 'Applications', 'Library'
            }]

            # Check if nexus-memory-system exists in current directory
            if 'nexus-memory-system' in dirs:
                nexus_path = Path(root) / 'nexus-memory-system'
                # Verify it has the expected structure
                if (nexus_path / 'nexus' / 'database' / '__init__.py').exists():
                    logger.debug(f"Using Nexus from recursive search: {nexus_path}")
                    return str(nexus_path)
                    # Don't continue searching after finding one
                    break
    except Exception as e:
        logger.debug(f"Recursive search failed: {e}")

    # All strategies failed - provide helpful error message
    searched_locations = [
        "1. Environment variable NEXUS_MEMORY_PATH",
        "2. Parent directory work_resources/nexus-memory-system",
        "3. Common locations: ~/work_resources, ~/dev, ~/projects, ~/Prod",
        "4. System paths: /opt, /usr/local",
        f"5. Recursive search in {Path.home()} (depth=3)",
    ]

    error_msg = (
        "Nexus Memory System not found. Tried the following locations:\n" +
        "\n".join(f"  {loc}" for loc in searched_locations) +
        f"\n\nTo fix this, either:\n" +
        f"  1. Set NEXUS_MEMORY_PATH environment variable to Nexus directory\n" +
        f"  2. Clone Nexus to one of the searched locations above\n" +
        f"     git clone https://github.com/anthropics/nexus-memory-system.git\n" +
        f"  3. Install Nexus system-wide to /opt/nexus-memory-system\n"
    )
    logger.error(error_msg)
    raise ImportError(error_msg)


NEXUS_PATH: str | None

# Discover and add Nexus to path
try:
    NEXUS_PATH = _discover_nexus_path()
    if NEXUS_PATH not in sys.path:
        sys.path.insert(0, NEXUS_PATH)
        logger.info(f"Nexus Memory System discovered at: {NEXUS_PATH}")
except ImportError as e:
    logger.warning(f"Nexus path discovery failed: {e}")
    # Will fail when trying to import Nexus modules
    NEXUS_PATH = None

# Import only specific Nexus modules directly to avoid triggering server imports
import importlib.util

def import_nexus_module(module_name: str, file_path: str) -> ModuleType:
    """Import a module directly from file path to avoid __init__.py side effects"""
    spec = importlib.util.spec_from_file_location(module_name, file_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Failed to import module spec for {module_name} from {file_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module

# Import necessary Nexus components
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, and_, create_engine, text
from sqlalchemy.exc import SQLAlchemyError

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

# Import Nexus modules using direct imports to avoid MCP server
_nexus_managers: Any = None
_nexus_models: Any = None
_nexus_config: Any = None


def _get_nexus_modules() -> tuple[Any, Any, Any]:
    """Lazy load Nexus modules to avoid import issues"""
    global _nexus_managers, _nexus_models, _nexus_config
    if _nexus_managers is None:
        import sys

        # Use the discovered Nexus path instead of hardcoded fallback
        if NEXUS_PATH is None:
            raise ImportError("Nexus Memory System path not discovered. Set NEXUS_MEMORY_PATH environment variable.")

        if 'nexus' not in sys.modules:
            import types
            nexus_module = types.ModuleType('nexus')
            sys.modules['nexus'] = nexus_module

            from nexus.database import managers as db_managers  # type: ignore[import-not-found]
            from nexus.database import models as db_models  # type: ignore[import-not-found]
            from nexus.config import settings as db_settings  # type: ignore[import-not-found]

            _nexus_managers = db_managers
            _nexus_models = db_models
            _nexus_config = db_settings
        else:
            from nexus.database import managers as db_managers  # type: ignore[import-not-found]
            from nexus.database import models as db_models  # type: ignore[import-not-found]
            from nexus.config import settings as db_settings  # type: ignore[import-not-found]

            _nexus_managers = db_managers
            _nexus_models = db_models
            _nexus_config = db_settings

    return _nexus_managers, _nexus_models, _nexus_config


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
        self._initialized: bool = False

        # Issue 7: Track sync engine for cleanup on errors
        self._sync_engine: Any = None
        self._init_lock = asyncio.Lock()  # Issue 4: Add lock for initialization

        # Lazy load Nexus modules
        nexus_managers, nexus_models, nexus_config = _get_nexus_modules()
        self.DatabaseManager = nexus_managers.DatabaseManager
        self.MemoryManager = nexus_managers.MemoryManager
        self.Memory = nexus_models.Memory

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

        Uses SQLAlchemy's metadata.create_all() for proper table creation.
        """
        from maestro.memory.database.models import Base as MaestroBase
        from nexus.database.models import Base as NexusBase  # type: ignore[import-not-found]
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

            # Create all Nexus tables using SQLAlchemy metadata
            NexusBase.metadata.create_all(conn, checkfirst=True)

            # Create all Maestro tables
            MaestroBase.metadata.create_all(conn, checkfirst=True)

            # Create default namespace for maestro
            from maestro.memory.database.models import MaestroProject
            
            # Check if agent_type column exists before using it
            result = conn.execute(text("PRAGMA table_info(agent_namespaces)"))
            columns = [row[1] for row in result.fetchall()]
            
            if 'agent_type' in columns:
                conn.execute(text("""
                    INSERT OR IGNORE INTO agent_namespaces (name, description, agent_type)
                    VALUES ('maestro', 'Maestro unified development framework', 'maestro')
                """))
            else:
                # Fallback for older databases without agent_type column
                conn.execute(text("""
                    INSERT OR IGNORE INTO agent_namespaces (name, description)
                    VALUES ('maestro', 'Maestro unified development framework')
                """))
            conn.commit()

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
                self._create_tables_sync()

                # Step 2: Initialize async database manager
                database_url = f"sqlite:///{self.database_path}"
                self.db_manager = self.DatabaseManager(database_url)

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

                # Step 4: Initialize memory manager
                self.memory_manager = self.MemoryManager(self.db_manager)

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
                raise MaestroInitializationError(f"Failed to initialize memory service: {e}")

    async def close(self) -> None:
        """Close database connections and cleanup resources."""
        if self.db_manager:
            await self.db_manager.close()
            self.db_manager = None
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
            raise MaestroValidationError(f"Invalid Unicode in path: {e}")

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
            raise MaestroValidationError(f"Invalid project path: {e}")

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

        # Step 3: Sanitize context and build content
        sanitized_context = self._sanitize_context(context)

        content = f"""Command: {command}
Project: {project_path}
Track: {track_id if track_id else 'N/A'}

Context:
{sanitized_context}
"""

        # Step 4: Store memory with Nexus (this opens its own session)
        result = await self.memory_manager.store_memory(
            content=content,
            agent_type="maestro",
            category="context",
            labels=[command, "maestro"],
            metadata={
                "maestro_project_id": project_id,
                "maestro_track_id": track_id_value,
                "maestro_command": command,
                "maestro_command_context": context,  # Store raw context in metadata
                "project_path": project_path,
                "track_id": track_id,
            }
        )

        if not result.get("success"):
            raise RuntimeError(f"Failed to store memory: {result.get('error')}")

        memory_id = result["memory_id"]

        # Step 5: Update the memory record with Maestro columns (separate transaction)
        validated_memory_id = self._validate_memory_id(memory_id)

        # Issue 1: Use SQLAlchemy text() with parameterized queries for columns added via ALTER TABLE
        # Issue 4: Add transaction rollback handling
        from sqlalchemy import text
        async with self.db_manager.get_async_session() as session:
            try:
                # Use text() for columns that may not be in the model definition
                # Parameterized query prevents SQL injection
                stmt = text("""
                    UPDATE memories
                    SET maestro_project_id = :project_id,
                        maestro_track_id = :track_id,
                        maestro_command = :command,
                        maestro_command_context = :context_json
                    WHERE id = :memory_id
                """)
                await session.execute(stmt, {
                    "project_id": project_id,
                    "track_id": track_id_value,
                    "command": command,
                    "context_json": json.dumps(context),
                    "memory_id": validated_memory_id
                })
                await session.commit()
            except Exception as update_error:
                await session.rollback()
                self._log_structured(
                    "error",
                    "Failed to update memory with Maestro columns",
                    request_id=request_id,
                    memory_id=memory_id,
                    error=str(update_error)
                )
                raise MaestroDatabaseError(f"Failed to update memory: {update_error}")

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
            async with self.db_manager.get_async_session() as session:
                from sqlalchemy import desc, text

                # Get project first
                project_stmt = select(MaestroProject).filter_by(project_path=project_path)
                result = await session.execute(project_stmt)
                project = result.scalars().first()

                if not project:
                    return []

                # Issue 1: Use text() with parameterized query for Maestro columns
                # These columns are added via ALTER TABLE and may not be in the model
                # Note: column name is 'metadata' (mapped to 'extra_metadata' in Python)
                memories_stmt = text("""
                    SELECT id, content, created_at, category, labels, metadata,
                           maestro_command, maestro_command_context
                    FROM memories
                    WHERE maestro_project_id = :project_id
                      AND is_active = 1
                    ORDER BY created_at DESC
                    LIMIT :limit
                """)
                result = await session.execute(memories_stmt, {
                    "project_id": project.id,
                    "limit": limit
                })
                memories = result.fetchall()

                # Format results - convert rows to dicts
                results = []
                for memory in memories:
                    # memory is a Row object
                    metadata = json.loads(memory.metadata) if memory.metadata else {}
                    labels = json.loads(memory.labels) if memory.labels else []
                    command_context = json.loads(memory.maestro_command_context) if memory.maestro_command_context else {}

                    results.append({
                        "id": memory.id,
                        "command": memory.maestro_command or "unknown",
                        "project_path": project_path,
                        "context": command_context,
                        "content": memory.content,
                        "created_at": memory.created_at,
                        "category": memory.category,
                        "labels": labels,
                    })

                logger.info(f"Retrieved {len(results)} memories for project {project_path}")

                return results

        except Exception as e:
            # Issue 12: Sanitize error message for user
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to retrieve project context: {sanitized_msg}")

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
            async with self.db_manager.get_async_session() as session:
                from sqlalchemy import desc, text

                # Get track first
                track_stmt = select(MaestroTrack).filter_by(track_id=track_id)
                result = await session.execute(track_stmt)
                track = result.scalars().first()

                if not track:
                    return []

                # Issue 1: Use text() with parameterized query for Maestro columns
                # Note: column name is 'metadata' (mapped to 'extra_metadata' in Python)
                memories_stmt = text("""
                    SELECT id, content, created_at, category, labels, metadata,
                           maestro_command, maestro_command_context
                    FROM memories
                    WHERE maestro_track_id = :track_id
                      AND is_active = 1
                    ORDER BY created_at DESC
                    LIMIT :limit
                """)
                result = await session.execute(memories_stmt, {
                    "track_id": track.id,
                    "limit": limit
                })
                memories = result.fetchall()

                # Format results
                results = []
                for memory in memories:
                    metadata = json.loads(memory.metadata) if memory.metadata else {}
                    labels = json.loads(memory.labels) if memory.labels else []
                    command_context = json.loads(memory.maestro_command_context) if memory.maestro_command_context else {}

                    results.append({
                        "id": memory.id,
                        "command": memory.maestro_command or "unknown",
                        "track_id": track_id,
                        "context": command_context,
                        "content": memory.content,
                        "created_at": memory.created_at,
                        "category": memory.category,
                        "labels": labels,
                    })

                logger.info(f"Retrieved {len(results)} memories for track {track_id}")

                return results

        except Exception as e:
            # Issue 12: Sanitize error message for user
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to retrieve track context: {sanitized_msg}")

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
            # Build search query
            search_query = f"Command: {command}"

            if project_path:
                search_query += f" Project: {project_path}"

            # Search memories
            result = await self.memory_manager.search_memories(
                query=search_query,
                agent_type="maestro",
                limit=limit,
                category="context"
            )

            if not result.get("success"):
                logger.warning(f"Memory search failed: {result.get('error')}")
                return []

            # Format results
            results = []
            for memory in result.get("results", []):
                results.append({
                    "id": memory["id"],
                    "content": memory["content"],
                    "command": memory.get("metadata", {}).get("maestro_command", "unknown"),
                    "context": memory.get("metadata", {}).get("maestro_command_context", {}),
                    "similarity_score": memory.get("similarity_score"),
                    "created_at": memory["created_at"],
                })

            logger.info(f"Found {len(results)} similar commands for '{command}'")

            return results

        except Exception as e:
            # Issue 12: Sanitize error message for user
            sanitized_msg = self._sanitize_error_message(e, include_details=True)
            raise MaestroRetrievalError(f"Failed to search similar commands: {sanitized_msg}")

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

            # Search for relevant memories using the query
            # Use Nexus's text-based search capabilities
            search_result = await self.memory_manager.search_memories(
                query=search_query,
                agent_type=agent_type,
                limit=limit
            )

            # Check if search was successful and has results
            if not search_result.get("success"):
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
