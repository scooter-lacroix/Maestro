# Sub-Track 03: DuckDB+SQLite PostgreSQL Replacement - Specification

## Overview

Eliminate Docker/PostgreSQL dependency with embedded database architecture using SQLite for OLTP and DuckDB for OLAP, with file-based coordination for cross-terminal support.

**Priority:** 3 (Infrastructure)
**Parent Track:** v2-refinements_20260112

## Functional Requirements

### FR-1: UnifiedStorageBackend
- Single backend class managing all storage components
- SQLite with WAL mode for transactional data
- DuckDB with SQLite scanner for analytics
- Clean initialization and teardown

### FR-2: OLTP Layer (SQLite)
- Session CRUD operations
- Claims management
- Handoffs persistence
- Memories storage
- WAL mode for concurrent reads

### FR-3: OLAP Layer (DuckDB)
- Analytics queries via SQLite scanner (no data duplication)
- Token statistics aggregation
- Dashboard data endpoints
- Fast columnar queries

### FR-4: File-Based Coordination
- Advisory file locking for cross-terminal support
- `active_sessions.json` registry
- Lock acquisition/release with timeout
- Graceful handling of stale locks

### FR-5: PostgreSQL Removal
- Remove all PostgreSQL references
- Remove Docker database setup from installer
- Remove pgserver/embedded postgres options
- Update all documentation

### FR-6: Directory Structure
- Create `~/.maestro/` on first run
- Subdirectories: memory.db, analytics.duckdb, vectors/, coordination/
- Migration path for existing installations

## New Files

- `maestro/memory/database/backends.py` - UnifiedStorageBackend
- `maestro/memory/coordination/file_locks.py` - Advisory locking

## Storage Architecture

```
~/.maestro/
├── memory.db           # SQLite with WAL (OLTP)
├── analytics.duckdb    # DuckDB (OLAP, reads SQLite)
├── vectors/            # LEANN vector store
├── coordination/
│   ├── active_sessions.json
│   └── locks/
└── .tantivy_index/     # Full-text search (Sub-Track 04)
```

## Acceptance Criteria

1. [ ] UnifiedStorageBackend initializes all components
2. [ ] SQLite OLTP operations work with WAL mode
3. [ ] DuckDB reads SQLite directly via scanner
4. [ ] File-based locking prevents conflicts
5. [ ] No PostgreSQL/Docker dependencies remain
6. [ ] Migration works for existing installations
7. [ ] All tests passing with >98% coverage
8. [ ] Tzar of Excellence review approved

## Out of Scope

- Vector store implementation (handled in Sub-Track 04)
- Full-text search (handled in Sub-Track 04)
- Remote/network database support
