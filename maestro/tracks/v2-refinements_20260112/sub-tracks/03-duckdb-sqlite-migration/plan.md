# Sub-Track 03: DuckDB+SQLite PostgreSQL Replacement - Plan

## Overview

Implementation plan for database architecture migration.

---

## Phase 1: UnifiedStorageBackend

### [x] Task 3.1: Create UnifiedStorageBackend
- [x] Write tests for backend initialization
- [x] Write tests for component lifecycle
- [x] Create `maestro/memory/database/backends.py`
- [x] Implement SQLite WAL mode configuration
- [x] Implement DuckDB SQLite scanner attachment
- [x] Add clean shutdown handling

---

## Phase 2: OLTP Layer

### [x] Task 3.2: Implement OLTP Layer (SQLite)
- [x] Write tests for session CRUD operations
- [x] Write tests for claims management
- [x] Write tests for handoffs persistence
- [x] Write tests for memories storage
- [x] Write tests for concurrent read access
- [x] Implement SQLite-based storage for all OLTP operations
- [x] Verify WAL mode enables concurrent reads

---

## Phase 3: OLAP Layer

### [x] Task 3.3: Implement OLAP Layer (DuckDB)
- [x] Write tests for DuckDB SQLite scanner attachment
- [x] Write tests for analytics queries
- [x] Write tests for token statistics aggregation
- [x] Implement DuckDB query methods
- [x] Create dashboard data endpoints
- [x] Verify no data duplication (reads SQLite directly)

---

## Phase 4: Coordination

### [x] Task 3.4: Implement File-Based Coordination
- [x] Write tests for advisory file locking
- [x] Write tests for lock timeout handling
- [x] Write tests for stale lock cleanup
- [x] Create `maestro/memory/coordination/file_locks.py`
- [x] Implement `active_sessions.json` registry
- [x] Implement cross-terminal lock acquisition/release

---

## Phase 5: PostgreSQL Removal

### [x] Task 3.5: Remove PostgreSQL Dependencies
- [x] Identify all PostgreSQL references in codebase
- [x] Write migration tests for existing data
- [x] Remove Docker database setup from installer
- [x] Remove pgserver/embedded postgres options
- [x] Update documentation to reflect new architecture

---

## Phase 6: Directory Structure

### [x] Task 3.6: Create ~/.maestro Directory Structure
- [x] Write tests for directory initialization
- [x] Write tests for first-run detection
- [x] Implement first-run directory creation
- [x] Create migration for existing installations
- [x] Handle permission errors gracefully

---

## Phase 7: Verification

### [x] Task 3.7: Maestro - User Manual Verification 'Sub-Track 03' (Protocol in workflow.md)
