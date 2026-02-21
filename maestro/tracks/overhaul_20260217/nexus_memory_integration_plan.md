# Nexus Memory System Integration Plan

## Overview

Integrate the Nexus Memory System's advanced features into Maestro, creating a unified memory architecture with:
- Hot Cache for semantic detection during agent loop
- Vector DB storage (Turso + HNSW) separate from index DBs
- High-performance memory compression
- Token-efficient retrieval (like LeIndex phase analysis)
- Subtle flash of relevant memories as suggestions

## Architecture Analysis

### Current Maestro Memory Infrastructure

1. **TursoStorageBackend** (`maestro/leindex/rust/src/memory/turso_backend.rs`)
   - libSQL-based storage (local + remote Turso support)
   - FTS5 full-text search
   - Tables: sessions, projects, memories, tracks, mcp_servers

2. **LeIndexProvider** (`crates/core/src/memory/leindex_provider.rs`)
   - Hybrid retrieval (Tantivy + vector similarity)
   - Graph-aware semantic signals
   - In-memory vector store

3. **TantivyMemory** (`crates/core/src/memory/tantivy.rs`)
   - Tantivy full-text search with BM25
   - Optional vector similarity
   - HybridRanker for score fusion

### Nexus Memory System Features

1. **VectorDatabase** (`nexus-vectors/src/database.rs`)
   - 384-dimensional embeddings (all-MiniLM-L6-v2)
   - Cosine similarity search
   - <10ms latency target for 1k vectors
   - Category and namespace filtering

2. **GraphTree** (`nexus-vectors/src/graph.rs`)
   - Hierarchical memory organization
   - Priority-based score boosting (High: 1.5, Medium: 1.2, Low: 1.0)
   - Ancestor weight aggregation
   - Depth-based scoring adjustments

3. **Hooks System** (`nexus-hooks/src/`)
   - Multi-layer extraction (Native, Monitor, Inactivity, Buffer)
   - Agent-agnostic session detection
   - Crash recovery from persistent buffer

## Integration Components

### Phase 1: Vector Store Integration

Create `crates/core/src/memory/nexus_store.rs`:

```rust
//! Nexus Vector Store - HNSW-backed vector storage for Maestro
//!
//! Integrates Nexus Memory System's vector database with Turso storage:
//! - 384-dimensional embeddings (compatible with all-MiniLM-L6-v2)
//! - HNSW indexing for fast approximate nearest neighbor search
//! - Graph tree for hierarchical relevance boosting
//! - Separate storage from index DBs for performance isolation
```

Key features:
- [ ] Implement `VectorStore` trait using HNSW
- [ ] Add graph tree for priority-based boosting
- [ ] Create async API for concurrent access
- [ ] Persist vectors to separate Turso database

### Phase 2: Hot Cache Implementation

Create `crates/core/src/memory/hot_cache.rs`:

```rust
//! Hot Cache - Semantic detection during agent loop
//!
//! Provides real-time memory suggestions during agent execution:
//! - Semantic similarity detection on agent output
//! - LRU cache with configurable TTL
//! - Background embedding computation
//! - Subtle suggestion flashing to UI
```

Key features:
- [ ] Implement semantic detector for agent output
- [ ] Create LRU cache with background refresh
- [ ] Add suggestion broadcast channel
- [ ] Integrate with Cockpit UI for subtle flashing

### Phase 3: Memory Compression

Create `crates/core/src/memory/compression.rs`:

```rust
//! Memory Compression - Token-efficient memory representation
//!
//! Provides high-performance compression for memory storage:
//! - Semantic summarization
//! - Key concept extraction
//! - Redundancy elimination
//! - Token budget management
```

Key features:
- [ ] Implement semantic compression algorithm
- [ ] Add key concept extraction
- [ ] Create token budget tracker
- [ ] Integrate with LeIndex phase analysis

### Phase 4: Embedding Service

Create `crates/core/src/memory/embedding.rs`:

```rust
//! Embedding Service - Vector embedding generation
//!
//! Provides embedding generation for memory content:
//! - Local ONNX Runtime (all-MiniLM-L6-v2)
//! - Batch embedding for efficiency
//! - Embedding caching
//! - GPU acceleration support (optional)
```

Key features:
- [ ] Integrate ONNX Runtime for local embeddings
- [ ] Add batch embedding API
- [ ] Create embedding cache
- [ ] Support optional GPU acceleration

### Phase 5: UI Integration

Modify Cockpit to display memory suggestions:

```rust
//! Memory Suggestion Panel - Subtle UI for memory hints
//!
//! Integrates with Cockpit TUI:
//! - Subtle flash animation for new suggestions
//! - Non-intrusive display
//! - Quick access to full memory
//! - Keyboard shortcuts for suggestion actions
```

## File Structure

```
crates/core/src/memory/
├── mod.rs              # Module exports
├── hybrid.rs           # Hybrid ranking (existing)
├── leindex_provider.rs # LeIndex provider (existing)
├── tantivy.rs          # Tantivy backend (existing)
├── nexus_store.rs      # NEW: Vector store with HNSW
├── hot_cache.rs        # NEW: Hot cache for suggestions
├── compression.rs      # NEW: Memory compression
├── embedding.rs        # NEW: Embedding service
└── types.rs            # NEW: Shared types

maestro/leindex/rust/src/memory/
├── turso_backend.rs    # Existing Turso storage
├── vector_db.rs        # NEW: Vector database schema
└── migrations/
    └── 006_vectors.sql # NEW: Vector storage migration
```

## Database Schema

### Vector Storage (Separate DB)

```sql
-- vectors.db (separate from maestro.db)
CREATE TABLE vector_embeddings (
    id INTEGER PRIMARY KEY,
    memory_id INTEGER NOT NULL,
    namespace_id INTEGER NOT NULL,
    embedding BLOB NOT NULL,  -- 384 floats as bytes
    embedding_model TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

CREATE INDEX idx_vectors_namespace ON vector_embeddings(namespace_id);
CREATE INDEX idx_vectors_memory ON vector_embeddings(memory_id);
```

### Memory Metadata Extensions

```sql
-- Add to existing memories table
ALTER TABLE memories ADD COLUMN embedding_status TEXT DEFAULT 'none';
ALTER TABLE memories ADD COLUMN embedding_model TEXT;
ALTER TABLE memories ADD COLUMN priority INTEGER DEFAULT 3;
ALTER TABLE memories ADD COLUMN compressed_content TEXT;
ALTER TABLE memories ADD COLUMN token_count INTEGER;
```

## API Design

### Hot Cache API

```rust
pub struct HotCache {
    /// Maximum cache entries
    capacity: usize,
    /// TTL in seconds
    ttl_secs: u64,
    /// Background embedding worker
    embedding_tx: mpsc::Sender<EmbeddingRequest>,
    /// Suggestion broadcast
    suggestion_tx: broadcast::Sender<MemorySuggestion>,
}

impl HotCache {
    /// Process agent output for semantic detection
    pub async fn process_output(&self, output: &str) -> Result<Vec<MemorySuggestion>>;

    /// Get current suggestions (non-blocking)
    pub fn get_suggestions(&self) -> Vec<MemorySuggestion>;

    /// Subscribe to suggestion updates
    pub fn subscribe(&self) -> broadcast::Receiver<MemorySuggestion>;

    /// Invalidate stale entries
    pub async fn invalidate_stale(&self);
}

pub struct MemorySuggestion {
    pub memory_id: i64,
    pub content_preview: String,
    pub relevance_score: f32,
    pub category: String,
    pub flash_intensity: f32, // 0.0-1.0 for UI animation
}
```

### Vector Store API

```rust
pub struct NexusVectorStore {
    /// HNSW index for fast ANN search
    index: RwLock<HnswIndex>,
    /// Graph tree for relevance boosting
    tree: RwLock<GraphTree>,
    /// Turso backend for persistence
    db: TursoStorageBackend,
    /// Embedding dimension (384)
    dimension: usize,
}

impl NexusVectorStore {
    /// Store embedding with metadata
    pub async fn store_embedding(
        &self,
        memory_id: i64,
        embedding: &[f32],
        metadata: EmbeddingMetadata,
    ) -> Result<()>;

    /// Search for similar memories
    pub async fn search_similar(
        &self,
        query: &[f32],
        namespace_id: i64,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<VectorSearchResult>>;

    /// Get boosted results using graph tree
    pub fn apply_graph_boost(
        &self,
        results: &mut [VectorSearchResult],
    );
}
```

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Vector search latency | <10ms | For 1k vectors |
| Hot cache hit rate | >80% | For frequently accessed memories |
| Suggestion flash delay | <50ms | From detection to UI update |
| Embedding generation | <5ms | Per 512-token chunk (local ONNX) |
| Memory compression ratio | >50% | Token reduction |
| Background refresh rate | 100/s | Embedding computations |

## Implementation Order

1. **Phase 1: Types and Traits** (2 days)
   - Create shared types in `types.rs`
   - Define trait interfaces
   - Add database migrations

2. **Phase 2: Embedding Service** (2 days)
   - Integrate ONNX Runtime
   - Implement batch embedding
   - Add caching layer

3. **Phase 3: Vector Store** (3 days)
   - Implement HNSW index
   - Add graph tree integration
   - Create persistence layer

4. **Phase 4: Hot Cache** (3 days)
   - Implement semantic detector
   - Create LRU cache with TTL
   - Add suggestion broadcasting

5. **Phase 5: Memory Compression** (2 days)
   - Implement compression algorithm
   - Add token budget tracking
   - Integrate with phase analysis

6. **Phase 6: UI Integration** (2 days)
   - Add suggestion panel to Cockpit
   - Implement subtle flash animation
   - Add keyboard shortcuts

## Testing Strategy

### Unit Tests
- Vector store CRUD operations
- HNSW search accuracy
- Graph tree boosting correctness
- Hot cache TTL behavior
- Compression ratio verification

### Integration Tests
- End-to-end memory flow
- Concurrent access patterns
- UI suggestion display
- Background embedding refresh

### Performance Tests
- Vector search latency under load
- Memory usage with large caches
- Embedding throughput benchmarks

## Dependencies

```toml
# crates/core/Cargo.toml additions
[dependencies]
# HNSW implementation
hnsw = "0.11"
# ONNX Runtime for embeddings
ort = "2.0"
# Additional utilities
lru = "0.12"
parking_lot = "0.12"
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| ONNX Runtime binary size | Use feature flag, document optional dependency |
| HNSW memory usage | Implement configurable index size limits |
| Embedding latency | Use background precomputation, cache aggressively |
| UI flickering | Use smooth animations, respect reduced motion settings |

## Success Criteria

1. Vector search returns results in <10ms for 1k vectors
2. Hot cache hit rate exceeds 80% in production use
3. Memory suggestions appear within 50ms of detection
4. Compression reduces token usage by >50%
5. Zero crashes or deadlocks under concurrent load
6. All tests pass with >90% code coverage

## Timeline

- **Week 1**: Types, traits, and database schema
- **Week 2**: Embedding service and vector store
- **Week 3**: Hot cache and compression
- **Week 4**: UI integration and testing

Total: 4 weeks for complete integration
