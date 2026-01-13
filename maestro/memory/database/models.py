"""
Maestro Unified Memory Database Models

This module defines the complete database schema for Maestro v2 unified memory system.
All models are self-contained and independent of external systems.
"""

from datetime import datetime, UTC
from typing import Optional, Dict, Any, List, Callable, Generator, Union
from typing import TYPE_CHECKING
from enum import Enum
from contextlib import contextmanager
from threading import local
from sqlalchemy import (
    Column,
    Integer,
    String,
    Text,
    DateTime,
    ForeignKey,
    JSON,
    Index,
    Boolean,
    Float,
    UniqueConstraint,
    CheckConstraint,
    event,
    Engine,
)
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import declarative_base, relationship, Session as ORMSession, sessionmaker
from sqlalchemy.orm import Mapped, mapped_column
from sqlalchemy.sql import func
from sqlalchemy.pool import NullPool
import logging
import os
import json
import re
from typing import Tuple, List

logger = logging.getLogger(__name__)

from typing import Type

# Define Base for runtime and type checking
from sqlalchemy.orm import DeclarativeBase

class Base(DeclarativeBase):
    pass


# ============================================================================
# AUDIT LOGGING INFRASTRUCTURE (Issue #20, #21)
# ============================================================================

# Audit log storage - in-memory for now, can be extended to file/database
_audit_callbacks: List[Callable[[Dict[str, Any]], None]] = []


def register_audit_callback(callback: Callable[[Dict[str, Any]], None]) -> None:
    """
    Register a callback to receive audit events.

    Args:
        callback: Function that takes audit event dict and returns None
    """
    _audit_callbacks.append(callback)


def unregister_audit_callback(callback: Callable[[Dict[str, Any]], None]) -> None:
    """
    Unregister an audit callback.

    Args:
        callback: Previously registered callback function
    """
    if callback in _audit_callbacks:
        _audit_callbacks.remove(callback)


def log_audit_event(
    operation: str,
    entity_type: str,
    entity_id: Any,
    user_id: Optional[str] = None,
    changes: Optional[Dict[str, Any]] = None,
    metadata: Optional[Dict[str, Any]] = None,
    status: str = "success",
) -> None:
    """
    Log an audit event for all mutable operations.

    Args:
        operation: Type of operation (create, update, delete, etc.)
        entity_type: Type of entity (Memory, FileClaim, Handoff, etc.)
        entity_id: ID of the affected entity
        user_id: ID of the user/agent performing the operation
        changes: Dictionary of changes made (for updates)
        metadata: Additional metadata about the operation
        status: Operation status (success, failure, attempted)
    """
    event = {
        "timestamp": datetime.now(UTC).isoformat(),
        "operation": operation,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "user_id": user_id or "system",
        "changes": changes or {},
        "metadata": metadata or {},
        "status": status,
    }

    # Log to Python logging
    logger.info(
        f"AUDIT: {operation} {entity_type}:{entity_id} by {user_id} - {status}",
        extra={"audit_event": event}
    )

    # Notify registered callbacks
    for callback in _audit_callbacks:
        try:
            callback(event)
        except Exception as e:
            logger.error(f"Audit callback failed: {e}")


class AuditLoggable:
    """
    Mixin for models that support audit logging.
    """

    _audit_user_id: Optional[str] = None
    _audit_enabled: bool = True

    def set_audit_user(self, user_id: str) -> None:
        """Set the user ID for audit logging on this instance."""
        self._audit_user_id = user_id

    def set_audit_enabled(self, enabled: bool) -> None:
        """Enable or disable audit logging for this instance."""
        self._audit_enabled = enabled

    def _log_create(self) -> None:
        """Log creation of this entity."""
        if not self._audit_enabled:
            return
        entity_id = getattr(self, 'id', None) or getattr(self, 'session_id', None) or getattr(self, 'claim_id', None)
        entity_type = self.__class__.__name__
        log_audit_event(
            operation="create",
            entity_type=entity_type,
            entity_id=entity_id,
            user_id=self._audit_user_id,
            status="success"
        )

    def _log_update(self, changes: Dict[str, Any]) -> None:
        """Log update to this entity."""
        if not self._audit_enabled:
            return
        entity_id = getattr(self, 'id', None) or getattr(self, 'session_id', None) or getattr(self, 'claim_id', None)
        entity_type = self.__class__.__name__
        log_audit_event(
            operation="update",
            entity_type=entity_type,
            entity_id=entity_id,
            user_id=self._audit_user_id,
            changes=changes,
            status="success"
        )

    def _log_delete(self) -> None:
        """Log deletion of this entity."""
        if not self._audit_enabled:
            return
        entity_id = getattr(self, 'id', None) or getattr(self, 'session_id', None) or getattr(self, 'claim_id', None)
        entity_type = self.__class__.__name__
        log_audit_event(
            operation="delete",
            entity_type=entity_type,
            entity_id=entity_id,
            user_id=self._audit_user_id,
            status="success"
        )


# ============================================================================
# ACCESS CONTROL HOOKS (Issue #20)
# ============================================================================

class AccessDecision:
    """Result of an access control check."""
    def __init__(self, allowed: bool, reason: str = "") -> None:
        self.allowed = allowed
        self.reason = reason


# Type for access check functions
AccessCheckFunc = Callable[
    [str, str, Optional[str]],  # (operation, entity_type, entity_id)
    AccessDecision
]

_default_access_check: Optional[AccessCheckFunc] = None


def set_default_access_check(func: AccessCheckFunc) -> None:
    """
    Set the default access control check function.

    The function should have signature:
        (operation: str, entity_type: str, entity_id: Optional[str]) -> AccessDecision

    For local development, this can be a permissive function.
    For production, this should check user permissions.

    Args:
        func: Access control check function
    """
    global _default_access_check
    _default_access_check = func


def check_access(
    operation: str,
    entity_type: str,
    entity_id: Optional[str] = None,
    user_id: Optional[str] = None,
) -> AccessDecision:
    """
    Check if a user/agent has permission to perform an operation.

    This is a hook that can be overridden for production access control.
    For local development, it defaults to permissive.

    Args:
        operation: Type of operation (create, read, update, delete)
        entity_type: Type of entity (Memory, FileClaim, etc.)
        entity_id: Optional specific entity ID
        user_id: Optional user/agent ID to check

    Returns:
        AccessDecision with allowed flag and optional reason
    """
    if _default_access_check is not None:
        return _default_access_check(operation, entity_type, entity_id)

    # Default: permissive for local development
    return AccessDecision(allowed=True, reason="Local development - no access control")


# Validation constants
MAX_CONTENT_SIZE = 10 * 1024 * 1024  # 10MB maximum content size
MAX_STRING_LENGTH = 500  # Maximum length for string fields
MAX_JSON_FIELD_SIZE = 1 * 1024 * 1024  # 1MB maximum for JSON fields


class ValidationError(ValueError):
    """
    Raised when input validation fails.

    Issue #29: Provides detailed error messages with context and remediation steps.
    """

    field_name: str
    valid_values: Optional[List[str]]

    def __init__(self, message: str, field_name: str = "unknown", valid_values: Optional[List[str]] = None) -> None:
        super().__init__(message)
        self.field_name = field_name
        self.valid_values = valid_values

    def get_context(self) -> Dict[str, Any]:
        """Get error context for better error reporting"""
        context = {
            "field": self.field_name,
            "message": str(self),
        }
        if self.valid_values:
            context["valid_values"] = ", ".join(self.valid_values)  # Convert list to string
        return context


def validate_content_length(content: str, field_name: str = "content") -> str:
    """
    Validate that content length is within acceptable limits.

    Issue #29: Enhanced error messages with remediation steps.

    Args:
        content: The content string to validate
        field_name: Name of the field for error messages

    Returns:
        The validated content

    Raises:
        ValidationError: If content exceeds maximum size
    """
    if content is None:
        raise ValidationError(
            f"{field_name} cannot be None",
            field_name=field_name,
            valid_values=["non-empty string"]
        )

    if not isinstance(content, str):
        raise ValidationError(
            f"{field_name} must be a string, got {type(content).__name__}",
            field_name=field_name,
            valid_values=["string"]
        )

    content_length = len(content.encode('utf-8'))  # Use byte length
    if content_length > MAX_CONTENT_SIZE:
        raise ValidationError(
            f"{field_name} exceeds maximum size of {MAX_CONTENT_SIZE} bytes "
            f"(got {content_length} bytes). "
            f"Remediation: Split content into smaller chunks or compress before storing.",
            field_name=field_name,
            valid_values=[f"< {MAX_CONTENT_SIZE} bytes"]
        )

    return content


def validate_json_field(value: Any, field_name: str = "field") -> Any:
    """
    Validate that a JSON field is properly structured and within size limits.

    Issue #29: Enhanced error messages with remediation steps.

    Args:
        value: The value to validate (dict, list, or None)
        field_name: Name of the field for error messages

    Returns:
        The validated value

    Raises:
        ValidationError: If value is not valid JSON or exceeds size limits
    """
    import json

    if value is None:
        return None

    # Check if it's a valid JSON-serializable type
    try:
        serialized = json.dumps(value, ensure_ascii=False)
    except (TypeError, ValueError) as e:
        raise ValidationError(
            f"{field_name} contains non-JSON-serializable data: {e}. "
            f"Remediation: Use only JSON-serializable types (str, int, float, bool, list, dict, None).",
            field_name=field_name,
            valid_values=["JSON-serializable types"]
        )

    # Check size
    if len(serialized.encode('utf-8')) > MAX_JSON_FIELD_SIZE:
        raise ValidationError(
            f"{field_name} exceeds maximum JSON size of {MAX_JSON_FIELD_SIZE} bytes. "
            f"Remediation: Reduce the size or split into multiple fields.",
            field_name=field_name,
            valid_values=[f"< {MAX_JSON_FIELD_SIZE} bytes serialized"]
        )

    return value


def sanitize_string(value: Optional[str], max_length: int = MAX_STRING_LENGTH) -> Optional[str]:
    """
    Sanitize string inputs by truncating and removing null bytes.

    Args:
        value: The string to sanitize
        max_length: Maximum allowed length

    Returns:
        Sanitized string or None
    """
    if value is None:
        return None

    if not isinstance(value, str):
        value = str(value)

    # Remove null bytes and other problematic characters
    value = value.replace('\x00', '')

    # Truncate if necessary
    if len(value) > max_length:
        value = value[:max_length]

    return value


# ============================================================================
# SECRET DETECTION (Issue #23)
# ============================================================================

# Common secret patterns - these should not be stored in memory
SECRET_PATTERNS = {
    # API Keys
    r'(?i)(api[_-]?key|apikey|key)["\']?\s*[:=]\s*["\']?([a-zA-Z0-9_\-]{20,})': "API key",
    r'(?i)(sk[_-]?[a-zA-Z0-9]{20,})': "Secret key (sk_...)",
    r'(?i)(AKIA[0-9A-Z]{16})': "AWS access key",

    # Tokens
    r'(?i)(bearer[_-]?token|access[_-]?token|auth[_-]?token)["\']?\s*[:=]\s*["\']?([a-zA-Z0-9_\-\.]{20,})': "Bearer/auth token",
    r'(?i)(github[_-]?token|gh[_-]?token|ghp|gho|ghu|ghs|ghr)["\']?\s*[:=]\s*["\']?([a-zA-Z0-9_\-]{36,})': "GitHub token",

    # Passwords
    r'(?i)(password|passwd|pwd)["\']?\s*[:=]\s*["\']?([^"\'\s]{8,})': "Password",

    # JWT
    r'eyJ[a-zA-Z0-9_\-]*\.[a-zA-Z0-9_\-]*\.[a-zA-Z0-9_\-]*': "JWT token",

    # Private keys (start of common key formats)
    r'-----BEGIN [A-Z]+ PRIVATE KEY-----': "Private key",
    r'-----BEGIN RSA PRIVATE KEY-----': "RSA private key",
    r'-----BEGIN EC PRIVATE KEY-----': "EC private key",
    r'-----BEGIN OPENSSH PRIVATE KEY-----': "OpenSSH private key",

    # Database connection strings
    r'(mongodb|mysql|redis)://[^"\':\s]+:[^"\':\s]+@': "Database connection string",

    # API endpoints with keys
    r'https?://[^"\':\s]+:[^"\':\s]+@': "URL with embedded credentials",

    # OAuth tokens
    r'(?i)(oauth[_-]?token|refresh[_-]?token)["\']?\s*[:=]\s*["\']?([a-zA-Z0-9_\-]{20,})': "OAuth token",

    # Slack tokens
    r'xox[baprs]-[0-9]{12}-[0-9]{12}-[0-9]{12}-[a-zA-Z0-9]{24}': "Slack token",

    # Stripe keys
    r'(?i)(sk|pk)_(live|test)_[0-9a-zA-Z]{24,}': "Stripe API key",
}


class SecretDetectedError(ValidationError):
    """Raised when a secret is detected in content"""
    def __init__(self, message: str, secret_type: str, matched_content: str) -> None:
        super().__init__(message)
        self.secret_type = secret_type
        self.matched_content = matched_content


def detect_secrets(content: str) -> List[Tuple[str, str, str]]:
    """
    Detect potential secrets in content.

    Args:
        content: Content to scan

    Returns:
        List of (secret_type, pattern_matched, context) tuples
    """
    detected = []

    for pattern, secret_type in SECRET_PATTERNS.items():
        for match in re.finditer(pattern, content):
            matched_text = match.group(0)
            # Get context (surrounding characters)
            start = max(0, match.start() - 20)
            end = min(len(content), match.end() + 20)
            context = content[start:end]

            detected.append((secret_type, matched_text[:50], context))

    return detected


def redact_secrets(content: str, replacement: str = "***REDACTED***") -> str:
    """
    Redact detected secrets from content.

    Args:
        content: Content to redact
        replacement: Replacement text

    Returns:
        Content with secrets redacted
    """
    redacted = content

    for pattern in SECRET_PATTERNS.keys():
        redacted = re.sub(pattern, replacement, redacted, flags=re.IGNORECASE)

    return redacted


def validate_no_secrets(content: str, allow_redaction: bool = False) -> str:
    """
    Validate content contains no secrets, optionally redacting them.

    Args:
        content: Content to validate
        allow_redaction: If True, redact secrets instead of raising error

    Returns:
        Validated content (or redacted content if allow_redaction=True)

    Raises:
        SecretDetectedError: If secrets detected and allow_redaction=False
    """
    detected = detect_secrets(content)

    if detected:
        if allow_redaction:
            logger.warning(f"Secrets detected and redacted in content: {[d[0] for d in detected]}")
            return redact_secrets(content)
        else:
            secret_types = [d[0] for d in detected]
            raise SecretDetectedError(
                f"Content contains potential secrets: {', '.join(set(secret_types))}. "
                f"Set allow_redaction=True to automatically redact secrets.",
                secret_type=secret_types[0] if secret_types else "unknown",
                matched_content=detected[0][1] if detected else ""
            )

    return content


class MemoryCategory(str, Enum):
    """Memory categories for organizing memories by type and purpose"""

    FACT = "fact"
    PATTERN = "pattern"
    DECISION = "decision"
    CONTEXT = "context"
    TEMPORARY = "temporary"
    OBSERVATION = "observation"


class MemoryImportance(str, Enum):
    """Importance levels for memories"""

    CRITICAL = "critical"
    HIGH = "high"
    NORMAL = "normal"
    LOW = "low"


class ClaimStatus(str, Enum):
    """Status of file claims for coordination"""

    ACTIVE = "active"
    RELEASED = "released"
    EXPIRED = "expired"
    REVOKED = "revoked"


class HandoffStatus(str, Enum):
    """Status of session handoffs"""

    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    RESUMED = "resumed"
    ABANDONED = "abandoned"
    COMPLETED = "completed"


class SessionStatus(str, Enum):
    """Status of sessions"""

    ACTIVE = "active"
    PAUSED = "paused"
    COMPLETED = "completed"
    TERMINATED = "terminated"


class Memory(Base):
    """
    Core memory storage for all Maestro operations

    Stores extracted context, facts, patterns, and decisions with
    Maestro-specific extensions for project and track association.
    """

    __tablename__ = "memories"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore
    # Core content
    content = Column(Text, nullable=False)  # type: ignore
    summary = Column(Text, nullable=True)  # type: ignore
    category = Column(String(50), nullable=False, default=MemoryCategory.CONTEXT.value, index=True)  # type: ignore
    importance = Column(String(50), nullable=False, default=MemoryImportance.NORMAL.value, index=True)  # type: ignore

    # Source information
    source = Column(String(200), nullable=True)  # type: ignore  # What created this memory
    session_id = Column(String(200), nullable=True, index=True)  # type: ignore

    # Maestro extensions
    # Issue #19: Add ON DELETE CASCADE to clean up memories when project/track deleted
    project_id = Column(Integer, ForeignKey("maestro_projects.id", ondelete="CASCADE"), nullable=True, index=True)  # type: ignore
    track_id = Column(Integer, ForeignKey("maestro_tracks.id", ondelete="CASCADE"), nullable=True, index=True)  # type: ignore
    command = Column(String(100), nullable=True, index=True)  # type: ignore
    command_context = Column(JSON, nullable=True)  # type: ignore

    # Embedding for semantic search
    embedding_id = Column(Integer, nullable=True)  # type: ignore  # Reference to vector table

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False, index=True)  # type: ignore # pylint: disable=not-callable
    expires_at = Column(DateTime(timezone=True), nullable=True, index=True)  # type: ignore
    last_accessed = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)  # type: ignore # pylint: disable=not-callable

    # Metadata
    # Issue #27: Standardized to `metadata` (was `meta_data`)
    # Note: `metadata` is a reserved word in SQLAlchemy, so we use `meta_data`
    # but provide a property accessor
    _metadata_internal = Column("meta_data", JSON, nullable=True)  # type: ignore
    tags = Column(JSON, nullable=True)  # type: ignore  # List of tags

    # Relationships
    project = relationship("MaestroProject", backref="memories")  # type: ignore[assignment]
    track = relationship("MaestroTrack", backref="memories")  # type: ignore[assignment]

    # Indexes - including composite indexes for common query patterns
    __table_args__ = (
        # Single column indexes
        Index("idx_memories_category_importance", "category", "importance"),
        Index("idx_memories_session", "session_id"),
        Index("idx_memories_command", "command"),
        Index("idx_memories_created", "created_at"),
        Index("idx_memories_expires", "expires_at"),

        # Composite indexes for common query patterns
        # Issue #13: Add composite index on (session_id, category, importance)
        Index("idx_memories_session_category_importance", "session_id", "category", "importance"),

        # Composite index for project/track queries
        Index("idx_memories_project_created", "project_id", "created_at"),

        # Composite index for track queries with time
        Index("idx_memories_track_created", "track_id", "created_at"),

        # Composite index for project+track queries
        Index("idx_memories_project_track", "project_id", "track_id"),

        # Composite index for active memories filtering
        Index("idx_memories_active", "category", "importance", "created_at"),
    )

    def __repr__(self) -> str:
        return f"<Memory(id={self.id}, category='{self.category}', content='{self.content[:50]}...')>"

    def get_metadata(self) -> Optional[Dict[str, Any]]:
        """
        Issue #27: Property accessor for metadata.
        Standardizes the API to use `get_metadata()` instead of accessing `_metadata_internal`.

        Note: We can't use @property decorator for 'metadata' because it conflicts
        with SQLAlchemy's Base.metadata attribute.
        """
        return self._metadata_internal  # type: ignore

    def set_metadata(self, value: Optional[Dict[str, Any]]) -> None:
        """Setter for metadata."""
        self._metadata_internal = value  # type: ignore

    def to_dict(self) -> Dict[str, Any]:
        """Convert memory to dictionary"""
        return {
            "id": self.id,
            "content": self.content,
            "summary": self.summary,
            "category": self.category,
            "importance": self.importance,
            "source": self.source,
            "session_id": self.session_id,
            "project_id": self.project_id,
            "track_id": self.track_id,
            "command": self.command,
            "command_context": self.command_context,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "expires_at": self.expires_at.isoformat() if self.expires_at else None,
            "last_accessed": self.last_accessed.isoformat() if self.last_accessed else None,
            "metadata": self._metadata_internal or {},  # Issue #27: Use property
            "tags": self.tags or [],
        }

    def is_expired(self) -> bool:
        """Check if memory has expired"""
        if self.expires_at is None:
            return False
        return datetime.now(UTC) > self.expires_at.replace(tzinfo=UTC)  # type: ignore


class AgentNamespace(Base):
    """
    Memory namespaces for agent isolation

    Allows multiple agents to maintain separate memory spaces
    while enabling selective sharing when needed.
    """

    __tablename__ = "agent_namespaces"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore
    name = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore
    description = Column(Text, nullable=True)  # type: ignore

    # Namespace owner
    owner_type = Column(String(50), nullable=False)  # type: ignore  # agent, project, track
    owner_id = Column(String(200), nullable=False, index=True)  # type: ignore

    # Access control
    is_public = Column(Boolean, default=False, nullable=False)  # type: ignore
    allowed_readers = Column(JSON, nullable=True)  # type: ignore  # List of agent IDs
    allowed_writers = Column(JSON, nullable=True)  # type: ignore  # List of agent IDs

    # Configuration
    config = Column(JSON, nullable=True)  # type: ignore

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)  # type: ignore
    updated_at = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)  # type: ignore


    # Relationships
    memories = relationship("Memory", secondary="namespace_memories", backref="namespaces")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_namespaces_owner", "owner_type", "owner_id"),
        CheckConstraint("owner_type IN ('agent', 'project', 'track')", name="check_owner_type"),
    )

    def __repr__(self) -> str:
        return f"<AgentNamespace(id={self.id}, name='{self.name}', owner='{self.owner_id}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert namespace to dictionary"""
        return {
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "owner_type": self.owner_type,
            "owner_id": self.owner_id,
            "is_public": self.is_public,
            "allowed_readers": self.allowed_readers or [],
            "allowed_writers": self.allowed_writers or [],
            "config": self.config or {},
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "updated_at": self.updated_at.isoformat() if self.updated_at else None,
        }


class NamespaceMemory(Base):
    """
    Junction table for namespace-memory associations
    """

    __tablename__ = "namespace_memories"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore
    namespace_id = Column(Integer, ForeignKey("agent_namespaces.id"), nullable=False, index=True)  # type: ignore
    memory_id = Column(Integer, ForeignKey("memories.id"), nullable=False, index=True)  # type: ignore
    added_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)  # type: ignore


    __table_args__ = (
        UniqueConstraint("namespace_id", "memory_id", name="unique_namespace_memory"),
        Index("idx_ns_memory_namespace", "namespace_id"),
        Index("idx_ns_memory_memory", "memory_id"),
    )


class FileClaim(Base, AuditLoggable):
    """
    File claims for multi-agent coordination

    Prevents concurrent modifications to the same files by different agents.
    Implements lease-based claims with TTL for automatic expiration.

    Includes version tracking for optimistic concurrency control (Issue #15).
    """

    __tablename__ = "file_claims"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore

    # Claim information
    claim_id = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore
    agent_id = Column(String(200), nullable=False, index=True)  # type: ignore
    session_id = Column(String(200), nullable=True, index=True)  # type: ignore

    # File patterns (glob patterns for flexible matching)
    file_patterns = Column(JSON, nullable=False)  # type: ignore  # List of glob patterns

    # Claim details
    status = Column(String(50), nullable=False, default=ClaimStatus.ACTIVE.value, index=True)  # type: ignore
    is_exclusive = Column(Boolean, default=True, nullable=False)  # type: ignore

    # Reason for claim
    reason = Column(Text, nullable=True)  # type: ignore
    task_description = Column(Text, nullable=True)  # type: ignore

    # Timing
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)  # type: ignore
    expires_at = Column(DateTime(timezone=True), nullable=False, index=True)  # type: ignore
    released_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore

    # Issue #15: Version tracking for optimistic concurrency control
    version = Column(Integer, default=1, nullable=False)  # type: ignore
    updated_at = Column(DateTime(timezone=True), server_default=func.now(), onupdate=func.now(), nullable=False)  # type: ignore

    # Maestro context
    # Issue #19: Add ON DELETE CASCADE to clean up file claims when project/track deleted
    project_id = Column(Integer, ForeignKey("maestro_projects.id", ondelete="CASCADE"), nullable=True)  # type: ignore
    track_id = Column(Integer, ForeignKey("maestro_tracks.id", ondelete="CASCADE"), nullable=True)  # type: ignore


    # Relationships
    project = relationship("MaestroProject", backref="file_claims")  # type: ignore[assignment]
    track = relationship("MaestroTrack", backref="file_claims")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_file_claims_agent", "agent_id"),
        Index("idx_file_claims_status", "status"),
        Index("idx_file_claims_expires", "expires_at"),
        Index("idx_file_claims_version", "version"),  # For concurrent updates
        CheckConstraint("status IN ('active', 'released', 'expired', 'revoked')", name="check_claim_status"),
        CheckConstraint("version >= 1", name="check_version_positive"),
    )

    def __repr__(self) -> str:
        return f"<FileClaim(id={self.id}, claim_id='{self.claim_id}', agent='{self.agent_id}', v={self.version})>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert claim to dictionary"""
        return {
            "id": self.id,
            "claim_id": self.claim_id,
            "agent_id": self.agent_id,
            "session_id": self.session_id,
            "file_patterns": self.file_patterns,
            "status": self.status,
            "is_exclusive": self.is_exclusive,
            "reason": self.reason,
            "task_description": self.task_description,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "expires_at": self.expires_at.isoformat() if self.expires_at else None,
            "released_at": self.released_at.isoformat() if self.released_at else None,
            "project_id": self.project_id,
            "track_id": self.track_id,
            "version": self.version,
            "updated_at": self.updated_at.isoformat() if self.updated_at else None,
        }

    def is_valid(self) -> bool:
        """Check if claim is still valid"""
        if self.status != ClaimStatus.ACTIVE.value:
            return False
        if datetime.now(UTC) > self.expires_at.replace(tzinfo=UTC):
            return False
        return True

    def increment_version(self) -> int:
        """Increment version counter and return new version."""
        # In SQLAlchemy ORM, accessing the attribute gives the value, not the Column
        current_version = int(self.version) if self.version is not None else 1
        new_version = current_version + 1
        self.version = new_version  # type: ignore
        return new_version


class Handoff(Base):
    """
    Session handoffs for continuity across sessions

    Stores the complete state of a session for resumption by another agent or session.
    Uses YAML format for human-readable handoff documents.
    """

    __tablename__ = "handoffs"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore

    # Handoff identification
    handoff_id = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore
    title = Column(String(500), nullable=False)  # type: ignore

    # Session information
    from_session_id = Column(String(200), nullable=False)  # type: ignore
    to_session_id = Column(String(200), nullable=True)  # type: ignore
    from_agent_id = Column(String(200), nullable=False)  # type: ignore
    to_agent_id = Column(String(200), nullable=True)  # type: ignore

    # Status
    status = Column(String(50), nullable=False, default=HandoffStatus.PENDING.value, index=True)  # type: ignore

    # Handoff content (YAML formatted string)
    context_yaml = Column(Text, nullable=False)  # type: ignore

    # Parsed context (JSON for structured access)
    context_data = Column(JSON, nullable=True)  # type: ignore

    # Metadata
    project_path = Column(String(500), nullable=True)  # type: ignore
    summary = Column(Text, nullable=True)  # type: ignore
    tags = Column(JSON, nullable=True)  # type: ignore

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False, index=True)  # type: ignore
    resumed_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore
    completed_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore

    # Maestro context
    # Issue #19: Add ON DELETE CASCADE to clean up handoffs when project/track deleted
    project_id = Column(Integer, ForeignKey("maestro_projects.id", ondelete="CASCADE"), nullable=True)  # type: ignore
    track_id = Column(Integer, ForeignKey("maestro_tracks.id", ondelete="CASCADE"), nullable=True)  # type: ignore

    # Relationships
    project = relationship("MaestroProject", backref="handoffs")  # type: ignore[assignment]
    track = relationship("MaestroTrack", backref="handoffs")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_handoffs_from_session", "from_session_id"),
        Index("idx_handoffs_to_session", "to_session_id"),
        Index("idx_handoffs_status", "status"),
        Index("idx_handoffs_created", "created_at"),
        CheckConstraint(
            "status IN ('pending', 'in_progress', 'resumed', 'abandoned', 'completed')",
            name="check_handoff_status"
        ),
    )

    def __repr__(self) -> str:
        return f"<Handoff(id={self.id}, handoff_id='{self.handoff_id}', title='{self.title}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert handoff to dictionary"""
        return {
            "id": self.id,
            "handoff_id": self.handoff_id,
            "title": self.title,
            "from_session_id": self.from_session_id,
            "to_session_id": self.to_session_id,
            "from_agent_id": self.from_agent_id,
            "to_agent_id": self.to_agent_id,
            "status": self.status,
            "context_yaml": self.context_yaml,
            "context_data": self.context_data,
            "project_path": self.project_path,
            "summary": self.summary,
            "tags": self.tags or [],
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "resumed_at": self.resumed_at.isoformat() if self.resumed_at else None,
            "completed_at": self.completed_at.isoformat() if self.completed_at else None,
            "project_id": self.project_id,
            "track_id": self.track_id,
        }

    def is_pickable(self) -> bool:
        """Check if handoff is available for resumption"""
        return self.status in (HandoffStatus.PENDING.value, HandoffStatus.IN_PROGRESS.value)


class ContinuityLedger(Base):
    """
    Continuity ledgers for tracking session progress

    Maintains a chronological record of session activities, decisions,
    and outcomes for continuity tracking and analysis.
    """

    __tablename__ = "continuity_ledgers"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore

    # Ledger identification
    ledger_id = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore
    session_id = Column(String(200), nullable=False, index=True)  # type: ignore
    agent_id = Column(String(200), nullable=False, index=True)  # type: ignore

    # Entry information
    entry_type = Column(String(100), nullable=False, index=True)  # type: ignore  # decision, action, outcome, observation
    title = Column(String(500), nullable=False)  # type: ignore
    content = Column(Text, nullable=False)  # type: ignore

    # Metadata
    # Issue #27: Standardized to `metadata` (was `meta_data`)
    _metadata_internal = Column("meta_data", JSON, nullable=True)  # type: ignore
    parent_entry_id = Column(Integer, ForeignKey("continuity_ledgers.id"), nullable=True)  # type: ignore

    # Timing
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False, index=True)  # type: ignore
    sequence_number = Column(Integer, nullable=False, index=True)  # type: ignore

    # Maestro context
    # Issue #19: Add ON DELETE CASCADE to clean up ledgers when project/track deleted
    project_id = Column(Integer, ForeignKey("maestro_projects.id", ondelete="CASCADE"), nullable=True)  # type: ignore
    track_id = Column(Integer, ForeignKey("maestro_tracks.id", ondelete="CASCADE"), nullable=True)  # type: ignore

    # Relationships
    project = relationship("MaestroProject", backref="continuity_ledgers")  # type: ignore[assignment]
    track = relationship("MaestroTrack", backref="continuity_ledgers")  # type: ignore[assignment]
    parent = relationship("ContinuityLedger", remote_side=[id], backref="children")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_ledgers_session", "session_id"),
        Index("idx_ledgers_agent", "agent_id"),
        Index("idx_ledgers_entry_type", "entry_type"),
        Index("idx_ledgers_sequence", "session_id", "sequence_number"),
        CheckConstraint(
            "entry_type IN ('decision', 'action', 'outcome', 'observation', 'question', 'answer')",
            name="check_entry_type"
        ),
    )

    def __repr__(self) -> str:
        return f"<ContinuityLedger(id={self.id}, ledger_id='{self.ledger_id}', type='{self.entry_type}')>"

    def get_metadata(self) -> Optional[Dict[str, Any]]:
        """
        Issue #27: Property accessor for metadata.
        """
        return self._metadata_internal  # type: ignore

    def set_metadata(self, value: Optional[Dict[str, Any]]) -> None:
        """Setter for metadata."""
        self._metadata_internal = value  # type: ignore

    def to_dict(self) -> Dict[str, Any]:
        """Convert ledger entry to dictionary"""
        return {
            "id": self.id,
            "ledger_id": self.ledger_id,
            "session_id": self.session_id,
            "agent_id": self.agent_id,
            "entry_type": self.entry_type,
            "title": self.title,
            "content": self.content,
            "metadata": self._metadata_internal or {},
            "parent_entry_id": self.parent_entry_id,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "sequence_number": self.sequence_number,
            "project_id": self.project_id,
            "track_id": self.track_id,
        }


class TaskSpecification(Base):
    """
    Task specifications for persistent task tracking

    Stores structured task definitions that can be referenced across sessions
    and tracked through completion.
    """

    __tablename__ = "task_specifications"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore

    # Task identification
    task_id = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore
    title = Column(String(500), nullable=False)  # type: ignore

    # Task details
    description = Column(Text, nullable=True)  # type: ignore
    specification = Column(JSON, nullable=False)  # type: ignore  # Structured task specification
    requirements = Column(JSON, nullable=True)  # type: ignore  # List of requirements
    acceptance_criteria = Column(JSON, nullable=True)  # type: ignore  # List of acceptance criteria

    # Task classification
    task_type = Column(String(100), nullable=True, index=True)  # type: ignore  # feature, bugfix, refactor, etc.
    priority = Column(String(50), nullable=True, index=True)  # type: ignore  # critical, high, normal, low
    complexity = Column(Integer, nullable=True)  # type: ignore  # 1-10 scale

    # Status tracking
    status = Column(String(50), nullable=False, default="pending", index=True)  # type: ignore  # pending, in_progress, completed, blocked
    progress = Column(Float, default=0.0)  # type: ignore  # 0.0 to 1.0

    # Assignment
    assigned_to = Column(String(200), nullable=True, index=True)  # type: ignore  # Agent ID
    session_id = Column(String(200), nullable=True, index=True)  # type: ignore

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)  # type: ignore
    updated_at = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)  # type: ignore
    started_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore
    completed_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore
    due_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore

    # Maestro context
    # Issue #19: Add ON DELETE CASCADE to clean up tasks when project/track deleted
    project_id = Column(Integer, ForeignKey("maestro_projects.id", ondelete="CASCADE"), nullable=True, index=True)  # type: ignore
    track_id = Column(Integer, ForeignKey("maestro_tracks.id", ondelete="CASCADE"), nullable=True, index=True)  # type: ignore
    # Also cascade for parent task deletion
    parent_task_id = Column(Integer, ForeignKey("task_specifications.id", ondelete="CASCADE"), nullable=True)  # type: ignore

    # Relationships
    project = relationship("MaestroProject", backref="tasks")  # type: ignore[assignment]
    track = relationship("MaestroTrack", backref="tasks")  # type: ignore[assignment]
    parent_task = relationship("TaskSpecification", remote_side=[id], backref="subtasks")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_tasks_type_status", "task_type", "status"),
        Index("idx_tasks_priority", "priority"),
        Index("idx_tasks_assigned", "assigned_to"),
        CheckConstraint("progress >= 0.0 AND progress <= 1.0", name="check_progress_range"),
        CheckConstraint("complexity >= 1 AND complexity <= 10", name="check_complexity_range"),
    )

    def __repr__(self) -> str:
        return f"<TaskSpecification(id={self.id}, task_id='{self.task_id}', title='{self.title}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert task to dictionary"""
        return {
            "id": self.id,
            "task_id": self.task_id,
            "title": self.title,
            "description": self.description,
            "specification": self.specification,
            "requirements": self.requirements or [],
            "acceptance_criteria": self.acceptance_criteria or [],
            "task_type": self.task_type,
            "priority": self.priority,
            "complexity": self.complexity,
            "status": self.status,
            "progress": self.progress,
            "assigned_to": self.assigned_to,
            "session_id": self.session_id,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "updated_at": self.updated_at.isoformat() if self.updated_at else None,
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "completed_at": self.completed_at.isoformat() if self.completed_at else None,
            "due_at": self.due_at.isoformat() if self.due_at else None,
            "project_id": self.project_id,
            "track_id": self.track_id,
            "parent_task_id": self.parent_task_id,
        }


class Session(Base, AuditLoggable):
    """
    Session tracking for Maestro operations

    Tracks individual sessions (conversations, work sessions) with their
    metadata, timing, and relationships for continuity and analysis.

    Includes unique constraint handling for UUID collisions (Issue #18).
    """

    __tablename__ = "sessions"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore

    # Session identification
    session_id = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore

    # Session details
    session_type = Column(String(100), nullable=False, index=True)  # type: ignore  # cli, tui, api, agent
    title = Column(String(500), nullable=True)  # type: ignore
    description = Column(Text, nullable=True)  # type: ignore

    # Agent association
    agent_id = Column(String(200), nullable=True, index=True)  # type: ignore
    agent_name = Column(String(200), nullable=True)  # type: ignore

    # Status
    status = Column(String(50), nullable=False, default=SessionStatus.ACTIVE.value, index=True)  # type: ignore

    # Context
    project_path = Column(String(500), nullable=True)  # type: ignore
    working_directory = Column(String(500), nullable=True)  # type: ignore

    # Metadata
    # Issue #27: Standardized to `metadata` (was `meta_data`)
    _metadata_internal = Column("meta_data", JSON, nullable=True)  # type: ignore
    tags = Column(JSON, nullable=True)  # type: ignore

    # Statistics
    message_count = Column(Integer, default=0)  # type: ignore
    tool_use_count = Column(Integer, default=0)  # type: ignore
    memory_count = Column(Integer, default=0)  # type: ignore

    # Parent session (for session chains)
    parent_session_id = Column(String(200), nullable=True)  # type: ignore

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False, index=True)  # type: ignore
    started_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore
    ended_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore
    last_activity = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)  # type: ignore


    # Maestro context
    # Issue #19: Add ON DELETE CASCADE to clean up sessions when project/track deleted
    project_id = Column(Integer, ForeignKey("maestro_projects.id", ondelete="CASCADE"), nullable=True)  # type: ignore
    track_id = Column(Integer, ForeignKey("maestro_tracks.id", ondelete="CASCADE"), nullable=True)  # type: ignore

    # Relationships
    project = relationship("MaestroProject", backref="sessions")  # type: ignore[assignment]
    track = relationship("MaestroTrack", backref="sessions")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_sessions_agent", "agent_id"),
        Index("idx_sessions_type_status", "session_type", "status"),
        Index("idx_sessions_activity", "last_activity"),
        CheckConstraint(
            "status IN ('active', 'paused', 'completed', 'terminated')",
            name="check_session_status"
        ),
        CheckConstraint(
            "session_type IN ('cli', 'tui', 'api', 'agent', 'track')",
            name="check_session_type"
        ),
    )

    def __repr__(self) -> str:
        return f"<Session(id={self.id}, session_id='{self.session_id}', type='{self.session_type}')>"

    def get_metadata(self) -> Optional[Dict[str, Any]]:
        """
        Issue #27: Property accessor for metadata.
        """
        return self._metadata_internal  # type: ignore

    def set_metadata(self, value: Optional[Dict[str, Any]]) -> None:
        """Setter for metadata."""
        self._metadata_internal = value  # type: ignore

    def to_dict(self) -> Dict[str, Any]:
        """Convert session to dictionary"""
        return {
            "id": self.id,
            "session_id": self.session_id,
            "session_type": self.session_type,
            "title": self.title,
            "description": self.description,
            "agent_id": self.agent_id,
            "agent_name": self.agent_name,
            "status": self.status,
            "project_path": self.project_path,
            "working_directory": self.working_directory,
            "metadata": self._metadata_internal or {},
            "tags": self.tags or [],
            "message_count": self.message_count,
            "tool_use_count": self.tool_use_count,
            "memory_count": self.memory_count,
            "parent_session_id": self.parent_session_id,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "ended_at": self.ended_at.isoformat() if self.ended_at else None,
            "last_activity": self.last_activity.isoformat() if self.last_activity else None,
            "project_id": self.project_id,
            "track_id": self.track_id,
        }

    def is_active(self) -> bool:
        """Check if session is currently active"""
        return self.status == SessionStatus.ACTIVE.value  # type: ignore

    def duration_seconds(self) -> Optional[int]:
        """Calculate session duration in seconds"""
        if not self.started_at:
            return None

        end = self.ended_at if self.ended_at else datetime.now(UTC)
        start = self.started_at.replace(tzinfo=UTC) if self.started_at.tzinfo is None else self.started_at
        end = end.replace(tzinfo=UTC) if end.tzinfo is None else end

        return int((end - start).total_seconds())


class MaestroProject(Base):
    """
    Maestro project registry

    Tracks all projects managed by Maestro with their metadata.
    """

    __tablename__ = "maestro_projects"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore
    project_path = Column(String, unique=True, nullable=False, index=True)  # type: ignore
    project_name = Column(String(200), nullable=True)  # type: ignore
    description = Column(Text, nullable=True)  # type: ignore
    project_type = Column(String(50), nullable=True, index=True)  # type: ignore  # greenfield, brownfield
    tech_stack = Column(JSON, nullable=True)  # type: ignore
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)  # type: ignore
    last_active = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True, index=True)  # type: ignore

    __table_args__ = (
        Index("idx_maestro_project_path", "project_path"),
        Index("idx_maestro_project_type", "project_type"),
    )

    def __repr__(self) -> str:
        return f"<MaestroProject(id={self.id}, name='{self.project_name}', path='{self.project_path}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert project to dictionary"""
        return {
            "id": self.id,
            "project_path": self.project_path,
            "project_name": self.project_name,
            "description": self.description,
            "project_type": self.project_type,
            "tech_stack": self.tech_stack or {},
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "last_active": self.last_active.isoformat() if self.last_active else None,
        }

    @classmethod
    async def get_or_create(cls, session: AsyncSession, project_path: str, **kwargs: Any) -> "MaestroProject":
        """Get existing project or create a new one."""
        from sqlalchemy import select
        stmt = select(cls).where(cls.project_path == project_path)
        result = await session.execute(stmt)
        instance = result.scalars().first()
        if instance:
            return instance

        instance = cls(project_path=project_path, **kwargs)
        session.add(instance)
        return instance


class MaestroTrack(Base):
    """
    Maestro track registry

    Tracks all development tracks within projects.
    """

    __tablename__ = "maestro_tracks"

    id = Column(Integer, primary_key=True, index=True)  # type: ignore
    track_id = Column(String(200), unique=True, nullable=False, index=True)  # type: ignore
    project_id = Column(Integer, ForeignKey("maestro_projects.id"), nullable=False, index=True)  # type: ignore
    title = Column(String(500), nullable=False)  # type: ignore
    description = Column(Text, nullable=True)  # type: ignore
    status = Column(String(50), nullable=False, default="new", index=True)  # type: ignore
    track_type = Column(String(50), nullable=True, index=True)  # type: ignore

    # Track metadata
    phase_count = Column(Integer, default=0)  # type: ignore
    current_phase = Column(Integer, default=0)  # type: ignore
    total_tasks = Column(Integer, default=0)  # type: ignore
    completed_tasks = Column(Integer, default=0)  # type: ignore

    # Timestamps
    created_at = Column(DateTime(timezone=True), server_default=func.now(), nullable=False)  # type: ignore
    updated_at = Column(DateTime(timezone=True), onupdate=func.now(), nullable=True)  # type: ignore

    started_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore
    completed_at = Column(DateTime(timezone=True), nullable=True)  # type: ignore

    # Relationships
    project = relationship("MaestroProject", backref="tracks")  # type: ignore[assignment]

    __table_args__ = (
        Index("idx_maestro_track_id", "track_id"),
        Index("idx_maestro_project_tracks", "project_id"),
        Index("idx_maestro_track_status", "status"),
        Index("idx_maestro_track_type", "track_type"),
    )

    def __repr__(self) -> str:
        return f"<MaestroTrack(id={self.id}, track_id='{self.track_id}', title='{self.title}')>"

    def to_dict(self) -> Dict[str, Any]:
        """Convert track to dictionary"""
        return {
            "id": self.id,
            "track_id": self.track_id,
            "project_id": self.project_id,
            "title": self.title,
            "description": self.description,
            "status": self.status,
            "track_type": self.track_type,
            "phase_count": self.phase_count,
            "current_phase": self.current_phase,
            "total_tasks": self.total_tasks,
            "completed_tasks": self.completed_tasks,
            "created_at": self.created_at.isoformat() if self.created_at else None,
            "updated_at": self.updated_at.isoformat() if self.updated_at else None,
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "completed_at": self.completed_at.isoformat() if self.completed_at else None,
        }

    @classmethod
    async def get_or_create(cls, session: AsyncSession, track_id: str, **kwargs: Any) -> "MaestroTrack":
        """Get existing track or create a new one."""
        from sqlalchemy import select
        stmt = select(cls).where(cls.track_id == track_id)
        result = await session.execute(stmt)
        instance = result.scalars().first()
        if instance:
            return instance

        instance = cls(track_id=track_id, **kwargs)
        session.add(instance)
        return instance


def get_engine_url(db_path: Optional[str] = None) -> str:
    """
    Get the database engine URL

    Args:
        db_path: Path to SQLite database file

    Returns:
        SQLAlchemy database URL
    """
    import os

    if db_path is None:
        db_path = os.path.expanduser("~/.maestro/memory.db")

    return f"sqlite:///{db_path}"


# ============================================================================
# DISK SPACE CHECKING (Issue #16)
# ============================================================================

class DiskSpaceError(Exception):
    """Raised when disk space is insufficient for database operations."""
    pass


def check_disk_space(
    path: str,
    required_bytes: int = 10 * 1024 * 1024,  # Default 10MB
    safety_margin: float = 0.1,  # 10% safety margin
) -> bool:
    """
    Check if there is sufficient disk space for database operations.

    Args:
        path: Path to check disk space for
        required_bytes: Minimum bytes required
        safety_margin: Additional safety margin (0.1 = 10%)

    Returns:
        True if sufficient space

    Raises:
        DiskSpaceError: If insufficient disk space
        OSError: If disk space check fails
    """
    import os
    import shutil

    try:
        # Get the disk stats for the path
        stat = shutil.disk_usage(os.path.dirname(os.path.abspath(path)))

        # Calculate available space with safety margin
        available = stat.free
        required_with_margin = int(required_bytes * (1 + safety_margin))

        if available < required_with_margin:
            raise DiskSpaceError(
                f"Insufficient disk space: {available} bytes available, "
                f"required {required_with_margin} bytes (including {safety_margin*100}% margin)"
            )

        return True

    except OSError as e:
        # Re-raise OS errors (including disk space errors)
        logger.error(f"Disk space check failed for {path}: {e}")
        raise


def ensure_disk_space_before_write(
    db_path: str,
    estimated_write_size: int = 1024 * 1024,  # Default 1MB
) -> None:
    """
    Ensure sufficient disk space before database write operations.

    This should be called before any write operation to prevent
    database corruption due to disk full conditions.

    Args:
        db_path: Database file path
        estimated_write_size: Estimated size of data to be written

    Raises:
        DiskSpaceError: If insufficient disk space
    """
    try:
        check_disk_space(db_path, required_bytes=estimated_write_size)
    except DiskSpaceError:
        # Log the error for audit
        log_audit_event(
            operation="write_attempt_failed",
            entity_type="database",
            entity_id=db_path,
            changes={"required_bytes": estimated_write_size},
            metadata={"reason": "insufficient_disk_space"},
            status="failure"
        )
        raise


# ============================================================================
# UUID COLLISION HANDLING (Issue #18)
# ============================================================================

class UUIDCollisionError(Exception):
    """Raised when a UUID collision is detected and retries are exhausted."""

    def __init__(self, message: str, attempts: int, collided_id: str) -> None:
        super().__init__(message)
        self.attempts = attempts
        self.collided_id = collided_id


def generate_unique_session_id(
    session: ORMSession,
    max_attempts: int = 5,
    prefix: str = "session",
) -> str:
    """
    Generate a unique session ID with collision retry logic.

    In the extremely unlikely event of a UUID collision, this function
    will retry with new UUIDs up to max_attempts times.

    Args:
        session: SQLAlchemy session for checking uniqueness
        max_attempts: Maximum number of attempts before raising error
        prefix: Optional prefix for the session ID

    Returns:
        Unique session ID string

    Raises:
        UUIDCollisionError: If unable to generate unique ID after max_attempts
    """
    import uuid

    for attempt in range(max_attempts):
        # Generate a new UUID-based ID
        session_id = f"{prefix}-{uuid.uuid4().hex[:16]}"

        # Check if it already exists
        from sqlalchemy import select
        existing = session.execute(
            select(Session).where(Session.session_id == session_id)
        ).scalar_one_or_none()

        if existing is None:
            # No collision, return the unique ID
            return session_id

        # Collision detected - log and retry
        logger.warning(
            f"UUID collision detected for session_id '{session_id}' "
            f"(attempt {attempt + 1}/{max_attempts}). Retrying..."
        )

        # Log audit event for collision
        log_audit_event(
            operation="uuid_collision",
            entity_type="Session",
            entity_id=session_id,
            metadata={"attempt": attempt + 1, "max_attempts": max_attempts},
            status="retry"
        )

    # All attempts exhausted
    raise UUIDCollisionError(
        f"Failed to generate unique session ID after {max_attempts} attempts",
        attempts=max_attempts,
        collided_id=session_id
    )


# Global engine registry for proper connection management
_engines: Dict[str, "Engine"] = {}


# ============================================================================
# CONNECTION POOL CONFIGURATION (Issue #24)
# ============================================================================

# SQLite-specific pool configuration
# For SQLite, we use a smaller pool since SQLite has limited concurrent write support
SQLITE_POOL_SIZE = 5
SQLITE_MAX_OVERFLOW = 10  # Maximum additional connections beyond pool_size
SQLITE_POOL_TIMEOUT = 30  # Seconds to wait before giving up on connection
SQLITE_POOL_RECYCLE = 3600  # Recycle connections after 1 hour


def _register_engine(engine: "Engine", db_path: Optional[str] = None) -> None:
    """Register an engine for later cleanup"""
    key = db_path or "default"
    _engines[key] = engine


def _cleanup_engine(db_path: Optional[str] = None) -> None:
    """Dispose and cleanup an engine"""
    from sqlalchemy import create_engine

    key = db_path or "default"
    if key in _engines:
        _engines[key].dispose()
        del _engines[key]


def cleanup_all_engines() -> None:
    """Dispose all registered engines for proper cleanup"""
    for engine in list(_engines.values()):
        engine.dispose()
    _engines.clear()


def create_tables(engine: Optional["Engine"] = None, db_path: Optional[str] = None, pool_size: Optional[int] = None, max_overflow: Optional[int] = None) -> "Engine":
    """
    Create all database tables

    Issue #24: Added connection pooling configuration for SQLite.

    Args:
        engine: SQLAlchemy engine (optional)
        db_path: Path to database file (used if engine not provided)
        pool_size: Connection pool size (default: SQLITE_POOL_SIZE)
        max_overflow: Max overflow connections (default: SQLITE_MAX_OVERFLOW)

    Returns:
        Created SQLAlchemy engine
    """
    from sqlalchemy import create_engine
    from sqlalchemy.pool import QueuePool
    import os

    if engine is None:
        engine_url = get_engine_url(db_path)
        # Ensure directory exists before creating engine
        if db_path:
            os.makedirs(os.path.dirname(db_path), exist_ok=True)

        # Issue #24: Use QueuePool with configurable limits for SQLite
        # This prevents unlimited connection growth
        pool_size = pool_size or SQLITE_POOL_SIZE
        max_overflow = max_overflow or SQLITE_MAX_OVERFLOW

        engine = create_engine(
            engine_url,
            poolclass=QueuePool,
            pool_size=pool_size,
            max_overflow=max_overflow,
            pool_timeout=SQLITE_POOL_TIMEOUT,
            pool_recycle=SQLITE_POOL_RECYCLE,
            connect_args={"check_same_thread": False},
        )
        _register_engine(engine, db_path)

    # Enable WAL mode for better concurrency
    from sqlalchemy import text
    with engine.begin() as conn:
        conn.execute(text("PRAGMA journal_mode=WAL"))
        conn.execute(text("PRAGMA foreign_keys=ON"))
    # No explicit commit needed - engine.begin() is a context manager

    # Create all tables
    Base.metadata.create_all(engine)

    return engine


def get_session(engine: Optional["Engine"] = None, db_path: Optional[str] = None, pool_size: Optional[int] = None, max_overflow: Optional[int] = None) -> ORMSession:
    """
    Get a database session

    Issue #24: Added connection pooling configuration.

    Note: The caller is responsible for closing the session.
    Consider using the session context manager for proper cleanup.
    Note: Tables must be created before using this session (call create_tables first).

    Args:
        engine: SQLAlchemy engine (optional)
        db_path: Path to database file (used if engine not provided)
        pool_size: Connection pool size (default: SQLITE_POOL_SIZE)
        max_overflow: Max overflow connections (default: SQLITE_MAX_OVERFLOW)

    Returns:
        SQLAlchemy session
    """
    from sqlalchemy import create_engine
    from sqlalchemy.orm import sessionmaker, Session as ORMSession
    from sqlalchemy.pool import QueuePool
    import os

    if engine is None:
        engine_url = get_engine_url(db_path)
        # Ensure directory exists before creating engine
        if db_path:
            os.makedirs(os.path.dirname(db_path), exist_ok=True)

        # Issue #24: Use QueuePool with configurable limits
        pool_size = pool_size or SQLITE_POOL_SIZE
        max_overflow = max_overflow or SQLITE_MAX_OVERFLOW

        engine = create_engine(
            engine_url,
            poolclass=QueuePool,
            pool_size=pool_size,
            max_overflow=max_overflow,
            pool_timeout=SQLITE_POOL_TIMEOUT,
            pool_recycle=SQLITE_POOL_RECYCLE,
            connect_args={"check_same_thread": False},
        )
        _register_engine(engine, db_path)

    SessionLocal = sessionmaker(bind=engine, expire_on_commit=False)
    return SessionLocal()


@contextmanager
def get_session_context(engine: Optional["Engine"] = None, db_path: Optional[str] = None) -> Generator[ORMSession, None, None]:
    """
    Context manager for database sessions with automatic cleanup.

    Usage:
        with get_session_context() as session:
            session.add(model)
            session.commit()

    Args:
        engine: SQLAlchemy engine (optional)
        db_path: Path to database file (used if engine not provided)

    Yields:
        SQLAlchemy session
    """
    from contextlib import contextmanager as _contextmanager

    session = get_session(engine, db_path)
    try:
        yield session
        session.commit()
    except Exception:
        session.rollback()
        raise
    finally:
        session.close()


# ============================================================================
# THREAD-LOCAL SESSION MANAGEMENT (Issue #31: Concurrency Fixes)
# ============================================================================

# Thread-local storage for session management
_thread_local = local()


@contextmanager
def get_thread_local_session(engine: Optional["Engine"] = None, db_path: Optional[str] = None) -> Generator[ORMSession, None, None]:
    """
    Get a thread-local database session with automatic cleanup.

    This ensures each thread gets its own session instance, preventing
    SQLAlchemy session thread-safety violations.

    Usage:
        with get_thread_local_session() as session:
            # Use session
            pass

    Args:
        engine: SQLAlchemy engine (optional)
        db_path: Path to database file (used if engine not provided)

    Yields:
        Thread-local SQLAlchemy session
    """
    from sqlalchemy import create_engine
    from sqlalchemy.pool import QueuePool

    # Check if thread already has a session
    if hasattr(_thread_local, 'session') and _thread_local.session is not None:
        # Verify session is still valid
        try:
            from sqlalchemy import func, select
            _thread_local.session.execute(select(func.count()))
            yield _thread_local.session
            return
        except Exception:
            # Session is stale, close and create new one
            try:
                _thread_local.session.close()
            except Exception:
                pass

    # Create new session for this thread
    if engine is None:
        engine_url = get_engine_url(db_path)
        if db_path:
            os.makedirs(os.path.dirname(db_path), exist_ok=True)

        engine = create_engine(
            engine_url,
            poolclass=QueuePool,
            pool_size=SQLITE_POOL_SIZE,
            max_overflow=SQLITE_MAX_OVERFLOW,
            pool_timeout=SQLITE_POOL_TIMEOUT,
            pool_recycle=SQLITE_POOL_RECYCLE,
            connect_args={"check_same_thread": False},
        )
        _register_engine(engine, db_path)

    SessionLocal = sessionmaker(bind=engine, expire_on_commit=False)
    _thread_local.session = SessionLocal()

    try:
        yield _thread_local.session
    except Exception:
        _thread_local.session.rollback()
        raise
    finally:
        # Clean up thread-local session
        try:
            _thread_local.session.close()
        except Exception:
            pass
        _thread_local.session = None


def cleanup_thread_local_session() -> None:
    """
    Clean up the current thread's session.

    Should be called when a thread finishes to ensure proper cleanup.
    """
    if hasattr(_thread_local, 'session') and _thread_local.session is not None:
        try:
            _thread_local.session.close()
        except Exception:
            pass
        _thread_local.session = None
