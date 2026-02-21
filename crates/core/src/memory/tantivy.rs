//! Tantivy-based Memory Backend with Hybrid Search
//!
//! Provides a memory implementation using Tantivy for full-text search
//! with optional vector similarity search for hybrid ranking.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, FAST, STORED, TEXT};
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};
use uuid::Uuid;

use crate::memory::hybrid::HybridRanker;
use crate::traits::{Memory, SearchResult};

/// Tantivy-based memory backend with hybrid search capabilities
pub struct TantivyMemory {
    index: Index,
    reader: IndexReader,
    schema: Schema,
    writer: Arc<tokio::sync::RwLock<IndexWriter>>,
    /// In-memory storage for content and metadata (since we need full retrieval)
    storage: Arc<tokio::sync::RwLock<HashMap<String, StoredMemory>>>,
    /// In-memory vector store for similarity search
    vectors: Arc<tokio::sync::RwLock<HashMap<String, Vec<f32>>>>,
    /// Hybrid ranker for combining text and vector scores
    ranker: HybridRanker,
}

/// In-memory storage for content and metadata
#[derive(Debug, Clone)]
struct StoredMemory {
    content: String,
    metadata: JsonValue,
}

impl TantivyMemory {
    /// Create or open a Tantivy memory index at the given path
    pub async fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_options(path, 0.6, 0.4).await
    }

    /// Create a new TantivyMemory with custom hybrid ranking weights
    ///
    /// # Arguments
    /// * `path` - Directory path for the Tantivy index
    /// * `vector_weight` - Weight for vector similarity scores (0.0-1.0)
    /// * `text_weight` - Weight for text/BM25 scores (0.0-1.0)
    pub async fn with_options(
        path: impl Into<PathBuf>,
        vector_weight: f32,
        text_weight: f32,
    ) -> Result<Self> {
        let path = path.into();

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        // Build schema
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STORED | FAST);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("metadata", STORED);
        schema_builder.add_text_field("timestamp", STORED);
        let schema = schema_builder.build();

        // Create or open index
        let directory = tantivy::directory::MmapDirectory::open(&path)?;
        let index = if Index::exists(&directory)? {
            Index::open(directory)?
        } else {
            Index::create_in_dir(&path, schema.clone())?
        };

        // Create reader with manual reload
        let reader = index.reader_builder().try_into()?;

        // Create writer (50MB heap)
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            reader,
            schema,
            writer: Arc::new(tokio::sync::RwLock::new(writer)),
            storage: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            vectors: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ranker: HybridRanker::new(vector_weight, text_weight),
        })
    }

    /// Store a document with optional vector embedding
    ///
    /// # Arguments
    /// * `content` - The document content
    /// * `metadata` - JSON metadata
    /// * `vector` - Optional embedding vector for similarity search
    ///
    /// # Returns
    /// The document ID
    pub async fn store_with_vector(
        &self,
        content: &str,
        metadata: JsonValue,
        vector: Option<Vec<f32>>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Store in memory
        self.storage.write().await.insert(
            id.clone(),
            StoredMemory {
                content: content.to_string(),
                metadata: metadata.clone(),
            },
        );

        // Store vector if provided
        if let Some(vec) = vector {
            self.vectors.write().await.insert(id.clone(), vec);
        }

        // Index in Tantivy
        let id_field = self.schema.get_field("id")?;
        let content_field = self.schema.get_field("content")?;
        let metadata_field = self.schema.get_field("metadata")?;
        let timestamp_field = self.schema.get_field("timestamp")?;

        let mut doc = TantivyDocument::default();
        doc.add_text(id_field, &id);
        doc.add_text(content_field, content);
        doc.add_text(metadata_field, metadata.to_string());
        doc.add_text(timestamp_field, &timestamp);

        {
            let mut writer = self.writer.write().await;
            writer.add_document(doc)?;
            writer.commit()?;
        }

        // Reload reader
        self.reader.reload()?;

        Ok(id)
    }

    /// Perform hybrid search combining text and vector similarity
    ///
    /// # Arguments
    /// * `query` - Search query text
    /// * `query_vector` - Optional query embedding for vector search
    /// * `limit` - Maximum results to return
    ///
    /// # Returns
    /// Ranked search results
    pub async fn search_hybrid(
        &self,
        query: &str,
        query_vector: Option<&[f32]>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        if query.trim().is_empty() && query_vector.is_none() {
            return Ok(Vec::new());
        }

        // Perform text search
        let text_results = self.text_search(query, limit * 2).await?;

        // Perform vector search if query vector provided
        let vector_results = if let Some(vec) = query_vector {
            self.vector_search(vec, limit * 2).await
        } else {
            Vec::new()
        };

        // Merge with hybrid ranker
        let ranked = self.ranker.merge(&text_results, &vector_results, limit);

        // Fetch full documents for ranked results
        let storage = self.storage.read().await;
        let mut results = Vec::with_capacity(ranked.len());

        for r in ranked {
            if let Some(stored) = storage.get(&r.id) {
                #[allow(clippy::cast_possible_truncation)]
                results.push(SearchResult {
                    id: r.id.clone(),
                    content: stored.content.clone(),
                    metadata: stored.metadata.clone(),
                    score: r.final_score,
                });
            }
        }

        Ok(results)
    }

    /// Perform text-only search using Tantivy BM25
    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();
        let content_field = self.schema.get_field("content")?;
        let id_field = self.schema.get_field("id")?;

        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);

        // Handle query parse errors gracefully by returning empty results
        let parsed_query = match query_parser.parse_query(query) {
            Ok(q) => q,
            Err(_) => return Ok(Vec::new()),
        };

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(owned_value) = doc.get_first(id_field) {
                if let Some(id_val) = Value::as_str(&owned_value) {
                    #[allow(clippy::cast_possible_truncation)]
                    results.push((id_val.to_string(), score));
                }
            }
        }

        Ok(results)
    }

    /// Perform vector similarity search using cosine similarity
    async fn vector_search(&self, query: &[f32], limit: usize) -> Vec<(String, f32)> {
        let vectors = self.vectors.read().await;

        let mut scored: Vec<(String, f32)> = vectors
            .iter()
            .filter_map(|(id, vec)| {
                let sim = cosine_similarity(query, vec);
                if sim > 0.0 {
                    Some((id.clone(), sim))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    /// Reindex all documents (commit pending writes and reload)
    pub async fn reindex(&self) -> Result<()> {
        {
            let mut writer = self.writer.write().await;
            writer.commit()?;
        }
        self.reader.reload()?;
        Ok(())
    }

    /// Check if the memory backend is healthy
    pub async fn health_check(&self) -> bool {
        self.reader.reload().is_ok()
    }

    /// Get the number of documents in the index
    pub async fn doc_count(&self) -> Result<u64> {
        let searcher = self.reader.searcher();
        Ok(searcher.num_docs())
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    for (x, y) in a.iter().zip(b.iter()) {
        let x = f64::from(*x);
        let y = f64::from(*y);
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if !denom.is_finite() || denom < f64::EPSILON {
        return 0.0;
    }

    let raw = dot / denom;
    if !raw.is_finite() {
        return 0.0;
    }

    // Clamp to [0, 1]
    #[allow(clippy::cast_possible_truncation)]
    let sim = raw.clamp(0.0, 1.0) as f32;
    sim
}

#[async_trait]
impl Memory for TantivyMemory {
    async fn store(&self, content: &str, metadata: JsonValue) -> Result<String> {
        self.store_with_vector(content, metadata, None).await
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_hybrid(query, None, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    async fn create_test_memory() -> (TempDir, TantivyMemory) {
        let tmp = TempDir::new().unwrap();
        let memory = TantivyMemory::new(tmp.path()).await.unwrap();
        (tmp, memory)
    }

    #[tokio::test]
    async fn test_store_and_search() {
        let (_tmp, memory) = create_test_memory().await;

        let id = memory
            .store("Hello world", json!({"test": true}))
            .await
            .unwrap();

        assert!(!id.is_empty());

        let results = memory.search("Hello", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Hello world");
        assert_eq!(results[0].metadata["test"], true);
    }

    #[tokio::test]
    async fn test_search_ordering() {
        let (_tmp, memory) = create_test_memory().await;

        memory.store("Rust programming", json!({})).await.unwrap();
        memory.store("Python programming", json!({})).await.unwrap();
        memory.store("Rust is fast", json!({})).await.unwrap();

        let results = memory.search("Rust", 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 10);
    }

    #[tokio::test]
    async fn test_search_respects_limit() {
        let (_tmp, memory) = create_test_memory().await;

        for i in 0..20 {
            memory
                .store(&format!("Document {} about Rust", i), json!({}))
                .await
                .unwrap();
        }

        let results = memory.search("Rust", 5).await.unwrap();
        assert!(results.len() <= 5);
    }

    #[tokio::test]
    async fn test_empty_search() {
        let (_tmp, memory) = create_test_memory().await;

        let results = memory.search("nonexistent", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_empty_query_returns_empty() {
        let (_tmp, memory) = create_test_memory().await;
        memory.store("content", json!({})).await.unwrap();

        let results = memory.search("", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_whitespace_query_returns_empty() {
        let (_tmp, memory) = create_test_memory().await;
        memory.store("content", json!({})).await.unwrap();

        let results = memory.search("   ", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_metadata_preserved() {
        let (_tmp, memory) = create_test_memory().await;

        let meta = json!({
            "source": "test",
            "count": 42,
            "nested": {"key": "value"}
        });

        memory.store("test content", meta.clone()).await.unwrap();

        let results = memory.search("test", 10).await.unwrap();
        assert_eq!(results[0].metadata["source"], "test");
        assert_eq!(results[0].metadata["count"], 42);
        assert_eq!(results[0].metadata["nested"]["key"], "value");
    }

    #[tokio::test]
    async fn test_store_with_vector() {
        let (_tmp, memory) = create_test_memory().await;

        let vec = vec![1.0, 0.0, 0.0, 0.5];
        let id = memory
            .store_with_vector("vector test", json!({}), Some(vec))
            .await
            .unwrap();

        assert!(!id.is_empty());

        // Vector search should work
        let query = vec![0.9, 0.1, 0.0, 0.4];
        let results = memory.search_hybrid("", Some(&query), 10).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_hybrid_search_combines_scores() {
        let (_tmp, memory) = create_test_memory().await;

        // Store with vectors
        memory
            .store_with_vector("apple fruit", json!({}), Some(vec![1.0, 0.0]))
            .await
            .unwrap();
        memory
            .store_with_vector("banana fruit", json!({}), Some(vec![0.0, 1.0]))
            .await
            .unwrap();

        // Text search only
        let text_results = memory.search("apple", 10).await.unwrap();
        assert_eq!(text_results.len(), 1);

        // Vector search only
        let vec_results = memory
            .search_hybrid("", Some(&[0.9, 0.1]), 10)
            .await
            .unwrap();
        assert!(!vec_results.is_empty());

        // Hybrid search
        let hybrid_results = memory
            .search_hybrid("apple", Some(&[0.9, 0.1]), 10)
            .await
            .unwrap();
        assert!(!hybrid_results.is_empty());
    }

    #[tokio::test]
    async fn test_health_check() {
        let (_tmp, memory) = create_test_memory().await;
        assert!(memory.health_check().await);
    }

    #[tokio::test]
    async fn test_reindex() {
        let (_tmp, memory) = create_test_memory().await;
        memory.store("test", json!({})).await.unwrap();

        let result = memory.reindex().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_doc_count() {
        let (_tmp, memory) = create_test_memory().await;
        assert_eq!(memory.doc_count().await.unwrap(), 0);

        memory.store("doc1", json!({})).await.unwrap();
        assert_eq!(memory.doc_count().await.unwrap(), 1);

        memory.store("doc2", json!({})).await.unwrap();
        assert_eq!(memory.doc_count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_limit_zero() {
        let (_tmp, memory) = create_test_memory().await;
        memory.store("content", json!({})).await.unwrap();

        let results = memory.search("content", 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_persistence_across_reopen() {
        let tmp = TempDir::new().unwrap();

        {
            let memory = TantivyMemory::new(tmp.path()).await.unwrap();
            memory
                .store("persistent content", json!({"test": true}))
                .await
                .unwrap();
        }

        // Reopen - note: storage is in-memory, so it won't persist
        // But the index should still exist
        let memory = TantivyMemory::new(tmp.path()).await.unwrap();
        let count = memory.doc_count().await.unwrap();
        assert!(count > 0, "Index should have documents");
    }

    #[tokio::test]
    async fn test_special_characters_in_query() {
        let (_tmp, memory) = create_test_memory().await;
        memory.store("function foo()", json!({})).await.unwrap();

        // Special characters should not crash - they should be handled gracefully
        // by returning empty results when query parsing fails
        let result = memory.search("foo()", 10).await;
        assert!(result.is_ok(), "Search with special characters should not error");

        // Try a simpler query that should work
        let result = memory.search("function", 10).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_unicode_content() {
        let (_tmp, memory) = create_test_memory().await;
        memory
            .store("Hello \u{1F916} Unicode: \u{4e2d}\u{6587}", json!({}))
            .await
            .unwrap();

        let results = memory.search("Unicode", 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].content.contains("\u{4e2d}\u{6587}"));
    }

    // Cosine similarity tests
    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0];
        let b = vec![1.0, 2.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let sim = cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }
}
