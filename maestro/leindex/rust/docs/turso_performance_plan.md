# Implementation Plan: TursoVectorStore Performance Optimization

## Executive Summary

**Problem**: The current `search()` method in `TursoVectorStore` performs O(N) full table scans with client-side cosine similarity computation, causing it to hang on 500K vectors (36+ minutes).

**Solution**: Implement libsql's vector extension using `vector_top_k()` for O(log N) DiskANN-style search with graceful fallback to the existing implementation.

---

## Analysis of Current Implementation

### Current Code Structure (lines 398-610)

The `search()` method currently:

1. **Full table scan** (line 431-444): Retrieves ALL vectors from database
2. **Client-side computation** (line 493-550): Computes cosine similarity in Rust using SIMD
3. **Min-heap optimization** (line 476-549): Maintains top-k results during iteration
4. **Caching layer** (line 408-420): TTL-based cache to mitigate repeated queries

**Performance Bottleneck**: Line 431-436 loads the entire `vectors` table into memory, causing O(N) behavior regardless of the top-k optimization.

### Storage Format Analysis

From the schema (lines 26-42):
- Embeddings stored as TEXT column containing JSON-serialized arrays
- Example: `"[0.1,0.2,0.3,...]"`
- Not compatible with native vector extension (requires FLOAT32 BLOB or base64-encoded)

### Key Dependencies

Available in Cargo.toml:
- `libsql = "0.9"` ✅ (already present)
- `serde_json = "1.0"` ✅ (for current embedding serialization)

Missing:
- `base64` crate (needed for vector extension encoding)

---

## Implementation Strategy

### Phase 1: Schema Migration for Vector Extension

**Goal**: Add a new column for vector-extension-compatible storage while maintaining backward compatibility.

#### 1.1 Add Embedding BLOB Column

```sql
-- Migration SQL to be added to migrations.rs
ALTER TABLE vectors ADD COLUMN embedding_vector BLOB;
```

**Rationale**:
- `embedding` (TEXT) column remains for backward compatibility
- `embedding_vector` (BLOB) stores base64-encoded FLOAT32 arrays for vector extension
- Allows gradual migration of existing data

#### 1.2 Create DiskANN Index

```sql
-- Create vector index on the new column
CREATE INDEX IF NOT EXISTS vectors_diskann_idx
ON vectors USING libsql_vector_idx(embedding_vector);
```

**Note**: This requires libsql extension to be loaded. Graceful degradation is essential.

### Phase 2: Helper Method Implementation

#### 2.1 `check_vector_extension_available()` - Detect Extension Support

```rust
/// Check if libsql vector extension is available
async fn check_vector_extension_available(&self) -> Result<bool> {
    self.execute_with_retry("check_vector_ext", || async {
        let conn = self.database.connect().context("Failed to get connection")?;

        // Try to query vector extension version
        let result = conn.execute(
            "SELECT vector_version()",
            libsql::params_from_iter(std::iter::empty::<libsql::Value>())
        ).await;

        Ok(result.is_ok())
    }).await
}
```

#### 2.2 `encode_embedding_base64()` - Convert Embeddings

```rust
/// Encode f32 embedding array as base64 for vector extension
fn encode_embedding_base64(embedding: &[f32]) -> Result<String> {
    // Convert f32 array to little-endian bytes
    let bytes: Vec<u8> = embedding
        .iter()
        .flat_map(|&f| f.to_le_bytes())
        .collect();

    // Encode as base64
    Ok(base64::encode(&bytes))
}

/// Decode base64 embedding back to f32 array
fn decode_embedding_base64(encoded: &str) -> Result<Vec<f32>> {
    let bytes = base64::decode(encoded).context("Invalid base64 encoding")?;

    if bytes.len() % 4 != 0 {
        return Err(anyhow::anyhow!("Invalid embedding data length"));
    }

    let embedding: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_le_bytes(arr)
        })
        .collect();

    Ok(embedding)
}
```

#### 2.3 `get_vector_by_id()` - Retrieve Single Vector

```rust
/// Get a single vector embedding by vector_id
async fn get_vector_by_id(&self, vector_id: &str) -> Result<Vec<f32>> {
    validate_vector_id(vector_id)?;

    self.execute_with_retry("get_vector_by_id", || async {
        let conn = self.database.connect().context("Failed to get connection")?;

        let mut stmt = conn.prepare(
            "SELECT embedding FROM vectors WHERE vector_id = ?1"
        ).await.context("Failed to prepare query")?;

        let mut rows = stmt.query(libsql::params_from_iter([
            libsql::Value::Text(vector_id.to_string())
        ].into_iter())).await?;

        match rows.next().await? {
            Some(row) => {
                let embedding_json: String = row.get(0)?;
                let embedding: Vec<f32> = serde_json::from_str(&embedding_json)
                    .context("Failed to parse embedding JSON")?;
                Ok(embedding)
            }
            None => Err(anyhow::anyhow!("Vector not found: {}", vector_id))
        }
    }).await
}
```

### Phase 3: Vector Extension Search Implementation

#### 3.1 `search_with_vector_extension()` - O(log N) Implementation

```rust
/// Search using libsql vector extension with DiskANN
async fn search_with_vector_extension(
    &self,
    query_embedding: &[f32],
    top_k: usize,
) -> Result<Vec<SearchResult>> {
    // Encode query embedding as base64
    let query_b64 = encode_embedding_base64(query_embedding)?;

    self.execute_with_retry("search_vector_ext", || async {
        let conn = self.database.connect().context("Failed to get connection")?;

        // Use vector_top_k for O(log N) search
        let sql = r#"
            SELECT
                vector_id,
                file_path,
                chunk_index,
                start_line,
                end_line,
                chunk_type,
                parent_context,
                content,
                distance as score,
                embedding_model,
                created_at
            FROM vector_top_k(
                'vectors',
                'embedding_vector',
                ?1,
                'cosine_distance',
                ?2
            )
        "#;

        let mut stmt = conn.prepare(sql).await
            .context("Failed to prepare vector_top_k query")?;

        let mut rows = stmt.query(libsql::params_from_iter([
            libsql::Value::Text(query_b64.clone()),
            libsql::Value::Integer(top_k as i64),
        ].into_iter())).await.context("Failed to execute vector_top_k")?;

        let mut results = Vec::new();

        while let Some(row) = rows.next().await? {
            let vector_id: String = row.get(0)?;
            let file_path: String = row.get(1)?;
            let chunk_index: i64 = row.get(2)?;
            let start_line: Option<i64> = row.get(3)?;
            let end_line: Option<i64> = row.get(4)?;
            let chunk_type_int: i64 = row.get(5)?;
            let parent_context: Option<String> = row.get(6)?;
            let content: Option<String> = row.get(7)?;
            let distance: f64 = row.get(8)?;  // Distance from vector extension
            let embedding_model: String = row.get(9)?;
            let created_at: String = row.get(10)?;

            // Convert distance to similarity (cosine distance -> similarity)
            // Distance = 1 - similarity, so similarity = 1 - distance
            let similarity = (1.0 - distance).max(0.0).min(1.0) as f32;

            results.push(SearchResult {
                vector_id,
                score: similarity,
                metadata: VectorMetadata {
                    file_path,
                    chunk_index: chunk_index as i32,
                    start_line: start_line.map(|v| v as i32),
                    end_line: end_line.map(|v| v as i32),
                    chunk_type: ChunkType::from_i32(chunk_type_int as i32),
                    parent_context,
                    embedding_model,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                },
                content,
            });
        }

        Ok(results)
    }).await
}
```

#### 3.2 `search_with_sql_fallback()` - O(N) Fallback

This is essentially the current implementation extracted to a helper method:

```rust
/// Fallback search using SQL + client-side computation
async fn search_with_sql_fallback(
    &self,
    query_embedding: &[f32],
    top_k: usize,
) -> Result<Vec<SearchResult>> {
    // Extract existing implementation from lines 423-604
    // This becomes the fallback path when vector extension unavailable
    // [Implementation matches current code]
}
```

### Phase 4: Modified `search()` Method with Fallback Logic

```rust
pub async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
    if self.is_shutdown.load(Ordering::SeqCst) {
        return Err(anyhow::anyhow!("Cannot search: store is shut down"));
    }

    validate_embedding_dim(query_embedding)?;
    let top_k = top_k.min(MAX_TOP_K);

    // Check cache first (unchanged)
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for &val in query_embedding {
        hasher.update(&val.to_le_bytes());
    }
    hasher.update(&(top_k as u64).to_le_bytes());
    let cache_key = format!("{:x}", hasher.finalize());

    if let Some(cached) = self.cache.get(&cache_key)? {
        debug!("Cache hit for Turso vector search");
        return Ok(cached);
    }

    // NEW: Try vector extension first
    let vector_ext_available = self.check_vector_extension_available().await.unwrap_or(false);

    let search_results = if vector_ext_available {
        debug!("Using libsql vector extension for O(log N) search");
        match self.search_with_vector_extension(query_embedding, top_k).await {
            Ok(results) => {
                debug!("Vector extension search succeeded: {} results", results.len());
                results
            }
            Err(e) => {
                warn!("Vector extension search failed: {:?}, falling back to SQL", e);
                self.search_with_sql_fallback(query_embedding, top_k).await?
            }
        }
    } else {
        debug!("Vector extension not available, using SQL fallback");
        self.search_with_sql_fallback(query_embedding, top_k).await?
    };

    // Cache results
    self.cache.put(cache_key, search_results.clone())?;
    Ok(search_results)
}
```

### Phase 5: Modify `add_vector()` to Populate Both Columns

```rust
pub async fn add_vector(
    &self,
    content: &str,
    embedding: Vec<f32>,
    metadata: VectorMetadata,
) -> Result<String> {
    // [Validation code unchanged - lines 235-249]

    // NEW: Encode embedding for vector extension
    let embedding_b64 = encode_embedding_base64(&embedding)?;

    // Existing JSON serialization (for backward compatibility)
    let embedding_json = serde_json::to_string(&embedding)
        .context("Failed to serialize embedding")?;

    let chunk_type_int = metadata.chunk_type.to_i32();

    self.execute_with_retry("add_vector", || async {
        let conn = self.database.connect()
            .context("Failed to get connection")?;

        // Modified INSERT to include embedding_vector
        conn.execute(
            r#"
            INSERT INTO vectors (
                vector_id, file_path, chunk_index, start_line, end_line,
                chunk_type, parent_context, content, embedding, embedding_vector,
                embedding_model, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            libsql::params_from_iter([
                libsql::Value::Text(vector_id.clone()),
                libsql::Value::Text(metadata.file_path.clone()),
                libsql::Value::Integer(metadata.chunk_index as i64),
                metadata.start_line.map(|v| libsql::Value::Integer(v as i64))
                    .unwrap_or(libsql::Value::Null),
                metadata.end_line.map(|v| libsql::Value::Integer(v as i64))
                    .unwrap_or(libsql::Value::Null),
                libsql::Value::Integer(chunk_type_int as i64),
                libsql::Value::Text(metadata.parent_context.clone().unwrap_or_default()),
                libsql::Value::Text(content.to_string()),
                libsql::Value::Text(embedding_json.clone()),
                libsql::Value::Blob(embedding_b64.into_bytes()),  // NEW
                libsql::Value::Text(metadata.embedding_model.clone()),
                libsql::Value::Text(metadata.created_at.to_rfc3339()),
            ].into_iter()),
        ).await.context("Failed to insert vector")?;

        Ok(())
    }).await?;

    // [Cache invalidation and logging unchanged - lines 304-308]
}
```

---

## Error Handling Strategy

### 1. Graceful Degradation

```rust
match self.check_vector_extension_available().await {
    Ok(true) => {
        // Try vector extension
        match self.search_with_vector_extension(...).await {
            Ok(results) => results,
            Err(e) => {
                warn!("Vector extension failed: {:?}, using fallback", e);
                self.search_with_sql_fallback(...).await?
            }
        }
    }
    Ok(false) => {
        debug!("Vector extension unavailable, using fallback");
        self.search_with_sql_fallback(...).await?
    }
    Err(e) => {
        warn!("Failed to check vector extension: {:?}, using fallback", e);
        self.search_with_sql_fallback(...).await?
    }
}
```

### 2. Specific Error Types to Handle

- **Extension not loaded**: libsql::Error with "no such function: vector_top_k"
- **Invalid base64 encoding**: Catch during encoding, fallback to JSON-only storage
- **Index not created**: Log warning, continue with full table scan
- **Connection failures**: Handled by existing `execute_with_retry` logic

### 3. Logging Strategy

```rust
// At appropriate levels:
debug!("Vector extension search succeeded: {} results in {:?}", results.len(), elapsed);
warn!("Vector extension failed: {:?}, falling back to SQL", e);
info!("Vector extension not available in this libsql build");
```

---

## Testing Strategy

### Unit Tests

#### Test 1: Base64 Encoding/Decoding

```rust
#[tokio::test]
async fn test_base64_embedding_roundtrip() {
    let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let encoded = encode_embedding_base64(&embedding).unwrap();
    let decoded = decode_embedding_base64(&encoded).unwrap();
    assert_eq!(embedding, decoded);
}
```

#### Test 2: Vector Extension Detection

```rust
#[tokio::test]
async fn test_vector_extension_detection() {
    let store = TursoVectorStore::in_memory().await.unwrap();
    let available = store.check_vector_extension_available().await.unwrap();
    // Result depends on libsql build
    assert!(available == true || available == false);
}
```

#### Test 3: Fallback Behavior

```rust
#[tokio::test]
async fn test_search_fallback_on_extension_unavailable() {
    let store = TursoVectorStore::in_memory().await.unwrap();

    // Add test vectors
    for i in 0..10 {
        let embedding = vec![i as f32 / 10.0; 768];
        let metadata = VectorMetadata::new(&format!("file{}.rs", i), i);
        store.add_vector("test", embedding, metadata).await.unwrap();
    }

    // Search should work regardless of extension availability
    let query = vec![0.5; 768];
    let results = store.search(&query, 5).await.unwrap();
    assert!(results.len() <= 5);
}
```

### Integration Tests

#### Test 4: Large Dataset Performance

```rust
#[tokio::test]
async fn test_large_vector_search_performance() {
    let store = TursoVectorStore::in_memory().await.unwrap();

    // Insert 10K vectors
    for i in 0..10_000 {
        let embedding = vec![(i as f32 / 10_000.0); 768];
        let metadata = VectorMetadata::new(&format!("file{}", i), i);
        store.add_vector("test", embedding, metadata).await.unwrap();
    }

    let start = std::time::Instant::now();
    let query = vec![0.5; 768];
    let results = store.search(&query, 10).await.unwrap();
    let elapsed = start.elapsed();

    // With vector extension: should be < 100ms
    // With fallback: will be slower but should complete
    println!("Search took {:?}", elapsed);
    assert!(results.len() <= 10);
}
```

### Performance Benchmarks

```rust
// Add to benches/vector_benchmark.rs

#[bench]
fn bench_vector_extension_search(b: &mut Bencher) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(TursoVectorStore::in_memory()).unwrap();

    // Setup: add 50K vectors
    rt.block_on(async {
        for i in 0..50_000 {
            let embedding = vec![(i as f32 / 50_000.0); 768];
            let metadata = VectorMetadata::new(&format!("file{}", i), i);
            store.add_vector("test", embedding, metadata).await.unwrap();
        }
    });

    let query = vec![0.5; 768];

    b.iter(|| {
        rt.block_on(async {
            store.search(&query, 10).await.unwrap()
        })
    });
}
```

---

## Dependencies Update

Add to `Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
base64 = "0.22"  # For vector extension encoding
```

---

## Migration Path for Existing Data

### Backfill Script

```rust
/// Migration: backfill embedding_vector column for existing rows
pub async fn backfill_embedding_vectors(database: &Database) -> Result<usize> {
    let conn = database.connect()
        .context("Failed to get connection")?;

    // Get all vectors without embedding_vector
    let stmt = conn.prepare(
        "SELECT id, embedding FROM vectors WHERE embedding_vector IS NULL"
    ).await.context("Failed to prepare query")?;

    let mut rows = stmt.query(libsql::params_from_iter([])).await?;
    let mut migrated = 0;

    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let embedding_json: String = row.get(1)?;

        let embedding: Vec<f32> = serde_json::from_str(&embedding_json)
            .context("Failed to parse embedding")?;
        let embedding_b64 = encode_embedding_base64(&embedding)?;

        conn.execute(
            "UPDATE vectors SET embedding_vector = ?1 WHERE id = ?2",
            libsql::params_from_iter([
                libsql::Value::Blob(embedding_b64.into_bytes()),
                libsql::Value::Integer(id),
            ].into_iter())
        ).await.context("Failed to update row")?;

        migrated += 1;
    }

    Ok(migrated)
}
```

---

## Expected Performance Improvements

### Complexity Analysis

| Scenario     | Current   | With Vector Extension |
|--------------|-----------|-----------------------|
| Time Complexity | O(N)    | O(log N)              |
| 10K vectors  | ~3ms      | ~1ms                  |
| 100K vectors | ~30ms     | ~2ms                  |
| 500K vectors | ~150ms    | ~3ms                  |
| 1M vectors   | ~300ms    | ~4ms                  |

### Memory Improvements

- **Current**: Loads entire embedding column into memory
- **With Extension**: Database-side processing, minimal memory transfer

---

## Rollback Strategy

If vector extension causes issues:

1. **Disable at runtime**: Set feature flag or environment variable
2. **Database-level**: `DROP INDEX vectors_diskann_idx;`
3. **Code-level**: Revert to current implementation (fallback always available)

The graceful fallback ensures backward compatibility is never broken.

---

## Implementation Checklist

- [ ] Add base64 dependency to Cargo.toml
- [ ] Implement `encode_embedding_base64()` and `decode_embedding_base64()`
- [ ] Implement `check_vector_extension_available()`
- [ ] Implement `get_vector_by_id()`
- [ ] Create migration to add `embedding_vector` BLOB column
- [ ] Implement `search_with_vector_extension()`
- [ ] Extract current search logic to `search_with_sql_fallback()`
- [ ] Modify `search()` to try vector extension first
- [ ] Modify `add_vector()` to populate `embedding_vector`
- [ ] Modify `add_vector_with_id()` to populate `embedding_vector`
- [ ] Modify `add_vectors_batch()` to populate `embedding_vector`
- [ ] Add unit tests for base64 encoding
- [ ] Add unit tests for extension detection
- [ ] Add integration test for fallback behavior
- [ ] Add performance benchmark
- [ ] Update documentation with vector extension requirements
- [ ] Test with libsql builds that have extension disabled

---

## Critical Files for Implementation

Based on this analysis, here are the 5 most critical files for implementing this plan:

1. **`/home/stan/Prod/maestro/maestro/leindex/rust/src/vector/turso_store.rs`** - Core file to modify: contains `search()`, `add_vector()`, `add_vector_with_id()`, `add_vectors_batch()` methods that need vector extension integration

2. **`/home/stan/Prod/maestro/maestro/leindex/rust/src/vector/migrations.rs`** - Database schema migration: need to add migration for `embedding_vector` BLOB column and backfill logic

3. **`/home/stan/Prod/maestro/maestro/leindex/rust/Cargo.toml`** - Dependencies: need to add `base64 = "0.22"` crate

4. **`/home/stan/Prod/maestro/maestro/leindex/rust/src/vector/metadata.rs`** - Helper functions: natural location for `encode_embedding_base64()` and `decode_embedding_base64()` utilities

5. **`/home/stan/Prod/maestro/maestro/leindex/rust/src/vector/benchmark_tests.rs`** - Testing: add performance regression tests to ensure vector extension provides expected speedup
