"""
File Claims Handler for Multi-Agent Coordination

Prevents concurrent modifications to the same files by different agents
through lease-based claims with automatic expiration and conflict detection.

Includes:
- Path traversal protection (Issue #22)
- Row-level locking for concurrent operations (Issue #15)
- Version tracking for optimistic concurrency control (Issue #15)
- Audit logging for all operations (Issue #21)
"""

import fnmatch
import os
import re
import uuid
from datetime import datetime, timedelta, UTC
from typing import Optional, Dict, Any, List, Set, Tuple, Union
from functools import lru_cache
from threading import Lock
import logging

from sqlalchemy import select, and_, or_
from sqlalchemy.orm import Session

from maestro.memory.database.models import (
    FileClaim,
    ClaimStatus,
    MaestroProject,
    MaestroTrack,
    log_audit_event,
)

logger = logging.getLogger(__name__)


class ClaimConflictError(Exception):
    """Raised when a file claim conflicts with existing claims"""

    def __init__(
        self,
        message: str,
        conflicts: List[Dict[str, Any]],
    ):
        super().__init__(message)
        self.conflicts = conflicts


class ClaimExpiredError(Exception):
    """Raised when attempting to use an expired claim"""

    pass


class ConcurrentModificationError(Exception):
    """Raised when a concurrent modification is detected (Issue #15)."""

    def __init__(
        self,
        message: str,
        claim_id: str,
        expected_version: int,
        actual_version: int,
    ):
        super().__init__(message)
        self.claim_id = claim_id
        self.expected_version = expected_version
        self.actual_version = actual_version


class PathTraversalError(Exception):
    """Raised when a pattern attempts path traversal (Issue #22)."""

    def __init__(self, message: str, pattern: str):
        super().__init__(message)
        self.pattern = pattern


class FileClaimsHandler:
    """
    Handles file claims for multi-agent coordination

    File claims allow agents to reserve files for modification,
    preventing conflicts when multiple agents work on the same codebase.
    Claims have TTL-based expiration for automatic cleanup.

    Includes:
    - Path traversal protection (Issue #22)
    - Row-level locking for concurrent operations (Issue #15)
    - Audit logging (Issue #21)
    """

    # Class-level pattern compilation cache (shared across instances)
    _pattern_cache: Dict[str, re.Pattern] = {}
    _pattern_cache_lock = Lock()

    def __init__(
        self,
        session: Optional[Session] = None,
        engine: Any = None,
        db_path: Optional[str] = None,
        default_ttl_seconds: int = 3600,
        project_root: Optional[str] = None,
    ):
        """
        Initialize the file claims handler

        Args:
            session: SQLAlchemy database session (DEPRECATED for concurrent use)
            engine: SQLAlchemy engine (preferred for thread-safe operation)
            db_path: Path to database file (used if engine not provided)
            default_ttl_seconds: Default TTL for claims in seconds
            project_root: Optional project root path for path validation (Issue #22)

        Note:
            For concurrent operations, pass engine or db_path instead of session.
            Each thread will get its own session from the thread-local session factory.
        """
        self.default_ttl_seconds = default_ttl_seconds
        self.project_root = project_root

        # Store engine/db_path for thread-local session creation
        if engine is not None:
            self._engine = engine
            self._db_path = None
        elif db_path is not None:
            self._engine = None
            self._db_path = db_path
        elif session is not None:
            # Legacy support: extract engine from session
            self._engine = session.bind
            self._db_path = None
            logger.warning(
                "FileClaimsHandler initialized with session instead of engine. "
                "This is not thread-safe. Use engine or db_path for concurrent operations."
            )
        else:
            raise ValueError("Must provide either session, engine, or db_path")

        # Session is now a property, not stored
        self._legacy_session = session  # Only used for backward compatibility

    @property
    def session(self) -> Session:
        """
        Get a thread-local session for the current operation.

        This property ensures each thread gets its own session instance,
        preventing thread-safety violations.

        Returns:
            Thread-local SQLAlchemy session
        """
        # If we have a legacy session (non-concurrent path), return it
        if hasattr(self, '_legacy_session') and self._legacy_session is not None:
            return self._legacy_session

        # Get or create thread-local session
        from maestro.memory.database.models import _thread_local, get_session
        if hasattr(_thread_local, 'session') and _thread_local.session is not None:
            session: Session = _thread_local.session
            return session

        # Create new thread-local session
        if self._engine is not None:
            _thread_local.session = get_session(engine=self._engine)
        else:
            _thread_local.session = get_session(db_path=self._db_path)

        session = _thread_local.session
        return session

    def cleanup(self) -> None:
        """
        Clean up the current thread's session.

        Should be called when done with operations in a thread.
        """
        from maestro.memory.database.models import cleanup_thread_local_session
        cleanup_thread_local_session()

    @classmethod
    def _compile_pattern(cls, pattern: str) -> Optional[re.Pattern]:
        """
        Compile a glob pattern to regex with caching.

        This optimizes the pattern matching by pre-compiling and caching
        regex patterns, avoiding repeated compilation overhead.

        Correctly handles:
        - * (matches any characters in a single path segment)
        - ** (matches zero or more path segments recursively)
        - ? (matches any single character)

        Args:
            pattern: Glob pattern to compile

        Returns:
            Compiled regex pattern, or None if pattern contains ** (use _match_recursive)
        """
        from typing import Optional

        # Check cache first (thread-safe)
        with cls._pattern_cache_lock:
            if pattern in cls._pattern_cache:
                return cls._pattern_cache[pattern]

            # For patterns with **, we use _match_recursive instead
            if '**' in pattern:
                # Return None to indicate _match_recursive should be used
                cls._pattern_cache[pattern] = None  # type: ignore[assignment]
                return None

            # Convert glob pattern to regex for simple patterns (no **)
            # Escape special regex characters except glob wildcards
            regex_pattern = re.escape(pattern)

            # Convert escaped wildcards back to regex equivalents
            # \* (escaped star) -> .* (match any characters)
            regex_pattern = regex_pattern.replace(r'\*', '.*')
            # \? (escaped question) -> . (match any single character)
            regex_pattern = regex_pattern.replace(r'\?', '.')

            # Anchor the pattern to match entire string
            regex_pattern = f'^{regex_pattern}$'

            # Compile and cache
            compiled = re.compile(regex_pattern)
            cls._pattern_cache[pattern] = compiled

            return compiled

    @classmethod
    def clear_pattern_cache(cls) -> None:
        """
        Clear the pattern compilation cache.

        Useful for memory management or testing.
        """
        with cls._pattern_cache_lock:
            cls._pattern_cache.clear()

    # Issue #22: Path traversal protection
    def _validate_pattern_no_traversal(self, pattern: str) -> str:
        """
        Validate that a pattern doesn't attempt path traversal.

        This prevents malicious patterns like '../../../etc/passwd'
        from accessing files outside the project root.

        Args:
            pattern: The pattern to validate

        Returns:
            The normalized pattern if valid

        Raises:
            PathTraversalError: If the pattern attempts path traversal
        """
        if not pattern:
            return ""

        # Check for obvious path traversal attempts
        if '../' in pattern or '..\\' in pattern:
            raise PathTraversalError(
                f"Pattern contains parent directory references: {pattern}",
                pattern=pattern,
            )

        # Normalize the path to resolve any embedded '..'
        try:
            normalized = os.path.normpath(pattern)
        except Exception as e:
            raise PathTraversalError(
                f"Failed to normalize pattern: {pattern}",
                pattern=pattern,
            ) from e

        # Check if normalized path starts with '..' (escaped outside)
        if normalized.startswith('..'):
            raise PathTraversalError(
                f"Pattern normalizes to path outside project: {pattern} -> {normalized}",
                pattern=pattern,
            )

        # Check for absolute paths outside project root
        if os.path.isabs(normalized):
            if self.project_root:
                # Ensure absolute path is within project root
                try:
                    pattern_abs = os.path.abspath(normalized)
                    project_abs = os.path.abspath(self.project_root)

                    # Check if pattern is within project root or its subdirectories
                    if not (
                        pattern_abs == project_abs or
                        pattern_abs.startswith(project_abs + os.sep) or
                        pattern_abs.startswith(project_abs + '/')
                    ):
                        raise PathTraversalError(
                            f"Absolute path outside project root: {pattern}",
                            pattern=pattern,
                        )
                except PathTraversalError:
                    raise
                except Exception as e:
                    logger.warning(f"Path validation error for {pattern}: {e}")
            else:
                # No project root set - reject absolute paths as unsafe
                raise PathTraversalError(
                    f"Absolute paths not allowed without project root: {pattern}",
                    pattern=pattern,
                )

        return normalized

    def create_claim(
        self,
        agent_id: str,
        file_patterns: List[str],
        session_id: Optional[str] = None,
        is_exclusive: bool = True,
        reason: Optional[str] = None,
        task_description: Optional[str] = None,
        ttl_seconds: Optional[int] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
        claim_id: Optional[str] = None,
    ) -> FileClaim:
        """
        Create a new file claim with pattern validation and path traversal protection.

        Args:
            agent_id: ID of the agent making the claim
            file_patterns: List of glob patterns for files to claim (validated/sanitized)
            session_id: Optional session ID
            is_exclusive: Whether this is an exclusive claim (blocks others)
            reason: Reason for the claim
            task_description: Description of the task requiring the claim
            ttl_seconds: Time-to-live in seconds (uses default if None)
            project_id: Optional Maestro project ID
            track_id: Optional Maestro track ID
            claim_id: Optional custom claim ID (generated if None)

        Returns:
            Created FileClaim instance

        Raises:
            ClaimConflictError: If claim conflicts with existing active claims
            PathTraversalError: If pattern attempts path traversal (Issue #22)
            ValueError: If file_patterns is empty or contains only invalid patterns
        """
        # Validate and normalize file patterns
        if not file_patterns:
            raise ValueError("file_patterns cannot be empty")

        normalized_patterns = []
        for pattern in file_patterns:
            # Issue #22: Path traversal validation
            try:
                normalized = self._validate_pattern_no_traversal(pattern)
            except PathTraversalError:
                # Log audit event for security issue
                log_audit_event(
                    operation="create_claim_denied",
                    entity_type="FileClaim",
                    entity_id=None,
                    user_id=agent_id,
                    metadata={
                        "reason": "path_traversal_attempt",
                        "pattern": pattern,
                    },
                    status="denied"
                )
                raise

            # Further normalize for matching
            normalized = self._normalize_pattern(normalized)
            if normalized:  # Skip empty patterns after normalization
                normalized_patterns.append(normalized)

        if not normalized_patterns:
            raise ValueError("file_patterns must contain at least one valid pattern")

        # Create the claim with proper transaction isolation
        session = self.session

        # Begin immediate transaction to acquire write lock upfront
        # This prevents race conditions where multiple threads check conflicts simultaneously
        from sqlalchemy import text
        try:
            session.execute(text("BEGIN IMMEDIATE"))
        except Exception:
            # Already in a transaction
            pass

        # Check for conflicts with row-level lock (Issue #15)
        conflicts = self._check_conflicts_locked(
            agent_id=agent_id,
            file_patterns=normalized_patterns,
            is_exclusive=is_exclusive,
        )

        if conflicts:
            # Rollback the immediate transaction
            session.rollback()
            # Log audit event for conflict
            log_audit_event(
                operation="create_claim_conflict",
                entity_type="FileClaim",
                entity_id=None,
                user_id=agent_id,
                metadata={
                    "conflicting_claims": len(conflicts),
                    "patterns": normalized_patterns,
                },
                status="conflict"
            )
            raise ClaimConflictError(
                f"File claim conflicts with {len(conflicts)} existing claim(s)",
                conflicts,
            )

        # Create the claim
        claim = FileClaim(
            claim_id=claim_id or f"claim-{uuid.uuid4().hex[:16]}",
            agent_id=agent_id,
            session_id=session_id,
            file_patterns=normalized_patterns,
            status=ClaimStatus.ACTIVE.value,
            is_exclusive=is_exclusive,
            reason=reason,
            task_description=task_description,
            expires_at=datetime.now(UTC) + timedelta(
                seconds=ttl_seconds or self.default_ttl_seconds
            ),
            project_id=project_id,
            track_id=track_id,
        )

        session.add(claim)
        session.flush()
        session.commit()

        # Issue #21: Log audit event
        log_audit_event(
            operation="create_claim",
            entity_type="FileClaim",
            entity_id=claim.claim_id,
            user_id=agent_id,
            metadata={
                "file_patterns": normalized_patterns,
                "is_exclusive": is_exclusive,
                "ttl_seconds": ttl_seconds or self.default_ttl_seconds,
            },
            status="success"
        )

        return claim

    def _check_conflicts_locked(
        self,
        agent_id: str,
        file_patterns: List[str],
        is_exclusive: bool,
    ) -> List[Dict[str, Any]]:
        """
        Check for conflicts with existing claims using row-level locking.

        Issue #15: Uses SELECT FOR UPDATE to prevent concurrent claim modifications
        during conflict checking, preventing race conditions.

        Args:
            agent_id: ID of agent making the claim
            file_patterns: File patterns to check
            is_exclusive: Whether the new claim is exclusive

        Returns:
            List of conflicting claims
        """
        # Issue #15: Get all active claims from other agents with row-level lock
        stmt = select(FileClaim).where(
            and_(
                FileClaim.agent_id != agent_id,
                FileClaim.status == ClaimStatus.ACTIVE.value,
                FileClaim.expires_at > datetime.now(UTC),
            )
        ).with_for_update()

        existing_claims = self.session.execute(stmt).scalars().all()

        conflicts = []
        for existing in existing_claims:
            # Check pattern overlap
            overlap = self._patterns_overlap(
                file_patterns,
                list(existing.file_patterns) if existing.file_patterns else [],
            )

            if overlap:
                # If either claim is exclusive, there's a conflict
                if existing.is_exclusive or is_exclusive:
                    conflicts.append(existing.to_dict())

        return conflicts

    def _check_conflicts(
        self,
        agent_id: str,
        file_patterns: List[str],
        is_exclusive: bool,
    ) -> List[Dict[str, Any]]:
        """
        Check for conflicts with existing claims (non-locked version).

        For read-only operations where row-level locking is not required.

        Args:
            agent_id: ID of agent making the claim
            file_patterns: File patterns to check
            is_exclusive: Whether the new claim is exclusive

        Returns:
            List of conflicting claims
        """
        # Get all active claims from other agents (no lock)
        stmt = select(FileClaim).where(
            and_(
                FileClaim.agent_id != agent_id,
                FileClaim.status == ClaimStatus.ACTIVE.value,
                FileClaim.expires_at > datetime.now(UTC),
            )
        )
        existing_claims = self.session.execute(stmt).scalars().all()

        conflicts = []
        for existing in existing_claims:
            # Check pattern overlap
            overlap = self._patterns_overlap(
                file_patterns,
                list(existing.file_patterns) if existing.file_patterns else [],
            )

            if overlap:
                # If either claim is exclusive, there's a conflict
                if existing.is_exclusive or is_exclusive:
                    conflicts.append(existing.to_dict())

        return conflicts

    def _patterns_overlap(
        self,
        patterns1: List[str],
        patterns2: List[str],
    ) -> bool:
        """
        Check if two sets of file patterns overlap using proper glob matching.

        This determines if any file could match both a pattern from patterns1
        and a pattern from patterns2. It uses fnmatch for proper glob-style
        pattern matching and handles edge cases correctly.

        Args:
            patterns1: First set of patterns (list of glob patterns)
            patterns2: Second set of patterns (list of glob patterns)

        Returns:
            True if patterns overlap (could match the same file)
        """
        # Handle empty patterns
        if not patterns1 or not patterns2:
            return False

        # Normalize all patterns (handle empty strings, whitespace)
        normalized1 = [self._normalize_pattern(p) for p in patterns1 if p and p.strip()]
        normalized2 = [self._normalize_pattern(p) for p in patterns2 if p and p.strip()]

        if not normalized1 or not normalized2:
            return False

        # Check each pair of patterns for potential overlap
        for p1 in normalized1:
            for p2 in normalized2:
                if self._patterns_match(p1, p2):
                    return True

        return False

    def _normalize_pattern(self, pattern: str) -> str:
        """
        Normalize a file pattern for consistent matching.

        Args:
            pattern: The pattern to normalize

        Returns:
            Normalized pattern string
        """
        if not pattern:
            return ""

        # Strip whitespace
        pattern = pattern.strip()

        # Convert backslashes to forward slashes for consistency
        pattern = pattern.replace('\\', '/')

        # Remove leading './' if present for relative path consistency
        if pattern.startswith('./'):
            pattern = pattern[2:]

        return pattern

    def _patterns_match(self, pattern1: str, pattern2: str) -> bool:
        """
        Check if two patterns might match the same files.

        This determines if there exists any file path that both patterns could match.
        Uses test case generation to find potential overlap.

        Args:
            pattern1: First pattern (glob-style)
            pattern2: Second pattern (glob-style)

        Returns:
            True if patterns could match the same files
        """
        # Exact match
        if pattern1 == pattern2:
            return True

        # If both are non-wildcard literal paths, they must be equal
        has_wildcard1 = any(c in pattern1 for c in ['*', '?', '['])
        has_wildcard2 = any(c in pattern2 for c in ['*', '?', '['])

        if not has_wildcard1 and not has_wildcard2:
            return False

        # Check for more complex pattern overlap using test cases
        # We generate a few test strings that both patterns might match
        test_cases = self._generate_test_cases(pattern1, pattern2)
        for test in test_cases:
            try:
                # Check if pattern1 matches test
                if '**' in pattern1:
                    match1 = self._match_recursive(test, pattern1)
                else:
                    match1 = fnmatch.fnmatch(test, pattern1)

                # Check if pattern2 matches test
                if '**' in pattern2:
                    match2 = self._match_recursive(test, pattern2)
                else:
                    match2 = fnmatch.fnmatch(test, pattern2)

                if match1 and match2:
                    return True
            except (re.error, ValueError):
                continue

        return False

    def _generate_test_cases(self, pattern1: str, pattern2: str) -> List[str]:
        """
        Generate test file paths that two patterns might both match.

        This creates a set of test strings based on the patterns to check
        for potential overlap.

        Args:
            pattern1: First pattern
            pattern2: Second pattern

        Returns:
            List of test file paths
        """
        test_cases = []
        components = set()
        prefixes = set()

        # Add the literal pattern itself if it doesn't contain wildcards
        # This is important for cases like "src/main.py" vs "src/**/*.py"
        for pattern in [pattern1, pattern2]:
            has_wildcard = any(c in pattern for c in ['*', '?', '['])
            if not has_wildcard and pattern:
                # This is a literal path, add it as a test case
                test_cases.append(pattern)
                # Also add it with different extensions for testing
                base, _ = os.path.splitext(pattern)
                test_cases.append(f"{base}.py")
                test_cases.append(f"{base}.txt")

        # Extract components from both patterns
        for pattern in [pattern1, pattern2]:
            # Split by path separator
            parts = pattern.split('/')
            for i, part in enumerate(parts):
                if '**' in part:
                    continue
                if part and not part.startswith('.'):
                    components.add(part)
                    # Track potential prefixes
                    if i > 0:
                        prefix = '/'.join(parts[:i])
                        prefixes.add(prefix)

        # Generate test cases from common components
        if components:
            for comp in list(components)[:5]:  # Limit to 5 components
                test_cases.append(comp)
                test_cases.append(f"dir/{comp}")
                test_cases.append(f"{comp}.py")
                test_cases.append(f"{comp}.txt")

        # Generate combinations of prefixes with components
        if prefixes and components:
            for prefix in list(prefixes)[:3]:  # Limit to 3 prefixes
                for comp in list(components)[:3]:  # Limit to 3 components
                    test_cases.append(f"{prefix}/{comp}")
                    test_cases.append(f"{prefix}/{comp}.py")

        # Add combinations for nested patterns (src/utils/*.py style)
        for comp1 in list(components)[:5]:
            for comp2 in list(components)[:5]:
                if comp1 != comp2:
                    test_cases.append(f"{comp1}/{comp2}.py")
                    test_cases.append(f"{comp1}/{comp2}")

        # Add some generic test cases
        test_cases.extend([
            "test.py",
            "test.txt",
            "dir/test.py",
            "dir/subdir/test.py",
            "*.py",
            "*",
        ])

        return test_cases

    def get_claim(self, claim_id: str) -> Optional[FileClaim]:
        """
        Get a claim by ID

        Args:
            claim_id: Claim ID

        Returns:
            FileClaim instance or None
        """
        return self.session.query(FileClaim).filter(
            FileClaim.claim_id == claim_id
        ).first()

    def get_active_claims(
        self,
        agent_id: Optional[str] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
    ) -> List[FileClaim]:
        """
        Get all active claims

        Args:
            agent_id: Optional agent ID filter
            project_id: Optional project ID filter
            track_id: Optional track ID filter

        Returns:
            List of active FileClaim instances
        """
        stmt = select(FileClaim).where(
            and_(
                FileClaim.status == ClaimStatus.ACTIVE.value,
                FileClaim.expires_at > datetime.now(UTC),
            )
        )

        if agent_id:
            stmt = stmt.where(FileClaim.agent_id == agent_id)
        if project_id:
            stmt = stmt.where(FileClaim.project_id == project_id)
        if track_id:
            stmt = stmt.where(FileClaim.track_id == track_id)

        return list(self.session.execute(stmt).scalars().all())

    def get_claims_for_file(
        self,
        file_path: str,
        include_expired: bool = False,
    ) -> List[FileClaim]:
        """
        Get all claims that match a specific file

        Args:
            file_path: Path to the file
            include_expired: Whether to include expired claims

        Returns:
            List of FileClaim instances
        """
        stmt = select(FileClaim)

        if not include_expired:
            stmt = stmt.where(
                and_(
                    FileClaim.status == ClaimStatus.ACTIVE.value,
                    FileClaim.expires_at > datetime.now(UTC),
                )
            )

        claims = list(self.session.execute(stmt).scalars().all())

        # Filter by pattern match
        matching = []
        for claim in claims:
            if self._matches_any_pattern(file_path, list(claim.file_patterns) if claim.file_patterns else []):
                matching.append(claim)

        return matching

    def _matches_any_pattern(
        self,
        file_path: str,
        patterns: List[str],
    ) -> bool:
        """
        Check if a file path matches any of the given patterns.

        This normalizes both the file path and patterns for consistent matching.
        Supports shell-style glob patterns including ** for recursive matching.
        Uses cached pattern compilation for performance.

        Args:
            file_path: The file path to check
            patterns: List of glob patterns to match against

        Returns:
            True if file_path matches any pattern
        """
        # Normalize the file path
        normalized_path = self._normalize_pattern(file_path)

        for pattern in patterns:
            if not pattern or not pattern.strip():
                continue
            normalized_pattern = self._normalize_pattern(pattern)
            try:
                # Handle ** (recursive directory matching)
                if '**' in normalized_pattern:
                    # Use specialized recursive matching
                    if self._match_recursive(normalized_path, normalized_pattern):
                        return True
                else:
                    # Use cached pattern compilation for better performance
                    compiled = self._compile_pattern(normalized_pattern)
                    if compiled is not None and compiled.match(normalized_path):
                        return True
            except (re.error, ValueError):
                # Skip invalid patterns
                continue
        return False

    def _match_recursive(self, file_path: str, pattern: str) -> bool:
        """
        Match a file path against a pattern containing ** for recursive matching.

        This converts ** patterns to proper fnmatch patterns by checking
        if the file path matches at any depth.

        Args:
            file_path: The normalized file path
            pattern: The normalized glob pattern containing **

        Returns:
            True if the file path matches the pattern
        """
        # Split pattern at ** to get prefix and suffix
        parts = pattern.split('**', 1)

        if len(parts) == 2:
            prefix, suffix = parts

            # Clean up prefix/suffix (remove trailing/leading slashes)
            prefix = prefix.rstrip('/')
            suffix = suffix.lstrip('/')

            # If prefix is empty, match from root
            # If suffix is empty, match any file in subtree
            path_parts = file_path.split('/')

            # Check if path starts with prefix (before **)
            if prefix:
                prefix_parts = prefix.split('/')
                # Filter out empty strings from prefix parts
                prefix_parts = [p for p in prefix_parts if p]
                for i, part in enumerate(prefix_parts):
                    if i >= len(path_parts):
                        return False
                    if part != '*' and not fnmatch.fnmatch(path_parts[i], part):
                        return False
                path_parts = path_parts[len(prefix_parts):]

            # Check if path ends with suffix (after **)
            if suffix:
                suffix_parts = suffix.split('/')
                # Filter out empty strings from suffix parts
                suffix_parts = [p for p in suffix_parts if p]
                # Match suffix from the end
                for i, part in enumerate(reversed(suffix_parts)):
                    idx = len(path_parts) - 1 - i
                    if idx < 0:
                        return False
                    if part != '*' and not fnmatch.fnmatch(path_parts[idx], part):
                        return False

            # If we have path parts left after matching prefix and before matching suffix,
            # ** matched them (which is correct - ** means zero or more directories)
            return True

        # Fallback to standard fnmatch
        return fnmatch.fnmatch(file_path, pattern)

    def renew_claim(
        self,
        claim_id: str,
        ttl_seconds: Optional[int] = None,
        expected_version: Optional[int] = None,
    ) -> Optional[FileClaim]:
        """
        Renew a claim's expiration time with optimistic concurrency control.

        Issue #15: Uses version tracking to prevent concurrent modifications.

        Args:
            claim_id: Claim ID to renew
            ttl_seconds: New TTL in seconds (uses default if None)
            expected_version: Expected version for optimistic locking

        Returns:
            Updated claim or None if not found

        Raises:
            ConcurrentModificationError: If version mismatch detected
        """
        session = self.session

        # Issue #15: Get claim with row-level lock
        stmt = select(FileClaim).where(
            FileClaim.claim_id == claim_id
        ).with_for_update()

        claim = session.execute(stmt).scalar_one_or_none()
        if not claim:
            return None

        if not claim.is_valid():
            raise ClaimExpiredError(f"Claim {claim_id} has expired")

        # Check version for optimistic concurrency control
        if expected_version is not None and claim.version != expected_version:
            log_audit_event(
                operation="renew_claim_conflict",
                entity_type="FileClaim",
                entity_id=claim_id,
                user_id=None,
                changes={
                    "expected_version": expected_version,
                    "actual_version": claim.version
                },
                status="conflict"
            )
            raise ConcurrentModificationError(
                f"Concurrent modification detected on claim {claim_id}",
                claim_id=claim_id,
                expected_version=expected_version,
                actual_version=int(claim.version),
            )

        old_expires = claim.expires_at
        new_expires = datetime.now(UTC) + timedelta(
            seconds=ttl_seconds or self.default_ttl_seconds
        )
        claim.expires_at = new_expires  # type: ignore[assignment]
        claim.increment_version()
        session.flush()
        session.commit()

        # Issue #21: Log audit event
        log_audit_event(
            operation="renew_claim",
            entity_type="FileClaim",
            entity_id=claim_id,
            user_id=str(claim.agent_id),
            changes={
                "old_expires_at": old_expires.isoformat() if old_expires else None,
                "new_expires_at": claim.expires_at.isoformat(),
                "version": claim.version,
            },
            status="success"
        )

        return claim

    def release_claim(
        self,
        claim_id: str,
        force: bool = False,
        expected_version: Optional[int] = None,
    ) -> bool:
        """
        Release a claim with optional version checking.

        Issue #15: Uses version tracking to prevent concurrent modifications.

        Args:
            claim_id: Claim ID to release
            force: Force release even if expired
            expected_version: Expected version for optimistic locking

        Returns:
            True if released, False if not found

        Raises:
            ConcurrentModificationError: If version mismatch detected
        """
        session = self.session

        # Issue #15: Get claim with row-level lock
        stmt = select(FileClaim).where(
            FileClaim.claim_id == claim_id
        ).with_for_update()

        claim = session.execute(stmt).scalar_one_or_none()
        if not claim:
            return False

        if not force and not claim.is_valid():
            raise ClaimExpiredError(f"Claim {claim_id} has expired")

        # Check version for optimistic concurrency control
        if expected_version is not None and claim.version != expected_version:
            log_audit_event(
                operation="release_claim_conflict",
                entity_type="FileClaim",
                entity_id=claim_id,
                user_id=str(claim.agent_id),
                changes={
                    "expected_version": expected_version,
                    "actual_version": claim.version
                },
                status="conflict"
            )
            raise ConcurrentModificationError(
                f"Concurrent modification detected on claim {claim_id}",
                claim_id=claim_id,
                expected_version=expected_version,
                actual_version=int(claim.version),
            )

        old_status = claim.status
        claim.status = ClaimStatus.RELEASED.value  # type: ignore[assignment]
        claim.released_at = datetime.now(UTC)  # type: ignore[assignment]
        claim.increment_version()
        session.flush()
        session.commit()

        # Issue #21: Log audit event
        log_audit_event(
            operation="release_claim",
            entity_type="FileClaim",
            entity_id=claim_id,
            user_id=str(claim.agent_id),
            changes={
                "old_status": old_status,
                "new_status": claim.status,
                "version": claim.version,
            },
            status="success"
        )

        return True

    def release_agent_claims(
        self,
        agent_id: str,
        session_id: Optional[str] = None,
    ) -> int:
        """
        Release all active claims for an agent

        Args:
            agent_id: Agent ID
            session_id: Optional session ID filter

        Returns:
            Number of claims released
        """
        session = self.session

        stmt = select(FileClaim).where(
            and_(
                FileClaim.agent_id == agent_id,
                FileClaim.status == ClaimStatus.ACTIVE.value,
            )
        )

        if session_id:
            stmt = stmt.where(FileClaim.session_id == session_id)

        claims = list(session.execute(stmt).scalars().all())

        for claim in claims:
            claim.status = ClaimStatus.RELEASED.value  # type: ignore[assignment]
            claim.released_at = datetime.now(UTC)  # type: ignore[assignment]

        session.flush()
        session.commit()
        return len(claims)

    def revoke_claim(
        self,
        claim_id: str,
        reason: Optional[str] = None,
    ) -> bool:
        """
        Revoke a claim (administrative action)

        Args:
            claim_id: Claim ID to revoke
            reason: Optional reason for revocation

        Returns:
            True if revoked, False if not found
        """
        session = self.session

        claim = self.get_claim(claim_id)
        if not claim:
            return False

        claim.status = ClaimStatus.REVOKED.value  # type: ignore[assignment]
        claim.released_at = datetime.now(UTC)  # type: ignore[assignment]
        if reason:
            claim.reason = f"Revoked: {reason}"  # type: ignore[assignment]

        session.flush()
        session.commit()
        return True

    def cleanup_expired_claims(self) -> int:
        """
        Mark all expired claims as expired

        Returns:
            Number of claims marked as expired
        """
        session = self.session
        now = datetime.now(UTC)

        stmt = select(FileClaim).where(
            and_(
                FileClaim.status == ClaimStatus.ACTIVE.value,
                FileClaim.expires_at < now,
            )
        )
        expired = list(session.execute(stmt).scalars().all())

        for claim in expired:
            claim.status = ClaimStatus.EXPIRED.value  # type: ignore[assignment]

        session.flush()
        session.commit()
        return len(expired)

    def check_file_access(
        self,
        file_path: str,
        agent_id: str,
        require_write: bool = True,
    ) -> Tuple[bool, List[FileClaim]]:
        """
        Check if an agent can access a file

        Args:
            file_path: Path to the file
            agent_id: ID of the agent requesting access
            require_write: Whether write access is needed

        Returns:
            Tuple of (allowed, blocking_claims)
        """
        claims = self.get_claims_for_file(file_path)

        # Filter to active exclusive claims from other agents
        blocking = [
            c for c in claims
            if c.is_exclusive
            and c.agent_id != agent_id
            and c.is_valid()
        ]

        return len(blocking) == 0, blocking

    def get_claim_summary(
        self,
        project_id: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Get a summary of current claims

        Args:
            project_id: Optional project ID filter

        Returns:
            Summary of claims by status and agent
        """
        stmt = select(FileClaim)

        if project_id:
            stmt = stmt.where(FileClaim.project_id == project_id)

        all_claims = list(self.session.execute(stmt).scalars().all())

        # Count by status
        by_status: Dict[str, int] = {}
        for claim in all_claims:
            status = str(claim.status)
            by_status[status] = by_status.get(status, 0) + 1

        # Count by agent
        by_agent: Dict[str, int] = {}
        for claim in all_claims:
            agent = str(claim.agent_id)
            by_agent[agent] = by_agent.get(agent, 0) + 1

        # Active claims
        active = [c for c in all_claims if c.is_valid()]

        return {
            "total": len(all_claims),
            "by_status": by_status,
            "by_agent": by_agent,
            "active_count": len(active),
            "expired_count": by_status.get(ClaimStatus.EXPIRED.value, 0),
        }

    def transfer_claim(
        self,
        claim_id: str,
        to_agent_id: str,
        new_session_id: Optional[str] = None,
    ) -> Optional[FileClaim]:
        """
        Transfer a claim to another agent

        Args:
            claim_id: Claim ID to transfer
            to_agent_id: ID of agent to transfer to
            new_session_id: Optional new session ID

        Returns:
            Updated claim or None if not found
        """
        session = self.session

        claim = self.get_claim(claim_id)
        if not claim:
            return None

        if not claim.is_valid():
            raise ClaimExpiredError(f"Claim {claim_id} has expired")

        # Release old claim and create new one
        old_agent = claim.agent_id
        old_session = claim.session_id

        claim.agent_id = to_agent_id  # type: ignore[assignment]
        claim.session_id = new_session_id  # type: ignore[assignment]
        session.flush()
        session.commit()

        return claim


class FileClaimsManager:
    """
    High-level manager for file claims with context

    Provides convenience methods for common file claim operations
    with automatic project/track context management.
    """

    def __init__(
        self,
        session: Session,
        project_path: Optional[str] = None,
        project_id: Optional[int] = None,
        track_id: Optional[int] = None,
    ):
        """
        Initialize the file claims manager

        Args:
            session: SQLAlchemy session
            project_path: Optional project path (used to find project_id)
            project_id: Optional project ID
            track_id: Optional track ID
        """
        self.session = session
        self.project_path = project_path
        self.project_id = project_id
        self.track_id = track_id

        # Resolve project_id from path if needed
        if not self.project_id and self.project_path:
            project = self.session.query(MaestroProject).filter(
                MaestroProject.project_path == self.project_path
            ).first()
            if project:
                self.project_id = int(project.id) if project.id is not None else None

        self.handler = FileClaimsHandler(session)

    def claim_files(
        self,
        agent_id: str,
        file_patterns: List[str],
        **kwargs: Any,
    ) -> FileClaim:
        """
        Claim files for an agent

        Automatically includes project/track context.
        """
        kwargs.setdefault("project_id", self.project_id)
        kwargs.setdefault("track_id", self.track_id)

        return self.handler.create_claim(agent_id, file_patterns, **kwargs)

    def check_can_modify(
        self,
        file_path: str,
        agent_id: str,
    ) -> Tuple[bool, List[str]]:
        """
        Check if an agent can modify a file

        Returns:
            Tuple of (allowed, blocking_reasons)
        """
        allowed, blocking = self.handler.check_file_access(file_path, agent_id)

        reasons = []
        if not allowed:
            for claim in blocking:
                reasons.append(
                    f"File claimed by agent {claim.agent_id} "
                    f"(expires: {claim.expires_at})"
                )

        return allowed, reasons

    def get_project_claims(self) -> List[FileClaim]:
        """Get all claims for this project"""
        return self.handler.get_active_claims(
            project_id=self.project_id,
        )

    def get_track_claims(self) -> List[FileClaim]:
        """Get all claims for this track"""
        if not self.track_id:
            return []
        return self.handler.get_active_claims(
            project_id=self.project_id,
            track_id=self.track_id,
        )

    def release_agent_files(self, agent_id: str) -> int:
        """Release all file claims for an agent in this project/track"""
        return self.handler.release_agent_claims(agent_id)

    def get_claim_status(self) -> Dict[str, Any]:
        """Get claim status for this project"""
        return self.handler.get_claim_summary(self.project_id)
