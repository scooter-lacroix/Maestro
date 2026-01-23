# Concurrency Fix Implementation Plan for Maestro v2

## Executive Summary

This document provides a comprehensive, maximally detailed implementation plan to fix all identified concurrency issues in the Maestro v2 codebase. The plan addresses critical thread-safety violations, transaction management problems, and pattern compilation bugs that cause test failures.

**Issue Summary:**
1. **SQLAlchemy Session Thread-Safety Violation** - `FileClaimsHandler` shares a single `Session` instance across multiple threads
2. **SQLite Write Concurrency Limitations** - SQLite only supports one writer at a time
3. **Non-atomic version tracking** - version check and increment are separate operations
4. **Pattern compilation bug** - incorrect handling of `**` (recursive wildcard)

**Target:** All 7 concurrency tests must pass after implementation.

---

## Table of Contents

1. [Phase-by-Phase Implementation](#phase-by-phase-implementation)
2. [Detailed Code Changes](#detailed-code-changes)
3. [Database Schema Changes](#database-schema-changes)
4. [Test Fixture Changes](#test-fixture-changes)
5. [Validation Strategy](#validation-strategy)
6. [Risk Assessment](#risk-assessment)
7. [Rollback Plan](#rollback-plan)

---

## Phase-by-Phase Implementation

### Phase 1: Fix Session Management (CRITICAL - Root Cause)

**Problem:** `FileClaimsHandler` receives a single `Session` instance at initialization and shares it across all threads. SQLAlchemy sessions are NOT thread-safe.

**Files to Modify:**
- `/home/stan/Prod/maestro/maestro/memory/coordination/file_claims.py`
- `/home/stan/Prod/maestro/maestro/memory/database/models.py`

**Implementation Steps:**

#### Step 1.1: Create Thread-Local Session Factory

Add a new session factory function in `models.py`:

```python
# Add to models.py after line 1700

from threading import local
from contextlib import contextmanager

# Thread-local storage for session management
_thread_local = local()

@contextmanager
def get_thread_local_session(engine=None, db_path: str = None):
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
    from sqlalchemy.orm import sessionmaker, Session as ORMSession
    from sqlalchemy.pool import QueuePool
    import os

    # Check if thread already has a session
    if hasattr(_thread_local, 'session') and _thread_local.session is not None:
        # Verify session is still valid
        try:
            # Quick check if session is usable
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


def cleanup_thread_local_session():
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
```

#### Step 1.2: Modify FileClaimsHandler to Use Thread-Local Sessions

**BEFORE (Current Code - Lines 98-114 in file_claims.py):**

```python
def __init__(
    self,
    session: Session,
    default_ttl_seconds: int = 3600,
    project_root: Optional[str] = None,
):
    """
    Initialize the file claims handler

    Args:
        session: SQLAlchemy database session
        default_ttl_seconds: Default TTL for claims in seconds
        project_root: Optional project root path for path validation (Issue #22)
    """
    self.session = session
    self.default_ttl_seconds = default_ttl_seconds
    self.project_root = project_root
```

**AFTER (Fixed Code):**

```python
def __init__(
    self,
    session: Optional[Session] = None,
    engine=None,
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
    if self._legacy_session is not None:
        return self._legacy_session

    # Get or create thread-local session
    if hasattr(_thread_local, 'session') and _thread_local.session is not None:
        return _thread_local.session

    # Create new thread-local session
    from maestro.memory.database.models import get_session
    if self._engine is not None:
        _thread_local.session = get_session(engine=self._engine)
    else:
        _thread_local.session = get_session(db_path=self._db_path)

    return _thread_local.session


def cleanup(self):
    """
    Clean up the current thread's session.

    Should be called when done with operations in a thread.
    """
    cleanup_thread_local_session()
```

#### Step 1.3: Update All Methods to Use Thread-Local Sessions

Add context manager usage to all methods that perform database operations:

**Template for updates (apply to all methods):**

```python
def some_method(self, ...):
    """
    Method docstring...
    """
    # Ensure thread-local session
    session = self.session

    try:
        # Method logic here
        # Use explicit transactions
        with session.begin():
            # Database operations
            pass

        return result
    except Exception as e:
        session.rollback()
        raise
```

---

### Phase 2: Fix Transaction Management

**Problem:** Current code uses `flush()` without explicit transaction boundaries, leading to inconsistent state and lost updates.

**Implementation:**

#### Step 2.1: Replace flush() with Explicit Transactions

**BEFORE (Lines 349-350 in file_claims.py):**

```python
self.session.add(claim)
self.session.flush()
```

**AFTER:**

```python
# Use explicit transaction
with self.session.begin():
    self.session.add(claim)
    # Transaction commits automatically on exit
```

#### Step 2.2: Update All Transaction Boundaries

**Locations requiring fixes:**

1. **create_claim** (Line 349-350)
2. **renew_claim** (Line 889-890)
3. **release_claim** (Line 965-966)
4. **release_agent_claims** (Line 1015-1016)
5. **revoke_claim** (Line 1042)
6. **cleanup_expired_claims** (Line 1065)
7. **transfer_claim** (Line 1170)

**Example Pattern for create_claim:**

```python
def create_claim(self, ...) -> FileClaim:
    """
    Create a new file claim with pattern validation and path traversal protection.

    Uses explicit transaction boundaries for thread-safe concurrent operations.
    """
    # Validation code (unchanged)...

    # Conflict checking with row-level lock
    conflicts = self._check_conflicts_locked(
        agent_id=agent_id,
        file_patterns=normalized_patterns,
        is_exclusive=is_exclusive,
    )

    if conflicts:
        # Log and raise (unchanged)...

    # Create claim with explicit transaction
    try:
        with self.session.begin():
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

            self.session.add(claim)

            # Log audit event (within transaction)
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

            # Transaction commits here
            return claim

    except Exception as e:
        self.session.rollback()
        logger.error(f"Failed to create claim: {e}")
        raise
```

---

### Phase 3: Fix Pattern Compilation Bug

**Problem:** The `_compile_pattern` method doesn't correctly handle `**` (recursive wildcard) patterns. Line 144 overwrites the escaped `*` with `.*`, losing the distinction between `*` and `**`.

**BEFORE (Lines 117-153 in file_claims.py):**

```python
@classmethod
def _compile_pattern(cls, pattern: str) -> re.Pattern:
    """
    Compile a glob pattern to regex with caching.
    """
    # Check cache first
    with cls._pattern_cache_lock:
        if pattern in cls._pattern_cache:
            return cls._pattern_cache[pattern]

        # Convert glob pattern to regex
        # Escape special regex characters except glob wildcards
        regex_pattern = re.escape(pattern)

        # Convert escaped wildcards back to regex
        regex_pattern = regex_pattern.replace(r'\*', '.*')
        regex_pattern = regex_pattern.replace(r'\?', '.')

        # Handle ** (recursive directory matching)
        regex_pattern = regex_pattern.replace(r'\.\*', '.*')  # BUG: This line

        # Anchor the pattern
        regex_pattern = f'^{regex_pattern}$'

        # Compile and cache
        compiled = re.compile(regex_pattern)
        cls._pattern_cache[pattern] = compiled

        return compiled
```

**AFTER:**

```python
@classmethod
def _compile_pattern(cls, pattern: str) -> re.Pattern:
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
        Compiled regex pattern
    """
    # Check cache first (thread-safe)
    with cls._pattern_cache_lock:
        if pattern in cls._pattern_cache:
            return cls._pattern_cache[pattern]

        # Don't cache patterns with ** - use _match_recursive for those
        if '**' in pattern:
            # Return a placeholder pattern that will trigger _match_recursive
            # We cache a marker to avoid recompilation
            cls._pattern_cache[pattern] = None
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
```

#### Step 3.2: Update _matches_any_pattern to Handle **

**BEFORE (Lines 730-769):**

```python
def _matches_any_pattern(
    self,
    file_path: str,
    patterns: List[str],
) -> bool:
    """Check if a file path matches any of the given patterns."""
    normalized_path = self._normalize_pattern(file_path)

    for pattern in patterns:
        if not pattern or not pattern.strip():
            continue
        normalized_pattern = self._normalize_pattern(pattern)
        try:
            # Handle ** (recursive directory matching)
            if '**' in normalized_pattern:
                if self._match_recursive(normalized_path, normalized_pattern):
                    return True
            else:
                # Use cached pattern compilation for better performance
                compiled = self._compile_pattern(normalized_pattern)
                if compiled.match(normalized_path):
                    return True
        except (re.error, ValueError):
            # Skip invalid patterns
            continue
    return False
```

**AFTER:**

```python
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
```

---

### Phase 4: Fix Version Tracking (Atomic Operations)

**Problem:** Version check and increment happen separately, allowing race conditions where two threads can both pass the check before either increments.

**Solution:** Use database-level atomic operations with UPDATE ... WHERE version = ?

#### Step 4.1: Add Atomic Version Update Method to FileClaim Model

**Add to models.py after line 847:**

```python
def increment_version_atomic(self, session: Session, expected_version: int) -> bool:
    """
    Increment version using atomic database operation.

    This prevents race conditions by checking and updating the version
    in a single database operation.

    Args:
        session: SQLAlchemy session
        expected_version: Expected current version

    Returns:
        True if increment succeeded, False if version mismatch

    Raises:
        ConcurrentModificationError: If version mismatch detected
    """
    from sqlalchemy import update
    from maestro.memory.database.models import log_audit_event

    # Atomic update with version check
    stmt = (
        update(FileClaim)
        .where(FileClaim.id == self.id)
        .where(FileClaim.version == expected_version)
        .values(version=FileClaim.version + 1)
    )

    result = session.execute(stmt)

    if result.rowcount == 0:
        # Version mismatch - concurrent modification detected
        current_version = session.execute(
            select(FileClaim.version).where(FileClaim.id == self.id)
        ).scalar_one()

        log_audit_event(
            operation="version_conflict",
            entity_type="FileClaim",
            entity_id=self.claim_id,
            user_id=None,
            changes={
                "expected_version": expected_version,
                "actual_version": current_version
            },
            status="conflict"
        )

        raise ConcurrentModificationError(
            f"Concurrent modification detected on claim {self.claim_id}",
            claim_id=self.claim_id,
            expected_version=expected_version,
            actual_version=current_version,
        )

    # Refresh to get updated version
    session.refresh(self)
    return True
```

#### Step 4.2: Update renew_claim to Use Atomic Version Update

**BEFORE (Lines 831-906):**

```python
def renew_claim(
    self,
    claim_id: str,
    ttl_seconds: Optional[int] = None,
    expected_version: Optional[int] = None,
) -> Optional[FileClaim]:
    """Renew a claim's expiration time with optimistic concurrency control."""
    # Issue #15: Get claim with row-level lock
    stmt = select(FileClaim).where(
        FileClaim.claim_id == claim_id
    ).with_for_update()

    claim = self.session.execute(stmt).scalar_one_or_none()
    if not claim:
        return None

    if not claim.is_valid():
        raise ClaimExpiredError(f"Claim {claim_id} has expired")

    # Check version for optimistic concurrency control
    if expected_version is not None and claim.version != expected_version:
        # Log and raise error (non-atomic)...

    old_expires = claim.expires_at
    claim.expires_at = datetime.now(UTC) + timedelta(
        seconds=ttl_seconds or self.default_ttl_seconds
    )
    claim.increment_version()  # BUG: Not atomic
    self.session.flush()

    # Log audit event...

    return claim
```

**AFTER:**

```python
def renew_claim(
    self,
    claim_id: str,
    ttl_seconds: Optional[int] = None,
    expected_version: Optional[int] = None,
) -> Optional[FileClaim]:
    """
    Renew a claim's expiration time with optimistic concurrency control.

    Issue #15: Uses atomic version tracking to prevent concurrent modifications.

    Args:
        claim_id: Claim ID to renew
        ttl_seconds: New TTL in seconds (uses default if None)
        expected_version: Expected version for optimistic locking

    Returns:
        Updated claim or None if not found

    Raises:
        ConcurrentModificationError: If version mismatch detected
    """
    # Get claim with row-level lock
    stmt = select(FileClaim).where(
        FileClaim.claim_id == claim_id
    ).with_for_update()

    claim = self.session.execute(stmt).scalar_one_or_none()
    if not claim:
        return None

    if not claim.is_valid():
        raise ClaimExpiredError(f"Claim {claim_id} has expired")

    # Use atomic version update
    current_version = claim.version
    if expected_version is not None and current_version != expected_version:
        # Version mismatch detected
        log_audit_event(
            operation="renew_claim_conflict",
            entity_type="FileClaim",
            entity_id=claim_id,
            user_id=None,
            changes={
                "expected_version": expected_version,
                "actual_version": current_version
            },
            status="conflict"
        )
        raise ConcurrentModificationError(
            f"Concurrent modification detected on claim {claim_id}",
            claim_id=claim_id,
            expected_version=expected_version,
            actual_version=current_version,
        )

    # Perform updates in explicit transaction
    try:
        with self.session.begin():
            old_expires = claim.expires_at
            claim.expires_at = datetime.now(UTC) + timedelta(
                seconds=ttl_seconds or self.default_ttl_seconds
            )

            # Atomic version increment
            claim.increment_version_atomic(
                self.session,
                current_version
            )

            # Log audit event within transaction
            log_audit_event(
                operation="renew_claim",
                entity_type="FileClaim",
                entity_id=claim_id,
                user_id=claim.agent_id,
                changes={
                    "old_expires_at": old_expires.isoformat() if old_expires else None,
                    "new_expires_at": claim.expires_at.isoformat(),
                    "version": claim.version,
                },
                status="success"
            )

            return claim

    except ConcurrentModificationError:
        raise
    except Exception as e:
        logger.error(f"Failed to renew claim {claim_id}: {e}")
        raise
```

#### Step 4.3: Update release_claim Similarly

Apply the same atomic version update pattern to `release_claim` method.

---

## Database Schema Changes

### Add Triggers for Atomic Version Updates (Optional Enhancement)

For maximum safety, we can add database-level triggers to enforce version checking:

```sql
-- Add to database initialization (in create_tables function)

-- Trigger for atomic version updates on file_claims
CREATE TRIGGER IF NOT EXISTS file_claims_version_check
BEFORE UPDATE OF version ON file_claims
WHEN NEW.version <= OLD.version
BEGIN
    SELECT RAISE(ABORT, 'Version must be greater than current version');
END;
```

**Implementation in models.py (after line 1616):**

```python
# Add to create_tables function after WAL mode setup

# Create triggers for version enforcement
try:
    with engine.begin() as conn:
        # Trigger to prevent version downgrades
        conn.execute(text("""
            CREATE TRIGGER IF NOT EXISTS file_claims_version_check
            BEFORE UPDATE OF version ON file_claims
            WHEN NEW.version <= OLD.version
            BEGIN
                SELECT RAISE(ABORT, 'Version must be greater than current version');
            END
        """))
except Exception as e:
    logger.warning(f"Failed to create version check trigger: {e}")
```

---

## Test Fixture Changes

### Update Test Fixtures to Use Thread-Local Sessions

**BEFORE (Lines 76-79 in test_concurrency.py):**

```python
@pytest.fixture
def file_claims_handler(db_session, temp_db_path):
    """Create a FileClaimsHandler instance"""
    return FileClaimsHandler(db_session, project_root=str(temp_db_path.parent))
```

**AFTER:**

```python
@pytest.fixture
def file_claims_handler(temp_db_path):
    """
    Create a FileClaimsHandler instance with thread-safe configuration.

    Uses db_path instead of session to enable thread-local session creation.
    """
    from sqlalchemy import create_engine

    # Create engine for thread-local sessions
    engine = create_engine(
        f"sqlite:///{temp_db_path}",
        poolclass=QueuePool,
        pool_size=5,
        max_overflow=10,
        pool_timeout=30,
        connect_args={"check_same_thread": False},
    )

    # Enable WAL mode for better concurrency
    from sqlalchemy import text
    with engine.begin() as conn:
        conn.execute(text("PRAGMA journal_mode=WAL"))
        conn.execute(text("PRAGMA busy_timeout=5000"))  # 5 second timeout

    # Return handler with engine (thread-safe)
    return FileClaimsHandler(engine=engine, project_root=str(temp_db_path.parent))


@pytest.fixture
def db_session(temp_db_path):
    """
    Create a legacy database session for backward compatibility tests.

    Note: This fixture should NOT be used in concurrent tests.
    """
    engine = create_engine(f"sqlite:///{temp_db_path}")
    SessionLocal = sessionmaker(bind=engine)
    session = SessionLocal()
    yield session
    session.close()
```

### Add Cleanup Fixture for Thread-Local Sessions

```python
@pytest.fixture(autouse=True)
def cleanup_thread_local_sessions():
    """
    Automatically clean up thread-local sessions after each test.

    Prevents session leaks across tests.
    """
    yield

    # Cleanup after test
    try:
        from maestro.memory.database.models import cleanup_thread_local_session
        cleanup_thread_local_session()
    except Exception:
        pass
```

---

## Validation Strategy

### Step-by-Step Validation Plan

#### 1. Unit-Level Validation

**Test 1: Thread-Local Session Creation**

```python
def test_thread_local_sessions_are_distinct():
    """Verify each thread gets its own session"""
    from maestro.memory.database.models import get_thread_local_session
    import threading

    sessions = []
    def get_session():
        with get_thread_local_session(db_path=":memory:") as session:
            sessions.append(id(session))

    threads = [threading.Thread(target=get_session) for _ in range(10)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    # All session IDs should be unique
    assert len(set(sessions)) == 10
```

**Test 2: Pattern Compilation Correctness**

```python
def test_pattern_compilation_handles_recursive_wildcard():
    """Verify ** patterns are not compiled to regex"""
    handler = FileClaimsHandler(engine=create_engine("sqlite:///:memory:"))

    # ** patterns should return None (use _match_recursive)
    assert handler._compile_pattern("src/**/*.py") is None
    assert handler._compile_pattern("**/*.py") is None

    # Simple patterns should compile
    assert handler._compile_pattern("*.py") is not None
    assert handler._compile_pattern("src/*.py") is not None
```

**Test 3: Atomic Version Updates**

```python
def test_concurrent_version_updates_detect_conflicts():
    """Verify atomic version updates prevent conflicts"""
    from maestro.memory.database.models import create_tables, get_session

    engine = create_tables(db_path=":memory:")
    session = get_session(engine=engine)

    # Create a claim
    claim = FileClaim(
        claim_id="test-claim",
        agent_id="agent-1",
        file_patterns=["*.py"],
        status=ClaimStatus.ACTIVE.value,
        expires_at=datetime.now(UTC) + timedelta(hours=1),
    )
    session.add(claim)
    session.commit()

    # Try to update with wrong version
    with pytest.raises(ConcurrentModificationError):
        claim.increment_version_atomic(session, expected_version=999)
```

#### 2. Integration-Level Validation

**Test 4: Concurrent Non-Overlapping Claims**

```python
def test_concurrent_non_overlapping_claims_passes():
    """Should pass after fixes"""
    # Uses existing test - should now pass
    assert test_concurrent_non_overlapping_claims() == True
```

**Test 5: Concurrent Overlapping Claims**

```python
def test_concurrent_overlapping_clauses_with_exclusive_passes():
    """Should pass after fixes"""
    # Uses existing test - should now pass
    assert test_concurrent_overlapping_clauses_with_exclusive() == True
```

#### 3. Stress-Level Validation

**Test 6: High Concurrency Stress Test**

```python
def test_high_concurrency_stress():
    """Stress test with 100 concurrent operations"""
    from concurrent.futures import ThreadPoolExecutor

    handler = FileClaimsHandler(engine=create_engine("sqlite:///:memory:"))

    def create_and_release(agent_id):
        try:
            claim = handler.create_claim(
                agent_id=agent_id,
                file_patterns=[f"{agent_id}/*.py"],
                ttl_seconds=60,
            )
            handler.release_claim(claim.claim_id, expected_version=claim.version)
            return True
        except Exception as e:
            return False

    with ThreadPoolExecutor(max_workers=50) as executor:
        futures = [executor.submit(create_and_release, f"agent-{i}") for i in range(100)]
        results = [f.result() for f in futures]

    # All operations should succeed
    assert all(results)
```

#### 4. Regression-Level Validation

**Run all existing tests:**

```bash
# Run only concurrency tests
pytest /home/stan/Prod/maestro/maestro/memory/tests/test_concurrency.py -v

# Run all memory tests
pytest /home/stan/Prod/maestro/maestro/memory/tests/ -v

# Run full test suite
pytest /home/stan/Prod/maestro/ -v
```

---

## Risk Assessment

### High-Risk Changes

| Change | Risk | Mitigation |
|--------|------|------------|
| Thread-local session property | HIGH | Extensive testing, backward compatibility support |
| Transaction boundaries | MEDIUM | Careful review of all flush() calls, add rollback handling |
| Pattern compilation | LOW | Only affects ** patterns, well-tested pattern matching exists |
| Atomic version updates | MEDIUM | Database-level validation, comprehensive concurrent tests |

### Medium-Risk Changes

| Change | Risk | Mitigation |
|--------|------|------------|
| Test fixture updates | MEDIUM | Provide both old and new fixtures, deprecate gradually |
| Session lifecycle management | MEDIUM | Auto-cleanup fixtures, explicit documentation |

### Low-Risk Changes

| Change | Risk | Mitigation |
|--------|------|------------|
| Cache invalidation | LOW | Existing cache infrastructure is robust |
| Audit logging | LOW | No changes to audit events, just transaction placement |

---

## Rollback Plan

### If Critical Issues Arise

#### Option 1: Feature Flag (Recommended)

Add a feature flag to enable/disable thread-local sessions:

```python
# Add to file_claims.py

USE_THREAD_LOCAL_SESSIONS = os.getenv("MAESTRO_THREAD_LOCAL_SESSIONS", "true").lower() == "true"

@property
def session(self) -> Session:
    """Get session based on feature flag"""
    if USE_THREAD_LOCAL_SESSIONS:
        # Thread-local implementation
        ...
    else:
        # Legacy implementation
        return self._legacy_session
```

#### Option 2: Branch Rollback

1. Revert to commit before changes
2. Apply only pattern compilation fix (low risk)
3. Keep existing tests as-is

#### Option 3: Gradual Migration

1. Introduce new `ThreadSafeFileClaimsHandler` class
2. Keep old `FileClaimsHandler` unchanged
3. Gradually migrate tests to new class
4. Deprecate old class after validation

---

## Implementation Checklist

### Phase 1: Session Management (CRITICAL)
- [ ] Add `_thread_local` to models.py
- [ ] Implement `get_thread_local_session()` context manager
- [ ] Implement `cleanup_thread_local_session()` function
- [ ] Modify `FileClaimsHandler.__init__()` to accept engine/db_path
- [ ] Add `session` property to `FileClaimsHandler`
- [ ] Update all methods to use thread-local sessions
- [ ] Add deprecation warning for session-based initialization

### Phase 2: Transaction Management
- [ ] Replace all `flush()` calls with explicit transactions
- [ ] Add rollback handling in all methods
- [ ] Update audit logging to occur within transactions
- [ ] Add transaction lifecycle documentation

### Phase 3: Pattern Compilation
- [ ] Fix `_compile_pattern()` to return None for ** patterns
- [ ] Update `_matches_any_pattern()` to handle None from _compile_pattern
- [ ] Add unit tests for pattern compilation edge cases
- [ ] Verify ** pattern matching works correctly

### Phase 4: Atomic Version Tracking
- [ ] Add `increment_version_atomic()` to FileClaim model
- [ ] Update `renew_claim()` to use atomic version update
- [ ] Update `release_claim()` to use atomic version update
- [ ] Add database trigger for version enforcement (optional)
- [ ] Add unit tests for atomic version operations

### Phase 5: Test Updates
- [ ] Update `file_claims_handler` fixture to use engine
- [ ] Add `cleanup_thread_local_sessions` fixture
- [ ] Add thread-local session unit tests
- [ ] Add pattern compilation unit tests
- [ ] Add atomic version update unit tests
- [ ] Run all 7 concurrency tests and verify they pass
- [ ] Run full test suite to ensure no regressions

### Phase 6: Documentation
- [ ] Update API documentation for FileClaimsHandler
- [ ] Add concurrency best practices guide
- [ ] Document thread-safety guarantees
- [ ] Add migration guide from old API to new API

---

## Success Criteria

### Must Pass (All Required)

1. ✅ Test 1: `test_concurrent_non_overlapping_claims` - PASS
2. ✅ Test 2: `test_concurrent_overlapping_clauses_with_exclusive` - PASS
3. ✅ Test 3: `test_row_level_locking_prevents_race_condition` - PASS
4. ✅ Test 4: `test_concurrent_renew_detects_version_mismatch` - PASS
5. ✅ Test 5: `test_concurrent_release_detects_version_mismatch` - PASS
6. ✅ Test 6: `test_concurrent_pattern_compilation` - PASS
7. ✅ Test 7: `test_high_concurrency_file_claims` - PASS

### Additional Success Metrics

- ✅ Zero SQLAlchemy thread-safety warnings
- ✅ Zero session leaked warnings
- ✅ All existing tests still pass (no regressions)
- ✅ Performance within 10% of baseline
- ✅ Code coverage maintained or improved

---

## Timeline Estimate

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 1 | Session management fixes | 4 hours |
| Phase 2 | Transaction management | 2 hours |
| Phase 3 | Pattern compilation fix | 1 hour |
| Phase 4 | Atomic version tracking | 3 hours |
| Phase 5 | Test updates | 3 hours |
| Phase 6 | Documentation | 2 hours |
| **Total** | | **15 hours** |

---

## References

### Files to Modify

1. `/home/stan/Prod/maestro/maestro/memory/coordination/file_claims.py` (1,275 lines)
   - Lines 98-114: `__init__` method
   - Lines 117-153: `_compile_pattern` method
   - Lines 239-366: `create_claim` method
   - Lines 831-906: `renew_claim` method
   - Lines 908-982: `release_claim` method
   - All methods with `self.session.flush()`

2. `/home/stan/Prod/maestro/maestro/memory/database/models.py` (1,701 lines)
   - After line 1700: Add thread-local session functions
   - After line 847: Add `increment_version_atomic()` method
   - After line 1616: Add database triggers

3. `/home/stan/Prod/maestro/maestro/memory/tests/test_concurrency.py` (666 lines)
   - Lines 76-79: `file_claims_handler` fixture
   - Add new fixtures for thread-local sessions

### Key Concepts

- **Thread-local storage**: Using `threading.local()` to maintain per-thread state
- **SQLAlchemy session scope**: Sessions must not be shared across threads
- **Explicit transactions**: Using `with session.begin()` for atomic operations
- **Optimistic concurrency control**: Using version numbers to detect conflicts
- **Row-level locking**: Using `SELECT FOR UPDATE` to prevent concurrent modifications

### Related Issues

- Issue #15: Row-level locking and version tracking
- Issue #22: Path traversal protection (unrelated to concurrency)
- Issue #26: Query result caching (may need updates for thread-safety)

---

## Appendix: Complete Code Examples

### Example 1: Complete Thread-Local Session Implementation

```python
# In models.py

from threading import local
from contextlib import contextmanager
from sqlalchemy import select, create_engine
from sqlalchemy.orm import sessionmaker, Session as ORMSession
from sqlalchemy.pool import QueuePool
import os

_thread_local = local()


@contextmanager
def get_thread_local_session(engine=None, db_path: str = None):
    """
    Get a thread-local database session with automatic cleanup.

    This is the core fix for SQLAlchemy thread-safety violations.
    Each thread gets its own session instance.

    Usage:
        with get_thread_local_session() as session:
            result = session.execute(query)
            return result

    Args:
        engine: SQLAlchemy engine (optional)
        db_path: Path to database file (used if engine not provided)

    Yields:
        Thread-local SQLAlchemy session
    """
    # Check if thread already has a session
    if hasattr(_thread_local, 'session') and _thread_local.session is not None:
        try:
            # Quick validation
            _thread_local.session.execute(select(func.count()))
            yield _thread_local.session
            return
        except Exception:
            # Session is stale
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
        # Clean up
        try:
            _thread_local.session.close()
        except Exception:
            pass
        _thread_local.session = None


def cleanup_thread_local_session():
    """Clean up the current thread's session."""
    if hasattr(_thread_local, 'session') and _thread_local.session is not None:
        try:
            _thread_local.session.close()
        except Exception:
            pass
        _thread_local.session = None
```

### Example 2: Complete Fixed create_claim Method

```python
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

    Thread-safe implementation using explicit transactions and thread-local sessions.

    Args:
        agent_id: ID of the agent making the claim
        file_patterns: List of glob patterns for files to claim
        session_id: Optional session ID
        is_exclusive: Whether this is an exclusive claim
        reason: Reason for the claim
        task_description: Description of the task
        ttl_seconds: Time-to-live in seconds
        project_id: Optional Maestro project ID
        track_id: Optional Maestro track ID
        claim_id: Optional custom claim ID

    Returns:
        Created FileClaim instance

    Raises:
        ClaimConflictError: If claim conflicts with existing claims
        PathTraversalError: If pattern attempts path traversal
        ValueError: If file_patterns is empty
    """
    # Validate patterns
    if not file_patterns:
        raise ValueError("file_patterns cannot be empty")

    normalized_patterns = []
    for pattern in file_patterns:
        try:
            normalized = self._validate_pattern_no_traversal(pattern)
        except PathTraversalError:
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

        normalized = self._normalize_pattern(normalized)
        if normalized:
            normalized_patterns.append(normalized)

    if not normalized_patterns:
        raise ValueError("file_patterns must contain at least one valid pattern")

    # Check for conflicts with row-level lock
    conflicts = self._check_conflicts_locked(
        agent_id=agent_id,
        file_patterns=normalized_patterns,
        is_exclusive=is_exclusive,
    )

    if conflicts:
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

    # Create claim with explicit transaction
    session = self.session
    try:
        with session.begin():
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

    except Exception as e:
        logger.error(f"Failed to create claim for agent {agent_id}: {e}")
        raise
```

---

## End of Implementation Plan

This plan provides a complete, step-by-step guide to fixing all concurrency issues in the Maestro v2 codebase. Follow the phases in order, validate at each step, and ensure all 7 concurrency tests pass before considering the implementation complete.

For questions or clarifications, refer to the inline code comments and docstrings, which provide additional context and usage examples.
