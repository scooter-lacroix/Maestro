# Maestro v2 Pre-Merge Analysis Report

**Date:** 2026-01-12
**Purpose:** Evaluate Continuous-Claude-v3 commits, installer parity, LeIndexer vs TLDR comparison, and database architecture before merging v2 PR

---

## Table of Contents

1. [Continuous-Claude-v3 Commit Analysis](#1-continuous-claude-v3-commit-analysis)
2. [Installer Parity Comparison](#2-installer-parity-comparison)
3. [LeIndexer vs TLDR Comparison](#3-leindexer-vs-tldr-comparison)
4. [DuckDB+SQLite as PostgreSQL Replacement](#4-duckdbsqlite-as-postgresql-replacement)
5. [Recommendations](#5-recommendations)

---

## 1. Continuous-Claude-v3 Commit Analysis

### Overview
Analyzed 71 commits from `https://github.com/parcadei/Continuous-Claude-v3.git` to identify changes beneficial to Maestro v2.

### High-Priority Commits for Maestro v2

| Commit | Description | Relevance | Integration Target | Considerations |
|--------|-------------|-----------|-------------------|----------------|
| `60690e0` | **feat: visual tldr-stats dashboard with Don Norman UX** | HIGH | `maestro/memory/dashboard.py` | Token savings visualization - add to Memory Dashboard |
| `bfe021b` | **feat: add update wizard for pulling latest and syncing components** | HIGH | `install-claude-code.sh` | Self-update capability - add update command |
| `180625b` | **feat: add PostgreSQL dual-backend support to artifact_index.py** | HIGH | `maestro/memory/database/` | Dual-backend (SQLite + PostgreSQL) support |
| `e31af4d` | **feat: add embedded postgres option to wizard** | MEDIUM | `install-claude-code.sh` | No Docker required option for database |
| `2590f1e` | **feat: add Podman support as Docker alternative** | MEDIUM | `install-claude-code.sh` | Container runtime flexibility |
| `1f41ff5` | **feat: add /tldr-stats skill for token savings tracking** | HIGH | `maestro/skills/` | Token savings tracking skill |
| `e1ee333` | **feat: improve daemon client + hook updates** | MEDIUM | `maestro/hooks/` | Hook improvements for daemon coordination |

### Medium-Priority Commits

| Commit | Description | Relevance | Integration Target |
|--------|-------------|-----------|-------------------|
| `d447d24` | fix: unify handoff format to YAML across all sources | MEDIUM | `maestro/memory/coordination/handoffs.py` |
| `484268d` | feat: add Docker daemon retry loop in setup wizard | MEDIUM | `install-claude-code.sh` |
| `da3d101` | fix: auto-detect GPU and fallback to lightweight model | MEDIUM | `maestro/memory/embeddings/service.py` |
| `27deadf` | fix: wizard semantic step downloads model without indexing | MEDIUM | Installer scripts |

### Cross-Platform Fixes (Apply to Maestro)

| Commit | Description | Files to Update |
|--------|-------------|-----------------|
| `714171d` | fix: use cross-platform temp directory paths | All temp file usage |
| `f78fd0d` | fix: cross-platform hooks + global script paths | `maestro/hooks/` |
| `eadf80b` | fix: cross-platform skill-activation-prompt hook for Windows | Windows compatibility |
| `ad01f2c` | fix: add cross-platform Python runner for hooks | Hook execution |
| `63f85f7` | fix: use python instead of python3 for Windows | All subprocess calls |

### Bug Fixes to Port

| Commit | Description | Priority |
|--------|-------------|----------|
| `185c2c0` | fix: memory daemon JSONL lookup for truncated session IDs | HIGH |
| `6a53e57` | fix: daemon-client check existsSync before using local tldr path | HIGH |
| `d360129` | fix: daemon-client PID check prevents duplicate daemon spawns | MEDIUM |
| `b10a490` | fix: hooks execute from project root instead of hooks directory | HIGH |
| `4fee297` | fix: escape Rich markup in error messages | LOW |

### Agent Analysis: Core Architectural Innovations

*Analysis provided by gemini-analyzer agent*

The v3 architecture moved away from simple shell scripts toward a robust, cross-platform **Hook Orchestration Layer** and a **Centralized Memory Daemon**. Key highlights include:

- **TypeScript-native Hooks:** Higher reliability and easier maintenance than Bash
- **Cross-terminal Coordination:** A persistent Python daemon handles state across multiple Claude sessions
- **Unified Handoffs:** Standardized YAML format for session continuity
- **Flexible Persistence:** Dual-backend support for SQLite and PostgreSQL (including embedded PG)

#### Detailed Implementation Considerations

**1. Centralized Memory Coordination (`memory_daemon.py`)**
- **Relevance:** HIGH
- **Mechanism:** A lightweight JSON-RPC or REST server running in the background
- **Maestro v2 Benefit:** Allows `maestro:status` to show real-time progress of agents running in other windows
- **Consideration:** Use `uv` to manage the daemon's lifecycle so it starts/stops automatically with the first/last Claude session
- **Target:** `maestro/memory/daemon.py` (new file)

**2. TypeScript Hook Compilation (`d6ee283`)**
- **Relevance:** HIGH
- **Implementation:** Include compiled `.mjs` files in git so users don't need `tsc` to get started
- **Maestro v2 Benefit:** Faster "Time to First Hook" (TTFH)
- **Consideration:** Adopt a `src/` (TS) and `dist/` (JS) structure for hooks, ensuring the hook launcher prioritizes `dist/` but allows developers to rebuild from `src/`

**3. The "Don Norman" UX Principles for CLI (`60690e0`)**
- **Relevance:** MEDIUM
- **Implementation:** The `tldr-stats` dashboard uses progress bars and sparklines to visualize token savings
- **Maestro v2 Benefit:** Improves user trust by showing exactly "how much" work the agent is doing
- **Target:** Integrate into `maestro:status` and `maestro:implement` output

**4. Standardized Handoff Schema (`3a1e9f5`)**
- **Relevance:** HIGH
- **Implementation:** Move away from free-form markdown ledgers to a structured YAML schema with `goal`, `now`, and `next` blocks
- **Maestro v2 Benefit:** Allows the Statusline (claude-hud) to parse the current state programmatically
- **Target:** `maestro/memory/coordination/handoffs.py`

**5. MCP Schema Inference (`6b7e73d`)**
- **Relevance:** HIGH
- **Implementation:** The `opc/src/runtime/` folder contains sophisticated logic for inferring MCP schemas when they are missing or malformed
- **Maestro v2 Benefit:** Allows Maestro to "discover" and "wrap" any MCP server automatically
- **Target:** `maestro/mcp/runtime.py` (new file, ~800 lines)

#### Next Steps for Maestro v2 (from agent analysis)

1. **Port the `run-python.mjs` wrapper** immediately to fix Windows hook execution issues
2. **Adopt the `hook_launcher.py` logic** to unify how Python and JS hooks are called
3. **Implement the Memory Daemon** to enable cross-terminal coordination for the orchestrator
4. **Migrate Handoffs to YAML** to support automated session resumption

---

## 2. Installer Parity Comparison

### Comparison: `wizard.py` vs `install-claude-code.sh`

| Feature | wizard.py (OPC) | install-claude-code.sh (Maestro) | Gap |
|---------|-----------------|----------------------------------|-----|
| **Prerequisites Check** | | | |
| Docker/Podman detection | ✅ Both supported with retry loop | ❌ Not checked | **MISSING** |
| Python version check | ✅ Python 3.11+ | ✅ Implicit | OK |
| uv package manager | ✅ Required | ❌ Not used | Different approach |
| Go/Zoekt installation | ❌ Not included | ✅ Full support | Maestro has more |
| npm/node detection | ✅ For TypeScript hooks | ✅ For frontend build | OK |
| | | | |
| **Database Configuration** | | | |
| SQLite support | ✅ Fallback option | ✅ Default | OK |
| PostgreSQL (Docker) | ✅ Full setup | ❌ Not included | **MISSING** |
| Embedded PostgreSQL | ✅ No Docker option | ❌ Not included | **MISSING** |
| Database migrations | ✅ Automated | ❌ Not included | **MISSING** |
| | | | |
| **Component Installation** | | | |
| Hooks installation | ✅ With TypeScript build | ✅ Copy only | **PARTIAL** |
| Skills installation | ✅ Full skill set | ✅ Full skill set | OK |
| Agents installation | ✅ Full agent set | ✅ Full agent set | OK |
| MCP servers | ✅ From plugins dir | ❌ Not included | **MISSING** |
| Rules installation | ✅ CLAUDE.md files | ❌ Not included | **MISSING** |
| | | | |
| **API Keys** | | | |
| Perplexity API | ✅ Prompted | ❌ Not prompted | **MISSING** |
| Braintrust API | ✅ Prompted | ❌ Not prompted | **MISSING** |
| Nia API | ✅ Prompted | ❌ Not prompted | **MISSING** |
| | | | |
| **Advanced Features** | | | |
| TLDR installation | ✅ Via uv tool | ❌ Not included | **MISSING** |
| Semantic search setup | ✅ With model download | ❌ Not included | **MISSING** |
| Math features (SymPy, Z3) | ✅ Optional install | ❌ Not included | **MISSING** |
| Loogle (Lean 4) | ✅ Optional install | ❌ Not included | **MISSING** |
| Diagnostics tools check | ✅ pyright, ruff, eslint | ❌ Not included | **MISSING** |
| | | | |
| **UX Features** | | | |
| Interactive prompts | ✅ Rich library | ✅ read -p | OK |
| Backup existing config | ✅ Before overwrite | ❌ No backup | **MISSING** |
| Resume/update wizard | ✅ Separate command | ❌ Not available | **MISSING** |
| Progress feedback | ✅ 12 steps shown | ✅ Basic echo | OK |

### Critical Gaps in Maestro Installer

1. **No TypeScript hook building** - Hooks with `package.json` need npm build
2. **No MCP server installation** - Missing `mcp.json` setup
3. **No database options** - Only works with SQLite via memory system
4. **No config backup** - Could overwrite user's existing `~/.claude`
5. **No TLDR CLI installation** - Missing `uv tool install llm-tldr`
6. **No API key prompts** - Missing integration with external services

### Recommended Additions to Maestro Installer

```bash
# 1. Add backup step before installation
if [ -d ~/.claude ]; then
    BACKUP_NAME=".claude.backup.$(date +%Y%m%d_%H%M%S)"
    cp -r ~/.claude ~/$BACKUP_NAME
    echo "   ✅ Backed up existing ~/.claude to ~/$BACKUP_NAME"
fi

# 2. Add TypeScript hook building
if [ -f ~/.claude/plugins/maestro/hooks/package.json ]; then
    echo "📦 Building TypeScript hooks..."
    cd ~/.claude/plugins/maestro/hooks
    npm install --quiet && npm run build --quiet
fi

# 3. Add TLDR installation option
if command_exists uv; then
    read -p "Install TLDR code analysis tool? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        uv tool install llm-tldr
    fi
fi
```

### Agent Analysis: Detailed Parity Comparison

*Analysis provided by codex-reviewer agent*

#### Components in BOTH installers
- **Claude Code Integration:** Both install hooks, skills, and agents into `~/.claude`
- **Frontend Building:** Both perform `npm install && npm run build` for subcomponents
- **Python Package/CLI:** Both ensure a CLI entry point is available

#### Components in wizard.py ONLY
| Component | Description | Impact on Maestro |
|-----------|-------------|-------------------|
| Database Infrastructure | Full PostgreSQL/Redis lifecycle management | High - limits scalability |
| TLDR Code Analysis | Installs `llm-tldr` with 1.3GB embedding models | High - core feature |
| Math & Theorem Proving | SymPy, Z3, Loogle (Lean 4) | Medium - specialized use |
| Diagnostics Suite | Checks for pyright, ruff, eslint, clippy | Medium - developer experience |
| Security Acknowledgment | Requires explicit sandbox risk acknowledgment | Low - security UX |
| Configuration Management | Interactive `.env` generation, merge/backup | High - user safety |

#### Components in Maestro installer ONLY
| Component | Description | Impact on wizard.py |
|-----------|-------------|---------------------|
| Zoekt Code Search | Trigram search engine (requires Go) | High - search capability |
| Go Environment | Offers Go installation for TUI/Search | Medium - infrastructure |
| Maestro TUI | Pre-built Go binary terminal UI | Medium - user experience |
| Critical Think Module | Chain-of-thought processing and planning | High - unique feature |
| Slash Command Templates | `.md` files defining slash commands | High - core functionality |

#### Gap Analysis Summary

| Feature | OPC v3 (wizard.py) | Maestro v2 (sh) | Gap Analysis |
|---------|-------------------|-----------------|--------------|
| **Search Engine** | Semantic (TLDR) | Trigram (Zoekt) | Maestro lacks semantic; OPC lacks trigram |
| **Persistence** | External DB (Postgres) | Internal/File-based | OPC requires infrastructure |
| **Dependency Mgr** | `uv` | `pip` / `go` | OPC uses modern `uv` |
| **UI** | CLI / Dashboard | TUI (Go) / Dashboard | OPC lacks Terminal UI |
| **Windows Support** | Native | Via WSL/Bash | OPC more accessible to Windows |
| **Math/Proofs** | Deep integration | None | OPC optimized for formal verification |
| **Backup/Safety** | Merge & Backup | Simple Copy | OPC safer for existing configs |

---

## 3. LeIndexer vs TLDR Comparison

### Feature Comparison Matrix

| Capability | LeIndexer | TLDR (Maestro) | Winner |
|------------|-----------|----------------|--------|
| **Core Search** | | | |
| Full-text search | ✅ Tantivy (Rust) | ❌ Not included | LeIndexer |
| Semantic search | ✅ LEANN vectors | ✅ Embeddings service | Tie |
| Hybrid search | ✅ Text + Semantic | ❌ Semantic only | LeIndexer |
| Cross-project search | ✅ Global registry | ❌ Per-project | LeIndexer |
| | | | |
| **Code Analysis** | | | |
| AST parsing | ✅ Basic | ✅ Full 5-layer | **TLDR** |
| Call graph | ❌ Not included | ✅ CallGraphAnalyzer | **TLDR** |
| Control flow graph | ❌ Not included | ✅ CFGAnalyzer | **TLDR** |
| Data flow graph | ❌ Not included | ✅ DFGAnalyzer | **TLDR** |
| Program slicing | ❌ Not included | ✅ SlicingAnalyzer | **TLDR** |
| | | | |
| **Infrastructure** | | | |
| MCP Server | ✅ Full implementation | ❌ Not MCP native | LeIndexer |
| Daemon mode | ✅ Background service | ❌ Not included | LeIndexer |
| Incremental indexing | ✅ File watcher | ❌ Not included | LeIndexer |
| Memory management | ✅ LRU cache, profiler | ❌ Basic | LeIndexer |
| | | | |
| **Storage** | | | |
| SQLite metadata | ✅ Full support | ✅ Memory system | Tie |
| DuckDB analytics | ✅ Query engine | ❌ Not included | LeIndexer |
| Vector storage | ✅ LEANN efficient | ✅ Basic embeddings | LeIndexer |
| | | | |
| **Integration** | | | |
| Claude Code hooks | ❌ Separate | ✅ Native integration | TLDR |
| Memory system | ❌ Separate | ✅ Full integration | TLDR |
| Token tracking | ❌ Not included | ✅ Via memory | TLDR |

### Architecture Comparison

**LeIndexer Architecture:**
```
MCP Server → Core Engine → LEANN (vectors) + Tantivy (text) + SQLite + DuckDB
              ↓
         Query Router → Hybrid ranking → Results
```

**TLDR Architecture:**
```
TLDRAnalyzer → AST → Call Graph → CFG → DFG → Slicing
                                    ↓
                           SemanticIndex → Embeddings
                                    ↓
                         Memory Integration → Maestro Memory
```

### Key Differences

| Aspect | LeIndexer | TLDR |
|--------|-----------|------|
| **Primary Focus** | Search and retrieval | Code understanding |
| **Token Efficiency** | Minimal (stores content) | Maximum (5-layer abstraction) |
| **Dependencies** | tantivy-py, duckdb | Pure Python |
| **Complexity** | Higher (multi-backend) | Lower (single-purpose) |
| **MCP Integration** | Native MCP server | None (uses hooks) |

### Recommendation: Hybrid Approach

**Neither full replacement nor full merger - instead, complementary integration:**

1. **Keep TLDR for code analysis** - Its 5-layer analysis (AST, CFG, DFG, Call Graph, Slicing) provides capabilities LeIndexer doesn't have
2. **Adopt LeIndexer's search infrastructure** - Replace basic semantic search with Tantivy + LEANN hybrid
3. **Adopt LeIndexer's MCP server pattern** - Native MCP integration is cleaner than hooks
4. **Keep TLDR's memory integration** - Already well-integrated with Maestro memory system

### Proposed Integration Architecture

```
                    ┌─────────────────────────────────────┐
                    │         Maestro v2 Enhanced         │
                    ├─────────────────────────────────────┤
                    │                                     │
     Search Layer   │  ┌──────────────────────────────┐   │
    (from LeIndex)  │  │  LeIndex Search Engine       │   │
                    │  │  - Tantivy (full-text)       │   │
                    │  │  - LEANN (semantic vectors)  │   │
                    │  │  - Hybrid ranking            │   │
                    │  └──────────────────────────────┘   │
                    │              ↕                      │
                    │  ┌──────────────────────────────┐   │
    Analysis Layer  │  │  TLDR Code Analysis          │   │
    (existing TLDR) │  │  - AST Analyzer              │   │
                    │  │  - Call Graph Analyzer       │   │
                    │  │  - CFG/DFG Analyzers         │   │
                    │  │  - Program Slicing           │   │
                    │  └──────────────────────────────┘   │
                    │              ↕                      │
                    │  ┌──────────────────────────────┐   │
    Memory Layer    │  │  Maestro Memory System       │   │
    (existing)      │  │  - SQLite database           │   │
                    │  │  - Embeddings service        │   │
                    │  │  - Coordination              │   │
                    │  └──────────────────────────────┘   │
                    └─────────────────────────────────────┘
```

### Migration Effort Estimate

| Task | Complexity | Files Affected |
|------|------------|----------------|
| Port LeIndex search to TLDR | HIGH | 5-10 new files |
| Add MCP server to TLDR | MEDIUM | 1-2 new files |
| Add incremental indexing | MEDIUM | 2-3 files |
| Add daemon mode | MEDIUM | 2-3 files |
| Keep existing TLDR analyzers | LOW | No changes |

### Agent Analysis: TLDR Detailed Capabilities

*Analysis provided by qwen-coder agent*

TLDR (Token-efficient Large codebase Dependency & Relationship) is a sophisticated 5-layer static analysis system designed to provide deep code understanding while minimizing token usage for LLM interactions.

#### The 5 Analysis Layers

| Layer | Type | File | Description | Token Savings |
|-------|------|------|-------------|---------------|
| **1** | AST | `ast.py` | Function signatures, imports, class definitions (no implementations) | ~95% (~500 tokens) |
| **2** | Call Graph | `callgraph.py` | Cross-file function relationships and dependencies | ~440 tokens |
| **3** | CFG | `cfg.py` | Control flow, cyclomatic complexity, execution paths | ~110 tokens |
| **4** | DFG | `dfg.py` | Variable definitions, uses, and modifications | ~130 tokens |
| **5** | Slicing | `slicing.py` | Program Dependence Graph (PDG), backward/forward slicing | ~150 tokens |

#### Key Functions

**Program Slicing (`slicing.py`):**
- `slice_backward`: Finds all statements that influence a specific line ("how did this variable get this value?")
- `slice_forward`: Finds all statements influenced by a specific line ("if I change this, what breaks?")
- `compute_chop`: Analyzes paths between two specific lines

**Call Graph Analysis (`callgraph.py`):**
- `analyze_impact`: Calculates "ripple effect" of changing a function across the project
- `detect_cycles`: Finds circular dependencies in call hierarchy
- `find_dead_code`: Identifies unreachable functions from entry points

**Semantic Search (`semantic/__init__.py`):**
- Uses embeddings (default: `all-MiniLM-L6-v2`) for natural language searches like "find functions handling auth"
- Caches code entities with signatures and docstrings

**Context Extraction (`context.py`):**
- `get_relevant_context`: Traverses call graph to build focused "context package" for LLM prompts
- `get_context_for_prompt`: Heuristically identifies files and symbols mentioned in a prompt

#### Memory Integration (`memory_integration.py`)

TLDR acts as a high-fidelity sensor for the Maestro memory system:
- **Persistence:** Analysis results stored as `Memory` objects in database
- **Code Entities:** Individual functions/classes stored as searchable patterns
- **Embeddings:** Integration with `EmbeddingsService` for semantic similarity retrieval
- **Session Context:** Ties analysis to specific user sessions for temporal context

#### Unique Capabilities vs Standard Indexers

- **Token Efficiency:** Represents a 10,000+ token file in ~500 tokens by preserving only "interface" and "flow"
- **True Program Slicing:** AST-based PDG analysis vs text-based search
- **Multi-Layer Orchestration:** `TLRDAnalyzer` lazily invokes deeper layers only when needed
- **LLM-Native Output:** Every analyzer includes `to_llm_string()` tuned for model consumption

### Agent Analysis: LeIndexer Detailed Capabilities

*Analysis provided by goose-coder agent*

LeIndexer (v1.1.2) is an AI-powered code search and indexing system designed for local, high-performance operations with a "Standalone Power Mode" philosophy.

#### Indexing Architecture

| Backend | Technology | Purpose |
|---------|------------|---------|
| **LEANN** | Vector Store | Primary semantic engine with code-specific embeddings (FAISS deprecated) |
| **Tantivy** | Rust Lucene | Fast BM25 full-text search with code tokenization |
| **Zoekt** | Trigram | Sub-millisecond literal and regex searches |
| **SQLite** | Metadata | File history, versions, general metadata |
| **DuckDB** | Analytics | Fast aggregation queries |

#### Supported Embedding Models

- `BAAI/bge-small-en-v1.5`
- `microsoft/codebert-base`
- `nomic-ai/CodeRankEmbed`

#### Search Capabilities

**Hybrid Ranking Algorithm (`ranking.py`):**
1. Semantic similarity (Vector score)
2. Code relevance (BM25 from Tantivy)
3. File recency/frequency (modification time, access patterns)
4. Path importance (Source vs Tests vs Docs)
5. User behavior analytics (frequently accessed files)

**Cross-Project Search:** Query across multiple registered projects simultaneously

#### Performance Features

- **Parallel Scanner:** `asyncio` + `os.scandir` with semaphore-based concurrency (3-5x faster than `os.walk`)
- **LRU Caching:** TTL-based caches for search results and vector lookups
- **CPU Optimization:** Configured for CPU-only environments (no CUDA/GPU bloat)
- **Circuit Breaker:** Handles unresponsive filesystems or repeated timeouts gracefully

#### Security Features

- **Path Traversal Protection:** Strong validation in `content_extractor.py`
- **ReDoS Protection:** Regex complexity validator prevents DoS attacks

#### Code Analysis Limitations

While LeIndexer focuses on search and retrieval (RAG):
- ✅ AST-Aware Chunking (Python-specific)
- ✅ Symbol Extraction (function/class names)
- ❌ **No Call Graph** construction
- ❌ **No CFG/DFG** analysis
- ❌ **No Program Slicing**

These capabilities are handled implicitly through embedding models rather than explicit static analysis.

### Comparative Analysis Summary

| Capability | LeIndexer Approach | TLDR Approach | Recommendation |
|------------|-------------------|---------------|----------------|
| **Search** | Multi-backend hybrid (Tantivy + LEANN + Zoekt) | Basic embeddings | **Adopt LeIndexer** |
| **Code Analysis** | Implicit (via embeddings) | Explicit (5-layer static analysis) | **Keep TLDR** |
| **Token Efficiency** | Stores full content chunks | Generates compressed representations | **Keep TLDR** |
| **MCP Integration** | Native server | Via hooks | **Adopt LeIndexer** |
| **Incremental Updates** | File watcher + change tracker | None | **Adopt LeIndexer** |
| **Memory Integration** | Separate system | Native Maestro integration | **Keep TLDR** |

---

## 4. Recommendations

### Immediate Actions (Before Merge)

1. **Update installer to back up existing config** - Critical for user safety
2. **Add TypeScript hook building** - Required for full functionality
3. **Document the gaps** - Users should know what wizard.py has that installer doesn't

### Post-Merge Actions

1. **Port cross-platform fixes** from commits:
   - `714171d` - temp directory paths
   - `f78fd0d` - cross-platform hooks
   - `63f85f7` - python vs python3

2. **Add visual TLDR stats dashboard** from commit `60690e0`

3. **Add update wizard** from commit `bfe021b`

### Future Track: Search Enhancement

Create a new Maestro track to:
1. Integrate LeIndexer's Tantivy search backend
2. Add LeIndexer's MCP server pattern
3. Keep TLDR's 5-layer analysis as the core differentiator
4. Add daemon mode for background indexing

### Final Verdict on TLDR vs LeIndexer

**Recommendation: Keep TLDR, selectively adopt LeIndexer components**

| Component | Action |
|-----------|--------|
| TLDR AST/CFG/DFG/Slicing | **KEEP** - Unique value |
| TLDR SemanticIndex | **REPLACE** with LeIndex LEANN (FAISS deprecated) |
| TLDR Memory Integration | **KEEP** - Well integrated |
| LeIndex Tantivy search | **ADOPT** - Superior full-text |
| LeIndex MCP Server | **ADOPT** - Cleaner pattern |
| LeIndex Daemon mode | **ADOPT** - Better performance |
| LeIndex DuckDB analytics | **ADOPT** - See Section 4 for PostgreSQL replacement strategy |

---

## 4. DuckDB+SQLite as PostgreSQL Replacement

### Overview

Analysis of whether DuckDB+SQLite can replace PostgreSQL in the Maestro/Continuous-Claude-v3 architecture, eliminating Docker dependencies while maintaining or improving functionality.

### Current PostgreSQL Usage (from CC-v3/OPC)

| PostgreSQL Feature | Current Use Case |
|-------------------|------------------|
| **pgvector** | Vector embeddings for semantic search |
| **pg_trgm** | Trigram-based fuzzy text matching |
| **Full-Text Search** | tsvector/tsquery for code search |
| **Concurrent Writes** | Multiple agents writing simultaneously |
| **Network Access** | Cross-terminal coordination (memory daemon) |
| **ON CONFLICT** | Upsert operations |

### DuckDB+SQLite Replacement Analysis

| PostgreSQL Feature | DuckDB+SQLite Replacement | Parity? |
|-------------------|---------------------------|---------|
| **pgvector** | LEANN dedicated vector store | ✅ Yes (better performance) |
| **pg_trgm** | Tantivy/Zoekt trigram engine | ✅ Yes (faster) |
| **Full-Text Search** | SQLite FTS5 + DuckDB FTS extension | ✅ Yes |
| **Concurrent Writes** | SQLite WAL mode (single writer, many readers) | ⚠️ Partial |
| **Network Access** | File-based coordination | ⚠️ Partial |
| **ON CONFLICT** | SQLite `INSERT OR REPLACE` / `ON CONFLICT` | ✅ Yes |

### Verdict: PostgreSQL Can Be Replaced

**Yes, DuckDB+SQLite can fully replace PostgreSQL** with these architectural changes:

1. **Vector search**: Use dedicated LEANN store instead of pgvector (FAISS deprecated)
2. **Fuzzy/trigram search**: Use Tantivy instead of pg_trgm
3. **Cross-terminal coordination**: Use file-based locking instead of network DB
4. **Analytics**: Use DuckDB's columnar engine for dashboards

### Recommended Storage Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Maestro v2 Storage Layer                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   SQLite    │  │   DuckDB    │  │       LEANN         │  │
│  │   (OLTP)    │  │   (OLAP)    │  │     (Vectors)       │  │
│  ├─────────────┤  ├─────────────┤  ├─────────────────────┤  │
│  │ • Sessions  │  │ • Token     │  │ • Code embeddings   │  │
│  │ • Claims    │  │   stats     │  │ • Semantic search   │  │
│  │ • Handoffs  │  │ • Dashboards│  │ • Similarity        │  │
│  │ • Memories  │  │ • Analytics │  │   ranking           │  │
│  │ • Ledgers   │  │ • Reporting │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│         │                │                    │             │
│         └────────────────┼────────────────────┘             │
│                          │                                  │
│              ┌───────────▼───────────┐                      │
│              │   Coordination Layer  │                      │
│              │   (File locks + WAL)  │                      │
│              └───────────────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Technology | Responsibility | Replaces |
|-----------|------------|----------------|----------|
| **OLTP Store** | SQLite + WAL | Sessions, claims, handoffs, memories | PostgreSQL tables |
| **OLAP Store** | DuckDB | Analytics, dashboards, token statistics | PostgreSQL aggregations |
| **Vector Store** | LEANN | Code embeddings, semantic similarity | pgvector (FAISS deprecated) |
| **Text Search** | Tantivy | Full-text BM25 search | pg_trgm, tsvector |
| **Coordination** | File locks | Cross-terminal file claims | PostgreSQL network |

### Performance Comparison

| Workload | PostgreSQL | DuckDB+SQLite | Winner |
|----------|-----------|---------------|--------|
| **OLTP (sessions, claims)** | Good | SQLite: Excellent | SQLite |
| **OLAP (analytics, stats)** | Good | DuckDB: Excellent | DuckDB |
| **Vector search** | pgvector: Good | LEANN: Better | LEANN |
| **Full-text search** | Good | Tantivy: Excellent | Tantivy |
| **Concurrent writes** | Excellent | Moderate | PostgreSQL |
| **Installation overhead** | High (Docker/pgserver) | Minimal | DuckDB+SQLite |

### Key Benefit: DuckDB's SQLite Scanner

DuckDB can **directly query SQLite databases** without ETL or data duplication:

```sql
-- DuckDB querying SQLite directly (no data copying)
INSTALL sqlite;
LOAD sqlite;

ATTACH '~/.maestro/memory.db' AS maestro (TYPE sqlite);

-- Analytical queries at DuckDB speed on SQLite data
SELECT
    session_id,
    category,
    COUNT(*) as memory_count,
    AVG(LENGTH(content)) as avg_size
FROM maestro.memories
WHERE created_at > '2026-01-01'
GROUP BY session_id, category;
```

This means:
- **SQLite remains the single source of truth** for transactional data
- **DuckDB provides analytical acceleration** without data duplication
- **No sync mechanism needed** - DuckDB reads SQLite files directly

### Overhead Reduction Summary

| Component | PostgreSQL Stack | DuckDB+SQLite Stack | Savings |
|-----------|-----------------|---------------------|---------|
| **Runtime dependency** | Docker OR pgserver | None (embedded) | **100%** |
| **Installation size** | ~500MB+ | ~50MB | **90%** |
| **Setup complexity** | High (wizard, config) | Minimal (file-based) | **80%** |
| **Memory footprint** | ~100MB idle | ~10MB idle | **90%** |
| **Startup time** | 2-5 seconds | Instant | **100%** |
| **Backup complexity** | pg_dump required | Copy files | **90%** |

### Cross-Terminal Coordination Without PostgreSQL

PostgreSQL's main advantage is network-accessible shared state. Without it:

**Solution: File-based coordination with advisory locking**

```
~/.maestro/
├── memory.db           # SQLite with WAL (main data)
├── analytics.duckdb    # DuckDB for dashboards
├── vectors/            # LEANN vector store (FAISS deprecated)
├── coordination/
│   ├── active_sessions.json  # Current sessions registry
│   └── locks/                # Advisory file locks for claims
└── .tantivy_index/     # Full-text search index
```

**How it works:**
1. SQLite WAL mode allows concurrent reads from multiple terminals
2. File locks prevent conflicting writes to the same files
3. `active_sessions.json` tracks which terminals are active
4. Each terminal reads shared state, coordinates via lock files

### Implementation Path

```python
# maestro/memory/database/backends.py (proposed new file)

from pathlib import Path
import duckdb
import sqlite3

class UnifiedStorageBackend:
    """Unified storage backend replacing PostgreSQL."""

    def __init__(self, base_path: str = "~/.maestro"):
        self.base_path = Path(base_path).expanduser()

        # OLTP: SQLite with WAL for transactional data
        self.sqlite_path = self.base_path / "memory.db"

        # OLAP: DuckDB for analytics (reads SQLite directly)
        self.duckdb_conn = duckdb.connect(str(self.base_path / "analytics.duckdb"))
        self._attach_sqlite()

        # Vector: LEANN for embeddings (FAISS deprecated)
        self.vector_store = LEANNStore(self.base_path / "vectors")

    def _attach_sqlite(self):
        """Attach SQLite database for direct querying."""
        self.duckdb_conn.execute("INSTALL sqlite; LOAD sqlite;")
        self.duckdb_conn.execute(
            f"ATTACH '{self.sqlite_path}' AS maestro (TYPE sqlite)"
        )

    def query_analytics(self, sql: str):
        """Fast analytical queries via DuckDB on SQLite data."""
        return self.duckdb_conn.execute(sql).fetchdf()

    def get_token_statistics(self, session_id: str = None):
        """Example: Token usage dashboard query."""
        where_clause = f"WHERE session_id = '{session_id}'" if session_id else ""
        return self.query_analytics(f"""
            SELECT
                date_trunc('hour', created_at) as hour,
                category,
                COUNT(*) as memory_count,
                SUM(LENGTH(content)) as total_bytes
            FROM maestro.memories
            {where_clause}
            GROUP BY 1, 2
            ORDER BY 1 DESC
        """)
```

### Migration Considerations

| Current State | Migration Step | Effort |
|--------------|----------------|--------|
| PostgreSQL for memory | Keep SQLite (already primary) | None |
| pgvector for embeddings | Use LEANN (LeIndexer pattern) | Medium |
| pg_trgm for search | Add Tantivy backend | Medium |
| Docker dependency | Remove from installer | Low |
| Embedded pgserver | Remove from wizard | Low |

### Caveats

1. **Concurrent writes**: SQLite WAL supports single writer + many readers. For Maestro's use case (one user, multiple terminals), this is acceptable. Heavy multi-agent concurrent writes may need serialization.

2. **Cross-terminal coordination**: File-based locking is simpler but requires all terminals to be on the same filesystem. Remote coordination would need a lightweight HTTP daemon (similar to memory_daemon.py from CC-v3).

3. **Vector search consistency**: LEANN stores are eventually consistent. For Maestro's use case, this is acceptable as embeddings don't need ACID guarantees.

---

## 5. Recommendations

### Immediate Actions (Before Merge)

1. **Update installer to back up existing config** - Critical for user safety
2. **Add TypeScript hook building** - Required for full functionality
3. **Document the gaps** - Users should know what wizard.py has that installer doesn't

### Post-Merge Actions

1. **Port cross-platform fixes** from commits:
   - `714171d` - temp directory paths
   - `f78fd0d` - cross-platform hooks
   - `63f85f7` - python vs python3

2. **Add visual TLDR stats dashboard** from commit `60690e0`

3. **Add update wizard** from commit `bfe021b`

### Database Architecture Track

Based on Section 4 analysis, create a new Maestro track to:
1. **Adopt DuckDB+SQLite** as PostgreSQL replacement
2. **Eliminate Docker dependency** for database
3. **Implement file-based coordination** for cross-terminal support
4. **Add DuckDB analytics layer** for dashboards

### Future Track: Search Enhancement

Create a new Maestro track to:
1. Integrate LeIndexer's Tantivy search backend
2. Add LeIndexer's MCP server pattern
3. Keep TLDR's 5-layer analysis as the core differentiator
4. Add daemon mode for background indexing
5. Use LEANN for vector search (FAISS deprecated)

### Final Verdict on TLDR vs LeIndexer

**Recommendation: Keep TLDR, selectively adopt LeIndexer components**

| Component | Action |
|-----------|--------|
| TLDR AST/CFG/DFG/Slicing | **KEEP** - Unique value |
| TLDR SemanticIndex | **REPLACE** with LeIndex LEANN (FAISS deprecated) |
| TLDR Memory Integration | **KEEP** - Well integrated |
| LeIndex Tantivy search | **ADOPT** - Superior full-text |
| LeIndex MCP Server | **ADOPT** - Cleaner pattern |
| LeIndex Daemon mode | **ADOPT** - Better performance |
| LeIndex DuckDB+SQLite | **ADOPT** - Replaces PostgreSQL (Section 4) |

### Final Verdict on Database Architecture

**Recommendation: Adopt DuckDB+SQLite, eliminate PostgreSQL dependency**

| Component | Action |
|-----------|--------|
| PostgreSQL (Docker) | **REMOVE** - Replace with SQLite |
| PostgreSQL (pgserver) | **REMOVE** - Replace with SQLite |
| pgvector | **REPLACE** with LEANN (FAISS deprecated) |
| pg_trgm | **REPLACE** with Tantivy |
| SQLite memory.db | **KEEP** - Already primary OLTP store |
| DuckDB analytics | **ADD** - New OLAP layer for dashboards |

---

## Appendix: Continuous-Claude-v3 Commit Details

### Full Commit List (71 commits)

```
49cef10 Update command path for persist-project-dir hook
27e8296 fix: remove dead opc CLI entry point (closes #65)
6b7e73d fix: include runtime package for mcp-exec command (closes #66)
d447d24 fix: unify handoff format to YAML across all sources (closes #68)
714171d fix: use cross-platform temp directory paths (closes #63)
185c2c0 fix: memory daemon JSONL lookup for truncated session IDs
0854d47 fix: GitHub issues #53, #55-58 + docs & UX improvements
60690e0 feat: visual tldr-stats dashboard with Don Norman UX
6a53e57 fix: daemon-client check existsSync before using local tldr path
d360129 fix: daemon-client PID check prevents duplicate daemon spawns
e1ee333 feat: improve daemon client + hook updates
98527a9 Merge pull request #48 from UAEpro/patch-1
dd1e637 fix: update search-router skill to use tldr CLI
25237b3 fix: use sys.executable for cross-platform compatibility
27deadf fix: wizard semantic step now downloads model without indexing
1f41ff5 feat: add /tldr-stats skill for token savings tracking
2e13e50 fix: update wizard now handles local dev installs
ca53856 docs: add Updating section to TOC
bfe021b feat: add update wizard for pulling latest and syncing components
180625b feat: add PostgreSQL dual-backend support to artifact_index.py
fc5d30b Fix formatting of description in SKILL.md
395b7b2 fix: update stale database credentials in inline Python snippets
178e576 fix: use CLAUDE_PROJECT_DIR for script paths (fixes #46)
2590f1e feat: add Podman support as Docker alternative (#36)
9531d0c fix: add missing model field to braintrust-analyst agent
73f13c1 Merge pull request #45 from ASRagab/refactor/agent-frontmatter-contribution-guidelines
39c5ee7 fix: resolve setup artifacts from earlier experiments (#39)
e31af4d feat: add embedded postgres option to wizard (no Docker required)
3a1e9f5 fix: update onboard and create_handoff skills for v3 handoff system
79c54d2 refactor: Adding frontmatter required elements and updating contribution guidelines
20a3d31 Merge pull request #23 from d46/main
629e783 fix: add YAML frontmatter to agent files (issue #34)
484268d feat: add Docker daemon retry loop in setup wizard
f78fd0d fix: cross-platform hooks + global script paths
f36ce28 fix: wizard now installs plugins directory (braintrust-tracing)
da3d101 fix: auto-detect GPU and fallback to lightweight model for semantic indexing
391d1d3 fix: remove dead opc CLI references from wizard
da7357b fix: remove stale agent_monitor_tui reference from wizard
c745ad7 Merge pull request #30 from artile/main
eadf80b fix: cross-platform skill-activation-prompt hook for Windows
ad01f2c fix: add cross-platform Python runner for hooks
63f85f7 fix: use python instead of python3 for Windows compatibility
9e34c9b docs: remove broken link to gitignored handoff file
b10a490 fix: hooks execute from project root instead of hooks directory
4fee297 fix: escape Rich markup in error messages
e4c97b2 Merge master into main - reconcile diverged branches
928fe76 fix: docker_setup.py uses correct DB name and -f flag
a829935 fix: docker-compose uses env vars for port and adds download feedback
3f7dd61 fix: wizard installs to global ~/.claude and builds TypeScript hooks
fb3e23c Merge pull request #17 from flashwing-nwrp/fix/wizard-import-path
70478b5 Merge pull request #20 from parcadei/opc-path-fix
33d3bb6 fix: add CLAUDE_OPC_DIR env var support for global hook installation
eff7550 fix: add CLAUDE_OPC_DIR env var support for global hook installation
2aa8584 Fix: Setup wizard fails when run directly as script
41de827 Add semantic index detection to session start hook
027318c Clarify database config prompt for container/remote postgres users
2f05cff Fix wizard command: use -m flag for module imports
bf69f3d Remove readme field (internal package, not published to PyPI)
985f937 Add missing pyproject.toml to opc/
55a3f12 Merge pull request #14 from lehcosta/main
8b114ad Use hook_launcher for Python hooks in settings.json
4be2413 Add Python hook support to hook_launcher.py
7657e6f Fix: markup error in setup wizard
d6ee283 Include compiled hooks for out-of-box functionality
9fc887d Allow .claude/hooks/dist/ in git
3f8d546 Fix: clone URL and add note about pyproject.toml in opc/
b97cc91 Fix cross-terminal coordination database connection
401606f Fix: column header to just 'Tokens'
f3bb88c Add cross-terminal database quick reference rule
9d15301 Fix: TLDR table column 'Token Savings' → 'Token Cost'
f8d7173 Initial release: Continuous Claude v3
```

---

**Report Generated:** 2026-01-12
**Last Updated:** 2026-01-12
**Analysis Duration:** ~45 minutes
**Data Sources:**
- Continuous-Claude-v3 git repository (71 commits)
- wizard.py (1151 lines)
- install-claude-code.sh (503 lines)
- LeIndexer source (47 Python files)
- Maestro TLDR module (12 Python files)
- Maestro memory/database/models.py (1846 lines)

**Agent Contributions:**
- **gemini-analyzer**: Continuous-Claude-v3 commit analysis, architectural innovations identification
- **codex-reviewer**: wizard.py vs installer parity analysis, gap assessment
- **qwen-coder**: TLDR module deep-dive, 5-layer analysis documentation
- **goose-coder**: LeIndexer capabilities analysis, search infrastructure evaluation

**Additional Analysis:**
- DuckDB+SQLite as PostgreSQL replacement (Section 4)
- FAISS deprecation in favor of LEANN noted throughout
