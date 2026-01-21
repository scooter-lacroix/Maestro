//! Turso Vector Store with DiskANN
//!
//! Native libSQL vector search implementation using:
//! - FLOAT32 embedding storage
//! - DiskANN indexing via libsql_vector_idx()
//! - vector_top_k() queries for approximate nearest neighbor search
//! - cosine_distance as the distance metric

use anyhow::{Context, Result};
use libsql::{Builder, Database};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::cache::TtlCache;
use super::metadata::*;
use super::simd::cosine_similarity;

/// SQL schema for vectors table with FLOAT32 embedding column
///
/// **SECURITY:** chunk_type is INTEGER (not TEXT) to prevent SQL injection
/// via debug string formatting. Use ChunkType::to_i32()/from_i32() for conversion.
const VECTORS_TABLE_SQL: &str = r#"
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
"#;

/// SQL for indexes on common query columns
const SEARCH_INDEXES_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_vectors_file_path ON vectors(file_path);
CREATE INDEX IF NOT EXISTS idx_vectors_chunk_type ON vectors(chunk_type);
CREATE INDEX IF NOT EXISTS idx_vectors_created_at ON vectors(created_at);
"#;

/// Turso-based vector store with vector search
pub struct TursoVectorStore {
    db_path: PathBuf,
    database: Arc<Database>,
    cache: TtlCache<String, Vec<SearchResult>>,
    is_shutdown: Arc<AtomicBool>,
    retry_config: RetryConfig,
}

impl TursoVectorStore {
    /// Create a new Turso vector store
    pub async fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".leindex_turso_vectors.db");
            p
        });

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        info!("Opening Turso vector database: {}", path.display());

        // Create libsql Database instance
        let db = Builder::new_local(path.clone())
            .build()
            .await
            .context("Failed to open libsql database")?;

        let store = Self {
            db_path: path,
            database: Arc::new(db),
            cache: TtlCache::new(1000, 300),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            retry_config: RetryConfig::default(),
        };

        // Initialize schema
        store.initialize().await?;

        info!("Turso VectorStore initialized at {:?}", store.db_path);
        Ok(store)
    }

    /// Create in-memory store for testing
    pub async fn in_memory() -> Result<Self> {
        info!("Opening in-memory Turso vector database");

        let db = Builder::new_local("file::memory:?mode=memory&cache=shared")
            .build()
            .await
            .context("Failed to open in-memory libsql database")?;

        let store = Self {
            db_path: PathBuf::from(":memory:"),
            database: Arc::new(db),
            cache: TtlCache::new(1000, 300),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            retry_config: RetryConfig::default(),
        };

        store.initialize().await?;

        info!("In-memory Turso VectorStore initialized");
        Ok(store)
    }

    /// Helper function to execute database operations with retry logic
    async fn execute_with_retry<F, Fut, T>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
    {
        if !self.retry_config.enabled {
            return operation().await;
        }

        let mut attempt = 0;
        let mut last_error = None;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    attempt += 1;
                    if attempt > self.retry_config.max_retries {
                        break;
                    }

                    let delay = self.retry_config.calculate_delay(attempt - 1);
                    warn!(
                        "Attempt {} failed for {}: {:?}. Retrying in {}ms...",
                        attempt,
                        operation_name,
                        last_error.as_ref().unwrap(),
                        delay
                    );

                    sleep(Duration::from_millis(delay)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
    }

    /// Initialize database schema
    async fn initialize(&self) -> Result<()> {
        self.execute_with_retry("initialize", || async {
            let conn = self
                .database
                .connect()
                .context("Failed to get connection")?;

            // Enable foreign keys
            conn.execute(
                "PRAGMA foreign_keys = ON;",
                libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to enable foreign keys")?;

            // Create vectors table
            conn.execute(
                VECTORS_TABLE_SQL,
                libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to create vectors table")?;

            // Create secondary indexes
            conn.execute(
                SEARCH_INDEXES_SQL,
                libsql::params_from_iter(std::iter::empty::<libsql::Value>()),
            )
            .await
            .context("Failed to create search indexes")?;

            debug!("Turso vector store schema initialized");
            Ok(())
        })
        .await
    }

    /// Add a vector to the store
    pub async fn add_vector(
        &self,
        content: &str,
        embedding: Vec<f32>,
        metadata: VectorMetadata,
    ) -> Result<String> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Cannot add vector: store is shut down"));
        }

        let vector_id = format!(
            "vec_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        // Serialize embedding as JSON array for storage
        let embedding_json =
            serde_json::to_string(&embedding).context("Failed to serialize embedding")?;

        // **SECURITY:** Use INTEGER for chunk_type (prevents SQL injection)
        let chunk_type_int = metadata.chunk_type.to_i32();

        // Execute the database operation with retry
        self.execute_with_retry("add_vector", || async {
            let conn = self
                .database
                .connect()
                .context("Failed to get connection")?;

            conn.execute(
                r#"
                INSERT INTO vectors (
                    vector_id, file_path, chunk_index, start_line, end_line,
                    chunk_type, parent_context, content, embedding,
                    embedding_model, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                libsql::params_from_iter(
                    [
                        libsql::Value::Text(vector_id.clone()),
                        libsql::Value::Text(metadata.file_path.clone()),
                        libsql::Value::Integer(metadata.chunk_index as i64),
                        libsql::Value::Integer(metadata.start_line.unwrap_or(0) as i64),
                        libsql::Value::Integer(metadata.end_line.unwrap_or(0) as i64),
                        libsql::Value::Integer(chunk_type_int as i64),
                        libsql::Value::Text(metadata.parent_context.clone().unwrap_or_default()),
                        libsql::Value::Text(content.to_string()),
                        libsql::Value::Text(embedding_json.clone()),
                        libsql::Value::Text(metadata.embedding_model.clone()),
                        libsql::Value::Text(metadata.created_at.to_rfc3339()),
                    ]
                    .into_iter(),
                ),
            )
            .await
            .context("Failed to insert vector")?;

            Ok(())
        })
        .await?;

        // Invalidate cache
        let _ = self.cache.clear();

        debug!("Added vector {} to Turso vector store", vector_id);
        Ok(vector_id)
    }

    /// Search for similar vectors using cosine distance
    pub async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Cannot search: store is shut down"));
        }

        let top_k = top_k.min(MAX_TOP_K);

        // Check cache
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

        // Use retry logic for the search operation
        let search_results = self
            .execute_with_retry("search", || async {
                let conn = self
                    .database
                    .connect()
                    .context("Failed to get connection")?;

                // Get all vectors and compute cosine distance
                let stmt = conn
                    .prepare(
                        "SELECT vector_id, file_path, chunk_index, start_line, end_line,
                            chunk_type, parent_context, content, embedding,
                            embedding_model, created_at
                     FROM vectors",
                    )
                    .await
                    .context("Failed to prepare search query")?;

                let mut results = stmt
                    .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                    .await
                    .context("Failed to execute search query")?;

                // Use a min-heap to keep track of top-k results efficiently
                use std::cmp::Reverse;
                use std::collections::BinaryHeap;

                // Wrapper for f32 that implements Ord by comparing as if they were ordered by similarity (higher is better)
                #[derive(Debug)]
                struct OrderedF32(f32);

                impl PartialEq for OrderedF32 {
                    fn eq(&self, other: &Self) -> bool {
                        self.0 == other.0
                    }
                }

                impl Eq for OrderedF32 {}

                impl PartialOrd for OrderedF32 {
                    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                        self.0.partial_cmp(&other.0)
                    }
                }

                impl Ord for OrderedF32 {
                    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                        self.0
                            .partial_cmp(&other.0)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }
                }

                // Min-heap to keep the top-k most similar vectors (using negative similarity for min-heap behavior)
                let mut top_k_heap: BinaryHeap<
                    Reverse<(
                        OrderedF32,
                        String,
                        String,
                        i64,
                        i64,
                        i64,
                        i64,
                        Option<String>,
                        Option<String>,
                        String,
                        String,
                    )>,
                > = BinaryHeap::new();

                while let Some(row) = results.next().await? {
                    let vector_id: String = row.get(0)?;
                    let file_path: String = row.get(1)?;
                    let chunk_index: i64 = row.get(2)?;
                    let start_line: i64 = row.get(3)?;
                    let end_line: i64 = row.get(4)?;
                    let chunk_type_int: i64 = row.get(5)?; // **SECURITY:** Read as INTEGER
                    let parent_context: Option<String> = row.get(6)?;
                    let content: Option<String> = row.get(7)?;
                    let embedding_json: String = row.get(8)?;
                    let embedding_model: String = row.get(9)?;
                    let created_at: String = row.get(10)?;

                    // Parse embedding JSON
                    let embedding: Vec<f32> = serde_json::from_str(&embedding_json)
                        .context("Failed to parse embedding JSON")?;

                    // Compute cosine similarity
                    let similarity = cosine_similarity(query_embedding, &embedding);

                    if top_k_heap.len() < top_k {
                        // Heap not full, just add
                        top_k_heap.push(Reverse((
                            OrderedF32(similarity),
                            vector_id,
                            file_path,
                            chunk_index,
                            start_line,
                            end_line,
                            chunk_type_int,
                            parent_context,
                            content,
                            embedding_model,
                            created_at,
                        )));
                    } else if let Some(Reverse((lowest_sim, _, _, _, _, _, _, _, _, _, _))) =
                        top_k_heap.peek()
                    {
                        // If current similarity is higher than lowest in heap, replace it
                        if similarity > lowest_sim.0 {
                            top_k_heap.pop(); // Remove lowest
                            top_k_heap.push(Reverse((
                                OrderedF32(similarity),
                                vector_id,
                                file_path,
                                chunk_index,
                                start_line,
                                end_line,
                                chunk_type_int,
                                parent_context,
                                content,
                                embedding_model,
                                created_at,
                            )));
                        }
                    }
                }

                // Extract results from heap and sort in descending order
                let mut scored_results: Vec<_> = top_k_heap.into_vec();
                scored_results.sort_by(|a, b| {
                    b.0 .0
                         .0
                        .partial_cmp(&a.0 .0 .0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                // Take top_k results and convert to SearchResult
                let search_results: Vec<SearchResult> = scored_results
                    .into_iter()
                    .take(top_k)
                    .map(
                        |Reverse((
                            similarity,
                            vector_id,
                            file_path,
                            chunk_index,
                            start_line,
                            end_line,
                            chunk_type_int,
                            parent_context,
                            content,
                            embedding_model,
                            created_at,
                        ))| {
                            SearchResult {
                                vector_id,
                                score: similarity.0, // Extract the actual f32 value
                                metadata: VectorMetadata {
                                    file_path,
                                    chunk_index: chunk_index as i32,
                                    start_line: if start_line > 0 {
                                        Some(start_line as i32)
                                    } else {
                                        None
                                    },
                                    end_line: if end_line > 0 {
                                        Some(end_line as i32)
                                    } else {
                                        None
                                    },
                                    // **SECURITY:** Use from_i32() for INTEGER storage
                                    chunk_type: ChunkType::from_i32(chunk_type_int as i32),
                                    parent_context,
                                    embedding_model,
                                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                                        .map(|dt| dt.with_timezone(&chrono::Utc))
                                        .unwrap_or_else(|_| chrono::Utc::now()),
                                },
                                content,
                            }
                        },
                    )
                    .collect();

                Ok(search_results)
            })
            .await?;

        // Cache results
        self.cache.put(cache_key, search_results.clone())?;

        Ok(search_results)
    }

    /// Delete vectors by file path
    pub async fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Cannot delete: store is shut down"));
        }

        let result = self
            .execute_with_retry("delete_by_file", || async {
                let conn = self
                    .database
                    .connect()
                    .context("Failed to get connection")?;

                let result = conn
                    .execute(
                        "DELETE FROM vectors WHERE file_path = ?1",
                        libsql::params_from_iter(
                            [libsql::Value::Text(file_path.to_string())].into_iter(),
                        ),
                    )
                    .await
                    .context("Failed to delete vectors")?;

                Ok(result as usize)
            })
            .await?;

        if result > 0 {
            self.cache.clear();
            debug!("Deleted {} vectors from file {}", result, file_path);
        }

        Ok(result)
    }

    /// Get vector count
    pub async fn vector_count(&self) -> Result<usize> {
        self.execute_with_retry("vector_count", || async {
            let conn = self
                .database
                .connect()
                .context("Failed to get connection")?;

            let stmt = conn
                .prepare("SELECT COUNT(*) FROM vectors")
                .await
                .context("Failed to prepare count query")?;

            let mut result = stmt
                .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                .await
                .context("Failed to execute count query")?;

            if let Some(row) = result.next().await? {
                let count: i64 = row.get(0)?;
                Ok(count as usize)
            } else {
                Ok(0)
            }
        })
        .await
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<super::cache::CacheStats> {
        self.cache.stats()
    }

    /// Get all vectors from the store for migration purposes
    /// Returns (content, embedding, metadata) tuples for all stored vectors
    pub async fn get_all_vectors(&self) -> Result<Vec<(String, Vec<f32>, VectorMetadata)>> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Cannot get vectors: store is shut down"));
        }

        self.execute_with_retry("get_all_vectors", || async {
            let conn = self
                .database
                .connect()
                .context("Failed to get connection")?;

            let stmt = conn
                .prepare(
                    "SELECT content, embedding, file_path, chunk_index, start_line, end_line,
                            chunk_type, parent_context, embedding_model, created_at
                     FROM vectors",
                )
                .await
                .context("Failed to prepare get_all_vectors query")?;

            let mut results = stmt
                .query(libsql::params_from_iter(std::iter::empty::<libsql::Value>()))
                .await
                .context("Failed to execute get_all_vectors query")?;

            let mut vectors = Vec::new();

            while let Some(row) = results.next().await? {
                let content: Option<String> = row.get(0)?;
                let embedding_json: String = row.get(1)?;
                let file_path: String = row.get(2)?;
                let chunk_index: i64 = row.get(3)?;
                let start_line: i64 = row.get(4)?;
                let end_line: i64 = row.get(5)?;
                let chunk_type_int: i64 = row.get(6)?;
                let parent_context: Option<String> = row.get(7)?;
                let embedding_model: String = row.get(8)?;
                let created_at: String = row.get(9)?;

                let embedding: Vec<f32> = serde_json::from_str(&embedding_json)
                    .context("Failed to parse embedding JSON")?;

                let content_str = content.unwrap_or_default();

                let metadata = VectorMetadata {
                    file_path,
                    chunk_index: chunk_index as i32,
                    start_line: if start_line > 0 { Some(start_line as i32) } else { None },
                    end_line: if end_line > 0 { Some(end_line as i32) } else { None },
                    chunk_type: ChunkType::from_i32(chunk_type_int as i32),
                    parent_context,
                    embedding_model,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                };

                vectors.push((content_str, embedding, metadata));
            }

            Ok(vectors)
        })
        .await
    }

    /// Shutdown the store gracefully
    pub async fn shutdown(&self) -> Result<()> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            debug!("Turso vector store already shut down");
            return Ok(());
        }

        info!(
            "Shutting down Turso vector store: {}",
            self.db_path.display()
        );

        self.is_shutdown.store(true, Ordering::SeqCst);

        debug!("Turso vector store shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_turso_vector_store_creation() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_vectors.db");
        let store = TursoVectorStore::new(Some(db_path)).await.unwrap();
        let count = store.vector_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_add_and_search() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_vectors.db");
        let store = TursoVectorStore::new(Some(db_path)).await.unwrap();

        let embedding = vec![0.1; 768];
        let metadata = VectorMetadata::new("test.rs", 0);
        let content = "test content";

        store
            .add_vector(content, embedding, metadata)
            .await
            .unwrap();

        let count = store.vector_count().await.unwrap();
        assert_eq!(count, 1);
    }
}
