//! # Configuration file I/O for Pi-Mono
//!
//! This module handles loading, saving, and managing Pi-Mono configuration files.
//!
//! ## Configuration Directory
//!
//! The default configuration directory is `~/.maestro/config/` and the default
//! configuration file is `pi-mono.yaml`.
//!
//! ## Example
//!
//! ```rust,no_run
//! use maestro_pi_mono::config::io;
//!
//! // Load configuration (creates default if doesn't exist)
//! let config = io::load_config().unwrap();
//!
//! // Save configuration
//! io::save_config(&config).unwrap();
//! ```

use crate::config::models::{PiMonoConfig, ProviderConfig, ExecutionSettings};
use crate::error::{Result, Error, ConfigError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::Write;

/// Configuration directory name within the user's home directory
const CONFIG_DIR_NAME: &str = ".maestro";
/// Subdirectory within config directory for Pi-Mono configs
const CONFIG_SUBDIR: &str = "config";
/// Default configuration file name
const CONFIG_FILE_NAME: &str = "pi-mono.yaml";

/// Get the default config directory path
///
/// Returns `~/.maestro/config/` or an error if the home directory cannot be determined.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
///
/// let config_dir = io::config_dir().unwrap();
/// assert!(config_dir.ends_with(".maestro/config"));
/// ```
pub fn config_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| Error::Config(ConfigError::InvalidPath {
            path: "Home directory not found".to_string(),
        }))?;

    Ok(home_dir.join(CONFIG_DIR_NAME).join(CONFIG_SUBDIR))
}

/// Get the config file path
///
/// Returns `~/.maestro/config/pi-mono.yaml` or an error if the home directory cannot be determined.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
///
/// let config_path = io::config_path().unwrap();
/// assert!(config_path.ends_with("pi-mono.yaml"));
/// ```
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

/// Ensure config directory exists
///
/// Creates the `~/.maestro/config/` directory if it doesn't exist.
/// Returns the path to the config directory.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
///
/// let config_dir = io::ensure_config_dir().unwrap();
/// assert!(config_dir.exists());
/// ```
pub fn ensure_config_dir() -> Result<PathBuf> {
    let dir = config_dir()?;

    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| Error::Config(ConfigError::LoadFailed {
            location: dir.to_string_lossy().to_string(),
            reason: format!("failed to create directory: {}", e),
        }))?;
    }

    Ok(dir)
}

/// Load config from file, or return default if file doesn't exist
///
/// Attempts to load `~/.maestro/config/pi-mono.yaml`. If the file doesn't exist,
/// returns a default configuration. If the file exists but is invalid, returns an error.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
///
/// let config = io::load_config().unwrap();
/// assert!(config.enabled);
/// ```
pub fn load_config() -> Result<PiMonoConfig> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(default_config());
    }

    load_config_from_path(&path)
}

/// Load config from a specific path
///
/// Loads configuration from the specified path. Returns an error if the file
/// doesn't exist or contains invalid YAML.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
/// use std::path::Path;
///
/// let config = io::load_config_from_path(Path::new("/path/to/config.yaml"));
/// ```
pub fn load_config_from_path(path: &Path) -> Result<PiMonoConfig> {
    let contents = std::fs::read_to_string(path).map_err(|e| Error::Config(ConfigError::LoadFailed {
        location: path.to_string_lossy().to_string(),
        reason: format!("failed to read file: {}", e),
    }))?;

    let config: PiMonoConfig = serde_yaml::from_str(&contents)
        .map_err(|e| Error::Config(ConfigError::LoadFailed {
            location: path.to_string_lossy().to_string(),
            reason: format!("failed to parse YAML: {}", e),
        }))?;

    validate_config(&config)?;

    Ok(config)
}

/// Save config to file
///
/// Saves configuration to `~/.maestro/config/pi-mono.yaml`. Creates the config
/// directory if it doesn't exist.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
/// use maestro_pi_mono::config::models::PiMonoConfig;
///
/// let config = PiMonoConfig::default();
/// io::save_config(&config).unwrap();
/// ```
pub fn save_config(config: &PiMonoConfig) -> Result<()> {
    let path = config_path()?;
    save_config_to_path(config, &path)
}

/// Save config to a specific path
///
/// Saves configuration to the specified path. Creates parent directories if needed.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
/// use maestro_pi_mono::config::models::PiMonoConfig;
/// use std::path::Path;
///
/// let config = PiMonoConfig::default();
/// io::save_config_to_path(&config, Path::new("/custom/path/config.yaml"));
/// ```
pub fn save_config_to_path(config: &PiMonoConfig, path: &Path) -> Result<()> {
    validate_config(config)?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Config(ConfigError::LoadFailed {
                location: parent.to_string_lossy().to_string(),
                reason: format!("failed to create directory: {}", e),
            }))?;
        }
    }

    let yaml = serde_yaml::to_string(config)
        .map_err(|e| Error::Config(ConfigError::LoadFailed {
            location: path.to_string_lossy().to_string(),
            reason: format!("failed to serialize YAML: {}", e),
        }))?;

    let mut file = std::fs::File::create(path).map_err(|e| Error::Config(ConfigError::LoadFailed {
        location: path.to_string_lossy().to_string(),
        reason: format!("failed to create file: {}", e),
    }))?;

    file.write_all(yaml.as_bytes())
        .map_err(|e| Error::Config(ConfigError::LoadFailed {
            location: path.to_string_lossy().to_string(),
            reason: format!("failed to write file: {}", e),
        }))?;

    file.flush()
        .map_err(|e| Error::Config(ConfigError::LoadFailed {
            location: path.to_string_lossy().to_string(),
            reason: format!("failed to flush file: {}", e),
        }))?;

    Ok(())
}

/// Generate default configuration
///
/// Creates a default configuration with sensible defaults including
/// all standard providers configured but not enabled.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
///
/// let config = io::default_config();
/// assert!(config.enabled);
/// assert_eq!(config.version, "1.0");
/// assert_eq!(config.providers.len(), 5);
/// ```
pub fn default_config() -> PiMonoConfig {
    let mut providers = HashMap::new();

    providers.insert("anthropic".to_string(), ProviderConfig {
        display_name: "Anthropic".to_string(),
        is_configured: false,
        env_var: "ANTHROPIC_API_KEY".to_string(),
    });

    providers.insert("openai".to_string(), ProviderConfig {
        display_name: "OpenAI".to_string(),
        is_configured: false,
        env_var: "OPENAI_API_KEY".to_string(),
    });

    providers.insert("google".to_string(), ProviderConfig {
        display_name: "Google".to_string(),
        is_configured: false,
        env_var: "GEMINI_API_KEY".to_string(),
    });

    providers.insert("groq".to_string(), ProviderConfig {
        display_name: "Groq".to_string(),
        is_configured: false,
        env_var: "GROQ_API_KEY".to_string(),
    });

    providers.insert("openrouter".to_string(), ProviderConfig {
        display_name: "OpenRouter".to_string(),
        is_configured: false,
        env_var: "OPENROUTER_API_KEY".to_string(),
    });

    PiMonoConfig {
        version: "1.0".to_string(),
        enabled: true,
        path: None,
        version_info: None,
        providers,
        model_preferences: Vec::new(),
        role_assignments: HashMap::new(),
        settings: ExecutionSettings::default(),
    }
}

/// Validate configuration
///
/// Validates that the configuration has valid values for all required fields.
/// Returns an error if validation fails.
///
/// # Example
///
/// ```rust
/// use maestro_pi_mono::config::io;
/// use maestro_pi_mono::config::models::PiMonoConfig;
///
/// let config = PiMonoConfig::default();
/// io::validate_config(&config).unwrap(); // Ok for valid config
/// ```
pub fn validate_config(config: &PiMonoConfig) -> Result<()> {
    // Validate version is not empty
    if config.version.is_empty() {
        return Err(Error::Config(ConfigError::MissingField {
            field: "version".to_string(),
        }));
    }

    // Validate timeout is reasonable (1 second to 1 hour)
    if config.settings.timeout < 1 || config.settings.timeout > 3600 {
        return Err(Error::Config(ConfigError::LoadFailed {
            location: "settings.timeout".to_string(),
            reason: format!("timeout must be between 1 and 3600 seconds, got {}", config.settings.timeout),
        }));
    }

    // Validate parallel_limit is reasonable (1 to 64)
    if config.settings.parallel_limit < 1 || config.settings.parallel_limit > 64 {
        return Err(Error::Config(ConfigError::LoadFailed {
            location: "settings.parallel_limit".to_string(),
            reason: format!("parallel_limit must be between 1 and 64, got {}", config.settings.parallel_limit),
        }));
    }

    // Validate provider configurations
    for (name, provider) in &config.providers {
        if provider.display_name.is_empty() {
            return Err(Error::Config(ConfigError::MissingField {
                field: format!("providers.{}.display_name", name),
            }));
        }
        if provider.env_var.is_empty() {
            return Err(Error::Config(ConfigError::MissingField {
                field: format!("providers.{}.env_var", name),
            }));
        }
    }

    // Validate model preferences have non-empty model_id and provider
    for (idx, pref) in config.model_preferences.iter().enumerate() {
        if pref.model_id.is_empty() {
            return Err(Error::Config(ConfigError::MissingField {
                field: format!("model_preferences[{}].model_id", idx),
            }));
        }
        if pref.provider.is_empty() {
            return Err(Error::Config(ConfigError::MissingField {
                field: format!("model_preferences[{}].provider", idx),
            }));
        }
    }

    // Validate role assignments have non-empty model_id and provider
    for (role, assignment) in &config.role_assignments {
        if assignment.model_id.is_empty() {
            return Err(Error::Config(ConfigError::MissingField {
                field: format!("role_assignments.{}.model_id", role),
            }));
        }
        if assignment.provider.is_empty() {
            return Err(Error::Config(ConfigError::MissingField {
                field: format!("role_assignments.{}.provider", role),
            }));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper function to create a temporary config file with content
    fn create_temp_config_file(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("pi-mono.yaml");
        fs::write(&config_file, content).unwrap();
        (temp_dir, config_file)
    }

    mod config_dir_tests {
        use super::*;

        #[test]
        fn test_config_dir_returns_path() {
            let config_dir = config_dir().unwrap();
            let path_str = config_dir.to_string_lossy();
            assert!(path_str.contains(".maestro"));
            assert!(path_str.contains("config"));
        }

        #[test]
        fn test_config_dir_is_consistent() {
            let dir1 = config_dir().unwrap();
            let dir2 = config_dir().unwrap();
            assert_eq!(dir1, dir2);
        }
    }

    mod config_path_tests {
        use super::*;

        #[test]
        fn test_config_path_returns_yaml_file() {
            let path = config_path().unwrap();
            assert!(path.ends_with("pi-mono.yaml"));
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("pi-mono.yaml"));
        }

        #[test]
        fn test_config_path_is_consistent() {
            let path1 = config_path().unwrap();
            let path2 = config_path().unwrap();
            assert_eq!(path1, path2);
        }

        #[test]
        fn test_config_path_contains_config_dir() {
            let path = config_path().unwrap();
            let config_dir = super::config_dir().unwrap();
            assert!(path.starts_with(config_dir));
        }
    }

    mod ensure_config_dir_tests {
        use super::*;

        #[test]
        fn test_ensure_config_dir_creates_directory() {
            // Note: In a real scenario, we'd use dependency injection
            // For this test, we just verify the function works in isolation
            let result = ensure_config_dir();
            assert!(result.is_ok());
        }

        #[test]
        fn test_ensure_config_dir_returns_existing_directory() {
            // Call ensure_config_dir twice - second call should succeed
            let result1 = ensure_config_dir();
            assert!(result1.is_ok());

            let result2 = ensure_config_dir();
            assert!(result2.is_ok());

            assert_eq!(result1.unwrap(), result2.unwrap());
        }
    }

    mod default_config_tests {
        use super::*;

        #[test]
        fn test_default_config_generates_valid_structure() {
            let config = default_config();

            assert_eq!(config.version, "1.0");
            assert!(config.enabled);
            assert!(config.path.is_none());
            assert!(config.version_info.is_none());
        }

        #[test]
        fn test_default_config_has_all_providers() {
            let config = default_config();

            assert_eq!(config.providers.len(), 5);
            assert!(config.providers.contains_key("anthropic"));
            assert!(config.providers.contains_key("openai"));
            assert!(config.providers.contains_key("google"));
            assert!(config.providers.contains_key("groq"));
            assert!(config.providers.contains_key("openrouter"));
        }

        #[test]
        fn test_default_config_providers_not_configured() {
            let config = default_config();

            for (_name, provider) in &config.providers {
                assert!(!provider.is_configured);
            }
        }

        #[test]
        fn test_default_config_provider_env_vars() {
            let config = default_config();

            assert_eq!(config.providers["anthropic"].env_var, "ANTHROPIC_API_KEY");
            assert_eq!(config.providers["openai"].env_var, "OPENAI_API_KEY");
            assert_eq!(config.providers["google"].env_var, "GEMINI_API_KEY");
            assert_eq!(config.providers["groq"].env_var, "GROQ_API_KEY");
            assert_eq!(config.providers["openrouter"].env_var, "OPENROUTER_API_KEY");
        }

        #[test]
        fn test_default_config_empty_preferences_and_roles() {
            let config = default_config();

            assert!(config.model_preferences.is_empty());
            assert!(config.role_assignments.is_empty());
        }

        #[test]
        fn test_default_config_execution_settings() {
            let config = default_config();

            assert_eq!(config.settings.timeout, 300);
            assert_eq!(config.settings.parallel_limit, 4);
            assert!(config.settings.chain_mode);
            assert!(config.settings.streaming);
        }
    }

    mod validate_config_tests {
        use super::*;

        #[test]
        fn test_validate_config_accepts_valid_config() {
            let config = default_config();
            assert!(validate_config(&config).is_ok());
        }

        #[test]
        fn test_validate_config_rejects_empty_version() {
            let mut config = default_config();
            config.version = "".to_string();
            assert!(validate_config(&config).is_err());
        }

        #[test]
        fn test_validate_config_rejects_timeout_too_low() {
            let mut config = default_config();
            config.settings.timeout = 0;
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("timeout"));
        }

        #[test]
        fn test_validate_config_rejects_timeout_too_high() {
            let mut config = default_config();
            config.settings.timeout = 4000;
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("timeout"));
        }

        #[test]
        fn test_validate_config_accepts_valid_timeout_range() {
            let mut config = default_config();

            for timeout in [1, 60, 300, 600, 3600] {
                config.settings.timeout = timeout;
                assert!(validate_config(&config).is_ok());
            }
        }

        #[test]
        fn test_validate_config_rejects_parallel_limit_too_low() {
            let mut config = default_config();
            config.settings.parallel_limit = 0;
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("parallel_limit"));
        }

        #[test]
        fn test_validate_config_rejects_parallel_limit_too_high() {
            let mut config = default_config();
            config.settings.parallel_limit = 100;
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("parallel_limit"));
        }

        #[test]
        fn test_validate_config_accepts_valid_parallel_limit_range() {
            let mut config = default_config();

            for limit in [1, 2, 4, 8, 16, 32, 64] {
                config.settings.parallel_limit = limit;
                assert!(validate_config(&config).is_ok());
            }
        }

        #[test]
        fn test_validate_config_rejects_empty_provider_display_name() {
            let mut config = default_config();
            config.providers.get_mut("anthropic").unwrap().display_name = "".to_string();
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("display_name"));
        }

        #[test]
        fn test_validate_config_rejects_empty_provider_env_var() {
            let mut config = default_config();
            config.providers.get_mut("openai").unwrap().env_var = "".to_string();
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("env_var"));
        }

        #[test]
        fn test_validate_config_rejects_empty_model_id() {
            let mut config = default_config();
            config.model_preferences.push(crate::config::models::ModelPreference {
                model_id: "".to_string(),
                provider: "anthropic".to_string(),
                tier: crate::config::models::ModelTier::Balanced,
                is_default: false,
            });
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("model_id"));
        }

        #[test]
        fn test_validate_config_rejects_empty_provider_in_preferences() {
            let mut config = default_config();
            config.model_preferences.push(crate::config::models::ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "".to_string(),
                tier: crate::config::models::ModelTier::Balanced,
                is_default: false,
            });
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("provider"));
        }

        #[test]
        fn test_validate_config_rejects_empty_model_id_in_role() {
            let mut config = default_config();
            config.role_assignments.insert(
                "test_role".to_string(),
                crate::config::models::RoleAssignment {
                    model_id: "".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("model_id"));
        }

        #[test]
        fn test_validate_config_rejects_empty_provider_in_role() {
            let mut config = default_config();
            config.role_assignments.insert(
                "test_role".to_string(),
                crate::config::models::RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );
            let result = validate_config(&config);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("provider"));
        }
    }

    mod load_config_from_path_tests {
        use super::*;

        #[test]
        fn test_load_config_from_path_with_valid_yaml() {
            let yaml_content = r#"
version: "1.0"
enabled: true
path: null
version_info: null

providers:
  anthropic:
    display_name: "Anthropic"
    is_configured: false
    env_var: "ANTHROPIC_API_KEY"

model_preferences: []
role_assignments: {}

settings:
  timeout: 300
  parallel_limit: 4
  chain_mode: true
  streaming: true
"#;

            let (_temp_dir, config_file) = create_temp_config_file(yaml_content);
            let config = load_config_from_path(&config_file).unwrap();

            assert_eq!(config.version, "1.0");
            assert!(config.enabled);
            assert_eq!(config.providers.len(), 1);
        }

        #[test]
        fn test_load_config_from_path_with_invalid_yaml() {
            let invalid_yaml = r#"
version: "1.0
enabled: [invalid
"#;

            let (_temp_dir, config_file) = create_temp_config_file(invalid_yaml);
            let result = load_config_from_path(&config_file);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("parse"));
        }

        #[test]
        fn test_load_config_from_path_nonexistent_file() {
            let temp_dir = TempDir::new().unwrap();
            let nonexistent_path = temp_dir.path().join("nonexistent.yaml");

            let result = load_config_from_path(&nonexistent_path);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("read"));
        }

        #[test]
        fn test_load_config_from_path_with_validation_error() {
            let yaml_content = r#"
version: ""
enabled: true
path: null
version_info: null

providers: {}
model_preferences: []
role_assignments: {}

settings:
  timeout: 300
  parallel_limit: 4
  chain_mode: true
  streaming: true
"#;

            let (_temp_dir, config_file) = create_temp_config_file(yaml_content);
            let result = load_config_from_path(&config_file);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("version"));
        }

        #[test]
        fn test_load_config_from_path_preserves_all_fields() {
            let yaml_content = r#"
version: "1.0"
enabled: true
path: "/usr/local/bin/pi"
version_info: "0.49.3"

providers:
  anthropic:
    display_name: "Anthropic"
    is_configured: true
    env_var: "ANTHROPIC_API_KEY"
  openai:
    display_name: "OpenAI"
    is_configured: false
    env_var: "OPENAI_API_KEY"

model_preferences:
  - model_id: "claude-sonnet-4-5"
    provider: "anthropic"
    tier: "Balanced"
    is_default: true
  - model_id: "gpt-4o"
    provider: "openai"
    tier: "Reasoning"
    is_default: false

role_assignments:
  scout:
    model_id: "claude-haiku-4-5"
    provider: "anthropic"
    fallback_models:
      - "gpt-4o-mini"
    use_reasoning: null
  architect:
    model_id: "claude-sonnet-4-5"
    provider: "anthropic"
    fallback_models: null
    use_reasoning: true

settings:
  timeout: 600
  parallel_limit: 8
  chain_mode: false
  streaming: false
"#;

            let (_temp_dir, config_file) = create_temp_config_file(yaml_content);
            let config = load_config_from_path(&config_file).unwrap();

            assert_eq!(config.version, "1.0");
            assert!(config.enabled);
            assert_eq!(config.path.as_ref().unwrap(), "/usr/local/bin/pi");
            assert_eq!(config.version_info.as_ref().unwrap(), "0.49.3");
            assert_eq!(config.providers.len(), 2);
            assert_eq!(config.model_preferences.len(), 2);
            assert_eq!(config.role_assignments.len(), 2);
            assert_eq!(config.settings.timeout, 600);
            assert_eq!(config.settings.parallel_limit, 8);
            assert!(!config.settings.chain_mode);
            assert!(!config.settings.streaming);
        }
    }

    mod save_config_tests {
        use super::*;

        #[test]
        fn test_save_config_creates_valid_yaml() {
            let temp_dir = TempDir::new().unwrap();
            let config_file = temp_dir.path().join("test-config.yaml");

            let config = default_config();
            save_config_to_path(&config, &config_file).unwrap();

            assert!(config_file.exists());

            // Read it back and verify
            let loaded = load_config_from_path(&config_file).unwrap();
            assert_eq!(loaded.version, config.version);
            assert_eq!(loaded.enabled, config.enabled);
            assert_eq!(loaded.providers.len(), config.providers.len());
        }

        #[test]
        fn test_save_config_creates_parent_directory() {
            let temp_dir = TempDir::new().unwrap();
            let nested_dir = temp_dir.path().join("nested").join("directory");
            let config_file = nested_dir.join("config.yaml");

            assert!(!nested_dir.exists());

            let config = default_config();
            save_config_to_path(&config, &config_file).unwrap();

            assert!(nested_dir.exists());
            assert!(config_file.exists());
        }

        #[test]
        fn test_save_config_roundtrip() {
            let temp_dir = TempDir::new().unwrap();
            let config_file = temp_dir.path().join("roundtrip.yaml");

            let original_config = {
                let mut config = default_config();
                config.enabled = false;
                config.path = Some("/custom/path".to_string());
                config.version_info = Some("2.0.0".to_string());
                config.providers.get_mut("anthropic").unwrap().is_configured = true;
                config.settings.timeout = 450;
                config.settings.parallel_limit = 6;
                config
            };

            save_config_to_path(&original_config, &config_file).unwrap();
            let loaded_config = load_config_from_path(&config_file).unwrap();

            assert_eq!(loaded_config.version, original_config.version);
            assert_eq!(loaded_config.enabled, original_config.enabled);
            assert_eq!(loaded_config.path, original_config.path);
            assert_eq!(loaded_config.version_info, original_config.version_info);
            assert_eq!(loaded_config.providers.len(), original_config.providers.len());
            assert_eq!(loaded_config.providers["anthropic"].is_configured, true);
            assert_eq!(loaded_config.settings.timeout, 450);
            assert_eq!(loaded_config.settings.parallel_limit, 6);
        }

        #[test]
        fn test_save_config_rejects_invalid_config() {
            let temp_dir = TempDir::new().unwrap();
            let config_file = temp_dir.path().join("invalid.yaml");

            let mut invalid_config = default_config();
            invalid_config.version = "".to_string();

            let result = save_config_to_path(&invalid_config, &config_file);
            assert!(result.is_err());
            assert!(!config_file.exists()); // File should not be created
        }

        #[test]
        fn test_save_config_overwrites_existing_file() {
            let temp_dir = TempDir::new().unwrap();
            let config_file = temp_dir.path().join("overwrite.yaml");

            // Write initial content
            fs::write(&config_file, "old content").unwrap();

            let config = default_config();
            save_config_to_path(&config, &config_file).unwrap();

            let content = fs::read_to_string(&config_file).unwrap();
            assert!(!content.contains("old content"));
            assert!(content.contains("version:"));
        }
    }

    mod load_config_tests {
        use super::*;

        #[test]
        fn test_load_config_returns_default_when_missing() {
            // This test uses the actual config_path which likely doesn't exist in test env
            // So it should return default config
            let config = load_config().unwrap();
            // Should get default config since file likely doesn't exist
            assert_eq!(config.version, "1.0");
            assert!(config.enabled);
        }

        #[test]
        fn test_load_config_validates_loaded_config() {
            // We can't easily test this without polluting the actual config directory,
            // but we test the validation logic in load_config_from_path tests
            // This is more of an integration test scenario
            let result = load_config();
            assert!(result.is_ok());
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_full_config_lifecycle() {
            let temp_dir = TempDir::new().unwrap();
            let config_file = temp_dir.path().join("lifecycle.yaml");

            // 1. Create default config
            let config = default_config();
            assert_eq!(config.providers.len(), 5);

            // 2. Modify it
            let mut modified_config = config;
            modified_config.enabled = false;
            modified_config.settings.timeout = 500;

            // 3. Save it
            save_config_to_path(&modified_config, &config_file).unwrap();
            assert!(config_file.exists());

            // 4. Load it back
            let loaded_config = load_config_from_path(&config_file).unwrap();
            assert!(!loaded_config.enabled);
            assert_eq!(loaded_config.settings.timeout, 500);

            // 5. Validate it
            assert!(validate_config(&loaded_config).is_ok());
        }

        #[test]
        fn test_config_with_all_fields_populated() {
            let temp_dir = TempDir::new().unwrap();
            let config_file = temp_dir.path().join("full.yaml");

            let full_config = {
                let mut config = default_config();
                config.path = Some("/usr/bin/pi".to_string());
                config.version_info = Some("1.0.0".to_string());

                // Configure all providers
                for (_name, provider) in config.providers.iter_mut() {
                    provider.is_configured = true;
                }

                // Add model preferences
                config.model_preferences.push(crate::config::models::ModelPreference {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    tier: crate::config::models::ModelTier::Balanced,
                    is_default: true,
                });

                config
            };

            save_config_to_path(&full_config, &config_file).unwrap();
            let loaded = load_config_from_path(&config_file).unwrap();

            assert_eq!(loaded.path.unwrap(), "/usr/bin/pi");
            assert_eq!(loaded.version_info.unwrap(), "1.0.0");
            assert_eq!(loaded.providers.len(), 5);
            assert!(loaded.providers.values().all(|p| p.is_configured));
            assert_eq!(loaded.model_preferences.len(), 1);
        }

        #[test]
        fn test_multiple_config_versions() {
            let temp_dir = TempDir::new().unwrap();

            let versions = vec!["1.0", "1.1", "2.0"];

            for version in versions {
                let config_file = temp_dir.path().join(format!("config-v{}.yaml", version));

                let mut config = default_config();
                config.version = version.to_string();

                save_config_to_path(&config, &config_file).unwrap();
                let loaded = load_config_from_path(&config_file).unwrap();

                assert_eq!(loaded.version, version);
            }
        }

        #[test]
        fn test_config_serialization_deserialization_consistency() {
            let config = default_config();

            // Serialize to YAML
            let yaml = serde_yaml::to_string(&config).unwrap();

            // Deserialize back
            let deserialized: PiMonoConfig = serde_yaml::from_str(&yaml).unwrap();

            assert_eq!(config.version, deserialized.version);
            assert_eq!(config.enabled, deserialized.enabled);
            assert_eq!(config.providers.len(), deserialized.providers.len());
            assert_eq!(config.settings.timeout, deserialized.settings.timeout);
        }
    }
}
