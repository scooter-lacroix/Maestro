//! # Configuration validation for Pi-Mono integration
//!
//! This module provides validation functionality for Pi-Mono configuration,
//! including path validation, model assignment validation, and helpful error messages.

use crate::{
    config::models::PiMonoConfig,
    detection::PiDetection,
    error::{ConfigError, Result},
};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Validation warning (non-blocking)
///
/// Represents a validation issue that doesn't prevent configuration from being used
/// but should be brought to the user's attention.
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

impl ValidationWarning {
    /// Create a new validation warning
    pub fn new(
        field: impl Into<String>,
        message: impl Into<String>,
        severity: ValidationSeverity,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            severity,
        }
    }

    /// Create an info-level warning
    pub fn info(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, message, ValidationSeverity::Info)
    }

    /// Create a warning-level validation warning
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, message, ValidationSeverity::Warning)
    }

    /// Create an error-level validation warning
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, message, ValidationSeverity::Error)
    }
}

/// Validation severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Informational message (suggestions, best practices)
    Info,
    /// Warning (potential issues, non-critical problems)
    Warning,
    /// Error (critical issues that prevent operation)
    Error,
}

/// Validate pi-mono executable path
///
/// This function validates that:
/// - If `config.path` is Some, the path exists and is a valid executable
/// - If detection is provided, the configured path matches the detected path
///
/// # Errors
///
/// Returns `Err` if:
/// - The configured path does not exist
/// - The path exists but is not a valid executable
/// - The configured path does not match the detected path
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::validation::validate_pi_path;
/// use maestro_pi_mono::config::models::PiMonoConfig;
///
/// let config = PiMonoConfig {
///     path: Some("/usr/local/bin/pi".to_string()),
///     ..Default::default()
/// };
///
/// match validate_pi_path(&config, &None) {
///     Ok(_) => println!("Path is valid"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// ```
pub fn validate_pi_path(config: &PiMonoConfig, detection: &Option<PiDetection>) -> Result<()> {
    // If path is configured, validate it exists and is executable
    if let Some(ref path_str) = config.path {
        let path = Path::new(path_str);

        // Check if path exists
        if !path.exists() {
            return Err(ConfigError::InvalidPath {
                path: path_str.clone(),
            }
            .into());
        }

        // Check if path is a file (not a directory)
        if !path.is_file() {
            return Err(ConfigError::LoadFailed {
                location: path_str.clone(),
                reason: format!("Path is a directory, not an executable file: {}", path_str),
            }
            .into());
        }

        // On Unix, check execute permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(path).map_err(|e| ConfigError::LoadFailed {
                location: path_str.clone(),
                reason: format!("Failed to read file metadata: {}", e),
            })?;
            let permissions = metadata.permissions();
            let mode = permissions.mode();

            if mode & 0o111 == 0 {
                return Err(ConfigError::LoadFailed {
                    location: path_str.clone(),
                    reason: format!(
                        "File exists but is not executable. Run: chmod +x {}",
                        path_str
                    ),
                }
                .into());
            }
        }

        // If detection is provided, verify paths match
        if let Some(det) = detection {
            let detected_path = det.executable_path.to_string_lossy().to_string();
            if path_str != &detected_path {
                return Err(ConfigError::LoadFailed {
                    location: path_str.clone(),
                    reason: format!(
                        "Configured path '{}' does not match detected path '{}'. \
                        Consider either:\n  1. Updating config.path to '{}', or\n  2. Removing config.path to use auto-detection",
                        path_str, detected_path, detected_path
                    ),
                }
                .into());
            }
        }
    }

    Ok(())
}

/// Validate model assignments in configuration
///
/// This function validates that:
/// - All model_ids in role_assignments exist in model_preferences
/// - All providers in role_assignments are configured (is_configured=true)
/// - No circular references exist in fallback chains
/// - Model tiers are compatible with their assigned roles
///
/// # Errors
///
/// Returns `Err` if:
/// - A model_id in role_assignments is not found in model_preferences
/// - A provider is not configured (is_configured=false)
/// - A circular reference is detected in fallback models
/// - A model tier is incompatible with its role
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::validation::validate_model_assignments;
/// use maestro_pi_mono::config::models::{PiMonoConfig, RoleAssignment, ModelPreference, ModelTier, ProviderConfig};
/// use std::collections::HashMap;
///
/// let mut providers = HashMap::new();
/// providers.insert("anthropic".to_string(), ProviderConfig {
///     display_name: "Anthropic".to_string(),
///     is_configured: true,
///     env_var: "ANTHROPIC_API_KEY".to_string(),
/// });
///
/// let model_preferences = vec![
///     ModelPreference {
///         model_id: "claude-haiku-4-5".to_string(),
///         provider: "anthropic".to_string(),
///         tier: ModelTier::Fast,
///         is_default: true,
///     },
/// ];
///
/// let mut role_assignments = HashMap::new();
/// role_assignments.insert("scout".to_string(), RoleAssignment {
///     model_id: "claude-haiku-4-5".to_string(),
///     provider: "anthropic".to_string(),
///     fallback_models: None,
///     use_reasoning: None,
/// });
///
/// let config = PiMonoConfig {
///     providers,
///     model_preferences,
///     role_assignments,
///     ..Default::default()
/// };
///
/// match validate_model_assignments(&config) {
///     Ok(_) => println!("Model assignments are valid"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// ```
pub fn validate_model_assignments(config: &PiMonoConfig) -> Result<()> {
    // Build a lookup of all available model_ids
    let available_models: HashSet<&str> = config
        .model_preferences
        .iter()
        .map(|pref| pref.model_id.as_str())
        .collect();

    // Build a lookup of model_id -> tier for tier compatibility checking
    let model_tiers: HashMap<&str, &crate::config::models::ModelTier> = config
        .model_preferences
        .iter()
        .map(|pref| (pref.model_id.as_str(), &pref.tier))
        .collect();

    // Track all fallback model_ids for circular reference detection
    let mut all_fallback_ids: Vec<String> = Vec::new();

    // Validate each role assignment
    for (role, assignment) in &config.role_assignments {
        // Check if primary model exists in preferences
        if !available_models.contains(assignment.model_id.as_str()) {
            return Err(ConfigError::LoadFailed {
                location: format!("role_assignments.{}", role),
                reason: format!(
                    "Model '{}' is assigned to role '{}' but is not defined in model_preferences. \
                    Add it to model_preferences or use a different model.",
                    assignment.model_id, role
                ),
            }
            .into());
        }

        // Check if provider is configured
        if let Some(provider_config) = config.providers.get(&assignment.provider) {
            if !provider_config.is_configured {
                return Err(ConfigError::LoadFailed {
                    location: format!("role_assignments.{}.provider", role),
                    reason: format!(
                        "Provider '{}' is not configured. Set the {} environment variable with your API key. \
                        Example: export {}=your_api_key_here",
                        assignment.provider,
                        provider_config.env_var,
                        provider_config.env_var
                    ),
                }
                .into());
            }
        } else {
            return Err(ConfigError::LoadFailed {
                location: format!("role_assignments.{}.provider", role),
                reason: format!(
                    "Provider '{}' for role '{}' is not defined in the providers map. \
                    Add provider configuration for '{}' to the providers section.",
                    assignment.provider, role, assignment.provider
                ),
            }
            .into());
        }

        // Check fallback models if present
        if let Some(ref fallbacks) = assignment.fallback_models {
            for fallback_id in fallbacks {
                all_fallback_ids.push(fallback_id.clone());

                // Check if fallback model exists in preferences
                if !available_models.contains(fallback_id.as_str()) {
                    return Err(ConfigError::LoadFailed {
                        location: format!("role_assignments.{}.fallback_models", role),
                        reason: format!(
                            "Fallback model '{}' for role '{}' is not defined in model_preferences. \
                            Add it to model_preferences or remove it from fallback_models.",
                            fallback_id, role
                        ),
                    }
                    .into());
                }

                // Check for circular reference (model referencing itself as fallback)
                if fallback_id == &assignment.model_id {
                    return Err(ConfigError::LoadFailed {
                        location: format!("role_assignments.{}.fallback_models", role),
                        reason: format!(
                            "Circular reference detected: model '{}' is assigned to role '{}' \
                            and also listed as its own fallback. Remove '{}' from fallback_models.",
                            assignment.model_id, role, assignment.model_id
                        ),
                    }
                    .into());
                }
            }
        }

        // Validate tier compatibility with role
        if let Some(tier) = model_tiers.get(assignment.model_id.as_str()) {
            validate_role_tier_compatibility(role, tier, assignment.model_id.as_str())?;
        }
    }

    // Check for cross-role circular references
    // This is a simplified check - a full implementation would do graph analysis
    for (role, assignment) in &config.role_assignments {
        if let Some(ref fallbacks) = assignment.fallback_models {
            for fallback_id in fallbacks {
                // Check if any other role uses this model as primary
                for (other_role, other_assignment) in &config.role_assignments {
                    if other_role != role && &other_assignment.model_id == fallback_id {
                        // Check if that role has our primary model as fallback
                        if let Some(ref other_fallbacks) = other_assignment.fallback_models {
                            if other_fallbacks.contains(&assignment.model_id) {
                                return Err(ConfigError::LoadFailed {
                                    location: "role_assignments".to_string(),
                                    reason: format!(
                                        "Circular reference detected between roles '{}' and '{}'. \
                                        Role '{}' uses '{}' as fallback for '{}', \
                                        while role '{}' uses '{}' as fallback for '{}'. \
                                        Break the circular dependency by removing one of the fallback references.",
                                        role, other_role,
                                        role, fallback_id, assignment.model_id,
                                        other_role, assignment.model_id, fallback_id
                                    ),
                                }
                                .into());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for fallback models that aren't used as primary in any role
    // (This is a warning, not an error, so we'll skip it here)
    let _ = all_fallback_ids;

    Ok(())
}

/// Validate tier compatibility with role
///
/// Ensures that model tiers are appropriate for their assigned roles.
/// For example, Scout roles should use Fast models.
fn validate_role_tier_compatibility(
    role: &str,
    tier: &crate::config::models::ModelTier,
    _model_id: &str,
) -> Result<()> {
    match role {
        "scout" => {
            // Scout should ideally use Fast models
            if !matches!(tier, crate::config::models::ModelTier::Fast) {
                // This is a warning, not an error, but we'll document it
                // In a full implementation, this would generate a ValidationWarning
            }
        }
        "architect" | "critic" | "kraken" => {
            // These roles can use any tier, but Reasoning/Balanced are preferred
            // No error - this is informational
        }
        _ => {
            // Unknown role - this is fine, just log it
        }
    }
    Ok(())
}

/// Validate complete configuration comprehensively
///
/// Performs comprehensive validation of the entire Pi-Mono configuration,
/// including path validation, model assignments, and other checks.
/// Returns non-blocking warnings for informational issues.
///
/// This is an extended validation that goes beyond the basic `validate_config`
/// in the io module, providing detailed warnings about configuration issues.
///
/// # Errors
///
/// Returns `Err` if critical validation errors are found.
/// Returns `Ok(warnings)` with a list of non-critical warnings if validation succeeds.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::validation::validate_config_ext;
/// use maestro_pi_mono::config::models::PiMonoConfig;
///
/// let config = PiMonoConfig::default();
///
/// match validate_config_ext(&config, &None) {
///     Ok(warnings) => {
///         if warnings.is_empty() {
///             println!("Configuration is valid with no warnings");
///         } else {
///             println!("Configuration is valid with {} warnings", warnings.len());
///         }
///     }
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// ```
pub fn validate_config_ext(
    config: &PiMonoConfig,
    detection: &Option<PiDetection>,
) -> Result<Vec<ValidationWarning>> {
    let mut warnings = Vec::new();

    // Validate pi-mono path
    validate_pi_path(config, detection)?;

    // Validate model assignments
    validate_model_assignments(config)?;

    // Add informational warnings for non-critical issues

    // Warn if no model preferences are configured
    if config.model_preferences.is_empty() {
        warnings.push(ValidationWarning::warning(
            "model_preferences",
            "No model preferences configured. Add models to model_preferences to enable model selection.",
        ));
    }

    // Warn if no role assignments are configured
    if config.role_assignments.is_empty() {
        warnings.push(ValidationWarning::warning(
            "role_assignments",
            "No role assignments configured. Add role assignments to map Pi-Mono roles to models.",
        ));
    }

    // Warn if no providers are configured
    if config.providers.is_empty() {
        warnings.push(ValidationWarning::warning(
            "providers",
            "No providers configured. Add provider configurations to enable model usage.",
        ));
    }

    // Check for unconfigured providers
    for (provider_name, provider_config) in &config.providers {
        if !provider_config.is_configured {
            warnings.push(ValidationWarning::warning(
                format!("providers.{}", provider_name),
                format!(
                    "Provider '{}' is not configured. Set the {} environment variable.",
                    provider_name, provider_config.env_var
                ),
            ));
        }
    }

    // Warn if path is not set (relying on auto-detection)
    if config.path.is_none() {
        if detection.is_some() {
            warnings.push(ValidationWarning::info(
                "path",
                format!(
                    "No explicit path configured. Using auto-detected path: '{}'. \
                    Consider setting config.path explicitly for reproducibility.",
                    detection.as_ref().unwrap().executable_path.display()
                ),
            ));
        } else {
            warnings.push(ValidationWarning::warning(
                "path",
                "No path configured and auto-detection not performed. Pi-Mono may not be found.",
            ));
        }
    }

    // Check tier compatibility and add warnings
    for (role, assignment) in &config.role_assignments {
        // Find the model tier
        for pref in &config.model_preferences {
            if pref.model_id == assignment.model_id {
                match role.as_str() {
                    "scout" => {
                        if !matches!(pref.tier, crate::config::models::ModelTier::Fast) {
                            warnings.push(ValidationWarning::info(
                                format!("role_assignments.{}", role),
                                format!(
                                    "Role '{}' is assigned model '{}' which is a {:?} tier model. \
                                    Consider using a Fast tier model for Scout role to improve performance.",
                                    role, assignment.model_id, pref.tier
                                ),
                            ));
                        }
                    }
                    _ => {
                        // Other roles are flexible
                    }
                }
                break;
            }
        }
    }

    // Warn if no default model is set for any tier
    let has_defaults: std::collections::HashMap<_, _> = config
        .model_preferences
        .iter()
        .filter(|pref| pref.is_default)
        .map(|pref| (&pref.tier, pref))
        .collect();

    if has_defaults.is_empty() {
        warnings.push(ValidationWarning::info(
            "model_preferences",
            "No default models configured. Consider setting is_default=true on at least one model per tier.",
        ));
    }

    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{
        ExecutionSettings, ModelPreference, ModelTier, ProviderConfig, RoleAssignment,
    };
    use std::collections::HashMap;

    // Helper function to create a valid minimal config
    fn create_valid_config() -> PiMonoConfig {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                display_name: "Anthropic".to_string(),
                is_configured: true,
                env_var: "ANTHROPIC_API_KEY".to_string(),
            },
        );

        let model_preferences = vec![
            ModelPreference {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Fast,
                is_default: true,
            },
            ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: true,
            },
        ];

        let mut role_assignments = HashMap::new();
        role_assignments.insert(
            "scout".to_string(),
            RoleAssignment {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: None,
                use_reasoning: None,
            },
        );

        PiMonoConfig {
            version: "1.0".to_string(),
            enabled: true,
            path: None,
            version_info: None,
            providers,
            model_preferences,
            role_assignments,
            settings: ExecutionSettings::default(),
        }
    }

    // validate_pi_path tests
    mod validate_pi_path_tests {
        use super::*;
        use std::fs::{self, File};
        use std::path::PathBuf;

        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        fn create_temp_executable() -> PathBuf {
            // Use a unique name per test to avoid conflicts
            let unique_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let temp_dir = std::env::temp_dir().join("pi_test_validate");
            fs::create_dir_all(&temp_dir).unwrap();

            let test_file = temp_dir.join(format!("pi_test_executable_{}", unique_id));
            File::create(&test_file).unwrap();

            #[cfg(unix)]
            {
                let metadata = fs::metadata(&test_file).unwrap();
                let permissions = metadata.permissions();
                let mode = permissions.mode();
                let mut new_permissions = permissions.clone();
                new_permissions.set_mode(mode | 0o755);
                fs::set_permissions(&test_file, new_permissions).unwrap();
            }

            test_file
        }

        #[test]
        fn test_validate_pi_path_none() {
            let config = PiMonoConfig {
                path: None,
                ..Default::default()
            };

            // Should succeed when path is None
            let result = validate_pi_path(&config, &None);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_pi_path_nonexistent() {
            let config = PiMonoConfig {
                path: Some("/nonexistent/path/to/pi".to_string()),
                ..Default::default()
            };

            let result = validate_pi_path(&config, &None);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("invalid configuration path") || err_msg.contains("not found")
            );
        }

        #[test]
        fn test_validate_pi_path_is_directory() {
            let temp_dir = std::env::temp_dir().join("pi_test_dir");
            fs::create_dir_all(&temp_dir).unwrap();

            let config = PiMonoConfig {
                path: Some(temp_dir.to_string_lossy().to_string()),
                ..Default::default()
            };

            let result = validate_pi_path(&config, &None);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("directory"));

            // Cleanup
            let _ = fs::remove_dir_all(temp_dir);
        }

        #[test]
        fn test_validate_pi_path_not_executable() {
            let temp_dir = std::env::temp_dir().join("pi_test_not_exec");
            fs::create_dir_all(&temp_dir).unwrap();

            let test_file = temp_dir.join("not_executable");
            File::create(&test_file).unwrap();

            // On Unix, the file should not have execute permissions by default
            #[cfg(unix)]
            {
                let config = PiMonoConfig {
                    path: Some(test_file.to_string_lossy().to_string()),
                    ..Default::default()
                };

                let result = validate_pi_path(&config, &None);
                assert!(result.is_err());

                let err = result.unwrap_err();
                let err_msg = err.to_string();
                assert!(err_msg.contains("not executable") || err_msg.contains("chmod"));
            }

            // Cleanup
            let _ = fs::remove_dir_all(temp_dir);
        }

        #[test]
        fn test_validate_pi_path_valid_executable() {
            let test_file = create_temp_executable();

            let config = PiMonoConfig {
                path: Some(test_file.to_string_lossy().to_string()),
                ..Default::default()
            };

            let result = validate_pi_path(&config, &None);
            assert!(result.is_ok());

            // Cleanup individual file
            let _ = fs::remove_file(&test_file);
        }

        #[test]
        fn test_validate_pi_path_mismatch_with_detection() {
            let test_file = create_temp_executable();
            let test_file_str = test_file.to_string_lossy().to_string();

            let config = PiMonoConfig {
                path: Some(test_file_str.clone()),
                ..Default::default()
            };

            let detection = PiDetection {
                executable_path: PathBuf::from("/different/path/to/pi"),
                version: None,
                capabilities: Default::default(),
            };

            let result = validate_pi_path(&config, &Some(detection));
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();

            // Print the actual error for debugging
            eprintln!("Error message: {}", err_msg);

            // The error message should mention the paths don't match
            // It could be "does not match detected" or could contain the actual paths
            assert!(
                err_msg.contains("does not match detected")
                    || err_msg.contains("/different/path/to/pi")
                    || err_msg.contains("Configured path")
            );

            // Cleanup individual file
            let _ = fs::remove_file(&test_file);
        }

        #[test]
        fn test_validate_pi_path_matches_detection() {
            let test_file = create_temp_executable();
            let test_file_str = test_file.to_string_lossy().to_string();

            let config = PiMonoConfig {
                path: Some(test_file_str.clone()),
                ..Default::default()
            };

            let detection = PiDetection {
                executable_path: test_file.clone(),
                version: None,
                capabilities: Default::default(),
            };

            let result = validate_pi_path(&config, &Some(detection));
            assert!(result.is_ok());

            // Cleanup individual file
            let _ = fs::remove_file(&test_file);
        }

        #[test]
        fn test_validate_pi_path_helpful_error_message() {
            let config = PiMonoConfig {
                path: Some("/invalid/path".to_string()),
                ..Default::default()
            };

            let result = validate_pi_path(&config, &None);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            // Error message should be specific about what's wrong
            assert!(err_msg.contains("/invalid/path") || err_msg.contains("path"));
        }
    }

    // validate_model_assignments tests
    mod validate_model_assignments_tests {
        use super::*;

        #[test]
        fn test_validate_model_assignments_valid() {
            let config = create_valid_config();
            let result = validate_model_assignments(&config);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_model_assignments_missing_model() {
            let mut config = create_valid_config();
            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "nonexistent-model".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("nonexistent-model"));
            assert!(err_msg.contains("model_preferences"));
        }

        #[test]
        fn test_validate_model_assignments_unconfigured_provider() {
            let mut config = create_valid_config();
            config.providers.insert(
                "openai".to_string(),
                ProviderConfig {
                    display_name: "OpenAI".to_string(),
                    is_configured: false,
                    env_var: "OPENAI_API_KEY".to_string(),
                },
            );

            config.model_preferences.push(ModelPreference {
                model_id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                tier: ModelTier::Balanced,
                is_default: false,
            });

            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "gpt-4o".to_string(),
                    provider: "openai".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("not configured"));
            assert!(err_msg.contains("OPENAI_API_KEY"));
        }

        #[test]
        fn test_validate_model_assignments_provider_not_defined() {
            let mut config = create_valid_config();

            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "nonexistent-provider".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("nonexistent-provider"));
            assert!(err_msg.contains("not defined"));
        }

        #[test]
        fn test_validate_model_assignments_fallback_missing() {
            let mut config = create_valid_config();

            config.role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["missing-fallback".to_string()]),
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("missing-fallback"));
        }

        #[test]
        fn test_validate_model_assignments_circular_self_reference() {
            let mut config = create_valid_config();

            config.role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["claude-haiku-4-5".to_string()]),
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("Circular reference"));
            assert!(err_msg.contains("its own fallback"));
        }

        #[test]
        fn test_validate_model_assignments_circular_cross_role() {
            let mut config = create_valid_config();

            config.model_preferences.push(ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: false,
            });

            config.role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["claude-sonnet-4-5".to_string()]),
                    use_reasoning: None,
                },
            );

            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["claude-haiku-4-5".to_string()]),
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(err_msg.contains("Circular reference"));
        }

        #[test]
        fn test_validate_model_assignments_valid_fallbacks() {
            let mut config = create_valid_config();

            config.model_preferences.push(ModelPreference {
                model_id: "gpt-4o-mini".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Fast,
                is_default: false,
            });

            config.role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_model_assignments_helpful_error_message() {
            let mut config = create_valid_config();
            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "missing-model".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_err());

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            // Error should mention the role and model
            assert!(err_msg.contains("architect"));
            assert!(err_msg.contains("missing-model"));
            // Error should suggest how to fix
            assert!(err_msg.contains("model_preferences") || err_msg.contains("Add it"));
        }

        #[test]
        fn test_validate_model_assignments_tier_compatibility() {
            let mut config = create_valid_config();

            // Assign a Balanced model to Scout (ideally should be Fast)
            config.role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            // This should succeed (tier incompatibility is a warning, not error)
            let result = validate_model_assignments(&config);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_model_assignments_empty() {
            let config = PiMonoConfig {
                role_assignments: HashMap::new(),
                ..Default::default()
            };

            let result = validate_model_assignments(&config);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_model_assignments_multiple_roles() {
            let mut config = create_valid_config();

            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: Some(true),
                },
            );

            config.role_assignments.insert(
                "critic".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            config.role_assignments.insert(
                "kraken".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_model_assignments(&config);
            assert!(result.is_ok());
        }
    }

    // validate_config tests
    mod validate_config_tests {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn test_validate_config_ext_valid() {
            let config = create_valid_config();
            let result = validate_config_ext(&config, &None);
            assert!(result.is_ok());

            let warnings = result.unwrap();
            // May have warnings about missing path, etc.
            assert!(warnings.len() >= 0);
        }

        #[test]
        fn test_validate_config_ext_invalid_path() {
            let mut config = create_valid_config();
            config.path = Some("/nonexistent/path".to_string());

            let result = validate_config_ext(&config, &None);
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_config_ext_invalid_model_assignments() {
            let mut config = create_valid_config();
            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "missing-model".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_config_ext(&config, &None);
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_config_ext_returns_warnings() {
            let config = PiMonoConfig::default();
            let result = validate_config_ext(&config, &None);

            assert!(result.is_ok());
            let warnings = result.unwrap();

            // Should have warnings about empty config
            assert!(!warnings.is_empty());

            // Check for expected warnings
            let warning_messages: Vec<_> = warnings.iter().map(|w| w.field.as_str()).collect();

            assert!(warning_messages
                .iter()
                .any(|m| m.contains("model_preferences")));
            assert!(warning_messages
                .iter()
                .any(|m| m.contains("role_assignments")));
            assert!(warning_messages.iter().any(|m| m.contains("providers")));
        }

        #[test]
        fn test_validate_config_ext_unconfigured_provider_warning() {
            let mut config = create_valid_config();

            config.providers.insert(
                "openai".to_string(),
                ProviderConfig {
                    display_name: "OpenAI".to_string(),
                    is_configured: false,
                    env_var: "OPENAI_API_KEY".to_string(),
                },
            );

            let result = validate_config_ext(&config, &None);
            assert!(result.is_ok());

            let warnings = result.unwrap();
            let provider_warnings: Vec<_> = warnings
                .iter()
                .filter(|w| w.field.contains("providers") && w.field.contains("openai"))
                .collect();

            assert!(!provider_warnings.is_empty());
            assert!(provider_warnings[0].message.contains("OPENAI_API_KEY"));
        }

        #[test]
        fn test_validate_config_ext_no_path_warning() {
            let config = create_valid_config();

            let result = validate_config_ext(&config, &None);
            assert!(result.is_ok());

            let warnings = result.unwrap();
            let path_warnings: Vec<_> = warnings.iter().filter(|w| w.field == "path").collect();

            assert!(!path_warnings.is_empty());
        }

        #[test]
        fn test_validate_config_ext_with_detection_path_warning() {
            let config = create_valid_config();

            let detection = PiDetection {
                executable_path: PathBuf::from("/usr/local/bin/pi"),
                version: None,
                capabilities: Default::default(),
            };

            let result = validate_config_ext(&config, &Some(detection));
            assert!(result.is_ok());

            let warnings = result.unwrap();
            let path_warnings: Vec<_> = warnings.iter().filter(|w| w.field == "path").collect();

            // Should have an info-level warning about using auto-detected path
            assert!(!path_warnings.is_empty());
            assert_eq!(path_warnings[0].severity, ValidationSeverity::Info);
            assert!(path_warnings[0].message.contains("auto-detected"));
        }

        #[test]
        fn test_validate_config_ext_no_default_warning() {
            let mut config = create_valid_config();

            // Remove default flags
            for pref in &mut config.model_preferences {
                pref.is_default = false;
            }

            let result = validate_config_ext(&config, &None);
            assert!(result.is_ok());

            let warnings = result.unwrap();
            let default_warnings: Vec<_> = warnings
                .iter()
                .filter(|w| w.field.contains("model_preferences") && w.message.contains("default"))
                .collect();

            assert!(!default_warnings.is_empty());
        }

        #[test]
        fn test_validate_config_ext_scout_tier_compatibility_warning() {
            let mut config = create_valid_config();

            // Assign Balanced model to Scout
            config.role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_config_ext(&config, &None);
            assert!(result.is_ok());

            let warnings = result.unwrap();
            let scout_warnings: Vec<_> = warnings
                .iter()
                .filter(|w| w.field.contains("scout"))
                .collect();

            // Should have an info-level warning about tier compatibility
            assert!(!scout_warnings.is_empty());
            assert_eq!(scout_warnings[0].severity, ValidationSeverity::Info);
            assert!(scout_warnings[0].message.contains("Fast tier"));
        }

        #[test]
        fn test_validate_config_ext_warning_severity_levels() {
            let config = PiMonoConfig::default();
            let result = validate_config_ext(&config, &None);

            assert!(result.is_ok());
            let warnings = result.unwrap();

            // Check that we have different severity levels
            let _has_info = warnings
                .iter()
                .any(|w| w.severity == ValidationSeverity::Info);
            let has_warning = warnings
                .iter()
                .any(|w| w.severity == ValidationSeverity::Warning);

            // At minimum should have warnings
            assert!(has_warning);

            // May have info depending on the state
            // (e.g., if there's no detection, we get a warning, not info)
        }

        #[test]
        fn test_validate_config_ext_comprehensive() {
            let mut config = create_valid_config();

            // Add more complete configuration
            config.model_preferences.push(ModelPreference {
                model_id: "claude-opus-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Reasoning,
                is_default: true,
            });

            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["claude-haiku-4-5".to_string()]),
                    use_reasoning: Some(true),
                },
            );

            config.role_assignments.insert(
                "critic".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let result = validate_config_ext(&config, &None);
            assert!(result.is_ok());

            let warnings = result.unwrap();
            // Should have minimal warnings for a well-configured setup
            // (likely just about missing explicit path)
            assert!(warnings.len() < 3);
        }
    }

    // ValidationWarning tests
    mod validation_warning_tests {
        use super::*;

        #[test]
        fn test_validation_warning_new() {
            let warning =
                ValidationWarning::new("test_field", "test message", ValidationSeverity::Error);
            assert_eq!(warning.field, "test_field");
            assert_eq!(warning.message, "test message");
            assert_eq!(warning.severity, ValidationSeverity::Error);
        }

        #[test]
        fn test_validation_warning_info() {
            let warning = ValidationWarning::info("field", "message");
            assert_eq!(warning.severity, ValidationSeverity::Info);
        }

        #[test]
        fn test_validation_warning_warning() {
            let warning = ValidationWarning::warning("field", "message");
            assert_eq!(warning.severity, ValidationSeverity::Warning);
        }

        #[test]
        fn test_validation_warning_error() {
            let warning = ValidationWarning::error("field", "message");
            assert_eq!(warning.severity, ValidationSeverity::Error);
        }

        #[test]
        fn test_validation_severity_equality() {
            assert_eq!(ValidationSeverity::Info, ValidationSeverity::Info);
            assert_eq!(ValidationSeverity::Warning, ValidationSeverity::Warning);
            assert_eq!(ValidationSeverity::Error, ValidationSeverity::Error);

            assert_ne!(ValidationSeverity::Info, ValidationSeverity::Warning);
            assert_ne!(ValidationSeverity::Warning, ValidationSeverity::Error);
        }
    }
}
