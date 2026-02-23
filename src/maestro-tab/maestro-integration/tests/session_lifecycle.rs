//! Session Lifecycle Integration Tests
//!
//! Task 2.14: Integration Testing - Session Lifecycle

use maestro_integration::{MaestroSession, SessionManager};
use std::collections::HashMap;

/// Test 2.14.2: Session creation
#[test]
fn test_session_creation() {
    let manager = SessionManager::new();

    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "new_session".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    assert!(manager.exists("new_session"));
    assert_eq!(manager.count(), 1);
}

/// Test 2.14.3: Session attachment (simulated)
#[test]
fn test_session_attachment_simulation() {
    let manager = SessionManager::new();

    // Create session
    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "attach_test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    // Simulate attachment by retrieving session
    let retrieved = manager.get("attach_test");
    assert!(retrieved.is_some());

    let session = retrieved.unwrap();
    assert_eq!(session.name, "attach_test");
    assert_eq!(session.dimensions, (80, 24));
}

/// Test 2.14.4: Session termination
#[test]
fn test_session_termination() {
    let manager = SessionManager::new();

    // Create session
    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "terminate_test".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);
    assert!(manager.exists("terminate_test"));

    // Terminate (unregister)
    let removed = manager.unregister("terminate_test");
    assert!(removed.is_some());
    assert!(!manager.exists("terminate_test"));
    assert_eq!(manager.count(), 0);
}

/// Test 2.14.5: Session listing
#[test]
fn test_session_listing() {
    let manager = SessionManager::new();

    // Create multiple sessions
    for i in 0..5 {
        let session = MaestroSession {
            id: tab_api::tab::TabId(i),
            name: format!("list_session_{}", i),
            work_dir: "/tmp".to_string(),
            shell: "/bin/bash".to_string(),
            env: HashMap::new(),
            dimensions: (80, 24),
            created_at: std::time::Instant::now(),
        };
        manager.register(session);
    }

    // List all sessions
    let sessions = manager.list_sessions();
    assert_eq!(sessions.len(), 5);

    // Verify all expected sessions exist
    for i in 0..5 {
        assert!(sessions.contains(&format!("list_session_{}", i)));
    }
}

/// Test session creation with duplicate names (last one wins)
#[test]
fn test_session_duplicate_names() {
    let manager = SessionManager::new();

    // Create first session
    let session1 = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "duplicate".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env: HashMap::new(),
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session1);
    assert_eq!(manager.count(), 1);

    // Create second session with same name
    let session2 = MaestroSession {
        id: tab_api::tab::TabId(2),
        name: "duplicate".to_string(),
        work_dir: "/home".to_string(), // Different work_dir
        shell: "/bin/zsh".to_string(), // Different shell
        env: HashMap::new(),
        dimensions: (120, 40),
        created_at: std::time::Instant::now(),
    };

    manager.register(session2);

    // Should still be 1 session (last one overwrote)
    assert_eq!(manager.count(), 1);

    // Verify it's the second one
    let retrieved = manager.get("duplicate").unwrap();
    assert_eq!(retrieved.work_dir, "/home");
    assert_eq!(retrieved.shell, "/bin/zsh");
    assert_eq!(retrieved.dimensions, (120, 40));
}

/// Test session lifecycle with environment variables
#[test]
fn test_session_with_environment() {
    let manager = SessionManager::new();

    let mut env = HashMap::new();
    env.insert("EDITOR".to_string(), "vim".to_string());
    env.insert("HOME".to_string(), "/home/test".to_string());
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());

    let session = MaestroSession {
        id: tab_api::tab::TabId(1),
        name: "env_session".to_string(),
        work_dir: "/tmp".to_string(),
        shell: "/bin/bash".to_string(),
        env,
        dimensions: (80, 24),
        created_at: std::time::Instant::now(),
    };

    manager.register(session);

    let retrieved = manager.get("env_session").unwrap();
    assert_eq!(retrieved.env.get("EDITOR"), Some(&"vim".to_string()));
    assert_eq!(retrieved.env.get("HOME"), Some(&"/home/test".to_string()));
    assert_eq!(retrieved.env.len(), 3);
}
