//! # Model configuration structures for Pi-Mono
//!
//! This module defines the data structures for model configuration,
//! provider settings, role assignments, and execution preferences.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model tier classification
///
/// Represents different categories of models based on their capabilities
/// and intended use cases within the Pi-Mono system.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::models::ModelTier;
///
/// let tier = ModelTier::Balanced;
/// assert_eq!(tier, ModelTier::Balanced);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelTier {
    /// High-reasoning models for complex problem-solving
    Reasoning,
    /// Fast models for quick responses and simple tasks
    Fast,
    /// Balanced models offering good performance and quality
    Balanced,
    /// Models specialized in vision/image tasks
    Vision,
    /// Models optimized for code generation and analysis
    Coding,
}

/// Individual model preference
///
/// Defines a preferred model with its provider, tier classification,
/// and default status.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::models::{ModelPreference, ModelTier};
///
/// let pref = ModelPreference {
///     model_id: "claude-sonnet-4-5".to_string(),
///     provider: "anthropic".to_string(),
///     tier: ModelTier::Balanced,
///     is_default: true,
/// };
/// assert_eq!(pref.model_id, "claude-sonnet-4-5");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreference {
    /// Unique identifier for the model
    pub model_id: String,
    /// Provider name (e.g., "anthropic", "openai")
    pub provider: String,
    /// Model tier classification
    pub tier: ModelTier,
    /// Whether this is a default model for its tier
    pub is_default: bool,
}

/// Provider configuration
///
/// Contains configuration and authentication status for a model provider.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::models::ProviderConfig;
///
/// let provider = ProviderConfig {
///     display_name: "Anthropic".to_string(),
///     is_configured: true,
///     env_var: "ANTHROPIC_API_KEY".to_string(),
/// };
/// assert!(provider.is_configured);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Human-readable display name
    pub display_name: String,
    /// Whether the provider has valid credentials configured
    pub is_configured: bool,
    /// Environment variable name for API key
    pub env_var: String,
}

/// Role assignment with model mapping
///
/// Maps specific Pi-Mono roles to models with optional fallback
/// configurations.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::models::RoleAssignment;
///
/// let role = RoleAssignment {
///     model_id: "claude-sonnet-4-5".to_string(),
///     provider: "anthropic".to_string(),
///     fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
///     use_reasoning: Some(true),
/// };
/// assert_eq!(role.use_reasoning, Some(true));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    /// Primary model to use for this role
    pub model_id: String,
    /// Provider for the primary model
    pub provider: String,
    /// Optional fallback models if primary fails
    pub fallback_models: Option<Vec<String>>,
    /// Whether to use reasoning mode (if applicable)
    pub use_reasoning: Option<bool>,
}

/// Execution settings
///
/// Global execution configuration for Pi-Mono operations.
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::models::ExecutionSettings;
///
/// let settings = ExecutionSettings {
///     timeout: 300,
///     parallel_limit: 4,
///     chain_mode: true,
///     streaming: true,
/// };
/// assert_eq!(settings.timeout, 300);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSettings {
    /// Request timeout in seconds
    pub timeout: u64,
    /// Maximum parallel operations
    pub parallel_limit: u64,
    /// Enable chain mode for sequential operations
    pub chain_mode: bool,
    /// Enable streaming responses
    pub streaming: bool,
}

impl Default for ExecutionSettings {
    /// Creates default execution settings with reasonable defaults
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::config::models::ExecutionSettings;
    ///
    /// let settings = ExecutionSettings::default();
    /// assert_eq!(settings.timeout, 300);
    /// assert_eq!(settings.parallel_limit, 4);
    /// ```
    fn default() -> Self {
        Self {
            timeout: 300,
            parallel_limit: 4,
            chain_mode: true,
            streaming: true,
        }
    }
}

/// Main Pi-Mono configuration
///
/// Complete configuration structure for Pi-Mono integration,
/// including providers, models, roles, and execution settings.
///
/// # Examples
///
/// Creating a default configuration:
///
/// ```rust
/// use maestro_pi_mono::config::models::PiMonoConfig;
///
/// let config = PiMonoConfig::default();
/// assert!(config.enabled);
/// assert_eq!(config.version, "1.0");
/// ```
///
/// Creating with custom providers:
///
/// ```rust
/// use maestro_pi_mono::config::models::{PiMonoConfig, ProviderConfig};
/// use std::collections::HashMap;
///
/// let mut providers = HashMap::new();
/// providers.insert(
///     "anthropic".to_string(),
///     ProviderConfig {
///         display_name: "Anthropic".to_string(),
///         is_configured: true,
///         env_var: "ANTHROPIC_API_KEY".to_string(),
///     },
/// );
///
/// let config = PiMonoConfig {
///     providers,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiMonoConfig {
    /// Configuration schema version
    pub version: String,
    /// Whether Pi-Mono integration is enabled
    pub enabled: bool,
    /// Optional path to Pi-Mono installation
    pub path: Option<String>,
    /// Version information for Pi-Mono binary
    pub version_info: Option<String>,
    /// Configured providers
    pub providers: HashMap<String, ProviderConfig>,
    /// Model preferences by tier
    pub model_preferences: Vec<ModelPreference>,
    /// Role-to-model assignments
    pub role_assignments: HashMap<String, RoleAssignment>,
    /// Execution settings
    pub settings: ExecutionSettings,
}

impl Default for PiMonoConfig {
    /// Creates a default Pi-Mono configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use maestro_pi_mono::config::models::PiMonoConfig;
    ///
    /// let config = PiMonoConfig::default();
    /// assert_eq!(config.version, "1.0");
    /// assert!(config.enabled);
    /// assert!(config.providers.is_empty());
    /// assert!(config.model_preferences.is_empty());
    /// ```
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            enabled: true,
            path: None,
            version_info: None,
            providers: HashMap::new(),
            model_preferences: Vec::new(),
            role_assignments: HashMap::new(),
            settings: ExecutionSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ModelTier enum tests
    mod model_tier_tests {
        use super::*;

        #[test]
        fn test_model_tier_variants() {
            let reasoning = ModelTier::Reasoning;
            let fast = ModelTier::Fast;
            let balanced = ModelTier::Balanced;
            let vision = ModelTier::Vision;
            let coding = ModelTier::Coding;

            assert_eq!(reasoning, ModelTier::Reasoning);
            assert_eq!(fast, ModelTier::Fast);
            assert_eq!(balanced, ModelTier::Balanced);
            assert_eq!(vision, ModelTier::Vision);
            assert_eq!(coding, ModelTier::Coding);
        }

        #[test]
        fn test_model_tier_equality() {
            assert_eq!(ModelTier::Reasoning, ModelTier::Reasoning);
            assert_ne!(ModelTier::Reasoning, ModelTier::Fast);
        }

        #[test]
        fn test_model_tier_clone() {
            let tier = ModelTier::Balanced;
            let cloned = tier.clone();
            assert_eq!(tier, cloned);
        }
    }

    // ModelPreference tests
    mod model_preference_tests {
        use super::*;

        #[test]
        fn test_model_preference_creation() {
            let pref = ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: true,
            };

            assert_eq!(pref.model_id, "claude-sonnet-4-5");
            assert_eq!(pref.provider, "anthropic");
            assert_eq!(pref.tier, ModelTier::Balanced);
            assert!(pref.is_default);
        }

        #[test]
        fn test_model_preference_serialization() {
            let pref = ModelPreference {
                model_id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                tier: ModelTier::Reasoning,
                is_default: false,
            };

            let json = serde_json::to_string(&pref).unwrap();
            let deserialized: ModelPreference = serde_json::from_str(&json).unwrap();

            assert_eq!(pref.model_id, deserialized.model_id);
            assert_eq!(pref.provider, deserialized.provider);
            assert_eq!(pref.tier, deserialized.tier);
            assert_eq!(pref.is_default, deserialized.is_default);
        }

        #[test]
        fn test_model_preference_clone() {
            let pref = ModelPreference {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Fast,
                is_default: true,
            };

            let cloned = pref.clone();
            assert_eq!(pref.model_id, cloned.model_id);
            assert_eq!(pref.provider, cloned.provider);
            assert_eq!(pref.tier, cloned.tier);
            assert_eq!(pref.is_default, cloned.is_default);
        }
    }

    // ProviderConfig tests
    mod provider_config_tests {
        use super::*;

        #[test]
        fn test_provider_config_creation() {
            let provider = ProviderConfig {
                display_name: "Anthropic".to_string(),
                is_configured: true,
                env_var: "ANTHROPIC_API_KEY".to_string(),
            };

            assert_eq!(provider.display_name, "Anthropic");
            assert!(provider.is_configured);
            assert_eq!(provider.env_var, "ANTHROPIC_API_KEY");
        }

        #[test]
        fn test_provider_config_serialization() {
            let provider = ProviderConfig {
                display_name: "OpenAI".to_string(),
                is_configured: false,
                env_var: "OPENAI_API_KEY".to_string(),
            };

            let json = serde_json::to_string(&provider).unwrap();
            let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(provider.display_name, deserialized.display_name);
            assert_eq!(provider.is_configured, deserialized.is_configured);
            assert_eq!(provider.env_var, deserialized.env_var);
        }

        #[test]
        fn test_multiple_providers() {
            let providers = vec![
                ProviderConfig {
                    display_name: "Anthropic".to_string(),
                    is_configured: true,
                    env_var: "ANTHROPIC_API_KEY".to_string(),
                },
                ProviderConfig {
                    display_name: "OpenAI".to_string(),
                    is_configured: true,
                    env_var: "OPENAI_API_KEY".to_string(),
                },
                ProviderConfig {
                    display_name: "Google".to_string(),
                    is_configured: false,
                    env_var: "GEMINI_API_KEY".to_string(),
                },
            ];

            assert_eq!(providers.len(), 3);
            assert!(providers[0].is_configured);
            assert!(providers[1].is_configured);
            assert!(!providers[2].is_configured);
        }
    }

    // RoleAssignment tests
    mod role_assignment_tests {
        use super::*;

        #[test]
        fn test_role_assignment_creation() {
            let role = RoleAssignment {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
                use_reasoning: Some(true),
            };

            assert_eq!(role.model_id, "claude-sonnet-4-5");
            assert_eq!(role.provider, "anthropic");
            assert_eq!(role.fallback_models, Some(vec!["gpt-4o-mini".to_string()]));
            assert_eq!(role.use_reasoning, Some(true));
        }

        #[test]
        fn test_role_assignment_without_fallbacks() {
            let role = RoleAssignment {
                model_id: "claude-haiku-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: None,
                use_reasoning: None,
            };

            assert_eq!(role.model_id, "claude-haiku-4-5");
            assert!(role.fallback_models.is_none());
            assert!(role.use_reasoning.is_none());
        }

        #[test]
        fn test_role_assignment_serialization() {
            let role = RoleAssignment {
                model_id: "claude-opus-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: Some(vec!["claude-sonnet-4-5".to_string(), "gpt-4o".to_string()]),
                use_reasoning: Some(false),
            };

            let json = serde_json::to_string(&role).unwrap();
            let deserialized: RoleAssignment = serde_json::from_str(&json).unwrap();

            assert_eq!(role.model_id, deserialized.model_id);
            assert_eq!(role.provider, deserialized.provider);
            assert_eq!(role.fallback_models, deserialized.fallback_models);
            assert_eq!(role.use_reasoning, deserialized.use_reasoning);
        }

        #[test]
        fn test_multiple_fallback_models() {
            let role = RoleAssignment {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                fallback_models: Some(vec![
                    "gpt-4o-mini".to_string(),
                    "claude-haiku-4-5".to_string(),
                    "gemini-flash".to_string(),
                ]),
                use_reasoning: None,
            };

            let fallbacks = role.fallback_models.unwrap();
            assert_eq!(fallbacks.len(), 3);
            assert_eq!(fallbacks[0], "gpt-4o-mini");
            assert_eq!(fallbacks[1], "claude-haiku-4-5");
            assert_eq!(fallbacks[2], "gemini-flash");
        }
    }

    // ExecutionSettings tests
    mod execution_settings_tests {
        use super::*;

        #[test]
        fn test_execution_settings_default() {
            let settings = ExecutionSettings::default();

            assert_eq!(settings.timeout, 300);
            assert_eq!(settings.parallel_limit, 4);
            assert!(settings.chain_mode);
            assert!(settings.streaming);
        }

        #[test]
        fn test_execution_settings_custom() {
            let settings = ExecutionSettings {
                timeout: 600,
                parallel_limit: 8,
                chain_mode: false,
                streaming: false,
            };

            assert_eq!(settings.timeout, 600);
            assert_eq!(settings.parallel_limit, 8);
            assert!(!settings.chain_mode);
            assert!(!settings.streaming);
        }

        #[test]
        fn test_execution_settings_serialization() {
            let settings = ExecutionSettings {
                timeout: 120,
                parallel_limit: 2,
                chain_mode: true,
                streaming: false,
            };

            let json = serde_json::to_string(&settings).unwrap();
            let deserialized: ExecutionSettings = serde_json::from_str(&json).unwrap();

            assert_eq!(settings.timeout, deserialized.timeout);
            assert_eq!(settings.parallel_limit, deserialized.parallel_limit);
            assert_eq!(settings.chain_mode, deserialized.chain_mode);
            assert_eq!(settings.streaming, deserialized.streaming);
        }
    }

    // PiMonoConfig tests
    mod pi_mono_config_tests {
        use super::*;

        #[test]
        fn test_pi_mono_config_default() {
            let config = PiMonoConfig::default();

            assert_eq!(config.version, "1.0");
            assert!(config.enabled);
            assert!(config.path.is_none());
            assert!(config.version_info.is_none());
            assert!(config.providers.is_empty());
            assert!(config.model_preferences.is_empty());
            assert!(config.role_assignments.is_empty());
            assert_eq!(config.settings.timeout, 300);
        }

        #[test]
        fn test_pi_mono_config_full() {
            let mut providers = HashMap::new();
            providers.insert(
                "anthropic".to_string(),
                ProviderConfig {
                    display_name: "Anthropic".to_string(),
                    is_configured: true,
                    env_var: "ANTHROPIC_API_KEY".to_string(),
                },
            );

            let model_preferences = vec![ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: true,
            }];

            let mut role_assignments = HashMap::new();
            role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
                    use_reasoning: None,
                },
            );

            let config = PiMonoConfig {
                version: "1.0".to_string(),
                enabled: true,
                path: Some("/home/stan/pi-mono/pi".to_string()),
                version_info: Some("0.49.3".to_string()),
                providers,
                model_preferences,
                role_assignments,
                settings: ExecutionSettings::default(),
            };

            assert_eq!(config.version, "1.0");
            assert!(config.enabled);
            assert_eq!(config.path.unwrap(), "/home/stan/pi-mono/pi");
            assert_eq!(config.version_info.unwrap(), "0.49.3");
            assert_eq!(config.providers.len(), 1);
            assert_eq!(config.model_preferences.len(), 1);
            assert_eq!(config.role_assignments.len(), 1);
        }

        #[test]
        fn test_pi_mono_config_yaml_serialization() {
            let mut providers = HashMap::new();
            providers.insert(
                "anthropic".to_string(),
                ProviderConfig {
                    display_name: "Anthropic".to_string(),
                    is_configured: true,
                    env_var: "ANTHROPIC_API_KEY".to_string(),
                },
            );
            providers.insert(
                "openai".to_string(),
                ProviderConfig {
                    display_name: "OpenAI".to_string(),
                    is_configured: true,
                    env_var: "OPENAI_API_KEY".to_string(),
                },
            );

            let model_preferences = vec![
                ModelPreference {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    tier: ModelTier::Balanced,
                    is_default: true,
                },
                ModelPreference {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    tier: ModelTier::Fast,
                    is_default: true,
                },
            ];

            let mut role_assignments = HashMap::new();
            role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
                    use_reasoning: None,
                },
            );
            role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: Some(true),
                },
            );

            let config = PiMonoConfig {
                version: "1.0".to_string(),
                enabled: true,
                path: Some("/home/stan/pi-mono/pi".to_string()),
                version_info: Some("0.49.3".to_string()),
                providers,
                model_preferences,
                role_assignments,
                settings: ExecutionSettings {
                    timeout: 300,
                    parallel_limit: 4,
                    chain_mode: true,
                    streaming: true,
                },
            };

            // Test JSON serialization
            let json = serde_json::to_string_pretty(&config).unwrap();
            let deserialized: PiMonoConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(config.version, deserialized.version);
            assert_eq!(config.enabled, deserialized.enabled);
            assert_eq!(config.path, deserialized.path);
            assert_eq!(config.version_info, deserialized.version_info);
            assert_eq!(config.providers.len(), deserialized.providers.len());
            assert_eq!(
                config.model_preferences.len(),
                deserialized.model_preferences.len()
            );
            assert_eq!(
                config.role_assignments.len(),
                deserialized.role_assignments.len()
            );
        }

        #[test]
        fn test_pi_mono_config_with_all_providers() {
            let mut providers = HashMap::new();
            providers.insert(
                "anthropic".to_string(),
                ProviderConfig {
                    display_name: "Anthropic".to_string(),
                    is_configured: true,
                    env_var: "ANTHROPIC_API_KEY".to_string(),
                },
            );
            providers.insert(
                "openai".to_string(),
                ProviderConfig {
                    display_name: "OpenAI".to_string(),
                    is_configured: true,
                    env_var: "OPENAI_API_KEY".to_string(),
                },
            );
            providers.insert(
                "google".to_string(),
                ProviderConfig {
                    display_name: "Google".to_string(),
                    is_configured: false,
                    env_var: "GEMINI_API_KEY".to_string(),
                },
            );
            providers.insert(
                "groq".to_string(),
                ProviderConfig {
                    display_name: "Groq".to_string(),
                    is_configured: false,
                    env_var: "GROQ_API_KEY".to_string(),
                },
            );
            providers.insert(
                "openrouter".to_string(),
                ProviderConfig {
                    display_name: "OpenRouter".to_string(),
                    is_configured: false,
                    env_var: "OPENROUTER_API_KEY".to_string(),
                },
            );

            let config = PiMonoConfig {
                providers,
                ..Default::default()
            };

            assert_eq!(config.providers.len(), 5);
            assert!(config.providers["anthropic"].is_configured);
            assert!(config.providers["openai"].is_configured);
            assert!(!config.providers["google"].is_configured);
            assert!(!config.providers["groq"].is_configured);
            assert!(!config.providers["openrouter"].is_configured);
        }

        #[test]
        fn test_pi_mono_config_all_roles() {
            let mut role_assignments = HashMap::new();
            role_assignments.insert(
                "scout".to_string(),
                RoleAssignment {
                    model_id: "claude-haiku-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
                    use_reasoning: None,
                },
            );
            role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: Some(true),
                },
            );
            role_assignments.insert(
                "critic".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );
            role_assignments.insert(
                "kraken".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let config = PiMonoConfig {
                role_assignments,
                ..Default::default()
            };

            assert_eq!(config.role_assignments.len(), 4);
            assert!(config.role_assignments.contains_key("scout"));
            assert!(config.role_assignments.contains_key("architect"));
            assert!(config.role_assignments.contains_key("critic"));
            assert!(config.role_assignments.contains_key("kraken"));
        }

        #[test]
        fn test_pi_mono_config_clone() {
            let config = PiMonoConfig {
                version: "1.0".to_string(),
                enabled: true,
                path: Some("/test/path".to_string()),
                version_info: Some("0.49.3".to_string()),
                providers: HashMap::new(),
                model_preferences: vec![],
                role_assignments: HashMap::new(),
                settings: ExecutionSettings::default(),
            };

            let cloned = config.clone();
            assert_eq!(config.version, cloned.version);
            assert_eq!(config.enabled, cloned.enabled);
            assert_eq!(config.path, cloned.path);
        }
    }
}
