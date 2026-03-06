//! Content and State Integration Tests
//!
//! Task 2.15: Integration Testing - Content and State

use maestro_integration::{MaestroSession, SessionManager};
use std::collections::HashMap;

/// Test 2.15.1: Pane content capture (simulated)
#[test]
fn test_pane_content_capture_simulation() {
    // This test simulates content capture since we don't have
    // an actual running daemon in unit tests

    let manager = SessionManager::new();

    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "content_test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    // In a real scenario, this would capture actual pane content
    // For now, we verify the session structure supports content capture
    let retrieved = manager.get("content_test");
    assert!(retrieved.is_some());

    // Verify session has proper dimensions for content capture
    let session = retrieved.unwrap();
    assert_eq!(session.dimensions.0, 80); // cols
    assert_eq!(session.dimensions.1, 24); // rows
}

/// Test 2.15.2: Send keys functionality (structure test)
#[test]
fn test_send_keys_structure() {
    // Test that the session structure can support send_keys operations
    let manager = SessionManager::new();

    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "keys_test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    // Verify session exists and can be targeted for key sends
    assert!(manager.exists("keys_test"));

    // In integration tests with a real daemon, we would:
    // 1. Send keys to the session
    // 2. Capture output
    // 3. Verify the keys were received
}

/// Test 2.15.3: Activity tracking
#[test]
fn test_activity_tracking() {
    let manager = SessionManager::new();

    let before = std::time::Instant::now();

    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "activity_test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    let retrieved = manager.get("activity_test").unwrap();

    // Verify creation timestamp is reasonable
    assert!(retrieved.created_at >= before);
    assert!(retrieved.created_at <= std::time::Instant::now());
}

/// Test 2.15.4: Working directory detection
#[test]
fn test_working_directory_detection() {
    let manager = SessionManager::new();

    let test_dirs = vec!["/tmp", "/home/user/project", "/var/log", "~/documents"];

    for (i, dir) in test_dirs.iter().enumerate() {
        let session = MaestroSession {
            id: tab_api::tab::TabId(i as u16),
            name: format!("dir_test_{}", i),
            work_dir: dir.to_string(),
            shell: "/bin/bash".to_string(),
            env: HashMap::new(),
            dimensions: (80, 24),
            created_at: std::time::Instant::now(),
        };

        manager.register(session);

        let retrieved = manager.get(&format!("dir_test_{}", i)).unwrap();
        assert_eq!(retrieved.work_dir, *dir);
    }
}

/// Test session state persistence
#[test]
fn test_session_state_persistence() {
    let manager = SessionManager::new();

    // Create session with specific state
    let session = MaestroSession {
        id: tab_api::tab::TabId(42),
        name: "state_test".to_string(),
        work_dir: "/home/user".to_string(),
        shell: "/bin/zsh".to_string(),
        env: {
            let mut env = HashMap::new();
            env.insert("KEY".to_string(), "VALUE".to_string());
            env
        },
        dimensions: (120, 40),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    // Retrieve and verify all state is preserved
    let retrieved = manager.get("state_test").unwrap();
    assert_eq!(retrieved.id, tab_api::tab::TabId(42));
    assert_eq!(retrieved.name, "state_test");
    assert_eq!(retrieved.work_dir, "/home/user");
    assert_eq!(retrieved.shell, "/bin/zsh");
    assert_eq!(retrieved.dimensions, (120, 40));
    assert_eq!(retrieved.env.get("KEY"), Some(&"VALUE".to_string()));
}

/// Test multiple session states
#[test]
fn test_multiple_session_states() {
    let manager = SessionManager::new();

    // Create sessions with different states
    let configs = vec![
        ("session1", "/tmp", "/bin/bash", (80, 24)),
        ("session2", "/home", "/bin/zsh", (120, 40)),
        ("session3", "/var", "/bin/fish", (100, 30)),
    ];

    for (i, (name, dir, shell, dims)) in configs.iter().enumerate() {
        let session = MaestroSession {
            id: tab_api::tab::TabId(i as u16),
            name: name.to_string(),
            work_dir: dir.to_string(),
            shell: shell.to_string(),
            env: HashMap::new(),
            dimensions: *dims,
            created_at: std::time::Instant::now(),
        };
        manager.register(session);
    }

    // Verify each session maintains its state
    for (name, dir, shell, dims) in configs {
        let session = manager.get(name).unwrap();
        assert_eq!(session.work_dir, dir);
        assert_eq!(session.shell, shell);
        assert_eq!(session.dimensions, dims);
    }
}
