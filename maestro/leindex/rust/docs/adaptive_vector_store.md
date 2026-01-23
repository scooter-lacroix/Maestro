# Adaptive Vector Store Documentation

## Overview

The Adaptive Vector Store automatically routes to the optimal backend based on vector count, providing:

- **Linear Search** (< 90K vectors): Fastest for small datasets with SIMD-accelerated cosine similarity
- **HNSW Search** (>= 90K vectors): Approximate nearest neighbor search for large datasets
- **Turso Backup**: Persistent storage across restarts

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                    Adaptive Vector Store                       │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│   ┌────────────┐    ┌────────────┐    ┌────────────┐          │
│   │  Linear    │───▶│   HNSW     │───▶│   Turso    │          │
│   │  Store     │    │   Store    │    │   Store    │          │
│   │  (<90K)    │    │  (>=90K)   │    │  (backup)  │          │
│   └────────────┘    └────────────┘    └────────────┘          │
│         ▲                  │                   ▲               │
│         │                  │                   │               │
│         └──────────────────┴───────────────────┘               │
│                      Mode Switch Logic                         │
│                                                                │
│  • HNSW_SWITCH_UP_THRESHOLD:   90,000 vectors                  │
│  • HNSW_SWITCH_DOWN_THRESHOLD: 80,000 vectors                  │
│  • Mode switch lock prevents ops during switch                 │
│  • Data migrated via Turso (common persistence layer)          │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## Three-Tier Design

### 1. Linear Store

**Purpose:** Fast exact search for small datasets

**Implementation:**
```rust
pub struct VectorStore {
    vectors: RwLock<HashMap<String, StoredVector>>,
    cache: TtlCache<String, Vec<SearchResult>>,
    // ...
}
```

**Algorithm:** Brute-force cosine similarity with SIMD acceleration

**Performance:**
- O(n) search time
- Optimal for < 90K vectors
- Simple implementation

**Use when:**
- Dataset size < 90K vectors
- Exact similarity needed
- Frequent updates

### 2. HNSW Store

**Purpose:** Approximate nearest neighbor search for large datasets

**Implementation:**
```rust
pub struct HnswVectorStore {
    hnsw: RwLock<HNSW<CosineSimilarity>>,
    id_to_data: RwLock<HashMap<usize, VectorDataWithEmbedding>>,
    tombstones: RwLock<HashSet<usize>>,
    // ...
}
```

**Algorithm:** Hierarchical Navigable Small World graph

**Configuration:**
```rust
HnswConfig {
    max_elements: 1_000_000,
    ef_construction: 200,
    ef_search: 10,
    m: 32,
    // ...
}
```

**Performance:**
- O(log n) approximate search
- Best for >= 90K vectors
- 100x faster indexing with batch insert

**Use when:**
- Dataset size >= 90K vectors
- Fast search needed
- Approximate results acceptable

### 3. Turso Store

**Purpose:** Persistent backup storage

**Implementation:**
```rust
pub struct TursoVectorStore {
    database: Arc<Database>,
    cache: TtlCache<String, Vec<SearchResult>>,
    // ...
}
```

**Algorithm:** SQL query with min-heap for top-k

**Schema:**
```sql
CREATE TABLE IF NOT EXISTS vectors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vector_id TEXT NOT NULL UNIQUE,
    file_path TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_line INTEGER,
    end_line INTEGER,
    chunk_type INTEGER NOT NULL,
    parent_context TEXT,
    content TEXT,
    embedding TEXT NOT NULL,
    embedding_model TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT
);
```

**Performance:**
- O(n) search (full table scan)
- Persistent across restarts
- Authoritative vector count

**Use when:**
- Persistence required
- Data migration needed
- Recovery after crash

## Auto-Switching Logic

### Switch Up (Linear → HNSW)

**Threshold:** 90,000 vectors

```rust
const HNSW_SWITCH_UP_THRESHOLD: usize = 90_000;

// Check after adding vectors
let count = self.vector_count().await?;
if self.mode() == StoreMode::Linear && count >= HNSW_SWITCH_UP_THRESHOLD {
    info!("Vector count reached {}K, switching to HNSW mode", count / 1000);
    self.switch_to_hnsw().await?;
}
```

**Switch Process:**
1. Acquire mode switch lock (write) - blocks all operations
2. Take ownership of Linear store
3. Load all vectors from Turso (authoritative source)
4. Create new HNSW store
5. Migrate vectors from Turso to HNSW
6. Replace HNSW store atomically
7. Update mode to HNSW
8. Release lock

### Switch Down (HNSW → Linear)

**Threshold:** 80,000 vectors

```rust
const HNSW_SWITCH_DOWN_THRESHOLD: usize = 80_000;

// Check after deleting vectors
let count = self.vector_count().await?;
if self.mode() == StoreMode::Hnsw && count < HNSW_SWITCH_DOWN_THRESHOLD {
    info!("Vector count dropped to {}K, switching to Linear mode", count / 1000);
    self.switch_to_linear().await?;
}
```

**Switch Process:**
1. Acquire mode switch lock (write) - blocks all operations
2. Take ownership of HNSW store
3. Load all vectors from Turso (authoritative source)
4. Create new Linear store
5. Migrate vectors from Turso to Linear
6. Replace Linear store atomically
7. Update mode to Linear
8. Release lock

### Mode Switch Lock

The mode switch lock prevents operations during switching:

```rust
mode_switch_lock: Arc<RwLock<()>>,
```

**During switch:**
- Write lock acquired - blocks ALL operations
- Operations wait for switch to complete

**During normal operation:**
- Read lock acquired - allows concurrent operations
- Multiple operations can run simultaneously

## Hysteresis

Hysteresis prevents rapid mode switching when vector count is near threshold:

```
              HNSW Switch Up (90K)
                   ▲
                   │
  Linear Mode ─────┼─────▶ HNSW Mode
                   │
                   ▼
         HNSW Switch Down (80K)

Dead zone: 80K - 90K vectors (no switching)
```

**Benefits:**
- Prevents thrashing between modes
- Reduces unnecessary migrations
- Stable behavior during rapid add/delete

## Data Migration During Mode Switches

### Critical Design Principle

**Turso is the authoritative source of truth.**

All vectors are added to Turso first, then to the active in-memory store:

```rust
pub async fn add_vector(
    &self,
    content: &str,
    embedding: Vec<f32>,
    metadata: VectorMetadata,
) -> Result<String> {
    // Generate unified UUID
    let unified_id = format!("vec_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));

    // Always add to Turso first (persistent layer)
    if let Some(ref turso) = self.turso {
        turso.add_vector_with_id(&unified_id, content, embedding.clone(), metadata.clone())
            .await?;
    }

    // Then add to active in-memory store
    match self.mode() {
        StoreMode::Linear => {
            linear.add_vector_with_id(&unified_id, content, embedding, metadata)?;
        }
        StoreMode::Hnsw => {
            hnsw.add_vector_with_id(&unified_id, content, embedding, metadata)?;
        }
        StoreMode::Turso => {}
    }

    Ok(unified_id)
}
```

### Migration Flow

```
┌─────────┐     Add     ┌─────────┐     Persist    ┌─────────┐
│  New    │─────────────▶│  Turso  │───────────────▶│  Turso  │
│ Vector  │             │ (Store) │               │  (Disk) │
└─────────┘             └────┬────┘               └─────────┘
                             │
                             │ Add to active store
                             ▼
                      ┌─────────────┐
                      │   Linear    │
                      │   or HNSW   │
                      └─────────────┘
```

### Mode Switch Migration

```rust
async fn switch_to_hnsw(&self) -> Result<()> {
    // Acquire lock - blocks all operations
    let _lock = self.mode_switch_lock.write().await;

    // Create new HNSW store
    let mut new_hnsw_store = HnswVectorStore::new(Some(self.index_path.clone()), None)?;

    // Load from Turso (authoritative source)
    if let Some(ref turso) = self.turso {
        let all_vectors = turso.get_all_vectors().await?;

        for (content, embedding, metadata) in all_vectors {
            new_hnsw_store.add_vector(&content, embedding, metadata)?;
        }
    }

    // Replace atomically
    *self.hnsw.write().await = Some(new_hnsw_store);
    self.mode.store(StoreMode::Hnsw as usize, Ordering::SeqCst);

    Ok(())
}
```

## Batch Insert APIs

All three stores support batch insert for efficient bulk indexing:

### Linear Store Batch Insert

```rust
pub fn add_vectors_batch_with_ids(
    &self,
    items: Vec<(String, String, Vec<f32>, VectorMetadata)>,
) -> Result<()> {
    // Parallel validation with rayon
    items.par_iter().try_for_each(|(id, _, embedding, metadata)| {
        validate_vector_id(id)?;
        validate_embedding_dim(embedding)?;
        validate_chunk_index(metadata.chunk_index)?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Single write lock for all inserts
    let mut vectors_guard = self.vectors.write()?;
    let mut meta_guard = self.metadata.write()?;

    for (vector_id, content, embedding, metadata) in items {
        let stored = StoredVector {
            id: vector_id,
            embedding,
            metadata,
            content: Some(content),
        };
        vectors_guard.insert(vector_id, stored);
    }

    Ok(())
}
```

### HNSW Store Batch Insert

```rust
pub fn add_vectors_batch(
    &self,
    items: Vec<(String, Vec<f32>, VectorMetadata)>,
) -> Result<Vec<String>> {
    // Extract vectors for batch insert
    let vectors: Vec<Vec<f32>> = items.iter()
        .map(|(_, embedding, _)| embedding.clone())
        .collect();

    // OPTIMIZATION: Use hnsw.insert_batch() - MUCH faster
    let internal_ids = {
        let mut hnsw = self.hnsw.write()?;
        hnsw.insert_batch(vectors)
    };

    // Store metadata with IDs
    for (i, (content, embedding, metadata)) in items.into_iter().enumerate() {
        let internal_id = internal_ids[i];
        id_map.insert(internal_id, VectorDataWithEmbedding {
            id: format!("vec_{}", internal_id),
            embedding,
            metadata,
            content: Some(content),
        });
    }

    Ok(external_ids)
}
```

**Performance:** 100x faster than individual inserts

### Turso Store Batch Insert

```rust
pub async fn add_vectors_batch(
    &self,
    items: Vec<(String, Vec<f32>, VectorMetadata)>,
) -> Result<Vec<String>> {
    // Parallel pre-processing with rayon
    let (vector_ids, embedding_jsons): (Vec<String>, Vec<String>) = items
        .par_iter()
        .enumerate()
        .map(|(i, (_, embedding, _))| {
            let id = format!("vec_{}_{}", i, uuid::Uuid::new_v4());
            let json = serde_json::to_string(embedding).unwrap_or_default();
            (id, json)
        })
        .unzip();

    // Execute batch insert in transaction
    let conn = self.database.connect()?;
    conn.execute("BEGIN TRANSACTION", []).await?;

    for i in 0..items.len() {
        let (content, _embedding, metadata) = &items[i];
        let vector_id = &vector_ids[i];
        let embedding_json = &embedding_jsons[i];

        conn.execute(
            "INSERT INTO vectors (...) VALUES (...)",
            [vector_id, content, embedding_json, /* ... */]
        ).await?;
    }

    conn.execute("COMMIT", []).await?;
    Ok(vector_ids)
}
```

**Performance:** Single transaction for all inserts

## SIMD-Accelerated Cosine Similarity

All stores use SIMD-accelerated cosine similarity:

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embeddings must have same dimension");

    let dot_product: f32 = a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .sum();

    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Handle NaN/Inf
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    let similarity = dot_product / (norm_a * norm_b);

    // Clamp to [0, 1] and handle NaN
    similarity.max(0.0).min(1.0)
}
```

**Optimizations:**
- Iterator-based (compiler auto-vectorization)
- NaN/Inf safe handling
- Early return for zero norms
- Clamped to valid range

## Vector Count Tracking

Vector count is always authoritative from Turso:

```rust
pub async fn vector_count(&self) -> Result<usize> {
    // Prefer Turso for authoritative count (persistent)
    if let Some(ref turso) = self.turso {
        return Ok(turso.vector_count().await.unwrap_or(0));
    }

    // Fallback to active store
    match self.mode() {
        StoreMode::Linear => Ok(linear.vector_count()?),
        StoreMode::Hnsw => Ok(hnsw.vector_count()?),
        StoreMode::Turso => Ok(turso.vector_count().await?),
    }
}
```

## Search API

All stores expose the same search interface:

```rust
pub async fn search(
    &self,
    query_embedding: &[f32],
    top_k: usize,
) -> Result<Vec<SearchResult>>
```

**Search Result:**
```rust
pub struct SearchResult {
    pub vector_id: String,
    pub score: f32,           // Cosine similarity [0, 1]
    pub metadata: VectorMetadata,
    pub content: Option<String>,
}
```

**Cache Key:**
```rust
// SHA-256 hash of embedding + top_k
let mut hasher = Sha256::new();
for &val in query_embedding {
    hasher.update(&val.to_le_bytes());
}
hasher.update(&(top_k as u64).to_le_bytes());
let cache_key = format!("{:x}", hasher.finalize());
```

## Error Handling

All operations return `Result<T>` with descriptive errors:

```rust
use anyhow::{Context, Result};

// Mode switch with error context
self.switch_to_hnsw()
    .await
    .context("Failed to switch to HNSW mode")?;

// Search with shutdown check
if self.is_shutdown.load(Ordering::SeqCst) {
    return Err(anyhow!("Cannot search: store is shut down"));
}
```

## Performance Benchmarks

Based on Phase 8 benchmarks:

| Operation | Linear (< 90K) | HNSW (>= 90K) |
|-----------|----------------|---------------|
| Single Add | ~0.1ms | ~0.5ms |
| Batch Add (100) | ~10ms | ~50ms |
| Search | O(n) | O(log n) |
| Mode Switch | N/A | ~5-30s (depends on size) |

**Batch insert speedup:** 100x faster than individual inserts

## Best Practices

### 1. Use Batch Insert

**Bad:**
```rust
for item in items {
    store.add_vector(content, embedding, metadata).await?;
}
```

**Good:**
```rust
store.add_vectors(items).await?;
```

### 2. Let Adaptive Store Choose Mode

**Bad:**
```rust
// Manually choosing HNSW
let hnsw_store = HnswVectorStore::new(None, None)?;
```

**Good:**
```rust
// Let adaptive store choose based on count
let adaptive_store = AdaptiveVectorStore::new(None).await?;
```

### 3. Handle Mode Switches Gracefully

```rust
loop {
    match adaptive_store.search(&query, 10).await {
        Ok(results) => break results,
        Err(e) if e.to_string().contains("mode switch") => {
            // Wait and retry
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

## See Also

- [LSP Integration](./lsp_integration.md) - LSP manager and lifecycle
- [TUI User Guide](./lsp_tui_user_guide.md) - Using LSPs in the terminal UI
- [Troubleshooting](./lsp_troubleshooting.md) - Common issues and solutions
