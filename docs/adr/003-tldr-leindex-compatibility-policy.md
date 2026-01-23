# ADR 003: TLDR to LeIndex Migration and Compatibility Policy

## Status

**Accepted** - 2026-01-22

## Context

The Maestro project previously had two overlapping code analysis systems:
1. **TLDR (Too Long; Didn't Read)** - Python-based 5-layer code analysis (`maestro.tldr`)
2. **LeIndex** - Pure Rust implementation with enhanced features

Having two systems created confusion and maintenance burden. The Rust-based LeIndex provides:
- Better performance (native speed vs Python)
- Multi-language support (8 languages via tree-sitter)
- Enhanced features (full-text search, semantic embeddings, MCP server)
- Single implementation language (Rust-first architecture)

## Decision

### TLDR as Compatibility Alias

The `/maestro:tldr` command is retained as a **compatibility alias** to the LeIndex Rust implementation.

**Policy:**
- `/maestro:tldr` commands are implemented by calling LeIndex Rust core
- No Python `maestro.tldr` imports in runtime code (enforced by CI gate)
- Documentation clearly states TLDR is LeIndex-backed
- All new development uses LeIndex APIs directly

### Implementation Mapping

| Legacy TLDR Concept | LeIndex Equivalent |
|---------------------|-------------------|
| `from maestro.tldr import ...` | **FORBIDDEN** - use `leindex_core` crate |
| `/maestro:tldr ast <file>` | `maestro analyze <file> --analysis ast` |
| `/maestro:tldr callgraph <file>` | `maestro analyze <file> --analysis callgraph` |
| `/maestro:tldr callers <func>` | `maestro analyze <file> --analysis callgraph --filter` |
| `/maestro:tldr cfg <file>` | `maestro analyze <file> --analysis cfg` |
| `/maestro:tldr dfg <file>` | `maestro analyze <file> --analysis dfg` |
| `/maestro:tldr slice <file> <line>` | `maestro analyze <file> --analysis slicing` |
| `tldr warm .` | `maestro leindex init .` |
| `tldr context <file>` | `maestro leindex analyze <file>` |
| `tldr search "<query>"` | `maestro leindex search "<query>"` |

### File Locations

- **Canonical Implementation**: `maestro/leindex/rust/src/` (LeIndex core)
- **Legacy Reference**: `maestro/archive/tldr/` (historical only, no runtime use)
- **CLI Entry Point**: `crates/cli/src/main.rs` (routes to LeIndex)

### Command Naming

- **Preferred**: `/maestro:leindex` - Direct LeIndex access
- **Compatibility**: `/maestro:tldr` - Alias for backward compatibility
- **CLI**: `maestro analyze`, `maestro leindex` - Direct CLI usage

## Consequences

### Positive

- Single implementation reduces maintenance burden
- Better performance with Rust-native code
- Clear migration path for users
- CI gates prevent accidental TLDR imports
- Backward compatibility maintained via alias

### Negative

- Users need to learn new command names (mitigated by compatibility alias)
- Documentation needs to clearly distinguish between legacy and canonical

### Migration Path

For code using old TLDR imports:

```python
# OLD (forbidden)
from maestro.tldr import ContextExtractor

# NEW (use CLI or skill)
/maestro:leindex analyze <file>
# or
maestro analyze <file> --analysis all
```

For skills and documentation:

1. Update examples to use `/maestro:leindex` primary
2. Keep `/maestro:tldr` as secondary/compatibility option
3. Note that TLDR is LeIndex-backed

## Enforcement

### CI Gates

The following checks run in CI:

1. **No maestro.tldr imports outside archive/**
   ```bash
   rg -n "maestro\.tldr" --glob '!maestro/archive/**' \
       --glob '!*.txt' --glob '!**/tracks.md' \
       --glob '!**/plan.md' --glob '!Makefile' \
       --glob '!**/SKILL.md' --glob '!**/spec.md' .
   ```

2. **No archive/tldr execution paths**
   ```bash
   rg -n "from.*archive.*tldr|import.*archive.*tldr" \
       --glob '!*.txt' --glob '!*.md' \
       --glob '!maestro/archive/**' maestro/
   ```

### Local Check

```bash
make policy-check
```

## References

- ADR 001: CLI Ownership and Binary Naming
- ADR 002: Crate Reorganization
- LeIndex Documentation: `maestro/leindex/docs/`
- TLDR Archive: `maestro/archive/tldr/`
