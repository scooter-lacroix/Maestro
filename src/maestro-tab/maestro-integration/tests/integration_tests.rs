//! Integration Tests for Maestro-tab Integration
//!
//! Tests session lifecycle, content capture, and daemon interaction.

use maestro_integration::{MaestroSession, SessionManager, TransparencyConfig};
use std::collections::HashMap;

/// Test that SessionManager can register and manage sessions
#[test]
fn test_session_manager_lifecycle() {
    let manager = SessionManager::new();

    // Initially no sessions
    assert_eq!(manager.count(), 0);
    assert!(!manager.exists("test"));

    // Create and register a session
    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    // Session should now exist
    assert_eq!(manager.count(), 1);
    assert!(manager.exists("test"));

    // Retrieve session
    let retrieved = manager.get("test");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "test");
    assert_eq!(retrieved.work_dir, "/tmp");

    // Unregister session
    let removed = manager.unregister("test");
    assert!(removed.is_some());
    assert_eq!(manager.count(), 0);
    assert!(!manager.exists("test"));
}

/// Test session listing functionality
#[test]
fn test_session_listing() {
    let manager = SessionManager::new();

    // Create multiple sessions
    for i in 0..3 {
        let session = MaestroSession {
            id: tab_api::tab::TabId(i as u16),
            name: format!("session{}", i),
            work_dir: "/tmp".to_string(),
            shell: "/bin/bash".to_string(),
            env: HashMap::new(),
            dimensions: (80, 24),
            created_at: std::time::Instant::now(),
        };
        manager.register(session);
    }

    // List sessions
    let sessions = manager.list_sessions();
    assert_eq!(sessions.len(), 3);
    assert!(sessions.contains(&"session0".to_string()));
    assert!(sessions.contains(&"session1".to_string()));
    assert!(sessions.contains(&"session2".to_string()));
}

/// Test session retrieval by ID
#[test]
fn test_session_retrieval_by_id() {
    let manager = SessionManager::new();

    let session = MaestroSession {
        id: tab_api::tab::TabId(42),
        name: "by_id_test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    // Retrieve by ID
    let retrieved = manager.get_by_id(tab_api::tab::TabId(42));
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "by_id_test");

    // Non-existent ID
    let not_found = manager.get_by_id(tab_api::tab::TabId(999));
    assert!(not_found.is_none());
}

/// Test transparency configuration
#[test]
fn test_transparency_config() {
    // Default config
    let default = TransparencyConfig::default();
    assert!(default.enabled);
    assert_eq!(default.alpha, 200);

    // Custom alpha
    let custom = TransparencyConfig::new(128);
    assert!(custom.enabled);
    assert_eq!(custom.alpha, 128);

    // Disabled config
    let disabled = TransparencyConfig::disabled();
    assert!(!disabled.enabled);
    assert_eq!(disabled.alpha, 255);
}

/// Test transparency sequence generation
#[test]
fn test_transparency_sequence_generation() {
    use maestro_integration::transparency::{reset_transparency_sequence, transparency_sequence};

    // Test various alpha values
    for alpha in [0, 64, 128, 192, 255] {
        let seq = transparency_sequence(alpha);
        assert!(seq.starts_with('\x1b'));
        assert!(seq.contains("111"));
        assert!(seq.contains(&alpha.to_string()));
    }

    // Reset sequence
    let reset = reset_transparency_sequence();
    assert!(reset.starts_with('\x1b'));
    assert!(reset.contains("111"));
}

/// Test that session manager handles concurrent access
#[test]
fn test_session_manager_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let manager = Arc::new(SessionManager::new());
    let mut handles = vec![];

    // Spawn multiple threads to register sessions
    for i in 0..10 {
        let mgr = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let session = MaestroSession {
                id: tab_api::tab::TabId(i),
                name: format!("thread_session_{}", i),
                work_dir: "/tmp".to_string(),
                shell: "/bin/bash".to_string(),
                env: HashMap::new(),
                dimensions: (80, 24),
                created_at: std::time::Instant::now(),
            };
            mgr.register(session);
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all sessions were registered
    assert_eq!(manager.count(), 10);
}

/// Test session metadata conversion
#[test]
fn test_session_metadata_conversion() {
    use tab_api::tab::{TabId, TabMetadata};

    let metadata = TabMetadata {
        id: TabId(1),
        name: "test_tab".to_string(),
        doc: Some("/home/user/project".to_string()),
        env: {
            let mut env = HashMap::new();
            env.insert("EDITOR".to_string(), "vim".to_string());
            env
        },
        dimensions: (120, 40),
        shell: "/bin/zsh".to_string(),
        dir: "/home/user/project".to_string(),
        selected: 1234567890,
    };

    let session = MaestroSession::from_metadata(&metadata);

    assert_eq!(session.id, TabId(1));
    assert_eq!(session.name, "test_tab");
    assert_eq!(session.work_dir, "/home/user/project");
    assert_eq!(session.dimensions, (120, 40));
    assert!(session.env.contains_key("EDITOR"));
}

/// Test error types
#[test]
fn test_error_types() {
    use maestro_integration::MaestroTabError;

    let daemon_error = MaestroTabError::DaemonNotRunning;
    assert!(daemon_error.to_string().contains("Daemon"));

    let session_error = MaestroTabError::SessionNotFound("test".to_string());
    assert!(session_error.to_string().contains("test"));

    let auth_error = MaestroTabError::AuthFailed;
    assert!(auth_error.to_string().contains("Authentication"));
}
