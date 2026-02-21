# Maestro Nexus Memory Integration Guide

This guide explains how memory is integrated in Maestro, where the implementation lives, how to operate it from CLI, and how it relates to Memori migration/deprecation docs.

## 1. Architecture Overview

Maestro memory is implemented under `maestro/memory/` and centered on a local SQLite database (`~/.maestro/maestro.db` by default).

Key locations:

- `maestro/memory/service.py` — main integration service used by Maestro flows.
- `maestro/memory/database/models.py` — unified schema (projects, tracks, memories, namespaces, related entities).
- `maestro/memory/database/managers.py` — storage/retrieval/query managers.
- `maestro/memory/dashboard.py` — FastAPI dashboard API.
- `maestro/memory/cli.py` — Python module-level CLI entrypoints for memory operations.
- `maestro/memory/frontend/` — dashboard UI.

Important behavior:

- Memory is project/track-aware (records can be linked to Maestro projects and tracks).
- Agent namespaces are supported for isolation and scoped recall.
- Memory service includes Nexus path discovery and compatibility layers used by integrated workflows.

## 2. Command Usage (Current CLI Surface)

Use the memory commands currently implemented in the Rust `maestro` CLI:
```bash
# Start dashboard server
maestro memory serve
# Show memory system status
maestro memory status

# Scan directories for Maestro projects
maestro memory scan . --depth 3

# Store a memory directly
maestro memory store \
  --content "Implemented auth retry guard" \
  --category decision \
  --importance high
```

Command definitions are in `crates/cli/src/main.rs` (`MemoryCommands`).
## 3. Project Isolation and Namespaces

Isolation happens at multiple layers:

1. **Project/track linkage**
   - Memory records can include `project_id` and `track_id` associations.
   - This enables per-project/per-track filtering and avoids cross-project leakage during retrieval.

2. **Agent namespace isolation**
   - `agent_namespaces` and `namespace_memories` provide namespace scoping.
   - Namespaces support owner/reader/writer controls and agent-type tagging.

3. **Query-time filtering**
   - Retrieval/search paths support scoped queries (project, track, agent context), which is the operational boundary that keeps context relevant.

When adding new memory features, preserve this model: no global unscoped queries in automation paths unless explicitly intended.

## 4. Relationship to Memori Migration Docs

Memori is deprecated in favor of integrated Nexus memory. Use these docs together:

- `./memori_deprecation_notice.md` — policy/timeline and migration guidance.
- `./memori_nexus_feature_parity.md` — parity matrix and behavior comparison.
- `./memori_to_nexus_mapping.md` — field/category/type mapping details.

Operationally:

- Treat Nexus in Maestro as the source of truth for new development.
- Use migration tooling/workflows for legacy Memori data.
- Validate migrated data using search/stats/dashboard checks before removing legacy stores.

## 5. Troubleshooting and Validation

### Quick validation commands

```bash
# 1) Verify documented memory command group is available
maestro memory --help

# 2) Start dashboard and verify it serves
maestro memory serve
# open http://127.0.0.1:18765

# 3) Validate status path
maestro memory status

# 4) Validate scan path
maestro memory scan . --depth 2

# 5) Validate store path
maestro memory store \
  --content "Memory integration validation" \
  --category observation \
  --importance normal
```

### Common failure points

- **Database not found / empty results**
  - Confirm `~/.maestro/maestro.db` exists and commands are running in the expected user environment.

- **Dashboard unreachable**
  - Confirm `serve` is running and nothing else is using port `18765`.

- **Unexpected cross-project memories**
  - Check whether writes included project/track context and whether retrieval is using scoped filters.

- **Migration uncertainty**
  - Cross-check deprecation + parity + mapping docs above before changing schemas or migration mappings.
