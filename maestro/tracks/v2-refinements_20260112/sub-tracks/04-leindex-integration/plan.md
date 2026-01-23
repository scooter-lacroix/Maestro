# Sub-Track 04: LeIndex Integration - Plan

## Overview

Implementation plan for LeIndex as Maestro's unified search and analysis system.

---

## Phase 1: Module Structure

### [x] Task 4.1: Create LeIndex Module Structure
- [x] Create `maestro/leindex/` directory structure
- [x] Create `__init__.py` with public API stubs
- [x] Define interfaces for analyzers, search, and integration
- [x] Add module to Maestro's package configuration

---

## Phase 2: 5-Layer Analysis

### [x] Task 4.2: Implement 5-Layer Analysis (from TLDR)
- [x] Write tests for AST analyzer
- [x] Implement AST analyzer with `to_llm_string()`
- [x] Write tests for Call Graph analyzer
- [x] Implement Call Graph analyzer with `to_llm_string()`
- [x] Write tests for CFG analyzer
- [x] Implement CFG analyzer with `to_llm_string()`
- [x] Write tests for DFG analyzer
- [x] Implement DFG analyzer with `to_llm_string()`
- [x] Write tests for Slicing analyzer
- [x] Implement Slicing analyzer with `to_llm_string()`
- [x] Write integration tests for multi-layer analysis

---

## Phase 3: Search Backend

### [x] Task 4.3: Implement Tantivy Search Backend
- [x] Write tests for full-text indexing
- [x] Write tests for BM25 search queries
- [x] Write tests for index updates
- [x] Create `maestro/leindex/search/tantivy_backend.py`
- [x] Implement index creation
- [x] Implement search with BM25 ranking
- [x] Implement incremental updates

---

## Phase 4: Vector Store

### [x] Task 4.4: Implement LEANN Vector Store
- [x] Write tests for embedding storage
- [x] Write tests for semantic similarity search
- [x] Write tests for vector updates
- [x] Create `maestro/leindex/search/leann_store.py`
- [x] Implement vector indexing
- [x] Implement similarity retrieval
- [x] Add caching layer

---

## Phase 5: Hybrid Ranking

### [x] Task 4.5: Implement Hybrid Ranker
- [x] Write tests for ranking algorithm
- [x] Write tests for weight configuration
- [x] Create `maestro/leindex/search/hybrid_ranker.py`
- [x] Implement semantic + BM25 + recency scoring
- [x] Add configurable weight parameters
- [x] Implement cross-project ranking

---

## Phase 6: MCP Server

### [x] Task 4.6: Implement MCP Server
- [x] Write tests for MCP protocol compliance
- [x] Write tests for search tool endpoint
- [x] Write tests for analysis tool endpoint
- [x] Create `maestro/leindex/mcp_server.py`
- [x] Implement tool definitions
- [x] Implement request handling
- [x] Add to Maestro's MCP configuration

---

## Phase 7: Daemon Mode

### [x] Task 4.7: Implement Daemon Mode
- [x] Write tests for daemon lifecycle
- [x] Write tests for file watcher
- [x] Write tests for incremental updates
- [x] Create `maestro/leindex/daemon.py`
- [x] Implement background indexing
- [x] Implement file watcher integration
- [x] Add graceful shutdown handling

---

## Phase 8: Memory Integration

### [ ] Task 4.8: Integrate with Maestro Memory System
- [ ] Write tests for memory storage
- [ ] Write tests for entity retrieval
- [ ] Create `maestro/leindex/memory_integration.py`
- [ ] Connect to existing memory infrastructure
- [ ] Implement code entity storage as Memory objects
- [ ] Add session context for temporal analysis

---

## Phase 9: Verification

### [ ] Task 4.9: Maestro - User Manual Verification 'Sub-Track 04' (Protocol in workflow.md)
