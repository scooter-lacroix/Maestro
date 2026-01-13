---
description: Access LeIndex - Maestro's powerful code indexing and search system with full-text search, semantic embeddings, and 5-layer code analysis.
---

# Maestro LeIndex - Code Indexing & Search

Access **LeIndex** - Maestro's powerful code indexing and search system that combines full-text search (Tantivy BM25) with semantic embeddings for intelligent code discovery.

## Overview

LeIndex provides:
- **Fast full-text search** via Tantivy (BM25 ranking)
- **Semantic search** via vector embeddings
- **5-layer code analysis** (AST, Call Graph, CFG, DFG, Slicing)
- **File change tracking** and history
- **MCP server** for integration with Claude Code

## Usage

```bash
/maestro:leindex <command> [options]
```

## Commands

### Project Management

#### `init [path]`
Initialize and index a project.

**Example:**
```bash
/maestro:leindex init .
```

#### `status`
Show index statistics and status.

**Example:**
```bash
/maestro:leindex status
```

**Output includes:**
- Files indexed
- Total symbols extracted
- Index size
- Last update time

#### `reindex [path]`
Re-index the project (fresh build).

**Example:**
```bash
/maestro:leindex reindex src/
```

### Search

#### `search <query>` or `query <query>`
Search code with hybrid full-text + semantic search.

**Examples:**
```bash
# Basic search
/maestro:leindex search "authentication"

# Semantic search (find by behavior, not just keywords)
/maestro:leindex search "database connection pooling"

# Find function definitions
/maestro:leindex search "def process_payment"

# Search in specific file
/maestro:leindex search "user" --file src/models.py
```

#### `answer <question>`
RAG-style question answering over your codebase.

**Example:**
```bash
/maestro:leindex answer "How is authentication handled?"
```

### Code Analysis

#### `analyze <file> [layers]`
Run 5-layer analysis on a file.

**Available layers:** `ast`, `callgraph`, `cfg`, `dfg`, `slicing`

**Examples:**
```bash
# All layers
/maestro:leindex analyze src/auth.py

# Specific layers
/maestro:leindex analyze src/auth.py ast callgraph cfg

# Single layer
/maestro:leindex analyze src/utils.py slicing
```

#### `history <file>`
Show file change history and modifications.

**Example:**
```bash
/maestro:leindex history src/api/routes.py
```

### CLI Tools (Outside Claude Code)

LeIndex also provides standalone CLI tools:

```bash
# Search code
leindex-search "pattern"

# Search with AI answers
leindex-search --answer "how does auth work?"

# Batch search
leindex-search --batch queries.txt

# Index statistics
leindex stats
```

## MCP Server Integration

LeIndex runs as an MCP server for deep integration with Claude Code.

### Start MCP Server

```bash
leindex
```

### Available MCP Tools

When connected, Claude Code can use:

- `set_project_path(path)` - Set working project
- `index_project()` - Full project indexing
- `search_code(query, limit)` - Hybrid search
- `analyze_file(file_path, layers)` - 5-layer analysis
- `get_file_history(file_path)` - File history

## Integration with TLDR

LeIndex and TLDR work together:

| Feature | LeIndex | TLDR |
|---------|---------|------|
| **Search** | Full-text + semantic | N/A |
| **AST** | ✓ | ✓ |
| **Call Graph** | ✓ | ✓ |
| **CFG** | ✓ | ✓ |
| **DFG** | ✓ | ✓ |
| **Slicing** | ✓ | ✓ |
| **Context Extraction** | N/A | ✓ |
| **MCP Server** | ✓ | N/A |
| **Automatic Hooks** | N/A | ✓ |

**Use LeIndex when:** You need to search across your entire codebase
**Use TLDR when:** You need detailed analysis of specific files

## Examples

### Find a function by behavior

```bash
/maestro:leindex search "validate user token"
```

### Understand code complexity

```bash
/maestro:leindex analyze src/auth.py cfg
```

### See who uses a function

```bash
/maestro:leindex analyze src/services/payment.py callgraph
```

### Track file changes

```bash
/maestro:leindex history src/models.py
```

## Storage Backends

LeIndex uses multiple storage engines:

- **SQLite** - Metadata, cache, configuration
- **DuckDB** - Analytics and aggregation queries
- **Tantivy** - Full-text search with BM25 ranking
- **LEANN** - Vector embeddings for semantic search

## Performance

- **Indexing:** Incremental updates, only changed files
- **Search:** Sub-second for most queries
- **Memory:** Efficient caching with LRU eviction
- **Concurrency:** Async indexing with priority queues

## Configuration

LeIndex stores data in:
- `~/.claude/plugins/maestro/leindex/` - Index data
- `.leindex_data/` - Project-specific index (gitignored)

## Related Commands

- `/maestro:tldr` - 5-layer code analysis and context extraction
- `/maestro:configure` - Configure Maestro (including LeIndex MCP)

## See Also

- TLDR Code Analysis - `/maestro:tldr`
- LeIndex source - `maestro/leindex/`
