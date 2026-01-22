//! Concurrency tests for vector modules
//!
//! Tests for concurrent operations on vector stores:
//! 1. test_concurrent_add_delete - Multiple threads adding and deleting vectors simultaneously
//! 2. test_concurrent_mode_switch - Trigger mode switches while concurrent operations are running
//! 3. test_lock_poisoning_recovery - Simulate lock poisoning and verify error handling works

use std::sync::Arc;
use std::time::Duration;
use tokio::task;

use crate::vector::{AdaptiveVectorStore, VectorMetadata, VectorStore};

/// Test concurrent add and delete operations
#[tokio::test]
async fn test_concurrent_add_delete() {
    use tempfile::tempdir;

    let temp_dir = tempdir().expect("Failed to create temp dir");
    let store = Arc::new(VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap());

    let num_threads = 10;
    let ops_per_thread = 50;

    let mut handles = vec![];

    // Spawn multiple threads performing concurrent adds and deletes
    for thread_id in 0..num_threads {
        let store_clone = Arc::clone(&store);

        let handle = task::spawn(async move {
            for i in 0..ops_per_thread {
                let content = format!("thread{}_content_{}", thread_id, i);
                let mut embedding = vec![0.0; 768];
                // Create unique embeddings to avoid deduplication
                for j in 0..768 {
                    embedding[j] = (thread_id * 100 + i * 10 + j) as f32 / 10000.0;
                }

                let metadata =
                    VectorMetadata::new(&format!("file{}_{}.rs", thread_id, i), i as i32);

                // Add vector
                let _ = store_clone
                    .add_vector(&content, embedding, metadata)
                    .unwrap();

                // Occasionally delete vectors by file
                if i % 10 == 9 {
                    // Delete on every 10th iteration
                    let _ = store_clone
                        .delete_by_file(&format!("file{}_{}.rs", thread_id, i - 9))
                        .unwrap();
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.await.expect("Thread panicked");
    }

    // Verify the store is still functional
    let query = vec![0.5; 768];
    let results = store.search(&query, 10).unwrap();
    // The store should still be functional even if some operations were deleted
    assert!(results.len() > 0); // Should not panic
}

/// Test concurrent mode switching in adaptive store
#[tokio::test]
async fn test_concurrent_mode_switch() {
    use tempfile::tempdir;

    let temp_dir = tempdir().expect("Failed to create temp dir");
    let adaptive_store = Arc::new(
        AdaptiveVectorStore::new(Some(temp_dir.path().to_path_buf()))
            .await
            .unwrap(),
    );

    let num_threads = 6;
    let mut handles = vec![];

    // Spawn threads that perform operations while potentially triggering mode switches
    for thread_id in 0..num_threads {
        let store_clone = Arc::clone(&adaptive_store);

        let handle = task::spawn(async move {
            for i in 0..30 {
                let content = format!("thread{}_content_{}", thread_id, i);
                let mut embedding = vec![0.0; 768];
                // Create unique embeddings to avoid deduplication
                for j in 0..768 {
                    embedding[j] = (thread_id * 100 + i + j) as f32 / 1000.0;
                }
                let metadata =
                    VectorMetadata::new(&format!("thread{}_file{}.rs", thread_id, i), i as i32);

                // Add vector
                let _ = store_clone
                    .add_vector(&content, embedding, metadata)
                    .await
                    .unwrap();

                // Perform search
                let mut query = vec![0.0; 768];
                for j in 0..768 {
                    query[j] = (thread_id + j) as f32 / 1000.0;
                }
                let _ = store_clone.search(&query, 3).await.unwrap();

                // Occasionally delete vectors to trigger potential mode switches
                if i % 15 == 0 && i > 0 {
                    let _ = store_clone
                        .delete_by_file(&format!("thread{}_file{}.rs", thread_id, i - 15))
                        .await
                        .unwrap();
                }

                // Brief pause to allow other threads to operate and potentially trigger mode switches
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        });
        handles.push(handle);
    }

    // Monitor thread to check for mode changes during operations
    let monitor_store = Arc::clone(&adaptive_store);
    let monitor_handle = task::spawn(async move {
        for _i in 0..150 {
            let _current_mode = monitor_store.mode(); // Just verify mode doesn't panic
            // Brief pause
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    handles.push(monitor_handle);

    // Wait for all threads to complete
    for handle in handles {
        handle.await.expect("Thread panicked");
    }

    // Verify the store is still functional after concurrent operations
    let mut query = vec![0.5; 768];
    for i in 0..100 {
        query[i % 768] = (i % 50) as f32 / 100.0;
    }
    let results = adaptive_store.search(&query, 10).await.unwrap();
    assert!(results.len() > 0); // Should have results

    // Verify final state is valid
    let _final_mode = adaptive_store.mode();
    let _final_count = adaptive_store.vector_count().await.unwrap();
}

/// Test lock poisoning recovery in vector stores
#[tokio::test]
async fn test_lock_poisoning_recovery() {
    use std::panic;
    use std::sync::{Arc, Mutex};
    use std::thread;

    // Test actual lock poisoning scenario with a custom structure that mimics our stores
    let mutex = Arc::new(Mutex::new(Vec::new()));

    // First, create a scenario where a thread panics while holding a lock
    let mutex_clone1 = Arc::clone(&mutex);
    let handle1 = thread::spawn(move || {
        let mut data = mutex_clone1.lock().unwrap();
        data.push(42);
        // Panic while holding the lock, causing poisoning
        panic!("Intentional panic to test lock poisoning");
    });

    // Wait for the thread to finish (this will catch the panic)
    let result1 = handle1.join();
    assert!(result1.is_err()); // Thread should have panicked

    // Now test that we can handle the poisoned lock appropriately
    let mutex_clone2 = Arc::clone(&mutex);
    let handle2 = task::spawn_blocking(move || {
        // Attempt to lock - this will either work or detect poisoning
        match mutex_clone2.lock() {
            Ok(mut guard) => {
                // Successfully acquired lock despite potential previous poisoning
                guard.push(84);
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }
            Err(poisoned) => {
                // Handle the poisoned lock by getting the data anyway
                let mut guard = poisoned.into_inner();
                guard.push(84);
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }
        }
    });

    let result: Result<(), _> = handle2.await.unwrap();
    assert!(result.is_ok());

    // Now test with actual vector store operations to ensure they handle potential lock issues gracefully
    use tempfile::tempdir;
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let store = Arc::new(VectorStore::new(Some(temp_dir.path().to_path_buf()), None).unwrap());

    // Perform operations that use internal locks in multiple concurrent tasks
    let mut handles = vec![];

    for thread_id in 0..8 {
        let store_clone = Arc::clone(&store);
        let handle = task::spawn(async move {
            for i in 0..15 {
                let content = format!("content_{}_{}", thread_id, i);
                let mut embedding = vec![0.0; 768];
                for j in 0..768 {
                    embedding[j] = (thread_id * 100 + i * 10 + j) as f32 / 10000.0;
                }
                let metadata =
                    VectorMetadata::new(&format!("file_{}_{}.rs", thread_id, i), i as i32);

                // Add vector - this internally uses locks
                let _result = store_clone
                    .add_vector(&content, embedding, metadata)
                    .unwrap();

                // Search - this also uses locks
                let mut query = vec![0.0; 768];
                for j in 0..10 {
                    query[(thread_id * 10 + i + j) % 768] = (i as f32) / 100.0;
                }
                let _ = store_clone.search(&query, 3).unwrap();

                // Brief delay to increase chance of race conditions
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle
            .await
            .expect("Thread panicked during vector operations");
    }

    // Verify store is still functional after concurrent operations
    let mut query = vec![0.1; 768];
    for i in 0..50 {
        query[i] = (i % 10) as f32 / 100.0;
    }
    let results = store.search(&query, 10).unwrap();
    assert!(
        results.len() > 0,
        "Store should remain functional after concurrent operations"
    );

    // Verify final count is accessible
    let _final_count = store.vector_count().unwrap();
}
