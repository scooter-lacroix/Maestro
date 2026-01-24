//! # Model selection logic for Pi-Mono
//!
//! This module provides the `ModelSelector` which intelligently selects appropriate
//! models based on configuration, authentication status, and role requirements.

use crate::{
    config::models::{PiMonoConfig as ModelConfig, ModelTier, ModelPreference},
    discovery::{ModelInfo, ProviderStatus},
    error::{Result, Error},
    error::ConfigError,
};

/// Model selector for choosing appropriate models based on criteria
///
/// The `ModelSelector` provides intelligent model selection that considers:
/// - Tier-based model preferences
/// - Provider authentication status
/// - Role-specific model assignments
/// - Fallback model chains
///
/// # Examples
///
/// ```rust
/// use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ModelTier, ModelPreference, ProviderConfig}};
/// use std::collections::HashMap;
///
/// let mut providers = HashMap::new();
/// providers.insert("anthropic".to_string(), ProviderConfig {
///     display_name: "Anthropic".to_string(),
///     is_configured: true,
///     env_var: "ANTHROPIC_API_KEY".to_string(),
/// });
///
/// let config = PiMonoConfig {
///     providers,
///     model_preferences: vec![
///         ModelPreference {
///             model_id: "claude-sonnet-4-5".to_string(),
///             provider: "anthropic".to_string(),
///             tier: ModelTier::Balanced,
///             is_default: true,
///         },
///     ],
///     ..Default::default()
/// };
///
/// let selector = ModelSelector::new(&config);
/// ```
pub struct ModelSelector<'a> {
    config: &'a ModelConfig,
    available_models: Option<Vec<ModelInfo>>,
    provider_status: Option<Vec<ProviderStatus>>,
}

impl<'a> ModelSelector<'a> {
    /// Create a new model selector with configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::{ModelSelector, models::PiMonoConfig};
    ///
    /// let config = PiMonoConfig::default();
    /// let selector = ModelSelector::new(&config);
    /// ```
    pub fn new(config: &'a ModelConfig) -> Self {
        Self {
            config,
            available_models: None,
            provider_status: None,
        }
    }

    /// Set available models from discovery
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::{ModelSelector, models::PiMonoConfig};
    /// use maestro_pi_mono::discovery::ModelInfo;
    ///
    /// let config = PiMonoConfig::default();
    /// let selector = ModelSelector::new(&config)
    ///     .with_available_models(vec![ModelInfo {
    ///         provider: "anthropic".to_string(),
    ///         model_id: "claude-sonnet-4-5".to_string(),
    ///         context_window: "200k".to_string(),
    ///         max_output: "8k".to_string(),
    ///         supports_thinking: false,
    ///         supports_images: true,
    ///     }]);
    /// ```
    pub fn with_available_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.available_models = Some(models);
        self
    }

    /// Set provider status from discovery
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::{ModelSelector, models::PiMonoConfig};
    /// use maestro_pi_mono::discovery::ProviderStatus;
    ///
    /// let config = PiMonoConfig::default();
    /// let selector = ModelSelector::new(&config)
    ///     .with_provider_status(vec![ProviderStatus {
    ///         provider: "anthropic".to_string(),
    ///         is_configured: true,
    ///         env_var: "ANTHROPIC_API_KEY".to_string(),
    ///     }]);
    /// ```
    pub fn with_provider_status(mut self, status: Vec<ProviderStatus>) -> Self {
        self.provider_status = Some(status);
        self
    }

    /// Select model by tier (filtered by authentication)
    ///
    /// Returns the first model from the configured preferences that matches the tier
    /// and has an authenticated provider. Returns `Ok(None)` if no suitable model found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ModelTier, ModelPreference, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     model_preferences: vec![
    /// #         ModelPreference {
    /// #             model_id: "claude-sonnet-4-5".to_string(),
    /// #             provider: "anthropic".to_string(),
    /// #             tier: ModelTier::Balanced,
    /// #             is_default: true,
    /// #         },
    /// #     ],
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let model = selector.select_by_tier(ModelTier::Balanced).unwrap();
    /// assert!(model.is_some());
    /// ```
    pub fn select_by_tier(&self, tier: ModelTier) -> Result<Option<ModelPreference>> {
        let models = self.get_models_for_tier(tier);
        let filtered = self.filter_by_auth(&models);

        if filtered.is_empty() {
            return Ok(None);
        }

        // Return the first model (prioritize default)
        Ok(filtered
            .iter()
            .find(|m| m.is_default)
            .or(filtered.first())
            .cloned())
    }

    /// Select model by tier with fallback
    ///
    /// Attempts to select a model by tier, with the following fallback chain:
    /// 1. Try fallback_models from the primary choice
    /// 2. Try any model from the same tier
    /// 3. Try next lower tier (Reasoning → Balanced → Fast)
    ///
    /// Returns an error if no suitable model can be found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ModelTier, ModelPreference, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     model_preferences: vec![
    /// #         ModelPreference {
    /// #             model_id: "claude-sonnet-4-5".to_string(),
    /// #             provider: "anthropic".to_string(),
    /// #             tier: ModelTier::Balanced,
    /// #             is_default: true,
    /// #         },
    /// #     ],
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let model = selector.select_by_tier_with_fallback(ModelTier::Balanced).unwrap();
    /// ```
    pub fn select_by_tier_with_fallback(&self, tier: ModelTier) -> Result<ModelPreference> {
        // Try primary tier first
        if let Some(model) = self.select_by_tier(tier.clone())? {
            return Ok(model);
        }

        // Try fallback tiers: Reasoning -> Balanced -> Fast
        let fallback_tiers: Vec<ModelTier> = match tier {
            ModelTier::Reasoning => vec![ModelTier::Balanced, ModelTier::Fast],
            ModelTier::Balanced => vec![ModelTier::Fast],
            ModelTier::Fast => vec![],
            ModelTier::Vision => vec![ModelTier::Balanced, ModelTier::Fast],
            ModelTier::Coding => vec![ModelTier::Balanced, ModelTier::Fast],
        };

        for fallback_tier in fallback_tiers {
            if let Some(model) = self.select_by_tier(fallback_tier)? {
                return Ok(model);
            }
        }

        Err(Error::Config(ConfigError::LoadFailed {
            location: "model selection".to_string(),
            reason: format!("no available models for tier {:?} or fallback tiers", tier),
        }))
    }

    /// Select default model for a tier
    ///
    /// Returns the model marked as `is_default: true` for the given tier,
    /// filtered by authentication status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ModelTier, ModelPreference, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     model_preferences: vec![
    /// #         ModelPreference {
    /// #             model_id: "claude-sonnet-4-5".to_string(),
    /// #             provider: "anthropic".to_string(),
    /// #             tier: ModelTier::Balanced,
    /// #             is_default: true,
    /// #         },
    /// #     ],
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let model = selector.select_default_for_tier(ModelTier::Balanced).unwrap();
    /// assert!(model.is_some());
    /// ```
    pub fn select_default_for_tier(&self, tier: ModelTier) -> Result<Option<ModelPreference>> {
        let models = self.get_models_for_tier(tier);
        let filtered = self.filter_by_auth(&models);

        Ok(filtered.into_iter().find(|m| m.is_default))
    }

    /// Select model for a specific role
    ///
    /// Looks up the role in `role_assignments` and returns the configured model
    /// if the provider is authenticated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, RoleAssignment, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let mut role_assignments = HashMap::new();
    /// # role_assignments.insert("architect".to_string(), RoleAssignment {
    /// #     model_id: "claude-sonnet-4-5".to_string(),
    /// #     provider: "anthropic".to_string(),
    /// #     fallback_models: None,
    /// #     use_reasoning: Some(true),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     role_assignments,
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let model = selector.select_for_role("architect").unwrap();
    /// assert!(model.is_some());
    /// ```
    pub fn select_for_role(&self, role: &str) -> Result<Option<ModelPreference>> {
        let role_assignment = match self.config.role_assignments.get(role) {
            Some(assignment) => assignment,
            None => return Ok(None),
        };

        // Check if provider is configured
        if !self.is_provider_configured(&role_assignment.provider) {
            return Ok(None);
        }

        // Try to find the model in model_preferences to get tier info
        let model_pref = self.config.model_preferences.iter()
            .find(|m| m.model_id == role_assignment.model_id && m.provider == role_assignment.provider);

        if let Some(pref) = model_pref {
            // Verify auth status
            let filtered = self.filter_by_auth(&[pref.clone()]);
            if filtered.is_empty() {
                return Ok(None);
            }
            return Ok(Some(pref.clone()));
        }

        // If not in preferences, create a ModelPreference from the role assignment
        // We need to determine the tier - default to Balanced for unknown models
        let pref = ModelPreference {
            model_id: role_assignment.model_id.clone(),
            provider: role_assignment.provider.clone(),
            tier: ModelTier::Balanced, // Default tier for role-assigned models
            is_default: false,
        };

        let filtered = self.filter_by_auth(&[pref]);
        if filtered.is_empty() {
            return Ok(None);
        }
        Ok(filtered.into_iter().next())
    }

    /// Select model for role with fallback
    ///
    /// Attempts to select a model for a role, with the following fallback chain:
    /// 1. Try the primary role-assigned model
    /// 2. Try fallback_models from the role assignment
    /// 3. Try the Balanced tier
    ///
    /// Returns an error if no suitable model can be found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, RoleAssignment, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let mut role_assignments = HashMap::new();
    /// # role_assignments.insert("architect".to_string(), RoleAssignment {
    /// #     model_id: "claude-sonnet-4-5".to_string(),
    /// #     provider: "anthropic".to_string(),
    /// #     fallback_models: Some(vec!["claude-haiku-4-5".to_string()]),
    /// #     use_reasoning: Some(true),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     role_assignments,
    /// #     model_preferences: vec![],
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let model = selector.select_for_role_with_fallback("architect").unwrap();
    /// ```
    pub fn select_for_role_with_fallback(&self, role: &str) -> Result<ModelPreference> {
        let role_assignment = self.config.role_assignments.get(role)
            .ok_or_else(|| Error::Config(ConfigError::LoadFailed {
                location: "role assignment".to_string(),
                reason: format!("role '{}' not found in configuration", role),
            }))?;

        // Try primary model
        if let Some(model) = self.select_for_role(role)? {
            return Ok(model);
        }

        // Try fallback models
        if let Some(fallbacks) = &role_assignment.fallback_models {
            for fallback_id in fallbacks {
                // Try to find the fallback model in preferences
                if let Some(pref) = self.config.model_preferences.iter()
                    .find(|m| m.model_id == *fallback_id)
                {
                    if self.is_provider_configured(&pref.provider) {
                        let filtered = self.filter_by_auth(&[pref.clone()]);
                        if !filtered.is_empty() {
                            return Ok(filtered[0].clone());
                        }
                    }
                }
            }
        }

        // Final fallback: try Balanced tier
        self.select_by_tier_with_fallback(ModelTier::Balanced)
    }

    /// Get all available models for a tier
    ///
    /// Returns all models configured for the given tier, without filtering
    /// by authentication status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ModelTier, ModelPreference, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     model_preferences: vec![
    /// #         ModelPreference {
    /// #             model_id: "claude-sonnet-4-5".to_string(),
    /// #             provider: "anthropic".to_string(),
    /// #             tier: ModelTier::Balanced,
    /// #             is_default: true,
    /// #         },
    /// #     ],
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let models = selector.get_models_for_tier(ModelTier::Balanced);
    /// assert_eq!(models.len(), 1);
    /// ```
    pub fn get_models_for_tier(&self, tier: ModelTier) -> Vec<ModelPreference> {
        self.config
            .model_preferences
            .iter()
            .filter(|m| m.tier == tier)
            .cloned()
            .collect()
    }

    /// Check if a provider is configured
    ///
    /// Checks both the static configuration and any dynamically discovered
    /// provider status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// assert!(selector.is_provider_configured("anthropic"));
    /// assert!(!selector.is_provider_configured("openai"));
    /// ```
    pub fn is_provider_configured(&self, provider: &str) -> bool {
        // Check discovered status first (most up-to-date)
        if let Some(status_list) = &self.provider_status {
            if let Some(status) = status_list.iter().find(|s| s.provider.eq_ignore_ascii_case(provider)) {
                return status.is_configured;
            }
        }

        // Fall back to static configuration
        self.config
            .providers
            .get(provider)
            .map(|p| p.is_configured)
            .unwrap_or(false)
    }

    /// Filter model preferences by authentication status
    ///
    /// Returns only models from providers that are configured and authenticated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use maestro_pi_mono::config::{ModelSelector, models::{PiMonoConfig, ModelPreference, ModelTier, ProviderConfig}};
    /// # use std::collections::HashMap;
    /// # let mut providers = HashMap::new();
    /// # providers.insert("anthropic".to_string(), ProviderConfig {
    /// #     display_name: "Anthropic".to_string(),
    /// #     is_configured: true,
    /// #     env_var: "ANTHROPIC_API_KEY".to_string(),
    /// # });
    /// # let config = PiMonoConfig {
    /// #     providers,
    /// #     model_preferences: vec![],
    /// #     ..Default::default()
    /// # };
    /// let selector = ModelSelector::new(&config);
    /// let models = vec![
    ///     ModelPreference {
    ///         model_id: "claude-sonnet-4-5".to_string(),
    ///         provider: "anthropic".to_string(),
    ///         tier: ModelTier::Balanced,
    ///         is_default: true,
    ///     },
    ///     ModelPreference {
    ///         model_id: "gpt-4".to_string(),
    ///         provider: "openai".to_string(),
    ///         tier: ModelTier::Reasoning,
    ///         is_default: false,
    ///     },
    /// ];
    /// let filtered = selector.filter_by_auth(&models);
    /// assert_eq!(filtered.len(), 1);
    /// assert_eq!(filtered[0].provider, "anthropic");
    /// ```
    pub fn filter_by_auth(&self, models: &[ModelPreference]) -> Vec<ModelPreference> {
        models
            .iter()
            .filter(|m| self.is_provider_configured(&m.provider))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::models::{ProviderConfig, RoleAssignment};
    use std::collections::HashMap;

    // Helper function to create a test config
    fn create_test_config() -> ModelConfig {
        let mut providers = HashMap::new();
        providers.insert("anthropic".to_string(), ProviderConfig {
            display_name: "Anthropic".to_string(),
            is_configured: true,
            env_var: "ANTHROPIC_API_KEY".to_string(),
        });
        providers.insert("openai".to_string(), ProviderConfig {
            display_name: "OpenAI".to_string(),
            is_configured: false,
            env_var: "OPENAI_API_KEY".to_string(),
        });

        let model_preferences = vec![
            ModelPreference {
                model_id: "claude-opus-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Reasoning,
                is_default: true,
            },
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
            ModelPreference {
                model_id: "gpt-4".to_string(),
                provider: "openai".to_string(),
                tier: ModelTier::Reasoning,
                is_default: false,
            },
        ];

        let mut role_assignments = HashMap::new();
        role_assignments.insert("architect".to_string(), RoleAssignment {
            model_id: "claude-sonnet-4-5".to_string(),
            provider: "anthropic".to_string(),
            fallback_models: Some(vec!["claude-haiku-4-5".to_string()]),
            use_reasoning: Some(true),
        });
        role_assignments.insert("scout".to_string(), RoleAssignment {
            model_id: "claude-haiku-4-5".to_string(),
            provider: "anthropic".to_string(),
            fallback_models: None,
            use_reasoning: None,
        });

        ModelConfig {
            providers,
            model_preferences,
            role_assignments,
            ..Default::default()
        }
    }

    // Test: ModelSelector creation
    #[test]
    fn test_model_selector_creation() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        // Verify the selector was created (can't directly compare config due to lack of PartialEq)
        assert!(selector.available_models.is_none());
        assert!(selector.provider_status.is_none());
    }

    // Test: with_available_models builder
    #[test]
    fn test_with_available_models() {
        let config = create_test_config();
        let models = vec![
            ModelInfo {
                provider: "anthropic".to_string(),
                model_id: "claude-sonnet-4-5".to_string(),
                context_window: "200k".to_string(),
                max_output: "8k".to_string(),
                supports_thinking: false,
                supports_images: true,
            },
        ];

        let selector = ModelSelector::new(&config).with_available_models(models.clone());

        assert!(selector.available_models.is_some());
        assert_eq!(selector.available_models.unwrap().len(), 1);
    }

    // Test: with_provider_status builder
    #[test]
    fn test_with_provider_status() {
        let config = create_test_config();
        let status = vec![
            ProviderStatus {
                provider: "anthropic".to_string(),
                is_configured: true,
                env_var: "ANTHROPIC_API_KEY".to_string(),
            },
        ];

        let selector = ModelSelector::new(&config).with_provider_status(status.clone());

        assert!(selector.provider_status.is_some());
        assert_eq!(selector.provider_status.unwrap().len(), 1);
    }

    // Test: select_by_tier with configured provider
    #[test]
    fn test_select_by_tier_configured() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Balanced);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_id, "claude-sonnet-4-5");
    }

    // Test: select_by_tier with configured provider in reasoning tier
    #[test]
    fn test_select_by_tier_reasoning() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Reasoning);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_id, "claude-opus-4-5");
    }

    // Test: select_by_tier with no models for tier
    #[test]
    fn test_select_by_tier_no_models() {
        let config = ModelConfig::default();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Balanced);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // Test: select_by_tier_with_fallback primary available
    #[test]
    fn test_select_by_tier_with_fallback_primary() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier_with_fallback(ModelTier::Balanced);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.model_id, "claude-sonnet-4-5");
    }

    // Test: select_by_tier_with_fallback to lower tier
    #[test]
    fn test_select_by_tier_with_fallback_lower_tier() {
        let mut config = create_test_config();
        // Remove Vision tier models so we test fallback
        config.model_preferences = config.model_preferences
            .into_iter()
            .filter(|m| m.tier != ModelTier::Vision)
            .collect();

        let selector = ModelSelector::new(&config);

        // Vision tier has no models, should fall back to Balanced
        let result = selector.select_by_tier_with_fallback(ModelTier::Vision);

        assert!(result.is_ok());
        let model = result.unwrap();
        // Should get Balanced tier model
        assert_eq!(model.tier, ModelTier::Balanced);
    }

    // Test: select_by_tier_with_fallback failure
    #[test]
    fn test_select_by_tier_with_fallback_failure() {
        let config = ModelConfig::default();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier_with_fallback(ModelTier::Reasoning);

        assert!(result.is_err());
    }

    // Test: select_default_for_tier
    #[test]
    fn test_select_default_for_tier() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_default_for_tier(ModelTier::Balanced);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert!(model.unwrap().is_default);
    }

    // Test: select_default_for_tier no default
    #[test]
    fn test_select_default_for_tier_no_default() {
        let mut config = create_test_config();
        config.model_preferences = vec![
            ModelPreference {
                model_id: "gpt-4".to_string(),
                provider: "openai".to_string(),
                tier: ModelTier::Reasoning,
                is_default: false,
            },
        ];

        let selector = ModelSelector::new(&config);

        let result = selector.select_default_for_tier(ModelTier::Reasoning);

        assert!(result.is_ok());
        // No configured providers for this tier
        assert!(result.unwrap().is_none());
    }

    // Test: select_for_role with configured role
    #[test]
    fn test_select_for_role_configured() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_for_role("architect");

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_id, "claude-sonnet-4-5");
    }

    // Test: select_for_role with unknown role
    #[test]
    fn test_select_for_role_unknown() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_for_role("unknown_role");

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // Test: select_for_role_with_fallback
    #[test]
    fn test_select_for_role_with_fallback() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_for_role_with_fallback("architect");

        assert!(result.is_ok());
        let model = result.unwrap();
        assert_eq!(model.model_id, "claude-sonnet-4-5");
    }

    // Test: select_for_role_with_fallback unknown role
    #[test]
    fn test_select_for_role_with_fallback_unknown() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_for_role_with_fallback("unknown_role");

        assert!(result.is_err());
    }

    // Test: get_models_for_tier
    #[test]
    fn test_get_models_for_tier() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let models = selector.get_models_for_tier(ModelTier::Balanced);

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "claude-sonnet-4-5");
    }

    // Test: get_models_for_tier empty
    #[test]
    fn test_get_models_for_tier_empty() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let models = selector.get_models_for_tier(ModelTier::Coding);

        assert!(models.is_empty());
    }

    // Test: is_provider_configured from config
    #[test]
    fn test_is_provider_configured_from_config() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        assert!(selector.is_provider_configured("anthropic"));
        assert!(!selector.is_provider_configured("openai"));
        assert!(!selector.is_provider_configured("unknown"));
    }

    // Test: is_provider_configured from discovered status
    #[test]
    fn test_is_provider_configured_from_discovered() {
        let config = create_test_config();
        let status = vec![
            ProviderStatus {
                provider: "anthropic".to_string(),
                is_configured: false, // Override config
                env_var: "ANTHROPIC_API_KEY".to_string(),
            },
        ];

        let selector = ModelSelector::new(&config).with_provider_status(status);

        // Discovered status should take precedence
        assert!(!selector.is_provider_configured("anthropic"));
    }

    // Test: filter_by_auth
    #[test]
    fn test_filter_by_auth() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let models = vec![
            ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: true,
            },
            ModelPreference {
                model_id: "gpt-4".to_string(),
                provider: "openai".to_string(),
                tier: ModelTier::Reasoning,
                is_default: false,
            },
        ];

        let filtered = selector.filter_by_auth(&models);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].provider, "anthropic");
    }

    // Test: filter_by_auth empty input
    #[test]
    fn test_filter_by_auth_empty() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let models = vec![];
        let filtered = selector.filter_by_auth(&models);

        assert!(filtered.is_empty());
    }

    // Test: select_by_tier_reasoning_with_opus
    #[test]
    fn test_select_by_tier_reasoning_with_opus() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Reasoning);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_id, "claude-opus-4-5");
    }

    // Test: select_by_tier_fast
    #[test]
    fn test_select_by_tier_fast() {
        let config = create_test_config();
        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Fast);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_id, "claude-haiku-4-5");
    }

    // Test: empty config edge case
    #[test]
    fn test_empty_config() {
        let config = ModelConfig::default();
        let selector = ModelSelector::new(&config);

        assert!(!selector.is_provider_configured("any_provider"));
        assert!(selector.get_models_for_tier(ModelTier::Balanced).is_empty());

        let result = selector.select_by_tier(ModelTier::Balanced);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // Test: no authenticated providers
    #[test]
    fn test_no_authenticated_providers() {
        let mut config = create_test_config();
        // Mark all providers as unconfigured
        for provider in config.providers.values_mut() {
            provider.is_configured = false;
        }

        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Balanced);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // Test: missing roles
    #[test]
    fn test_missing_roles() {
        let config = ModelConfig::default();
        let selector = ModelSelector::new(&config);

        let result = selector.select_for_role("nonexistent_role");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let result = selector.select_for_role_with_fallback("nonexistent_role");
        assert!(result.is_err());
    }

    // Test: case insensitive provider matching
    #[test]
    fn test_case_insensitive_provider_matching() {
        let config = create_test_config();
        let status = vec![
            ProviderStatus {
                provider: "AntHrOpIc".to_string(), // Mixed case
                is_configured: true,
                env_var: "ANTHROPIC_API_KEY".to_string(),
            },
        ];

        let selector = ModelSelector::new(&config).with_provider_status(status);

        assert!(selector.is_provider_configured("anthropic"));
        assert!(selector.is_provider_configured("ANTHROPIC"));
        assert!(selector.is_provider_configured("Anthropic"));
    }

    // Test: multiple models same tier prefer default
    #[test]
    fn test_multiple_models_same_tier_prefer_default() {
        let mut config = create_test_config();
        config.model_preferences = vec![
            ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: true,
            },
            ModelPreference {
                model_id: "gpt-4o".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: false,
            },
        ];

        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier(ModelTier::Balanced);

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.is_some());
        assert!(model.unwrap().is_default);
    }

    // Test: fallback chain reasoning to fast
    #[test]
    fn test_fallback_chain_reasoning_to_fast() {
        let mut config = create_test_config();
        // Remove Reasoning tier models
        config.model_preferences = config.model_preferences
            .into_iter()
            .filter(|m| m.tier != ModelTier::Reasoning)
            .collect();

        let selector = ModelSelector::new(&config);

        let result = selector.select_by_tier_with_fallback(ModelTier::Reasoning);

        assert!(result.is_ok());
        let model = result.unwrap();
        // Should fall through to Balanced tier
        assert_eq!(model.tier, ModelTier::Balanced);
    }

    // Test: role fallback to balanced tier
    #[test]
    fn test_role_fallback_to_balanced_tier() {
        let mut config = create_test_config();
        // Role assignment points to unconfigured provider
        let mut role_assignments = HashMap::new();
        role_assignments.insert("architect".to_string(), RoleAssignment {
            model_id: "gpt-4".to_string(),
            provider: "openai".to_string(), // Not configured
            fallback_models: None,
            use_reasoning: None,
        });
        config.role_assignments = role_assignments;

        let selector = ModelSelector::new(&config);

        let result = selector.select_for_role_with_fallback("architect");

        assert!(result.is_ok());
        let model = result.unwrap();
        // Should fall back to Balanced tier (claude-sonnet-4-5)
        assert_eq!(model.provider, "anthropic");
    }
}
