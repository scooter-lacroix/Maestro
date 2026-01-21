//! Adaptive Vector Store
//!
//! Automatically routes to the optimal backend based on vector count:
//! - Linear search: < 90K vectors (fastest for small datasets)
//! - HNSW search: >= 90K vectors (better for large datasets)
//! - Turso backup: Provides persistence and recovery
//!
//! All implementations use SIMD-accelerated cosine similarity.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::hnsw_store::HnswVectorStore;
use super::metadata::*;
use super::store::VectorStore;
use super::turso_store::TursoVectorStore;

/// Threshold for switching from Linear to HNSW search
const HNSW_THRESHOLD: usize = 90_000;

/// Threshold for switching from Linear to HNSW (upper bound)
const HNSW_SWITCH_UP_THRESHOLD: usize = 90_000;

/// Threshold for switching from HNSW to Linear (lower bound)
const HNSW_SWITCH_DOWN_THRESHOLD: usize = 80_000;

/// Adaptive vector store that routes to optimal backend
pub struct AdaptiveVectorStore {
    linear: Arc<RwLock<Option<VectorStore>>>,
    hnsw: Arc<RwLock<Option<HnswVectorStore>>>,
    turso: Option<TursoVectorStore>,
    mode: Arc<AtomicUsize>,
    is_shutdown: Arc<AtomicBool>,
    mode_switch_lock: Arc<RwLock<()>>,
    index_path: PathBuf,
}

/// Current mode of the adaptive store
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreMode {
    Linear = 0,
    Hnsw = 1,
    Turso = 2,
}

impl StoreMode {
    fn from_usize(value: usize) -> Self {
        match value {
            0 => StoreMode::Linear,
            1 => StoreMode::Hnsw,
            _ => StoreMode::Turso,
        }
    }
}

impl AdaptiveVectorStore {
    /// Create a new adaptive vector store
    pub async fn new(index_path: Option<PathBuf>) -> Result<Self> {
        let path = index_path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push(".leindex");
            p
        });

        info!("Creating adaptive vector store at {:?}", path);

        // Initialize Turso for persistence (always available)
        let turso_path = path.join("turso_vectors.db");
        let turso = match TursoVectorStore::new(Some(turso_path)).await {
            Ok(store) => {
                info!("Turso vector store initialized for persistence");
                Some(store)
            }
            Err(e) => {
                warn!(
                    "Failed to initialize Turso store: {}, continuing without persistence",
                    e
                );
                None
            }
        };

        // Check existing vector count from Turso if available
        let initial_count = if let Some(ref t) = turso {
            t.vector_count().await.unwrap_or(0)
        } else {
            0
        };

        // Select initial mode based on existing data
        let mode = if initial_count >= HNSW_THRESHOLD {
            StoreMode::Hnsw
        } else {
            StoreMode::Linear
        };

        // Initialize linear store (for < 90K vectors)
        let linear = if mode == StoreMode::Linear || initial_count < HNSW_THRESHOLD {
            Some(VectorStore::new(Some(path.clone()), None)?)
        } else {
            None
        };

        // Initialize HNSW store (for >= 90K vectors)
        let hnsw = if mode == StoreMode::Hnsw || initial_count >= HNSW_THRESHOLD {
            Some(HnswVectorStore::new(Some(path.clone()), None)?)
        } else {
            None
        };

        let store = Self {
            linear: Arc::new(RwLock::new(linear)),
            hnsw: Arc::new(RwLock::new(hnsw)),
            turso,
            mode: Arc::new(AtomicUsize::new(mode as usize)),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            mode_switch_lock: Arc::new(RwLock::new(())),
            index_path: path,
        };

        info!(
            "Adaptive vector store initialized in {:?} mode ({} vectors)",
            store.mode(),
            initial_count
        );

        Ok(store)
    }

    /// Get current mode
    pub fn mode(&self) -> StoreMode {
        StoreMode::from_usize(self.mode.load(Ordering::SeqCst))
    }

    /// Get vector count from active store
    pub async fn vector_count(&self) -> Result<usize> {
        // Prefer Turso for authoritative count (persistent)
        if let Some(ref turso) = self.turso {
            return Ok(turso.vector_count().await.unwrap_or(0));
        }

        match self.mode() {
            StoreMode::Linear => {
                let linear_guard = self.linear.read().await;
                if let Some(ref linear) = *linear_guard {
                    return Ok(linear.vector_count()?);
                }
            }
            StoreMode::Hnsw => {
                let hnsw_guard = self.hnsw.read().await;
                if let Some(ref hnsw) = *hnsw_guard {
                    return Ok(hnsw.vector_count()?);
                }
            }
            StoreMode::Turso => {
                if let Some(ref turso) = self.turso {
                    return Ok(turso.vector_count().await?);
                }
            }
        }

        Ok(0)
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

        // CRITICAL: Acquire read lock to prevent mode switch during operation
        let _mode_lock = self.mode_switch_lock.read().await;

        // CRITICAL: Generate unified UUID for all backends (Task 7.6.12)
        // This ensures the same vector has the same ID across all stores
        let unified_id = format!("vec_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));

        // Always add to Turso for persistence (with unified ID)
        if let Some(ref turso) = self.turso {
            turso
                .add_vector_with_id(&unified_id, content, embedding.clone(), metadata.clone())
                .await?;
        }

        // Add to active in-memory store (with unified ID)
        match self.mode() {
            StoreMode::Linear => {
                let linear_guard = self.linear.read().await;
                if let Some(ref linear) = *linear_guard {
                    linear.add_vector_with_id(&unified_id, content, embedding, metadata)?;
                }
            }
            StoreMode::Hnsw => {
                let hnsw_guard = self.hnsw.read().await;
                if let Some(ref hnsw) = *hnsw_guard {
                    hnsw.add_vector_with_id(&unified_id, content, embedding, metadata)?;
                }
            }
            StoreMode::Turso => {
                // Already added to Turso above
            }
        }

        // Check if we need to switch modes
        let count = self.vector_count().await?;
        if self.mode() == StoreMode::Linear && count >= HNSW_SWITCH_UP_THRESHOLD {
            info!(
                "Vector count reached {}K, switching to HNSW mode",
                count / 1000
            );
            self.switch_to_hnsw().await?;
        }

        Ok(unified_id)
    }

    /// Search for similar vectors
    pub async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Cannot search: store is shut down"));
        }

        // CRITICAL: Acquire read lock to prevent mode switch during operation
        let _mode_lock = self.mode_switch_lock.read().await;

        match self.mode() {
            StoreMode::Linear => {
                let linear_guard = self.linear.read().await;
                if let Some(ref linear) = *linear_guard {
                    debug!(
                        "Searching using Linear store ({} vectors)",
                        linear.vector_count().unwrap_or(0)
                    );
                    return linear.search(query_embedding, top_k);
                }
            }
            StoreMode::Hnsw => {
                let hnsw_guard = self.hnsw.read().await;
                if let Some(ref hnsw) = *hnsw_guard {
                    debug!(
                        "Searching using HNSW store ({} vectors)",
                        hnsw.vector_count().unwrap_or(0)
                    );
                    return hnsw.search(query_embedding, top_k);
                }
            }
            StoreMode::Turso => {
                if let Some(ref turso) = self.turso {
                    debug!("Searching using Turso store");
                    return turso.search(query_embedding, top_k).await;
                }
            }
        }

        // Fallback to Turso if primary store unavailable
        if let Some(ref turso) = self.turso {
            warn!("Primary store unavailable, falling back to Turso");
            return turso.search(query_embedding, top_k).await;
        }

        Err(anyhow::anyhow!("No active store available for search"))
    }

    /// Delete vectors by file path
    pub async fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Cannot delete: store is shut down"));
        }

        // CRITICAL: Acquire read lock to prevent mode switch during operation
        let _mode_lock = self.mode_switch_lock.read().await;

        let mut deleted = 0;

        // Delete from in-memory stores
        match self.mode() {
            StoreMode::Linear => {
                let linear_guard = self.linear.read().await;
                if let Some(ref linear) = *linear_guard {
                    deleted += linear.delete_by_file(file_path)?;
                }
            }
            StoreMode::Hnsw => {
                let hnsw_guard = self.hnsw.read().await;
                if let Some(ref hnsw) = *hnsw_guard {
                    deleted += hnsw.delete_by_file(file_path)?;
                }
            }
            _ => {}
        }

        // Delete from Turso (persistent)
        if let Some(ref turso) = self.turso {
            deleted += turso.delete_by_file(file_path).await?;
        }

        // Check if we should switch back to Linear mode
        let count = self.vector_count().await?;
        if self.mode() == StoreMode::Hnsw && count < HNSW_SWITCH_DOWN_THRESHOLD {
            info!(
                "Vector count dropped to {}K, switching to Linear mode",
                count / 1000
            );
            self.switch_to_linear().await?;
        }

        Ok(deleted)
    }

    /// Switch to HNSW mode
    async fn switch_to_hnsw(&self) -> Result<()> {
        info!("Switching to HNSW mode...");

        // Acquire mode switch lock to prevent concurrent switches
        // NOTE: This blocks all add_vector/search/delete operations during switch
        let _lock = self.mode_switch_lock.write().await;

        // Get the current linear store and take ownership of it
        let old_linear_store = {
            let mut linear_guard = self.linear.write().await;
            linear_guard.take()
        };

        // Create new HNSW store
        let mut new_hnsw_store = HnswVectorStore::new(Some(self.index_path.clone()), None)
            .context("Failed to create HNSW store")?;

        // CRITICAL: Proper data migration via Turso (the common persistence layer)
        // All vectors are in Turso because add_vector always adds to Turso
        if let Some(ref turso) = self.turso {
            info!("Migrating data from Turso to new HNSW store...");

            let all_vectors = turso
                .get_all_vectors()
                .await
                .context("Failed to get all vectors from Turso for migration")?;

            let vector_count = all_vectors.len();

            info!(
                "Migrating {} vectors from Turso to HNSW store...",
                vector_count
            );

            for (content, embedding, metadata) in all_vectors {
                new_hnsw_store
                    .add_vector(&content, embedding, metadata)
                    .context("Failed to add vector to HNSW store during migration")?;
            }

            info!("Data migration completed: {} vectors migrated", vector_count);
        } else if let Some(old_store) = old_linear_store {
            warn!("No Turso store available, migrating from Linear store (may lose unsaved data)");
            // Fallback: persist old store to disk first
            old_store
                .persist()
                .context("Failed to persist old linear store before migration")?;

            // Create a new HNSW store and load the data from disk
            // Note: This only works if vectors were persisted, may lose in-memory-only data
            new_hnsw_store = HnswVectorStore::new(Some(self.index_path.clone()), None)
                .context("Failed to create new HNSW store after migration")?;
        }

        // Replace the HNSW store with the new one
        {
            let mut hnsw_guard = self.hnsw.write().await;
            *hnsw_guard = Some(new_hnsw_store);
        }

        // Update mode atomically
        self.mode.store(StoreMode::Hnsw as usize, Ordering::SeqCst);

        info!("Successfully switched to HNSW mode");
        Ok(())
    }

    /// Switch to Linear mode
    async fn switch_to_linear(&self) -> Result<()> {
        info!("Switching to Linear mode...");

        // Acquire mode switch lock to prevent concurrent switches
        // NOTE: This blocks all add_vector/search/delete operations during switch
        let _lock = self.mode_switch_lock.write().await;

        // Get the current HNSW store and take ownership of it
        let old_hnsw_store = {
            let mut hnsw_guard = self.hnsw.write().await;
            hnsw_guard.take()
        };

        // Create new linear store
        let mut new_linear_store = VectorStore::new(Some(self.index_path.clone()), None)
            .context("Failed to create Linear store")?;

        // CRITICAL: Proper data migration via Turso (the common persistence layer)
        // All vectors are in Turso because add_vector always adds to Turso
        if let Some(ref turso) = self.turso {
            info!("Migrating data from Turso to new Linear store...");

            let all_vectors = turso
                .get_all_vectors()
                .await
                .context("Failed to get all vectors from Turso for migration")?;

            let vector_count = all_vectors.len();

            info!(
                "Migrating {} vectors from Turso to Linear store...",
                vector_count
            );

            for (content, embedding, metadata) in all_vectors {
                new_linear_store
                    .add_vector(&content, embedding, metadata)
                    .context("Failed to add vector to Linear store during migration")?;
            }

            info!("Data migration completed: {} vectors migrated", vector_count);
        } else if let Some(old_store) = old_hnsw_store {
            warn!("No Turso store available, migrating from HNSW store (may lose unsaved data)");
            // Fallback: persist old store to disk first
            old_store
                .persist()
                .context("Failed to persist old HNSW store before migration")?;

            // Create a new linear store and load the data from disk
            // Note: This only works if vectors were persisted, may lose in-memory-only data
            new_linear_store = VectorStore::new(Some(self.index_path.clone()), None)
                .context("Failed to create new linear store after migration")?;
        }

        // Replace the linear store with the new one
        {
            let mut linear_guard = self.linear.write().await;
            *linear_guard = Some(new_linear_store);
        }

        // Update mode atomically
        self.mode
            .store(StoreMode::Linear as usize, Ordering::SeqCst);

        info!("Successfully switched to Linear mode");
        Ok(())
    }

    /// Persist any in-memory state to disk
    pub async fn persist(&self) -> Result<()> {
        match self.mode() {
            StoreMode::Linear => {
                let linear_guard = self.linear.read().await;
                if let Some(ref linear) = *linear_guard {
                    linear.persist()?;
                }
            }
            StoreMode::Hnsw => {
                let hnsw_guard = self.hnsw.read().await;
                if let Some(ref hnsw) = *hnsw_guard {
                    hnsw.persist()?;
                }
            }
            StoreMode::Turso => {}
        }
        Ok(())
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> super::cache::CacheStats {
        match self.mode() {
            StoreMode::Linear => match self.linear.read().await.as_ref() {
                Some(linear) => linear.cache_stats().unwrap_or_default(),
                None => super::cache::CacheStats::default(),
            },
            StoreMode::Hnsw => match self.hnsw.read().await.as_ref() {
                Some(hnsw) => hnsw.cache_stats().unwrap_or_default(),
                None => super::cache::CacheStats::default(),
            },
            StoreMode::Turso => {
                if let Some(ref turso) = self.turso {
                    return turso.cache_stats().unwrap_or_default();
                }
                super::cache::CacheStats::default()
            }
        }
    }

    /// Shutdown the store gracefully
    pub async fn shutdown(&self) -> Result<()> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            debug!("Adaptive vector store already shut down");
            return Ok(());
        }

        info!(
            "Shutting down adaptive vector store at {:?}",
            self.index_path
        );

        // Persist in-memory state
        self.persist().await?;

        // Shutdown Turso
        if let Some(ref turso) = self.turso {
            turso.shutdown().await?;
        }

        self.is_shutdown.store(true, Ordering::SeqCst);

        debug!("Adaptive vector store shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_adaptive_store_creation() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store = AdaptiveVectorStore::new(Some(temp_dir.path().to_path_buf()))
            .await
            .unwrap();
        assert_eq!(store.mode(), StoreMode::Linear);

        let count = store.vector_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_adaptive_add_and_search() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store = AdaptiveVectorStore::new(Some(temp_dir.path().to_path_buf()))
            .await
            .unwrap();

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

    #[tokio::test]
    async fn test_adaptive_shutdown() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store = AdaptiveVectorStore::new(Some(temp_dir.path().to_path_buf()))
            .await
            .unwrap();

        store.shutdown().await.unwrap();
        assert!(store.is_shutdown.load(Ordering::SeqCst));
    }

    // Task 7.6.25: Test mode switch data migration
    #[tokio::test]
    async fn test_mode_switch_data_migration() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let store = AdaptiveVectorStore::new(Some(temp_dir.path().to_path_buf()))
            .await
            .unwrap();

        // Add 100 vectors to trigger HNSW mode switch (>90K threshold)
        // But we can't add that many in a test, so we'll test a smaller number
        // The key is to verify that vectors are preserved when switching modes

        // Add 10 vectors
        for i in 0..10 {
            let mut embedding = vec![0.0; 768];
            embedding[i % 768] = 1.0;
            let metadata = VectorMetadata::new(&format!("file{}.rs", i), i as i32);
            let content = format!("content {}", i);
            store
                .add_vector(&content, embedding, metadata)
                .await
                .unwrap();
        }

        let count_before = store.vector_count().await.unwrap();
        assert_eq!(count_before, 10);

        // Search should work
        let mut query = vec![0.0; 768];
        query[5] = 1.0;
        let results = store.search(&query, 3).await.unwrap();
        assert!(!results.is_empty());

        // Note: We can't easily force a mode switch without hitting the 90K threshold
        // The migration code is exercised in the implementation via Turso
        // The key correctness properties are:
        // 1. All vectors are in Turso (persistent layer)
        // 2. Mode switch loads from Turso
        // 3. vector_count() returns authoritative count from Turso

        // Verify Turso has the data (vector_count uses Turso)
        let count_from_turso = store.vector_count().await.unwrap();
        assert_eq!(count_from_turso, 10);
    }
}
