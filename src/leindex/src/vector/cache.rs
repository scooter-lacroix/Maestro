//! TTL Cache
//!
//! Thread-safe LRU cache with time-to-live support.
//! Used for caching search results and embeddings.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cache entry with value and expiration time
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
    last_accessed: Instant,
}

/// Thread-safe LRU cache with TTL
pub struct TtlCache<K, V> {
    entries: Mutex<HashMap<K, CacheEntry<V>>>,
    max_size: usize,
    ttl: Duration,
    // Statistics
    stats: Mutex<CacheStats>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> TtlCache<K, V> {
    /// Create a new TTL cache
    pub fn new(max_size: usize, ttl_secs: u64) -> Self {
        Self {
            entries: Mutex::new(HashMap::with_capacity(max_size)),
            max_size,
            ttl: Duration::from_secs(ttl_secs),
            stats: Mutex::new(CacheStats::default()),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Result<Option<V>> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        let now = Instant::now();

        if let Some(entry) = entries.get_mut(key) {
            if entry.expires_at > now {
                // Update last accessed
                entry.last_accessed = now;
                self.stats
                    .lock()
                    .map_err(|e| anyhow!("Lock poisoned: {}", e))?
                    .hits += 1;
                return Ok(Some(entry.value.clone()));
            } else {
                // Expired
                entries.remove(key);
                let mut stats = self
                    .stats
                    .lock()
                    .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
                stats.misses += 1;
                stats.expirations += 1;
            }
        } else {
            self.stats
                .lock()
                .map_err(|e| anyhow!("Lock poisoned: {}", e))?
                .misses += 1;
        }

        Ok(None)
    }

    /// Put a value in the cache
    pub fn put(&self, key: K, value: V) -> Result<()> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        let now = Instant::now();

        // Evict if at capacity
        if entries.len() >= self.max_size && !entries.contains_key(&key) {
            // Find least recently accessed
            let lru_key = entries
                .iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone());

            if let Some(k) = lru_key {
                entries.remove(&k);
                self.stats
                    .lock()
                    .map_err(|e| anyhow!("Lock poisoned: {}", e))?
                    .evictions += 1;
            }
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                expires_at: now + self.ttl,
                last_accessed: now,
            },
        );
        Ok(())
    }

    /// Remove a specific key
    pub fn remove(&self, key: &K) -> Result<bool> {
        Ok(self
            .entries
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .remove(key)
            .is_some())
    }

    /// Clear all entries
    pub fn clear(&self) -> Result<()> {
        self.entries
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .clear();
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        Ok(self
            .stats
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .clone())
    }

    /// Reset statistics
    pub fn reset_stats(&self) -> Result<()> {
        *self
            .stats
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))? = CacheStats::default();
        Ok(())
    }

    /// Get current size
    pub fn len(&self) -> Result<usize> {
        Ok(self
            .entries
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .len())
    }

    /// Check if empty
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self
            .entries
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .is_empty())
    }
}

/// Vector deduplicator using content hashing
pub struct VectorDeduplicator {
    hash_to_id: Mutex<HashMap<String, String>>,
    reference_counts: Mutex<HashMap<String, u32>>,
}

impl VectorDeduplicator {
    pub fn new() -> Self {
        Self {
            hash_to_id: Mutex::new(HashMap::new()),
            reference_counts: Mutex::new(HashMap::new()),
        }
    }

    /// Hash content using SHA-256
    pub fn hash_content(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Unregister a vector (for deletions)
    pub fn unregister(&self, vector_id: &str) -> Result<()> {
        let mut counts = self
            .reference_counts
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        counts.remove(vector_id);

        // Note: Full reverse lookup (hash_to_id) cleanup is O(N) here.
        // In a real system we'd store bidirectionally or just let it expire.
        // For now we just remove the ref count to prevent "phantom" dedup.
        let mut hash_to_id = self
            .hash_to_id
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        hash_to_id.retain(|_, id| id != vector_id);
        Ok(())
    }

    /// Get existing vector ID for content hash
    pub fn get_vector_id(&self, content_hash: &str) -> Result<Option<String>> {
        Ok(self
            .hash_to_id
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .get(content_hash)
            .cloned())
    }

    /// Register a new vector
    pub fn register(&self, content_hash: String, vector_id: String) -> Result<()> {
        self.hash_to_id
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .insert(content_hash, vector_id.clone());
        self.reference_counts
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?
            .insert(vector_id, 1);
        Ok(())
    }

    /// Add a reference to an existing vector
    pub fn add_reference(&self, vector_id: &str) -> Result<u32> {
        let mut counts = self
            .reference_counts
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        let count = counts.entry(vector_id.to_string()).or_insert(0);
        *count += 1;
        Ok(*count)
    }

    /// Remove a reference
    pub fn remove_reference(&self, vector_id: &str) -> Result<u32> {
        let mut counts = self
            .reference_counts
            .lock()
            .map_err(|e| anyhow!("Lock poisoned: {}", e))?;
        if let Some(count) = counts.get_mut(vector_id) {
            *count = count.saturating_sub(1);
            Ok(*count)
        } else {
            Ok(0)
        }
    }
}

impl Default for VectorDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_cache_basic() {
        let cache: TtlCache<String, i32> = TtlCache::new(10, 60);

        cache.put("key1".to_string(), 42).unwrap();
        assert_eq!(cache.get(&"key1".to_string()).unwrap(), Some(42));
        assert_eq!(cache.get(&"key2".to_string()).unwrap(), None);
    }

    #[test]
    fn test_deduplicator() {
        let dedup = VectorDeduplicator::new();

        let hash = VectorDeduplicator::hash_content("test content");
        assert!(dedup.get_vector_id(&hash).unwrap().is_none());

        dedup.register(hash.clone(), "vec_1".to_string()).unwrap();
        assert_eq!(
            dedup.get_vector_id(&hash).unwrap(),
            Some("vec_1".to_string())
        );
    }
}
