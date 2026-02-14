# Maestro Memory System - Enhancements TODO

**Created:** 2026-02-13
**Track:** conductor-ralph-parity_20260213

---

## Missing Functionality Identified During Track Creation

### 1. CLI Memory Store Command

**Issue:** No CLI command to directly store memories from command line or scripts.

**Current State:**
- `maestro memory serve` - Web dashboard server
- `maestro memory status` - Show statistics
- No direct `store` or `add` command

**Desired CLI Surface:**
```bash
# Store a memory directly
maestro memory store \
  --category context \
  --content "Track conductor-ralph-parity_20260213 created with 7 subtracks" \
  --importance high \
  --tags "track,conductor,ralph-tui"

# Quick store (interactive mode)
maestro memory note
# Prompts for category, content, importance
```

### 2. Python API Module for Direct Access

**Issue:** No documented/simple Python API for external scripts to store memories.

**Current State:**
- Models exist in `maestro.memory.database.models`
- Must manually import and instantiate sessions
- No helper module like `maestro.memory.api` for simple operations

**Desired API:**
```python
from maestro.memory.api import store_memory, query_memories

# Simple store
store_memory(
    content="Track created with 7 subtracks",
    category="context",
    importance="high",
    track_id="conductor-ralph-parity_20260213"
)

# Simple query
memories = query_memories(
    category="context",
    track_id="conductor-ralph-parity_20260213",
    limit=10
)
```

### 3. Auto-Retrieval Enhancement

**Issue:** Memory system stores but doesn't automatically inject relevant memories into context.

**Desired Behavior:**
- When a track/session starts, automatically retrieve relevant memories
- Relevance based on: project path, track_id, tags, timestamp (recent)
- Configurable retrieval limits (token count, memory count)

**Proposed Implementation:**
```python
# maestro/memory/retrieval.py
class AutoRetrieval:
    def retrieve_for_context(self, project_path: str, track_id: str = None) -> List[Memory]:
        # Query by project, track, recent context memories
        # Sort by relevance (importance, recency, category)
        # Return top N memories within token budget
```

---

## Priority Order

| # | Feature | Priority | Dependency |
|---|---------|----------|-------------|
| 1 | Python API Module | High | None |
| 2 | CLI Store Command | Medium | Python API |
| 3 | Auto-Retrieval | High | Python API + Relevance scoring |

---

## Notes

- SQLAlchemy was missing during initial track creation - installed via `pip install sqlalchemy`
- Memory was successfully stored using direct model access after SQLAlchemy installation
- This TODO tracks needed enhancements for better user/developer experience
