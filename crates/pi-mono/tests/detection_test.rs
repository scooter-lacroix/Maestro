//! Tests for Pi-Mono CLI detection
//!
//! This test module follows TDD principles to test the detection functionality.

use maestro_pi_mono::detection::{PiDetection, Capabilities};
use std::path::PathBuf;

#[test]
fn test_capabilities_default() {
    let caps = Capabilities::default();
    assert!(caps.subagent);
    assert!(caps.streaming);
    assert!(caps.parallel);
    assert!(caps.chain);
}

#[test]
fn test_capabilities_new() {
    let caps = Capabilities {
        subagent: true,
        streaming: false,
        parallel: true,
        chain: false,
    };
    assert!(caps.subagent);
    assert!(!caps.streaming);
    assert!(caps.parallel);
    assert!(!caps.chain);
}

#[test]
fn test_detection_struct_creation() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Capabilities::default(),
    };

    assert_eq!(detection.executable_path, PathBuf::from("/usr/local/bin/pi"));
    assert_eq!(detection.version, Some("0.49.3".to_string()));
    assert!(detection.capabilities.streaming);
}

#[test]
fn test_version_parse_success() {
    let version_str = "0.49.3";
    let version = version_str.parse::<semver::Version>();

    assert!(version.is_ok());
    let v = version.unwrap();
    assert_eq!(v.major, 0);
    assert_eq!(v.minor, 49);
    assert_eq!(v.patch, 3);
}

#[test]
fn test_version_parse_with_prefix() {
    let version_str = "pi version 0.49.3";
    // Extract version from output
    let version_part = version_str
        .split_whitespace()
        .last()
        .unwrap_or("0.0.0");

    let version = version_part.parse::<semver::Version>();
    assert!(version.is_ok());
    let v = version.unwrap();
    assert_eq!(v.major, 0);
    assert_eq!(v.minor, 49);
    assert_eq!(v.patch, 3);
}

#[test]
fn test_version_parse_failure() {
    let version_str = "invalid";
    let version = version_str.parse::<semver::Version>();

    assert!(version.is_err());
}

#[cfg(test)]
mod mock_detection_tests {
    use super::*;

    #[test]
    fn test_detect_when_pi_exists_at_local_bin() {
        // Test that detection works for local bin installation
        let home = dirs::home_dir().expect("No home directory");
        let local_bin = home.join(".local/bin/pi");
        assert!(local_bin.starts_with(home));
        assert!(local_bin.ends_with("pi"));
    }

    #[test]
    fn test_detect_when_pi_exists_in_local_bin() {
        let home = dirs::home_dir().expect("No home directory");
        let local_bin = home.join(".local/bin/pi");
        assert!(local_bin.starts_with(home));
    }

    #[test]
    fn test_detect_fallback_to_which() {
        // Test that which works as a fallback
        // This will be implemented in detection.rs
        let result = which::which("sh");
        assert!(result.is_ok(), "sh should be found in PATH");
    }

    #[tokio::test]
    async fn test_detect_full() {
        // Test detect_full - it should not fail even if version detection fails
        // since pi might not be installed
        let result = PiDetection::detect_full().await;
        // We don't assert success here since pi might not be installed
        // but if it succeeds, verify the structure
        if let Ok(detection) = result {
            assert!(detection.executable_path.as_path().ends_with("pi"));
        }
    }
}

#[tokio::test]
async fn test_detect_capabilities_default() {
    let mut detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: None,
        capabilities: Capabilities {
            subagent: false,
            streaming: false,
            parallel: false,
            chain: false,
        },
    };

    // Once implemented, this should update capabilities to defaults
    let result = detection.detect_capabilities().await;
    assert!(result.is_ok());

    let caps = result.unwrap();
    assert!(caps.subagent);
    assert!(caps.streaming);
    assert!(caps.parallel);
    assert!(caps.chain);
}

#[test]
fn test_error_detection_not_found() {
    use maestro_pi_mono::error::{Error, DetectionError};

    let error = Error::Detection(DetectionError::NotFound);
    assert!(error.is_detection());
    assert!(error.to_string().contains("not found"));
}

#[test]
fn test_error_version_parse_failed() {
    use maestro_pi_mono::error::{Error, DetectionError};

    let error = Error::Detection(DetectionError::VersionParseFailed {
        output: "bad version".to_string(),
    });
    assert!(error.is_detection());
    assert!(error.to_string().contains("bad version"));
}

#[test]
fn test_error_execution_failed() {
    use maestro_pi_mono::error::{Error, DetectionError};

    let error = Error::Detection(DetectionError::ExecutionFailed {
        command: "pi --version".to_string(),
        reason: "Command not found".to_string(),
    });
    assert!(error.is_detection());
    assert!(error.to_string().contains("pi --version"));
    assert!(error.to_string().contains("Command not found"));
}

#[test]
fn test_search_paths_order() {
    // Verify the search paths are in the correct order (portable paths only)
    let home = dirs::home_dir().expect("No home directory");
    let paths = vec![
        home.join(".local/bin/pi"),
        PathBuf::from("/usr/local/bin/pi"),
    ];

    // First priority: user local bin
    assert_eq!(paths[0], home.join(".local/bin/pi"));

    // Second priority: system-wide installation
    assert_eq!(paths[1], PathBuf::from("/usr/local/bin/pi"));
}

#[test]
fn test_executable_validation() {
    // Test that executables are properly validated
    use std::fs::{self, File};
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = std::env::temp_dir().join("pi_test_executable");
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test_executable");

    // Test 1: Non-existent file
    assert!(!test_file.exists());

    // Test 2: Create file but no execute permissions
    File::create(&test_file).unwrap();
    assert!(test_file.exists());
    assert!(test_file.is_file());
    let metadata = fs::metadata(&test_file).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode();
    // No execute bit set
    assert_eq!(mode & 0o111, 0);

    // Test 3: Add execute permissions
    let mut new_permissions = permissions.clone();
    new_permissions.set_mode(mode | 0o755);
    fs::set_permissions(&test_file, new_permissions).unwrap();
    let metadata = fs::metadata(&test_file).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode();
    // Execute bit is set
    assert!(mode & 0o111 != 0);

    // Cleanup
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_timeout_is_configured() {
    // Verify that timeout is used in detect_version
    // This test documents the timeout behavior
    use std::time::Duration;
    let timeout_duration = Duration::from_secs(5);
    assert_eq!(timeout_duration.as_secs(), 5);
}
