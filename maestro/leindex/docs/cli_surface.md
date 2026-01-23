# LeIndex CLI Surface Specification

## Overview

LeIndex provides a comprehensive CLI surface for code analysis, indexing, and search. This document defines the canonical command interface.

## CLI Structure

```
maestro <command> [subcommand] [options]
```

## Commands

### 1. `analyze` - File-level code analysis

**Purpose:** Perform 5-layer analysis on individual files

**Usage:**
```bash
maestro analyze <path> [options]
```

**Options:**
- `--format <format>`: Output format: `json`, `llm` (balanced), `ultra` (default: `llm`)
- `--analysis <type>`: Analysis type: `ast`, `callgraph`, `cfg`, `dfg`, `slicing`, `all` (default: `all`)
- `--language <lang>`: Language (auto-detected if not specified): `python`, `typescript`, `rust`, `go`, `java`, `c`, `cpp`
- `--output <file>`: Write output to file instead of stdout

**Examples:**
```bash
# All layers on a file (LLM-ready output)
maestro analyze src/auth.py

# Specific analysis layer
maestro analyze src/auth.py --analysis callgraph

# JSON output for parsing
maestro analyze src/auth.py --format json

# Ultra-condensed for exploration
maestro analyze src/auth.py --format ultra
```

**Output Caps:**
- `json`: No cap (full structured data)
- `llm`: ~6000 chars per analysis layer
- `ultra`: ~2500 chars per analysis layer

---

### 2. `leindex` - Project-level operations

**Purpose:** Index, search, and analyze entire codebases

**Usage:**
```bash
maestro le-index <subcommand> [options]
```

#### Subcommands

##### `init <path>` - Initialize/index project

**Usage:**
```bash
maestro le-index init [path]
```

**Options:**
- `--force`: Re-index even if index exists

**Example:**
```bash
maestro le-index init .
```

##### `status` - Show index status

**Usage:**
```bash
maestro le-index status
```

**Output:**
- Files indexed
- Total symbols extracted
- Index size
- Last update time

##### `search <query>` - Search code

**Usage:**
```bash
maestro le-index search "<query>" [options]
```

**Options:**
- `--limit <n>`: Max results (default: 20)
- `--file <path>`: Search within specific file
- `--format <format>`: `json`, `text` (default: `text`)

**Examples:**
```bash
maestro le-index search "authentication"
maestro le-index search "database connection pooling" --limit 10
maestro le-index search "process_payment" --file src/services/payment.py
```

##### `analyze <path>` - 5-phase project analysis

**Usage:**
```bash
maestro le-index analyze <path> [options]
```

**Options:**
- `--phase <n>`: Run specific phase (1-5), or `all` (default)
- `--mode <mode>`: `ultra`, `balanced`, `verbose` (default: `balanced`)
- `--max-files <n>`: Max files to analyze (default: 25)
- `--max-chars <n>`: Max output chars (default: 12000)

**Phases:**
- `phase1` - Structural Scan: File listing, AST summary, language distribution
- `phase2` - Dependency Map: Import/usage frequency, module relationships
- `phase3` - Logic Flow: Call graph analysis, complexity metrics
- `phase4` - Critical Path: Complexity hotspots, technical debt candidates
- `phase5` - Optimization Report: Consolidated summary with recommendations

**Examples:**
```bash
# All phases (balanced mode)
maestro le-index analyze .

# Single phase (ultra-condensed)
maestro le-index analyze . --phase 1 --mode ultra

# Phase 3 with custom limits
maestro le-index analyze . --phase 3 --max-files 50 --max-chars 20000

# JSON output for orchestrate engine
maestro le-index analyze . --format json
```

**Output Caps:**
- `ultra`: ~2500 chars per file block
- `balanced`: ~6000 chars per file block
- `verbose`: No hard cap (subject to `--max-chars`)

##### `phase<n>` - Direct phase access (shorthand)

**Usage:**
```bash
maestro le-index phase1 <path> [options]
maestro le-index phase2 <path> [options]
maestro le-index phase3 <path> [options]
maestro le-index phase4 <path> [options]
maestro le-index phase5 <path> [options]
```

**Options:** Same as `analyze` subcommand

**Example:**
```bash
maestro le-index phase1 . --mode ultra
```

##### `context <target> [path]` - Generate context bundle

**Usage:**
```bash
maestro le-index context <target> [path]
```

**Arguments:**
- `target`: File path or function name
- `path`: Project root (default: current directory)

**Options:**
- `--format <format>`: `json`, `llm` (default: `llm`)
- `--include-callers`: Include caller analysis
- `--include-callees`: Include callee analysis

**Examples:**
```bash
# Context for a file
maestro le-index context src/auth.py

# Context for a function
maestro le-index context authenticate_user src/auth.py

# Context for entire project
maestro le-index context . --project

# JSON bundle for orchestrate
maestro le-index context src/auth.py --format json
```

---

### 3. `memory` - Memory system operations

**Purpose:** Manage Maestro memory system

**Usage:**
```bash
maestro memory <subcommand> [options]
```

#### Subcommands

- `serve` - Start memory dashboard web server
- `status` - Show memory system statistics
- `scan` - Scan directories for Maestro projects
- `import` - Import data from external memory systems
- `export` - Export memories to JSON

---

### 4. `tui` - Terminal UI

**Purpose:** Launch Maestro Cockpit TUI

**Usage:**
```bash
maestro tui
```

---

### 5. `mcp` - MCP server operations

**Purpose:** Manage MCP servers

**Usage:**
```bash
maestro mcp <subcommand> [options]
```

#### Subcommands

- `serve` - Start pooled stdio MCP servers on UNIX sockets
- `proxy <name>` - Bridge stdio to a pooled UNIX socket server
- `tool-search` - Meta MCP server: tool search + cross-server tool call

---

## Output Formats

### `json`

Machine-readable structured output. Used by:
- Orchestrate engine
- Cockpit TUI parsing
- Programmatic access

### `llm` (balanced)

LLM-actionable output optimized for prompt injection. Includes:
- Function signatures
- Line numbers
- Key call edges
- Complexity metrics

### `ultra`

Exploration-only, maximum compression. Useful for:
- Initial codebase orientation
- Large-scale scans
- Token-constrained scenarios

---

## Language Support

LeIndex supports 8 programming languages via tree-sitter:

| Language | Extensions | CLI Value |
|----------|-----------|-----------|
| Python | `.py` | `python`, `py` |
| TypeScript | `.ts`, `.tsx` | `typescript`, `ts` |
| JavaScript | `.js`, `.jsx` | `javascript`, `js` |
| Rust | `.rs` | `rust`, `rs` |
| Go | `.go` | `go` |
| Java | `.java` | `java` |
| C | `.c`, `.h` | `c` |
| C++ | `.cpp`, `.cc`, `.hpp`, `.h` | `cpp`, `c++` |

---

## TLDR Compatibility Mapping

| Old TLDR Command | LeIndex Equivalent |
|------------------|-------------------|
| `tldr warm .` | `maestro le-index init .` |
| `tldr context main.py` | `maestro le-index context main.py` |
| `tldr ast file.py` | `maestro analyze file.py --analysis ast` |
| `tldr callgraph file.py` | `maestro analyze file.py --analysis callgraph` |
| `tldr callers func` | `maestro le-index context func file.py --include-callers` |
| `tldr callees func` | `maestro le-index context func file.py --include-callees` |
| `tldr cfg file.py` | `maestro analyze file.py --analysis cfg` |
| `tldr dfg file.py` | `maestro analyze file.py --analysis dfg` |
| `tldr slice file.py 42` | `maestro analyze file.py --analysis slicing --line 42` |
| `tldr search "query"` | `maestro le-index search "query"` |
| `/phase1` | `maestro le-index phase1 .` |
| `/phase2` | `maestro le-index phase2 .` |
| `/phase3` | `maestro le-index phase3 .` |
| `/phase4` | `maestro le-index phase4 .` |
| `/phase5` | `maestro le-index phase5 .` |

---

## Context Bundle Format (JSON)

When `--format json` is specified, LeIndex returns structured data:

```json
{
  "task_id": "string",
  "description": "string",
  "files": [
    {
      "path": "string",
      "language": "string",
      "excerpts": {
        "ast": "string",
        "callgraph": "string",
        "cfg": "string",
        "dfg": "string"
      }
    }
  ],
  "analysis": {
    "phase1": "string",
    "phase2": "string",
    "phase3": "string",
    "phase4": "string",
    "phase5": "string"
  },
  "commands": ["string"]
}
```

This format is used by the orchestrate engine for token-efficient context packing.
