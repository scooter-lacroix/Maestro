"""
Database Managers for Maestro Memory System

Provides CRUD operations for all database models with support for
search, filtering, and embeddings-based semantic search.

Includes:
- Audit logging for all mutable operations (Issue #20, #21)
- Disk space checks before write operations (Issue #16)
- Row-level locking for concurrent operations (Issue #15)
- Access control hooks (Issue #20)
- Query result caching (Issue #26)
"""

from datetime import datetime, timedelta, UTC
import threading
from threading import Thread
from typing import Optional, Dict, Any, List, Type, TypeVar, Generic, cast, Generator
from contextlib import contextmanager
import uuid
import logging
import os
from functools import lru_cache
import hashlib
import json

from sqlalchemy import select, update, delete, func, and_, or_, text
from sqlalchemy.engine import CursorResult
from sqlalchemy.orm import Session as ORMSession, joinedload, selectinload


# ============================================================================
# QUERY RESULT CACHE (Issue #26)
# ============================================================================

class QueryResultCache:
    """
    Simple LRU cache for database query results.

    Issue #26: Implements caching with automatic invalidation on writes.
    Thread-safe for concurrent access.
    """

    def __init__(self, max_size: int = 128, ttl_seconds: int = 300):
        """
        Initialize the query cache.

        Args:
            max_size: Maximum number of cached results
            ttl_seconds: Time-to-live for cache entries (default 5 minutes)
        """
        self._cache: Dict[str, tuple[List[Any], datetime]] = {}  # key -> (result, expiry_time)
        self._max_size = max_size
        self._ttl_seconds = ttl_seconds
        self._lock = self._create_lock()

    def _create_lock(self) -> "threading.Lock":
        """Create a lock for thread-safe access"""
        import threading
        return threading.Lock()

    def _generate_key(self, model_name: str, query_params: Dict[str, Any]) -> str:
        """Generate a cache key from model name and query parameters"""
        # Sort params for consistent keys
        sorted_params = sorted(query_params.items())
        param_str = json.dumps(sorted_params, sort_keys=True, default=str)
        combined = f"{model_name}:{param_str}"
        return hashlib.md5(combined.encode()).hexdigest()

    def get(self, model_name: str, query_params: Dict[str, Any]) -> Optional[List[Any]]:
        """
        Get cached results if available and not expired.

        Args:
            model_name: Name of the model being queried
            query_params: Query parameters as dictionary

        Returns:
            Cached results or None if not found/expired
        """
        key = self._generate_key(model_name, query_params)

        with self._lock:
            if key not in self._cache:
                return None

            result, expiry = self._cache[key]
            if datetime.now(UTC) > expiry:
                # Expired, remove from cache
                del self._cache[key]
                return None

            # Move to end (LRU)
            self._cache[key] = (result, expiry)
            return result

    def set(self, model_name: str, query_params: Dict[str, Any], result: List[Any]) -> None:
        """
        Cache query results.

        Args:
            model_name: Name of the model being queried
            query_params: Query parameters as dictionary
            result: Query results to cache
        """
        key = self._generate_key(model_name, query_params)
        expiry = datetime.now(UTC) + timedelta(seconds=self._ttl_seconds)

        with self._lock:
            # Remove oldest if at capacity
            if len(self._cache) >= self._max_size and key not in self._cache:
                # Remove first item (oldest)
                oldest_key = next(iter(self._cache))
                del self._cache[oldest_key]

            self._cache[key] = (result, expiry)

    def invalidate(self, model_name: str, record_id: Optional[int] = None) -> None:
        """
        Invalidate cached results for a model.

        Issue #26: Called on writes to ensure cache consistency.

        Args:
            model_name: Name of the model to invalidate
            record_id: Optional specific record ID (for more selective invalidation)
        """
        with self._lock:
            # For simplicity, clear all on any write to ensure consistency
            # Since we use hash-based keys, we can't easily filter by model_name
            self._cache.clear()

    def clear(self) -> None:
        """Clear all cached results"""
        with self._lock:
            self._cache.clear()

    def stats(self) -> Dict[str, Any]:
        """Get cache statistics"""
        with self._lock:
            return {
                "size": len(self._cache),
                "max_size": self._max_size,
                "ttl_seconds": self._ttl_seconds,
            }


# Global cache instance
_global_cache: Optional[QueryResultCache] = None


def get_query_cache() -> QueryResultCache:
    """Get the global query cache instance"""
    global _global_cache
    if _global_cache is None:
        _global_cache = QueryResultCache()
    return _global_cache

from maestro.memory.database.models import (
    Memory,
    MemoryCategory,
    MemoryImportance,
    AgentNamespace,
    NamespaceMemory,
    FileClaim,
    ClaimStatus,
    Handoff,
    HandoffStatus,
    ContinuityLedger,
    TaskSpecification,
    Session,
    SessionStatus,
    MaestroProject,
    MaestroTrack,
    Base,
    ValidationError,
    validate_content_length,
    validate_json_field,
    sanitize_string,
    check_access,
    AccessDecision,
    log_audit_event,
    DiskSpaceError,
    ensure_disk_space_before_write,
    generate_unique_session_id,
    UUIDCollisionError,
    validate_no_secrets,  # Issue #23: Secret detection
    SecretDetectedError,  # Issue #23: Secret detection error
)

T = TypeVar("T", bound=Base)

logger = logging.getLogger(__name__)


# ============================================================================
# CONCURRENT MODIFICATION ERROR (Issue #15)
# ============================================================================

class ConcurrentModificationError(Exception):
    """
    Raised when a concurrent modification is detected.

    Issue #29: Enhanced error message with remediation steps.
    """

    def __init__(
        self,
        message: str,
        entity_type: str,
        entity_id: Any,
        expected_version: int,
        actual_version: int,
    ) -> None:
        remediation = (
            f"Remediation: Re-fetch the {entity_type} and retry the update. "
            f"Current version is {actual_version}, expected was {expected_version}."
        )
        full_message = f"{message} {remediation}"
        super().__init__(full_message)
        self.entity_type = entity_type
        self.entity_id = entity_id
        self.expected_version = expected_version
        self.actual_version = actual_version


class TransactionError(Exception):
    """Raised when a database transaction fails"""
    pass


class DatabaseManager:
    """
    Base database manager with common CRUD operations

    Provides generic create, read, update, and delete operations
    for any SQLAlchemy model with transaction management.

    Includes audit logging, access control checks, and disk space monitoring.
    Issue #26: Includes query result caching.
    """

    # Maximum retry attempts for transient failures
    MAX_RETRIES = 3

    # Default user/agent ID for audit logging when not specified
    _default_user_id: Optional[str] = None

    # Cache configuration
    _use_cache: bool = True

    @classmethod
    def set_default_user_id(cls, user_id: str) -> None:
        """Set the default user ID for audit logging."""
        cls._default_user_id = user_id

    @classmethod
    def set_cache_enabled(cls, enabled: bool) -> None:
        """Enable or disable query caching globally."""
        cls._use_cache = enabled

    def __init__(self, session: ORMSession, user_id: Optional[str] = None, use_cache: Optional[bool] = None) -> None:
        """
        Initialize the manager with a database session

        Issue #26: Added optional caching support.

        Args:
            session: SQLAlchemy session
            user_id: Optional user/agent ID for audit logging
            use_cache: Enable/disable caching for this instance (default: global setting)
        """
        self.session = session
        self._user_id = user_id or self._default_user_id
        self._use_cache = use_cache if use_cache is not None else self._use_cache
        self._cache = get_query_cache() if self._use_cache else None

        # Issue #31: Initialize thread tracking attributes
        self._cleanup_thread: Optional[Thread] = None
        self._cleanup_running: bool = False

    def _get_user_id(self) -> str:
        """Get the user ID for audit logging."""
        return self._user_id or "system"

    @contextmanager
    def _transaction(self, check_disk_space: bool = True) -> Generator[None, None, None]:
        """
        Context manager for explicit transaction handling with retry logic.

        Args:
            check_disk_space: Whether to check disk space before transaction

        Yields:
            None

        Raises:
            TransactionError: If transaction fails after retries
            DiskSpaceError: If insufficient disk space
        """
        # Issue #16: Pre-flight disk space check
        if check_disk_space:
            try:
                # Get database path from session
                bind = self.session.bind
                url = getattr(bind, "url", None)
                db_path = getattr(url, "database", None)
                if db_path:
                    ensure_disk_space_before_write(db_path)
            except DiskSpaceError:
                raise
            except Exception as e:
                logger.warning(f"Disk space check skipped: {e}")

        for attempt in range(self.MAX_RETRIES):
            try:
                # Begin transaction explicitly
                self.session.begin()
                yield
                # Commit if no exception
                self.session.commit()
                return
            except Exception as e:
                # Rollback on error
                self.session.rollback()

                # Log audit event for transaction failure
                if attempt == self.MAX_RETRIES - 1:
                    log_audit_event(
                        operation="transaction_failed",
                        entity_type="database",
                        entity_id="transaction",
                        user_id=self._get_user_id(),
                        metadata={"attempts": attempt + 1, "error": str(e)},
                        status="failure"
                    )
                    raise TransactionError(
                        f"Transaction failed after {self.MAX_RETRIES} attempts: {e}"
                    ) from e
                logger.warning(
                    f"Transaction attempt {attempt + 1} failed, retrying: {e}"
                )

    def _check_access(self, operation: str, model: Type[T]) -> AccessDecision:
        """
        Check access control for an operation.

        Args:
            operation: Type of operation (create, read, update, delete)
            model: Model class

        Returns:
            AccessDecision with allowed flag
        """
        entity_type = model.__name__
        decision = check_access(operation, entity_type, None, self._get_user_id())

        if not decision.allowed:
            log_audit_event(
                operation=f"{operation}_denied",
                entity_type=entity_type,
                entity_id=None,
                user_id=self._get_user_id(),
                metadata={"reason": decision.reason},
                status="denied"
            )

        return decision

    def create(self, model: Type[T], **kwargs: Any) -> T:
        """
        Create a new database record with validation and transaction management.

        Issue #26: Invalidates cache on write.

        Args:
            model: SQLAlchemy model class
            **kwargs: Model attributes

        Returns:
            Created model instance

        Raises:
            ValidationError: If validation fails
            TransactionError: If transaction fails
            PermissionError: If access denied
        """
        # Issue #20: Access control check
        decision = self._check_access("create", model)
        if not decision.allowed:
            raise PermissionError(f"Access denied: {decision.reason}")

        # Sanitize string inputs
        sanitized = {}
        for key, value in kwargs.items():
            if isinstance(value, str):
                sanitized[key] = sanitize_string(value)
            elif isinstance(value, (dict, list)):
                sanitized[key] = validate_json_field(value, f"{model.__name__}.{key}")
            else:
                sanitized[key] = value

        instance = model(**sanitized)

        # Set audit user ID if the model supports it
        if hasattr(instance, 'set_audit_user'):
            instance.set_audit_user(self._get_user_id())

        self.session.add(instance)
        self.session.flush()

        # Issue #21: Log audit event
        entity_id = getattr(instance, 'id', None) or getattr(instance, 'session_id', None)
        log_audit_event(
            operation="create",
            entity_type=model.__name__,
            entity_id=entity_id,
            user_id=self._get_user_id(),
            status="success"
        )

        # Call instance's audit log if available
        if hasattr(instance, '_log_create'):
            instance._log_create()

        # Issue #26: Invalidate cache on write
        if self._cache:
            self._cache.invalidate(model.__name__)

        return instance

    def get_by_id(self, model: Type[T], record_id: int) -> Optional[T]:
        """
        Get a record by ID

        Args:
            model: SQLAlchemy model class
            record_id: Primary key ID

        Returns:
            Model instance or None
        """
        stmt = select(model).where(getattr(model, "id") == record_id)
        return self.session.execute(stmt).scalar_one_or_none()

    def get_all(
        self,
        model: Type[T],
        limit: int = 100,
        offset: int = 0,
        use_cache: Optional[bool] = None,
        **filters: Any,
    ) -> List[T]:
        """
        Get all records matching filters

        Issue #26: Uses cache for frequently accessed data.

        Args:
            model: SQLAlchemy model class
            limit: Maximum records to return
            offset: Number of records to skip
            use_cache: Override default cache behavior
            **filters: Attribute filters

        Returns:
            List of model instances
        """
        # Issue #20: Access control check for reads
        decision = self._check_access("read", model)
        if not decision.allowed:
            logger.warning(f"Read access denied to {model.__name__}: {decision.reason}")
            return []

        # Issue #26: Check cache first
        should_cache = use_cache if use_cache is not None else self._use_cache
        if should_cache and self._cache and not filters:
            # Only cache simple queries (no filters) for now
            cache_key = {"limit": limit, "offset": offset}
            cached = self._cache.get(model.__name__, cache_key)
            if cached is not None:
                return cached

        stmt = select(model)

        for key, value in filters.items():
            if hasattr(model, key):
                stmt = stmt.where(getattr(model, key) == value)

        stmt = stmt.limit(limit).offset(offset)
        results = list(self.session.execute(stmt).scalars().all())

        # Issue #26: Cache results if appropriate
        if should_cache and self._cache and not filters:
            cache_key = {"limit": limit, "offset": offset}
            self._cache.set(model.__name__, cache_key, results)

        return results

    def update(
        self,
        model: Type[T],
        record_id: int,
        expected_version: Optional[int] = None,
        **kwargs: Any
    ) -> Optional[T]:
        """
        Update a record by ID with optional optimistic concurrency control.

        Issue #26: Invalidates cache on write.

        Args:
            model: SQLAlchemy model class
            record_id: Primary key ID
            expected_version: Expected version for optimistic locking (Issue #15)
            **kwargs: Attributes to update

        Returns:
            Updated model instance or None

        Raises:
            ConcurrentModificationError: If version mismatch detected
            PermissionError: If access denied
        """
        # Issue #20: Access control check
        decision = self._check_access("update", model)
        if not decision.allowed:
            raise PermissionError(f"Access denied: {decision.reason}")

        # Issue #15: Get with row-level lock for versioned models
        stmt = select(model).where(getattr(model, "id") == record_id)

        # Use SELECT FOR UPDATE for models with version field
        if hasattr(model, 'version'):
            stmt = stmt.with_for_update()

        instance = self.session.execute(stmt).scalar_one_or_none()

        if not instance:
            return None

        # Check version for optimistic concurrency control
        if expected_version is not None and hasattr(instance, 'version'):
            if instance.version != expected_version:
                log_audit_event(
                    operation="update_conflict",
                    entity_type=model.__name__,
                    entity_id=record_id,
                    user_id=self._get_user_id(),
                    changes={
                        "expected_version": expected_version,
                        "actual_version": instance.version
                    },
                    status="conflict"
                )
                raise ConcurrentModificationError(
                    f"Concurrent modification detected on {model.__name__}:{record_id}",
                    entity_type=model.__name__,
                    entity_id=record_id,
                    expected_version=expected_version,
                    actual_version=instance.version,
                )

        # Track changes for audit log
        changes = {}
        for key, value in kwargs.items():
            if hasattr(instance, key):
                old_value = getattr(instance, key)
                if old_value != value:
                    changes[key] = {"old": old_value, "new": value}
                setattr(instance, key, value)

        # Increment version if applicable
        if hasattr(instance, 'increment_version'):
            instance.increment_version()

        self.session.flush()

        # Issue #21: Log audit event
        if changes:
            log_audit_event(
                operation="update",
                entity_type=model.__name__,
                entity_id=record_id,
                user_id=self._get_user_id(),
                changes=changes,
                status="success"
            )

            # Call instance's audit log if available
            if hasattr(instance, '_log_update'):
                instance._log_update(changes)

        # Issue #26: Invalidate cache on write
        if self._cache:
            self._cache.invalidate(model.__name__)

        return instance

    def delete(self, model: Type[T], record_id: int) -> bool:
        """
        Delete a record by ID

        Issue #26: Invalidates cache on write.

        Args:
            model: SQLAlchemy model class
            record_id: Primary key ID

        Returns:
            True if deleted, False if not found

        Raises:
            PermissionError: If access denied
        """
        # Issue #20: Access control check
        decision = self._check_access("delete", model)
        if not decision.allowed:
            raise PermissionError(f"Access denied: {decision.reason}")

        instance = self.get_by_id(model, record_id)
        if instance:
            # Call instance's audit log before deletion
            if hasattr(instance, '_log_delete'):
                instance._log_delete()

            self.session.delete(instance)
            self.session.flush()

            # Issue #21: Log audit event
            log_audit_event(
                operation="delete",
                entity_type=model.__name__,
                entity_id=record_id,
                user_id=self._get_user_id(),
                status="success"
            )

            # Issue #26: Invalidate cache on write
            if self._cache:
                self._cache.invalidate(model.__name__)

            return True
        return False

    def count(self, model: Type[T], **filters: Any) -> int:
        """
        Count records matching filters

        Args:
            model: SQLAlchemy model class
            **filters: Attribute filters

        Returns:
            Count of matching records
        """
        stmt = select(func.count(getattr(model, "id")))  # pylint: disable=not-callable

        for key, value in filters.items():
            if hasattr(model, key):
                stmt = stmt.where(getattr(model, key) == value)

        result = self.session.execute(stmt).scalar()
        return result or 0


class MemoryManager(DatabaseManager):
    """
    Manager for Memory model operations

    Handles creation, retrieval, update, and deletion of memories
    with support for filtering by category, importance, session, and
    Maestro-specific context (project, track, command).
    """

    def create_memory(
        self,
        content: str,
        category: str = MemoryCategory.CONTEXT.value,
        importance: str = MemoryImportance.NORMAL.value,
        summary: Optional[str] = None,
        source: Optional[str] = None,
        session_id: Optional[str] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        command: Optional[str] = None,
        command_context: Optional[Dict[str, Any]] = None,
        metadata: Optional[Dict[str, Any]] = None,
        tags: Optional[List[str]] = None,
        ttl_seconds: Optional[int] = None,
        allow_secret_redaction: bool = True,  # Issue #23: Configurable secret handling
    ) -> Memory:
        """
        Create a new memory with input validation.

        Issue #23: Detects and redacts secrets in memory content.

        Args:
            content: Memory content (validated for max length)
            category: Memory category
            importance: Memory importance level
            summary: Optional summary
            source: Source of the memory
            session_id: Associated session ID
            project_id: Associated Maestro project ID
            track_id: Associated Maestro track ID
            command: Command that created this memory
            command_context: Additional command context (validated JSON)
            metadata: Additional metadata (validated JSON)
            tags: Tags for categorization
            ttl_seconds: Time-to-live in seconds (None for permanent)
            allow_secret_redaction: If True, redact detected secrets; if False, raise error

        Returns:
            Created Memory instance

        Raises:
            ValidationError: If content or JSON fields are invalid
            SecretDetectedError: If secrets detected and allow_secret_redaction=False
        """
        # Validate content length
        validated_content = validate_content_length(content, "content")

        # Issue #23: Check for secrets and redact or raise error
        try:
            validated_content = validate_no_secrets(
                validated_content,
                allow_redaction=allow_secret_redaction
            )
            if summary:
                summary = validate_no_secrets(summary, allow_redaction=allow_secret_redaction)
        except SecretDetectedError as e:
            if allow_secret_redaction:
                logger.warning(f"Secret detected and redacted in memory content: {e.secret_type}")
                validated_content = validate_no_secrets(content, allow_redaction=True)
                if summary:
                    summary = validate_no_secrets(summary, allow_redaction=True)
            else:
                # Log the attempt for security audit
                log_audit_event(
                    operation="create_memory_blocked",
                    entity_type="Memory",
                    entity_id=None,
                    user_id=self._get_user_id(),
                    changes={"secret_type": e.secret_type, "matched": e.matched_content},
                    metadata={"reason": "Secret detected in memory content"},
                    status="denied"
                )
                raise

        # Validate JSON fields
        validated_command_context = validate_json_field(command_context, "command_context")
        validated_metadata = validate_json_field(metadata, "metadata")
        validated_tags = validate_json_field(tags, "tags")

        expires_at = None
        if ttl_seconds is not None:
            expires_at = datetime.now(UTC) + timedelta(seconds=ttl_seconds)

        return self.create(
            Memory,
            content=validated_content,
            summary=summary,
            category=category,
            importance=importance,
            source=source,
            session_id=session_id,
            project_id=project_id,
            track_id=track_id,
            command=command,
            command_context=validated_command_context,
            metadata=validated_metadata,
            tags=validated_tags,
            expires_at=expires_at,
        )

    def get_memory(self, memory_id: int) -> Optional[Memory]:
        """Get a memory by ID"""
        return self.get_by_id(Memory, memory_id)

    def get_memories_by_session(
        self,
        session_id: str,
        limit: int = 100,
    ) -> List[Memory]:
        """Get all memories for a session"""
        return self.get_all(
            Memory,
            session_id=session_id,
            limit=limit,
        )

    def get_memories_by_category(
        self,
        category: str,
        limit: int = 100,
    ) -> List[Memory]:
        """Get memories by category"""
        return self.get_all(
            Memory,
            category=category,
            limit=limit,
        )

    def get_memories_by_project(
        self,
        project_id: int,
        limit: int = 100,
    ) -> List[Memory]:
        """Get memories for a Maestro project"""
        return self.get_all(
            Memory,
            project_id=project_id,
            limit=limit,
        )

    def get_memories_by_track(
        self,
        track_id: int,
        limit: int = 100,
    ) -> List[Memory]:
        """Get memories for a Maestro track"""
        return self.get_all(
            Memory,
            track_id=track_id,
            limit=limit,
        )

    def get_memories_by_command(
        self,
        command: str,
        limit: int = 100,
    ) -> List[Memory]:
        """Get memories created by a specific command"""
        return self.get_all(
            Memory,
            command=command,
            limit=limit,
        )

    def search_memories(
        self,
        query: str,
        category: Optional[str] = None,
        importance: Optional[str] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        limit: int = 10,
    ) -> List[Memory]:
        """
        Search memories by content (simple text search)

        For semantic search, use the search_memories_semantic method
        after configuring embeddings.

        Args:
            query: Search query string
            category: Filter by category
            importance: Filter by importance
            project_id: Filter by project
            track_id: Filter by track
            limit: Maximum results

        Returns:
            List of matching memories
        """
        stmt = select(Memory).where(
            and_(
                Memory.content.contains(query),
                or_(
                    Memory.expires_at.is_(None),
                    Memory.expires_at > datetime.now(UTC),
                ),
            )
        )

        if category:
            stmt = stmt.where(Memory.category == category)
        if importance:
            stmt = stmt.where(Memory.importance == importance)
        if project_id:
            stmt = stmt.where(Memory.project_id == project_id)
        if track_id:
            stmt = stmt.where(Memory.track_id == track_id)

        stmt = stmt.order_by(Memory.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def search_memories_semantic(
        self,
        query_embedding: List[float],
        limit: int = 10,
        threshold: float = 0.75,
        category: Optional[str] = None,
        project_id: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """
        Search memories semantically using embeddings

        Requires sqlite-vec to be configured and embeddings to be
        generated for memories.

        Args:
            query_embedding: Query vector from embedding service
            limit: Maximum results
            threshold: Minimum similarity threshold
            category: Filter by category
            project_id: Filter by project

        Returns:
            List of matching memories with similarity scores
        """
        try:
            import sqlite3
            import numpy as np
        except ImportError:
            return []

        bind = self.session.bind
        url = getattr(bind, "url", None)
        db_path = getattr(url, "database", None)
        if not db_path:
            return []

        try:
            conn = sqlite3.connect(db_path)
            conn.enable_load_extension(True)
            conn.load_extension("vec0")

            # Convert query embedding to bytes
            query_bytes = np.asarray(query_embedding, dtype=np.float32).tobytes()

            # Search for similar embeddings
            # Use higher limit to account for filtering by expires_at, category, etc.
            results = conn.execute("""
                SELECT embedding_id, distance
                FROM memory_embeddings
                WHERE embedding MATCH ?
                ORDER BY distance
                LIMIT ?
            """, (query_bytes, limit * 3)).fetchall()

            conn.close()

            if not results:
                return []

            # Convert distance to similarity and filter by threshold
            # Distance is Euclidean, convert to cosine-like score
            memory_scores = []
            for memory_id, distance in results:
                # Simple conversion: max(0, 1 - distance/sqrt(2))
                similarity = max(0, 1 - distance / 1.414)
                if similarity >= threshold:
                    memory_scores.append((memory_id, similarity))

            if not memory_scores:
                return []

            # Sort by similarity
            memory_scores.sort(key=lambda x: x[1], reverse=True)

            # Fetch memory records
            memory_ids = [mid for mid, _ in memory_scores[:limit]]
            scores = {mid: score for mid, score in memory_scores[:limit]}

            stmt = select(Memory).where(
                and_(
                    Memory.id.in_(memory_ids),
                    or_(
                        Memory.expires_at.is_(None),
                        Memory.expires_at > datetime.now(UTC),
                    ),
                )
            )

            if category:
                stmt = stmt.where(Memory.category == category)
            if project_id:
                stmt = stmt.where(Memory.project_id == project_id)

            memories = self.session.execute(stmt).scalars().all()

            # Combine with scores and sort by similarity
            results = []
            for memory in memories:
                if memory.id in scores:
                    result = memory.to_dict()
                    result["similarity"] = scores[memory.id]
                    results.append(result)

            results.sort(key=lambda x: x["similarity"], reverse=True)

            return results

        except Exception as e:
            logger.warning(f"Semantic search failed: {e}")
            return []

    def update_memory(
        self,
        memory_id: int,
        content: Optional[str] = None,
        summary: Optional[str] = None,
        category: Optional[str] = None,
        importance: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
        tags: Optional[List[str]] = None,
    ) -> Optional[Memory]:
        """Update a memory"""
        updates: Dict[str, Any] = {}
        if content is not None:
            updates["content"] = content
        if summary is not None:
            updates["summary"] = summary
        if category is not None:
            updates["category"] = category
        if importance is not None:
            updates["importance"] = importance
        if metadata is not None:
            updates["metadata"] = metadata
        if tags is not None:
            updates["tags"] = tags

        return self.update(Memory, record_id=memory_id, **updates)

    def delete_memory(self, memory_id: int) -> bool:
        """Delete a memory"""
        return self.delete(Memory, memory_id)

    def create_memory_with_embedding(
        self,
        content: str,
        embeddings_service: Any,
        category: str = MemoryCategory.CONTEXT.value,
        importance: str = MemoryImportance.NORMAL.value,
        summary: Optional[str] = None,
        source: Optional[str] = None,
        session_id: Optional[str] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        command: Optional[str] = None,
        command_context: Optional[Dict[str, Any]] = None,
        metadata: Optional[Dict[str, Any]] = None,
        tags: Optional[List[str]] = None,
        ttl_seconds: Optional[int] = None,
    ) -> Optional[Memory]:
        """
        Create a new memory and automatically generate embedding for semantic search.

        Args:
            content: Memory content (validated for max length)
            embeddings_service: EmbeddingsService instance for generating embeddings
            category: Memory category
            importance: Memory importance level
            summary: Optional summary
            source: Source of the memory
            session_id: Associated session ID
            project_id: Associated Maestro project ID
            track_id: Associated Maestro track ID
            command: Command that created this memory
            command_context: Additional command context (validated JSON)
            metadata: Additional metadata (validated JSON)
            tags: Tags for categorization
            ttl_seconds: Time-to-live in seconds (None for permanent)

        Returns:
            Created Memory instance or None if embedding generation fails
        """
        # Create the memory first
        memory = self.create_memory(
            content=content,
            category=category,
            importance=importance,
            summary=summary,
            source=source,
            session_id=session_id,
            project_id=project_id,
            track_id=track_id,
            command=command,
            command_context=command_context,
            metadata=metadata,
            tags=tags,
            ttl_seconds=ttl_seconds,
        )

        # Generate and store embedding
        if embeddings_service and embeddings_service.is_available():
            # Combine content and summary for embedding
            embed_text = content
            if summary:
                embed_text = f"{summary}\n\n{content}"

            success = embeddings_service.index_memory(memory.id, embed_text)
            if success:
                memory.embedding_id = memory.id  # Reference to vec table
                self.session.flush()

        return memory

    def cleanup_expired_memories(self) -> int:
        """
        Delete all expired memories

        Returns:
            Number of memories deleted
        """
        stmt = delete(Memory).where(Memory.expires_at < datetime.now(UTC))
        result = cast(CursorResult[Any], self.session.execute(stmt))
        self.session.flush()
        return int(result.rowcount or 0)

    def schedule_cleanup_job(
        self,
        interval_seconds: int = 3600,
    ) -> None:
        """
        Schedule automatic cleanup of expired memories.

        This creates a background job that runs periodically to clean up
        expired memories. The job runs in a separate thread.

        Args:
            interval_seconds: Interval between cleanup runs (default 1 hour)
        """
        import threading
        import time

        if self._cleanup_thread is not None and self._cleanup_thread.is_alive():
            logger.warning("Cleanup job already running")
            return

        self._cleanup_running = True

        def cleanup_worker() -> None:
            while self._cleanup_running:
                try:
                    deleted = self.cleanup_expired_memories()
                    if deleted > 0:
                        logger.info(f"Cleaned up {deleted} expired memories")
                except Exception as e:
                    logger.error(f"Cleanup job failed: {e}")

                # Wait for next interval or until stopped
                for _ in range(interval_seconds):
                    if not self._cleanup_running:
                        break
                    time.sleep(1)

        self._cleanup_thread = threading.Thread(
            target=cleanup_worker,
            daemon=True,
            name="MemoryCleanupJob"
        )
        self._cleanup_thread.start()
        logger.info(f"Cleanup job scheduled with {interval_seconds}s interval")

    def stop_cleanup_job(self) -> None:
        """
        Stop the automatic cleanup job.
        """
        self._cleanup_running = False
        if self._cleanup_thread is not None and self._cleanup_thread.is_alive():
            self._cleanup_thread.join(timeout=5)
            logger.info("Cleanup job stopped")

    def get_active_memories(self, limit: int = 100) -> List[Memory]:
        """Get all non-expired memories"""
        stmt = select(Memory).where(
            or_(
                Memory.expires_at.is_(None),
                Memory.expires_at > datetime.now(UTC),
            )
        ).order_by(Memory.created_at.desc()).limit(limit)

        return list(self.session.execute(stmt).scalars().all())

    def get_memories(self, limit: int = 100, offset: int = 0, **filters: Any) -> List[Memory]:
        """
        Generic retrieval of memories.

        Args:
            limit: Maximum records to return
            offset: Number of records to skip
            **filters: Attribute filters

        Returns:
            List of Memory instances
        """
        return self.get_all(Memory, limit=limit, offset=offset, **filters)

    def get_statistics(self) -> Dict[str, Any]:
        """
        Get statistics about stored memories.

        Returns:
            Dictionary with counts and other metrics
        """
        stats: Dict[str, Any] = {
            "total_count": self.count(Memory),
            "by_category": {},
            "by_importance": {},
            "active_count": self.count(
                Memory,
                expires_at=None  # This is a bit simplistic as it doesn't handle > now
            )
        }

        # More accurate active count
        stmt_active = select(func.count(Memory.id)).where(  # pylint: disable=not-callable
            or_(
                Memory.expires_at.is_(None),
                Memory.expires_at > datetime.now(UTC)
            )
        )
        stats["active_count"] = self.session.execute(stmt_active).scalar() or 0

        # Counts by category
        for category in MemoryCategory:
            stats["by_category"][category.value] = self.count(Memory, category=category.value)

        # Counts by importance
        for importance in MemoryImportance:
            stats["by_importance"][importance.value] = self.count(Memory, importance=importance.value)

        return stats


class NamespaceManager(DatabaseManager):
    """
    Manager for AgentNamespace operations

    Handles creation and management of memory namespaces for
    agent isolation and selective sharing.
    """

    def create_namespace(
        self,
        name: str,
        owner_type: str,
        owner_id: str,
        description: Optional[str] = None,
        is_public: bool = False,
        allowed_readers: Optional[List[str]] = None,
        allowed_writers: Optional[List[str]] = None,
        config: Optional[Dict[str, Any]] = None,
    ) -> AgentNamespace:
        """
        Create a new namespace

        Args:
            name: Unique namespace name
            owner_type: Type of owner (agent, project, track)
            owner_id: ID of the owner
            description: Optional description
            is_public: Whether namespace is publicly accessible
            allowed_readers: List of agent IDs allowed to read
            allowed_writers: List of agent IDs allowed to write
            config: Additional configuration

        Returns:
            Created AgentNamespace instance
        """
        return self.create(
            AgentNamespace,
            name=name,
            description=description,
            owner_type=owner_type,
            owner_id=owner_id,
            is_public=is_public,
            allowed_readers=allowed_readers,
            allowed_writers=allowed_writers,
            config=config,
        )

    def get_namespace_by_name(self, name: str) -> Optional[AgentNamespace]:
        """Get a namespace by name"""
        stmt = select(AgentNamespace).where(AgentNamespace.name == name)
        return self.session.execute(stmt).scalar_one_or_none()

    def get_namespaces_by_owner(
        self,
        owner_type: str,
        owner_id: str,
    ) -> List[AgentNamespace]:
        """Get all namespaces for an owner"""
        stmt = select(AgentNamespace).where(
            and_(
                AgentNamespace.owner_type == owner_type,
                AgentNamespace.owner_id == owner_id,
            )
        )
        return list(self.session.execute(stmt).scalars().all())

    def get_public_namespaces(self) -> List[AgentNamespace]:
        """Get all public namespaces"""
        stmt = select(AgentNamespace).where(AgentNamespace.is_public == True)
        return list(self.session.execute(stmt).scalars().all())

    def can_read_namespace(
        self,
        namespace: AgentNamespace,
        agent_id: str,
    ) -> bool:
        """Check if an agent can read a namespace"""
        if namespace.is_public:
            return True
        if namespace.allowed_readers is None:
            return False
        return agent_id in namespace.allowed_readers

    def can_write_namespace(
        self,
        namespace: AgentNamespace,
        agent_id: str,
    ) -> bool:
        """Check if an agent can write to a namespace"""
        if namespace.allowed_writers is None:
            return False
        return agent_id in namespace.allowed_writers

    def add_memory_to_namespace(
        self,
        memory_id: int,
        namespace_id: int,
    ) -> NamespaceMemory:
        """Add a memory to a namespace"""
        return self.create(
            NamespaceMemory,
            memory_id=memory_id,
            namespace_id=namespace_id,
        )

    def get_namespace_memories(
        self,
        namespace_id: int,
        limit: int = 100,
        load_relationships: bool = True,
    ) -> List[Memory]:
        """
        Get all memories in a namespace

        Args:
            namespace_id: Namespace ID
            limit: Maximum memories to return
            load_relationships: Whether to eagerly load relationships (project, track)

        Returns:
            List of Memory instances
        """
        stmt = (
            select(Memory)
            .join(NamespaceMemory)
            .where(NamespaceMemory.namespace_id == namespace_id)
        )

        if load_relationships:
            # Use selectinload to avoid N+1 query problem when accessing relationships
            stmt = stmt.options(
                selectinload(Memory.project),
                selectinload(Memory.track),
            )

        stmt = stmt.limit(limit)
        return list(self.session.execute(stmt).scalars().all())

    def remove_memory_from_namespace(
        self,
        memory_id: int,
        namespace_id: int,
    ) -> bool:
        """Remove a memory from a namespace"""
        stmt = delete(NamespaceMemory).where(
            and_(
                NamespaceMemory.memory_id == memory_id,
                NamespaceMemory.namespace_id == namespace_id,
            )
        )
        result = cast(CursorResult[Any], self.session.execute(stmt))
        self.session.flush()
        return int(result.rowcount or 0) > 0


class SessionManager(DatabaseManager):
    """
    Manager for Session operations

    Handles session tracking, status updates, and statistics.
    Includes UUID collision handling (Issue #18).
    """

    def create_session(
        self,
        session_id: Optional[str] = None,
        session_type: str = "cli",
        title: Optional[str] = None,
        description: Optional[str] = None,
        agent_id: Optional[str] = None,
        agent_name: Optional[str] = None,
        status: Optional[str] = None,
        project_path: Optional[str] = None,
        working_directory: Optional[str] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        parent_session_id: Optional[str] = None,
        auto_generate_id: bool = True,
    ) -> Session:
        """
        Create a new session with UUID collision handling.

        Args:
            session_id: Unique session identifier (auto-generated if None)
            session_type: Type of session (cli, tui, api, agent, track)
            title: Optional title
            description: Optional description
            agent_id: Associated agent ID
            agent_name: Associated agent name
            status: Optional session status (defaults to SessionStatus.ACTIVE.value)
            project_path: Project path
            working_directory: Working directory
            project_id: Maestro project ID
            track_id: Maestro track ID
            parent_session_id: Parent session ID for chains
            auto_generate_id: Whether to auto-generate ID with collision handling

        Returns:
            Created Session instance

        Raises:
            UUIDCollisionError: If unable to generate unique ID after retries
        """
        # Issue #18: Auto-generate session ID with collision handling
        if session_id is None and auto_generate_id:
            try:
                session_id = generate_unique_session_id(
                    self.session,
                    max_attempts=5,
                    prefix="session",
                )
            except UUIDCollisionError as e:
                log_audit_event(
                    operation="create_session_failed",
                    entity_type="Session",
                    entity_id=None,
                    user_id=self._get_user_id(),
                    metadata={"error": str(e), "attempts": e.attempts},
                    status="failure"
                )
                raise

        return self.create(
            Session,
            session_id=session_id,
            session_type=session_type,
            title=title,
            description=description,
            agent_id=agent_id,
            agent_name=agent_name,
            status=status or SessionStatus.ACTIVE.value,
            project_path=project_path,
            working_directory=working_directory,
            project_id=project_id,
            track_id=track_id,
            parent_session_id=parent_session_id,
        )

    def create_session_with_retry(
        self,
        max_retries: int = 3,
        **kwargs: Any,
    ) -> Session:
        """
        Create a new session with retry logic for unique constraint violations.

        This method handles the extremely unlikely case of UUID collisions
        by generating new IDs and retrying.

        Args:
            max_retries: Maximum number of retries
            **kwargs: Arguments passed to create_session

        Returns:
            Created Session instance

        Raises:
            UUIDCollisionError: If unable to create session after max_retries
        """
        from sqlalchemy.exc import IntegrityError

        last_exception = None
        for attempt in range(max_retries):
            try:
                # Don't auto-generate ID in the create call - we handle it here
                kwargs['auto_generate_id'] = False
                if 'session_id' not in kwargs or kwargs['session_id'] is None:
                    kwargs['session_id'] = generate_unique_session_id(
                        self.session,
                        max_attempts=1,
                        prefix="session",
                    )

                return self.create_session(**kwargs)

            except (IntegrityError, UUIDCollisionError) as e:
                last_exception = e
                logger.warning(
                    f"Session creation attempt {attempt + 1}/{max_retries} failed, "
                    f"retrying with new ID..."
                )
                # Clear the session_id to force regeneration
                kwargs['session_id'] = None
                self.session.rollback()

        raise UUIDCollisionError(
            f"Failed to create session after {max_retries} attempts",
            attempts=max_retries,
            collided_id=kwargs.get('session_id', 'unknown')
        ) from last_exception

    def get_session_by_id(self, session_id: str) -> Optional[Session]:
        """Get a session by its session_id string"""
        stmt = select(Session).where(Session.session_id == session_id)
        return self.session.execute(stmt).scalar_one_or_none()

    def get_active_sessions(self, limit: int = 50) -> List[Session]:
        """Get all active sessions"""
        stmt = (
            select(Session)
            .where(Session.status == SessionStatus.ACTIVE.value)
            .order_by(Session.last_activity.desc())
            .limit(limit)
        )
        return list(self.session.execute(stmt).scalars().all())

    def get_sessions_by_agent(
        self,
        agent_id: str,
        limit: int = 50,
    ) -> List[Session]:
        """Get sessions for an agent"""
        stmt = (
            select(Session)
            .where(Session.agent_id == agent_id)
            .order_by(Session.created_at.desc())
            .limit(limit)
        )
        return list(self.session.execute(stmt).scalars().all())

    def update_session_status(
        self,
        session_id: str,
        status: str,
    ) -> Optional[Session]:
        """Update session status"""
        session = self.get_session_by_id(session_id)
        if session:
            session_any = cast(Any, session)
            session_any.status = status
            if status == SessionStatus.COMPLETED.value:
                session_any.ended_at = datetime.now(UTC)
            self.session.flush()
        return session

    def increment_session_stats(
        self,
        session_id: str,
        message_count: int = 0,
        tool_use_count: int = 0,
        memory_count: int = 0,
    ) -> Optional[Session]:
        """Increment session statistics"""
        session = self.get_session_by_id(session_id)
        if session:
            session_any = cast(Any, session)
            session_any.message_count = (session_any.message_count or 0) + message_count
            session_any.tool_use_count = (session_any.tool_use_count or 0) + tool_use_count
            session_any.memory_count = (session_any.memory_count or 0) + memory_count
            self.session.flush()
        return session

    def end_session(self, session_id: str) -> Optional[Session]:
        """End a session"""
        return self.update_session_status(session_id, SessionStatus.COMPLETED.value)

    def pause_session(self, session_id: str) -> Optional[Session]:
        """Pause a session"""
        return self.update_session_status(session_id, SessionStatus.PAUSED.value)

    def resume_session(self, session_id: str) -> Optional[Session]:
        """Resume a paused session"""
        return self.update_session_status(session_id, SessionStatus.ACTIVE.value)


class ProjectManager(DatabaseManager):
    """
    Manager for MaestroProject operations

    Handles project registration and tracking.
    """

    def delete_project(self, project_id: int, cascade: bool = True) -> bool:
        """
        Delete a project by ID with optional cascading.

        Args:
            project_id: Project ID to delete
            cascade: If True, also delete associated tracks and their data

        Returns:
            True if deleted, False if not found
        """
        # Issue #19: Cascading delete for projects
        project = self.get_by_id(MaestroProject, project_id)
        if not project:
            return False

        if cascade:
            # Delete associated tracks first (they will cascade to memories via FK)
            from sqlalchemy import delete as sql_delete
            self.session.execute(
                sql_delete(MaestroTrack).where(MaestroTrack.project_id == project_id)
            )
            self.session.flush()

        # Delete the project (CASCADE on FKs will clean up related records)
        return self.delete(MaestroProject, project_id)

    def create_project(
        self,
        project_path: str,
        project_name: Optional[str] = None,
        description: Optional[str] = None,
        project_type: Optional[str] = None,
        tech_stack: Optional[Dict[str, Any]] = None,
    ) -> MaestroProject:
        """Create a new Maestro project"""
        return self.create(
            MaestroProject,
            project_path=project_path,
            project_name=project_name,
            description=description,
            project_type=project_type,
            tech_stack=tech_stack,
        )

    def get_project_by_path(self, project_path: str) -> Optional[MaestroProject]:
        """Get a project by path"""
        stmt = select(MaestroProject).where(MaestroProject.project_path == project_path)
        return self.session.execute(stmt).scalar_one_or_none()

    def get_or_create_project(
        self,
        project_path: str,
        **kwargs: Any,
    ) -> MaestroProject:
        """Get existing project or create new one"""
        project = self.get_project_by_path(project_path)
        if not project:
            project = self.create_project(project_path, **kwargs)
        return project

    def update_last_active(self, project_id: int) -> None:
        """Update the last_active timestamp for a project"""
        stmt = (
            update(MaestroProject)
            .where(MaestroProject.id == project_id)
            .values(last_active=datetime.now(UTC))
        )
        self.session.execute(stmt)
        self.session.flush()


class TrackManager(DatabaseManager):
    """
    Manager for MaestroTrack operations

    Handles track creation, updates, and status tracking.
    """

    def delete_track(self, track_db_id: int) -> bool:
        """
        Delete a track by database ID.

        Note: Due to ON DELETE CASCADE on foreign keys, associated
        memories, file_claims, handoffs, and ledgers will be automatically
        deleted by the database.

        Args:
            track_db_id: Database ID of the track (not the track_id string)

        Returns:
            True if deleted, False if not found
        """
        # Issue #19: Delete track - CASCADE on FKs handles cleanup
        track = self.get_by_id(MaestroTrack, track_db_id)
        if not track:
            return False
        return self.delete(MaestroTrack, track_db_id)

    def delete_track_by_id(self, track_id: str) -> bool:
        """
        Delete a track by its track_id string.

        Args:
            track_id: Track identifier string (e.g., "feature_20260110")

        Returns:
            True if deleted, False if not found
        """
        track = self.get_track_by_id(track_id)
        if not track:
            return False
        return self.delete_track(int(track.id))  # type: ignore[arg-type]

    def create_track(
        self,
        track_id: str,
        project_id: int,
        title: str,
        description: Optional[str] = None,
        status: str = "new",
        track_type: Optional[str] = None,
        phase_count: int = 0,
        total_tasks: int = 0,
    ) -> MaestroTrack:
        """Create a new Maestro track"""
        return self.create(
            MaestroTrack,
            track_id=track_id,
            project_id=project_id,
            title=title,
            description=description,
            status=status,
            track_type=track_type,
            phase_count=phase_count,
            total_tasks=total_tasks,
        )

    def get_track_by_id(self, track_id: str) -> Optional[MaestroTrack]:
        """Get a track by track_id string"""
        stmt = select(MaestroTrack).where(MaestroTrack.track_id == track_id)
        return self.session.execute(stmt).scalar_one_or_none()

    def get_tracks_by_project(
        self,
        project_id: int,
        status: Optional[str] = None,
    ) -> List[MaestroTrack]:
        """Get all tracks for a project"""
        stmt = select(MaestroTrack).where(MaestroTrack.project_id == project_id)
        if status:
            stmt = stmt.where(MaestroTrack.status == status)
        stmt = stmt.order_by(MaestroTrack.created_at.desc())
        return list(self.session.execute(stmt).scalars().all())

    def update_track_status(
        self,
        track_id: str,
        status: str,
    ) -> Optional[MaestroTrack]:
        """Update track status"""
        track = self.get_track_by_id(track_id)
        if track:
            track_any = cast(Any, track)
            track_any.status = status
            if status == "completed":
                track_any.completed_at = datetime.now(UTC)
            elif status == "in_progress" and not track_any.started_at:
                track_any.started_at = datetime.now(UTC)
            self.session.flush()
        return track

    def update_track_progress(
        self,
        track_id: str,
        current_phase: int,
        completed_tasks: int,
    ) -> Optional[MaestroTrack]:
        """Update track progress"""
        track = self.get_track_by_id(track_id)
        if track:
            track_any = cast(Any, track)
            track_any.current_phase = current_phase
            track_any.completed_tasks = completed_tasks
            self.session.flush()
        return track


@contextmanager
def get_manager(
    manager_class: Type[DatabaseManager],
    db_path: Optional[str] = None,
) -> Generator[DatabaseManager, None, None]:
    """
    Context manager for getting a database manager

    Args:
        manager_class: Manager class to instantiate
        db_path: Optional database path

    Yields:
        Manager instance with an active session
    """
    from maestro.memory.database.models import create_tables, get_session

    engine = create_tables(db_path=db_path)
    session = get_session(engine=engine)

    try:
        yield manager_class(session)
        session.commit()
    except Exception:
        session.rollback()
        raise
    finally:
        session.close()
        engine.dispose()


def get_managers(db_path: Optional[str] = None) -> Dict[str, DatabaseManager]:
    """
    Get all manager instances with a shared session

    Args:
        db_path: Optional database path

    Returns:
        Dictionary of manager instances
    """
    from maestro.memory.database.models import get_session

    session = get_session(db_path=db_path)

    return {
        "memory": MemoryManager(session),
        "namespace": NamespaceManager(session),
        "session": SessionManager(session),
        "project": ProjectManager(session),
        "track": TrackManager(session),
    }
