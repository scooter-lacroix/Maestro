//! # Configuration wizard for Pi-Mono
//!
//! This module provides an interactive configuration wizard that guides users
//! through setting up their Pi-Mono integration.
//!
//! ## Wizard Flow
//!
//! The wizard follows a 5-step process:
//! 1. **Detection** - Verify pi-mono CLI is detected
//! 2. **Provider Review** - Show provider authentication status
//! 3. **Model Selection** - Select models for each tier
//! 4. **Role Assignment** - Map models to agent roles
//! 5. **Confirmation & Save** - Review and save configuration
//!
//! ## Example
//!
//! ```rust,no_run
//! use maestro_pi_mono::config::wizard::ConfigWizard;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut wizard = ConfigWizard::new();
//!
//!     // Step 1: Detect pi-mono
//!     wizard.step1_detection().await.unwrap();
//!
//!     // Step 2: Review providers
//!     let providers = wizard.step2_provider_review().await.unwrap();
//!
//!     // Step 3: Select models
//!     wizard.step3_select_model("Balanced", "claude-sonnet-4-5").unwrap();
//!
//!     // Step 4: Assign roles
//!     wizard.step4_assign_role("architect", "claude-sonnet-4-5").unwrap();
//!
//!     // Step 5: Confirm and save
//!     wizard.step5_confirm_and_save().await.unwrap();
//! }
//! ```

use crate::{
    config::models::{ModelTier, PiMonoConfig, RoleAssignment},
    detection::PiDetection,
    discovery::{DiscoveryResult, ModelDiscovery},
    error::{Error, Result},
};
use std::collections::HashMap;

/// Current wizard step in the configuration flow
///
/// Represents the progression through the 5-step wizard process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardStep {
    /// Initial state - wizard just created
    Detection,
    /// After detection - reviewing provider status
    ProviderReview,
    /// Selecting models for each tier
    ModelSelection,
    /// Assigning models to roles
    RoleAssignment,
    /// Final confirmation before saving
    Confirmation,
    /// Wizard complete - configuration saved
    Complete,
}

impl WizardStep {
    /// Get the next step in the wizard flow
    pub fn next(&self) -> Option<WizardStep> {
        match self {
            WizardStep::Detection => Some(WizardStep::ProviderReview),
            WizardStep::ProviderReview => Some(WizardStep::ModelSelection),
            WizardStep::ModelSelection => Some(WizardStep::RoleAssignment),
            WizardStep::RoleAssignment => Some(WizardStep::Confirmation),
            WizardStep::Confirmation => Some(WizardStep::Complete),
            WizardStep::Complete => None,
        }
    }

    /// Get the previous step in the wizard flow
    pub fn prev(&self) -> Option<WizardStep> {
        match self {
            WizardStep::Detection => None,
            WizardStep::ProviderReview => Some(WizardStep::Detection),
            WizardStep::ModelSelection => Some(WizardStep::ProviderReview),
            WizardStep::RoleAssignment => Some(WizardStep::ModelSelection),
            WizardStep::Confirmation => Some(WizardStep::RoleAssignment),
            WizardStep::Complete => Some(WizardStep::Confirmation),
        }
    }
}

/// Wizard state tracking progress through configuration
///
/// Contains all state needed for the configuration wizard including
/// detection results, discovered models, selected models, and role assignments.
#[derive(Debug, Clone)]
pub struct WizardState {
    /// Current wizard step
    pub step: WizardStep,
    /// Detected pi-mono CLI information
    pub pi_detection: Option<PiDetection>,
    /// Model discovery results
    pub discovery_result: Option<DiscoveryResult>,
    /// Current configuration being built
    pub config: PiMonoConfig,
    /// Selected models by tier (tier name -> model_id)
    pub selected_models: HashMap<String, String>,
    /// Role assignments (role name -> model_id)
    pub role_assignments: HashMap<String, String>,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            step: WizardStep::Detection,
            pi_detection: None,
            discovery_result: None,
            config: PiMonoConfig::default(),
            selected_models: HashMap::new(),
            role_assignments: HashMap::new(),
        }
    }
}

impl WizardState {
    /// Create a new wizard state with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create wizard state from existing configuration
    pub fn from_config(config: PiMonoConfig) -> Self {
        let mut selected_models = HashMap::new();
        let mut role_assignments = HashMap::new();

        // Extract model preferences into selected_models
        for pref in &config.model_preferences {
            let tier_name = format!("{:?}", pref.tier);
            selected_models.insert(tier_name, pref.model_id.clone());
        }

        // Extract role assignments
        for (role, assignment) in &config.role_assignments {
            role_assignments.insert(role.clone(), assignment.model_id.clone());
        }

        Self {
            step: WizardStep::Detection,
            pi_detection: None,
            discovery_result: None,
            config,
            selected_models,
            role_assignments,
        }
    }

    /// Check if the wizard can proceed to the next step
    pub fn can_proceed(&self) -> bool {
        match self.step {
            WizardStep::Detection => self.pi_detection.is_some(),
            WizardStep::ProviderReview => {
                self.pi_detection.is_some() && self.discovery_result.is_some()
            }
            WizardStep::ModelSelection => {
                self.pi_detection.is_some() && self.discovery_result.is_some()
            }
            WizardStep::RoleAssignment => {
                // At least one model must be selected
                self.pi_detection.is_some()
                    && self.discovery_result.is_some()
                    && !self.selected_models.is_empty()
            }
            WizardStep::Confirmation => {
                // At least one role must be assigned
                self.pi_detection.is_some()
                    && self.discovery_result.is_some()
                    && !self.selected_models.is_empty()
                    && !self.role_assignments.is_empty()
            }
            WizardStep::Complete => true,
        }
    }

    /// Get all available tier names
    pub fn get_tiers(&self) -> Vec<String> {
        vec![
            "Reasoning".to_string(),
            "Fast".to_string(),
            "Balanced".to_string(),
            "Vision".to_string(),
            "Coding".to_string(),
        ]
    }

    /// Get all role names
    pub fn get_roles(&self) -> Vec<String> {
        vec![
            "scout".to_string(),
            "architect".to_string(),
            "critic".to_string(),
            "kraken".to_string(),
            "sentinel".to_string(),
            "warden".to_string(),
            "mender".to_string(),
            "cartographer".to_string(),
            "prism".to_string(),
        ]
    }
}

/// Configuration wizard for Pi-Mono integration
///
/// Guides users through interactive configuration setup.
pub struct ConfigWizard {
    /// Current wizard state
    state: WizardState,
    /// Model discovery service (created after detection)
    discovery: Option<ModelDiscovery>,
}

impl ConfigWizard {
    /// Create a new wizard with default state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// let wizard = ConfigWizard::new();
    /// assert_eq!(wizard.state().step, maestro_pi_mono::config::wizard::WizardStep::Detection);
    /// ```
    pub fn new() -> Self {
        Self {
            state: WizardState::new(),
            discovery: None,
        }
    }

    /// Create wizard from existing configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    /// use maestro_pi_mono::config::models::PiMonoConfig;
    ///
    /// let config = PiMonoConfig::default();
    /// let wizard = ConfigWizard::from_config(config);
    /// ```
    pub fn from_config(config: PiMonoConfig) -> Self {
        Self {
            state: WizardState::from_config(config),
            discovery: None,
        }
    }

    /// Get current wizard state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// let wizard = ConfigWizard::new();
    /// let state = wizard.state();
    /// assert_eq!(state.step, maestro_pi_mono::config::wizard::WizardStep::Detection);
    /// ```
    pub fn state(&self) -> &WizardState {
        &self.state
    }

    /// Step 1: Detect pi-mono CLI
    ///
    /// Attempts to detect the pi-mono executable and gather version info.
    /// Sets up the state for subsequent steps.
    ///
    /// # Errors
    ///
    /// Returns `Error::Detection` if pi-mono cannot be found.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut wizard = ConfigWizard::new();
    ///     wizard.step1_detection().await.unwrap();
    ///     assert!(wizard.state().pi_detection.is_some());
    /// }
    /// ```
    pub async fn step1_detection(&mut self) -> Result<()> {
        let detection = PiDetection::detect_full().await?;

        // Update state with detection info
        self.state.pi_detection = Some(detection.clone());

        // Update config path and version
        self.state.config.path = Some(detection.executable_path.to_string_lossy().to_string());
        self.state.config.version_info = detection.version.clone();

        // Set up model discovery
        let discovery = ModelDiscovery::new(detection);
        self.discovery = Some(discovery);

        // Move to next step
        self.state.step = WizardStep::ProviderReview;

        Ok(())
    }

    /// Step 2: Review provider authentication status
    ///
    /// Returns a list of provider names that are configured.
    /// Uses model discovery to determine which providers have valid credentials.
    ///
    /// # Errors
    ///
    /// Returns an error if model discovery fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut wizard = ConfigWizard::new();
    ///     wizard.step1_detection().await.unwrap();
    ///     let providers = wizard.step2_provider_review().await.unwrap();
    ///     println!("Configured providers: {:?}", providers);
    /// }
    /// ```
    pub async fn step2_provider_review(&mut self) -> Result<Vec<String>> {
        let discovery = self
            .discovery
            .as_mut()
            .ok_or_else(|| Error::Other("Model discovery not initialized".to_string()))?;

        // Discover available models
        let discovery_result = discovery.discover_models().await?;
        self.state.discovery_result = Some(discovery_result.clone());

        // Update config providers based on discovery
        for provider_status in &discovery_result.providers {
            let provider_config = crate::config::models::ProviderConfig {
                display_name: Self::provider_display_name(&provider_status.provider),
                is_configured: provider_status.is_configured,
                env_var: provider_status.env_var.clone(),
            };
            self.state
                .config
                .providers
                .insert(provider_status.provider.clone(), provider_config);
        }

        // Move to next step
        self.state.step = WizardStep::ModelSelection;

        // Return list of configured providers
        let configured: Vec<String> = discovery_result
            .providers
            .iter()
            .filter(|p| p.is_configured)
            .map(|p| p.provider.clone())
            .collect();

        Ok(configured)
    }

    /// Step 3: Select a model for a tier
    ///
    /// Associates a model ID with a tier. The model must have been
    /// discovered in step 2.
    ///
    /// # Errors
    ///
    /// Returns an error if the model was not discovered or tier is invalid.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut wizard = ConfigWizard::new();
    ///     wizard.step1_detection().await.unwrap();
    ///     wizard.step2_provider_review().await.unwrap();
    ///     wizard.step3_select_model("Balanced", "claude-sonnet-4-5").unwrap();
    /// }
    /// ```
    pub fn step3_select_model(&mut self, tier: &str, model_id: &str) -> Result<()> {
        // Validate tier
        let valid_tiers = self.state.get_tiers();
        if !valid_tiers.contains(&tier.to_string()) {
            return Err(Error::Other(format!("Invalid tier: {}", tier)));
        }

        // Validate model exists in discovery
        if let Some(discovery) = &self.state.discovery_result {
            let model_exists = discovery.models.iter().any(|m| m.model_id == model_id);
            if !model_exists {
                return Err(Error::Other(format!(
                    "Model '{}' not found in discovery results",
                    model_id
                )));
            }
        }

        // Add to selected models
        self.state
            .selected_models
            .insert(tier.to_string(), model_id.to_string());

        Ok(())
    }

    /// Step 4: Assign a model to a role
    ///
    /// Associates a model ID with an agent role. The model must have
    /// been selected in step 3.
    ///
    /// # Errors
    ///
    /// Returns an error if the role is invalid or model was not selected.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut wizard = ConfigWizard::new();
    ///     wizard.step1_detection().await.unwrap();
    ///     wizard.step2_provider_review().await.unwrap();
    ///     wizard.step3_select_model("Balanced", "claude-sonnet-4-5").unwrap();
    ///     wizard.step4_assign_role("architect", "claude-sonnet-4-5").unwrap();
    /// }
    /// ```
    pub fn step4_assign_role(&mut self, role: &str, model_id: &str) -> Result<()> {
        // Validate role
        let valid_roles = self.state.get_roles();
        if !valid_roles.contains(&role.to_string()) {
            return Err(Error::Other(format!("Invalid role: {}", role)));
        }

        // Validate model was selected
        if !self.state.selected_models.values().any(|id| id == model_id) {
            return Err(Error::Other(format!(
                "Model '{}' was not selected in step 3",
                model_id
            )));
        }

        // Find provider for this model
        let provider = self
            .state
            .discovery_result
            .as_ref()
            .and_then(|d| {
                d.models
                    .iter()
                    .find(|m| m.model_id == model_id)
                    .map(|m| m.provider.clone())
            })
            .ok_or_else(|| {
                Error::Other(format!("Cannot find provider for model '{}'", model_id))
            })?;

        // Add to role assignments
        self.state
            .role_assignments
            .insert(role.to_string(), model_id.to_string());

        // Update config role assignment
        self.state.config.role_assignments.insert(
            role.to_string(),
            RoleAssignment {
                model_id: model_id.to_string(),
                provider,
                fallback_models: None,
                use_reasoning: None,
            },
        );

        // Move to confirmation step if this is the first role assignment
        if self.state.step == WizardStep::ModelSelection {
            self.state.step = WizardStep::RoleAssignment;
        }

        Ok(())
    }

    /// Step 5: Confirm and save configuration
    ///
    /// Validates and saves the configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or saving fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut wizard = ConfigWizard::new();
    ///     wizard.step1_detection().await.unwrap();
    ///     wizard.step2_provider_review().await.unwrap();
    ///     wizard.step3_select_model("Balanced", "claude-sonnet-4-5").unwrap();
    ///     wizard.step4_assign_role("architect", "claude-sonnet-4-5").unwrap();
    ///     wizard.step5_confirm_and_save().await.unwrap();
    /// }
    /// ```
    pub async fn step5_confirm_and_save(&mut self) -> Result<()> {
        use crate::config::io;

        // Validate state before saving
        if !self.state.can_proceed() {
            return Err(Error::Other(
                "Cannot save: wizard requirements not met".to_string(),
            ));
        }

        // Build model preferences from selections
        self.state.config.model_preferences.clear();
        for (tier_name, model_id) in &self.state.selected_models {
            if let Some(discovery) = &self.state.discovery_result {
                if let Some(model_info) = discovery.models.iter().find(|m| &m.model_id == model_id)
                {
                    let tier = Self::parse_tier(tier_name)?;
                    self.state.config.model_preferences.push(
                        crate::config::models::ModelPreference {
                            model_id: model_id.clone(),
                            provider: model_info.provider.clone(),
                            tier,
                            is_default: true,
                        },
                    );
                }
            }
        }

        // Validate and save config
        io::validate_config(&self.state.config)?;
        io::save_config(&self.state.config)?;

        // Mark as complete
        self.state.step = WizardStep::Complete;

        Ok(())
    }

    /// Get suggested models for a tier
    ///
    /// Returns a list of model IDs that are suitable for the given tier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// let wizard = ConfigWizard::new();
    /// let models = wizard.get_suggested_models("Fast");
    /// ```
    pub fn get_suggested_models(&self, tier: &str) -> Vec<String> {
        let discovery = match &self.state.discovery_result {
            Some(d) => d,
            None => return Vec::new(),
        };

        let tier_lower = tier.to_lowercase();

        discovery
            .models
            .iter()
            .filter(|m| {
                // Filter based on tier heuristics
                match tier_lower.as_str() {
                    "fast" => {
                        m.model_id.contains("haiku")
                            || m.model_id.contains("flash")
                            || m.model_id.contains("mini")
                            || m.model_id.contains("nano")
                    }
                    "reasoning" => {
                        m.model_id.contains("opus")
                            || m.model_id.contains("o1")
                            || m.supports_thinking
                    }
                    "vision" => m.supports_images,
                    "coding" => {
                        m.model_id.contains("codex")
                            || m.model_id.contains("code")
                            || m.model_id.contains("gpt-4")
                    }
                    _ => true, // Balanced: accept most models
                }
            })
            .map(|m| m.model_id.clone())
            .collect()
    }

    /// Get current configuration
    ///
    /// Returns a reference to the configuration being built.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// let wizard = ConfigWizard::new();
    /// let config = wizard.config();
    /// assert!(config.enabled);
    /// ```
    pub fn config(&self) -> &PiMonoConfig {
        &self.state.config
    }

    /// Check if wizard can proceed to next step
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maestro_pi_mono::config::wizard::ConfigWizard;
    ///
    /// let wizard = ConfigWizard::new();
    /// // Before detection, cannot proceed
    /// assert!(!wizard.can_proceed());
    /// ```
    pub fn can_proceed(&self) -> bool {
        self.state.can_proceed()
    }

    /// Move to the next step
    ///
    /// # Errors
    ///
    /// Returns an error if already at the Complete step.
    pub fn next_step(&mut self) -> Result<()> {
        if let Some(next) = self.state.step.next() {
            self.state.step = next;
            Ok(())
        } else {
            Err(Error::Other("Already at final step".to_string()))
        }
    }

    /// Move to the previous step
    ///
    /// # Errors
    ///
    /// Returns an error if already at the Detection step.
    pub fn prev_step(&mut self) -> Result<()> {
        if let Some(prev) = self.state.step.prev() {
            self.state.step = prev;
            Ok(())
        } else {
            Err(Error::Other("Already at first step".to_string()))
        }
    }

    // Helper methods

    fn provider_display_name(provider: &str) -> String {
        match provider.to_lowercase().as_str() {
            "anthropic" => "Anthropic".to_string(),
            "openai" => "OpenAI".to_string(),
            "google" => "Google".to_string(),
            "groq" => "Groq".to_string(),
            "openrouter" => "OpenRouter".to_string(),
            _ => {
                // Capitalize first letter
                let mut chars = provider.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        }
    }

    fn parse_tier(tier: &str) -> Result<ModelTier> {
        match tier {
            "Reasoning" => Ok(ModelTier::Reasoning),
            "Fast" => Ok(ModelTier::Fast),
            "Balanced" => Ok(ModelTier::Balanced),
            "Vision" => Ok(ModelTier::Vision),
            "Coding" => Ok(ModelTier::Coding),
            _ => Err(Error::Other(format!("Invalid tier: {}", tier))),
        }
    }
}

impl Default for ConfigWizard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::{Capabilities, PiDetection};
    use crate::discovery::{ModelInfo, ProviderStatus};
    use std::path::PathBuf;
    use std::time::SystemTime;

    // Helper: Create a mock PiDetection for testing
    fn mock_detection() -> PiDetection {
        PiDetection {
            executable_path: PathBuf::from("/usr/local/bin/pi"),
            version: Some("0.49.3".to_string()),
            capabilities: Capabilities::default(),
        }
    }

    // Helper: Create a mock DiscoveryResult for testing
    fn mock_discovery_result() -> DiscoveryResult {
        let now = SystemTime::now();
        DiscoveryResult {
            models: vec![
                ModelInfo {
                    provider: "anthropic".to_string(),
                    model_id: "claude-sonnet-4-5".to_string(),
                    context_window: "200k".to_string(),
                    max_output: "8k".to_string(),
                    supports_thinking: true,
                    supports_images: true,
                },
                ModelInfo {
                    provider: "anthropic".to_string(),
                    model_id: "claude-haiku-4-5".to_string(),
                    context_window: "200k".to_string(),
                    max_output: "4k".to_string(),
                    supports_thinking: false,
                    supports_images: false,
                },
                ModelInfo {
                    provider: "openai".to_string(),
                    model_id: "gpt-4o".to_string(),
                    context_window: "128k".to_string(),
                    max_output: "4k".to_string(),
                    supports_thinking: false,
                    supports_images: true,
                },
            ],
            providers: vec![
                ProviderStatus {
                    provider: "anthropic".to_string(),
                    is_configured: true,
                    env_var: "ANTHROPIC_API_KEY".to_string(),
                },
                ProviderStatus {
                    provider: "openai".to_string(),
                    is_configured: true,
                    env_var: "OPENAI_API_KEY".to_string(),
                },
                ProviderStatus {
                    provider: "google".to_string(),
                    is_configured: false,
                    env_var: "GOOGLE_API_KEY".to_string(),
                },
            ],
            discovered_at: now,
            cache_expires: now + std::time::Duration::from_secs(86400),
        }
    }

    // WizardStep enum tests
    mod wizard_step_tests {
        use super::*;

        #[test]
        fn test_wizard_step_detection_next() {
            let step = WizardStep::Detection;
            assert_eq!(step.next(), Some(WizardStep::ProviderReview));
        }

        #[test]
        fn test_wizard_step_detection_prev() {
            let step = WizardStep::Detection;
            assert_eq!(step.prev(), None);
        }

        #[test]
        fn test_wizard_step_provider_review_next() {
            let step = WizardStep::ProviderReview;
            assert_eq!(step.next(), Some(WizardStep::ModelSelection));
        }

        #[test]
        fn test_wizard_step_provider_review_prev() {
            let step = WizardStep::ProviderReview;
            assert_eq!(step.prev(), Some(WizardStep::Detection));
        }

        #[test]
        fn test_wizard_step_model_selection_next() {
            let step = WizardStep::ModelSelection;
            assert_eq!(step.next(), Some(WizardStep::RoleAssignment));
        }

        #[test]
        fn test_wizard_step_role_assignment_next() {
            let step = WizardStep::RoleAssignment;
            assert_eq!(step.next(), Some(WizardStep::Confirmation));
        }

        #[test]
        fn test_wizard_step_confirmation_next() {
            let step = WizardStep::Confirmation;
            assert_eq!(step.next(), Some(WizardStep::Complete));
        }

        #[test]
        fn test_wizard_step_complete_next_none() {
            let step = WizardStep::Complete;
            assert_eq!(step.next(), None);
        }

        #[test]
        fn test_wizard_step_complete_prev() {
            let step = WizardStep::Complete;
            assert_eq!(step.prev(), Some(WizardStep::Confirmation));
        }

        #[test]
        fn test_wizard_step_equality() {
            assert_eq!(WizardStep::Detection, WizardStep::Detection);
            assert_eq!(WizardStep::ProviderReview, WizardStep::ProviderReview);
            assert_ne!(WizardStep::Detection, WizardStep::Complete);
        }
    }

    // WizardState tests
    mod wizard_state_tests {
        use super::*;

        #[test]
        fn test_wizard_state_default() {
            let state = WizardState::default();
            assert_eq!(state.step, WizardStep::Detection);
            assert!(state.pi_detection.is_none());
            assert!(state.discovery_result.is_none());
            assert!(state.selected_models.is_empty());
            assert!(state.role_assignments.is_empty());
        }

        #[test]
        fn test_wizard_state_new() {
            let state = WizardState::new();
            assert_eq!(state.step, WizardStep::Detection);
            assert!(state.pi_detection.is_none());
        }

        #[test]
        fn test_wizard_state_from_config_empty() {
            let config = PiMonoConfig::default();
            let state = WizardState::from_config(config);
            assert_eq!(state.step, WizardStep::Detection);
            assert!(state.selected_models.is_empty());
            assert!(state.role_assignments.is_empty());
        }

        #[test]
        fn test_wizard_state_from_config_with_preferences() {
            let mut config = PiMonoConfig::default();
            config
                .model_preferences
                .push(crate::config::models::ModelPreference {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    tier: ModelTier::Balanced,
                    is_default: true,
                });

            let state = WizardState::from_config(config);
            assert_eq!(
                state.selected_models.get("Balanced"),
                Some(&"claude-sonnet-4-5".to_string())
            );
        }

        #[test]
        fn test_wizard_state_from_config_with_roles() {
            let mut config = PiMonoConfig::default();
            config.role_assignments.insert(
                "architect".to_string(),
                RoleAssignment {
                    model_id: "claude-sonnet-4-5".to_string(),
                    provider: "anthropic".to_string(),
                    fallback_models: None,
                    use_reasoning: None,
                },
            );

            let state = WizardState::from_config(config);
            assert_eq!(
                state.role_assignments.get("architect"),
                Some(&"claude-sonnet-4-5".to_string())
            );
        }

        #[test]
        fn test_wizard_state_can_proceed_detection() {
            let mut state = WizardState::default();
            assert!(!state.can_proceed()); // No detection yet

            state.pi_detection = Some(mock_detection());
            assert!(state.can_proceed());
        }

        #[test]
        fn test_wizard_state_can_proceed_provider_review() {
            let mut state = WizardState {
                step: WizardStep::ProviderReview,
                pi_detection: Some(mock_detection()),
                discovery_result: None,
                ..Default::default()
            };
            assert!(!state.can_proceed()); // No discovery yet

            state.discovery_result = Some(mock_discovery_result());
            assert!(state.can_proceed());
        }

        #[test]
        fn test_wizard_state_can_proceed_model_selection() {
            let state = WizardState {
                step: WizardStep::ModelSelection,
                pi_detection: Some(mock_detection()),
                discovery_result: Some(mock_discovery_result()),
                ..Default::default()
            };
            assert!(state.can_proceed());
        }

        #[test]
        fn test_wizard_state_can_proceed_role_assignment() {
            let mut state = WizardState {
                step: WizardStep::RoleAssignment,
                pi_detection: Some(mock_detection()),
                discovery_result: Some(mock_discovery_result()),
                selected_models: HashMap::new(),
                ..Default::default()
            };
            assert!(!state.can_proceed()); // No models selected

            state
                .selected_models
                .insert("Balanced".to_string(), "claude-sonnet-4-5".to_string());
            assert!(state.can_proceed());
        }

        #[test]
        fn test_wizard_state_can_proceed_confirmation() {
            let mut state = WizardState {
                step: WizardStep::Confirmation,
                pi_detection: Some(mock_detection()),
                discovery_result: Some(mock_discovery_result()),
                selected_models: HashMap::new(),
                role_assignments: HashMap::new(),
                ..Default::default()
            };
            assert!(!state.can_proceed()); // No models or roles

            state
                .selected_models
                .insert("Balanced".to_string(), "claude-sonnet-4-5".to_string());
            assert!(!state.can_proceed()); // Still no roles

            state
                .role_assignments
                .insert("architect".to_string(), "claude-sonnet-4-5".to_string());
            assert!(state.can_proceed());
        }

        #[test]
        fn test_wizard_state_can_proceed_complete() {
            let state = WizardState {
                step: WizardStep::Complete,
                ..Default::default()
            };
            assert!(state.can_proceed());
        }

        #[test]
        fn test_wizard_state_get_tiers() {
            let state = WizardState::default();
            let tiers = state.get_tiers();
            assert_eq!(tiers.len(), 5);
            assert!(tiers.contains(&"Reasoning".to_string()));
            assert!(tiers.contains(&"Fast".to_string()));
            assert!(tiers.contains(&"Balanced".to_string()));
            assert!(tiers.contains(&"Vision".to_string()));
            assert!(tiers.contains(&"Coding".to_string()));
        }

        #[test]
        fn test_wizard_state_get_roles() {
            let state = WizardState::default();
            let roles = state.get_roles();
            assert_eq!(roles.len(), 9);
            assert!(roles.contains(&"scout".to_string()));
            assert!(roles.contains(&"architect".to_string()));
            assert!(roles.contains(&"critic".to_string()));
            assert!(roles.contains(&"kraken".to_string()));
            assert!(roles.contains(&"sentinel".to_string()));
            assert!(roles.contains(&"warden".to_string()));
            assert!(roles.contains(&"mender".to_string()));
            assert!(roles.contains(&"cartographer".to_string()));
            assert!(roles.contains(&"prism".to_string()));
        }
    }

    // ConfigWizard creation tests
    mod config_wizard_creation_tests {
        use super::*;

        #[test]
        fn test_config_wizard_new() {
            let wizard = ConfigWizard::new();
            assert_eq!(wizard.state().step, WizardStep::Detection);
            assert!(wizard.state().pi_detection.is_none());
            assert!(wizard.discovery.is_none());
        }

        #[test]
        fn test_config_wizard_default() {
            let wizard = ConfigWizard::default();
            assert_eq!(wizard.state().step, WizardStep::Detection);
        }

        #[test]
        fn test_config_wizard_from_config() {
            let config = PiMonoConfig::default();
            let wizard = ConfigWizard::from_config(config);
            assert_eq!(wizard.state().step, WizardStep::Detection);
        }

        #[test]
        fn test_config_wizard_state_returns_reference() {
            let wizard = ConfigWizard::new();
            let state = wizard.state();
            assert_eq!(state.step, WizardStep::Detection);
        }

        #[test]
        fn test_config_wizard_config_returns_reference() {
            let wizard = ConfigWizard::new();
            let config = wizard.config();
            assert!(config.enabled);
            assert_eq!(config.version, "1.0");
        }
    }

    // step3_select_model tests
    mod step3_select_model_tests {
        use super::*;

        #[test]
        fn test_step3_select_model_valid() {
            let mut wizard = ConfigWizard::new();
            // Set up discovery
            wizard.state.discovery_result = Some(mock_discovery_result());

            let result = wizard.step3_select_model("Balanced", "claude-sonnet-4-5");
            assert!(result.is_ok());
            assert_eq!(
                wizard.state.selected_models.get("Balanced"),
                Some(&"claude-sonnet-4-5".to_string())
            );
        }

        #[test]
        fn test_step3_select_model_invalid_tier() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let result = wizard.step3_select_model("InvalidTier", "claude-sonnet-4-5");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid tier"));
        }

        #[test]
        fn test_step3_select_model_not_discovered() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let result = wizard.step3_select_model("Balanced", "nonexistent-model");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn test_step3_select_model_multiple_tiers() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            wizard
                .step3_select_model("Fast", "claude-haiku-4-5")
                .unwrap();
            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();

            assert_eq!(wizard.state.selected_models.len(), 2);
            assert_eq!(
                wizard.state.selected_models.get("Fast"),
                Some(&"claude-haiku-4-5".to_string())
            );
            assert_eq!(
                wizard.state.selected_models.get("Balanced"),
                Some(&"claude-sonnet-4-5".to_string())
            );
        }

        #[test]
        fn test_step3_select_model_overwrite() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();
            wizard.step3_select_model("Balanced", "gpt-4o").unwrap();

            assert_eq!(
                wizard.state.selected_models.get("Balanced"),
                Some(&"gpt-4o".to_string())
            );
        }
    }

    // step4_assign_role tests
    mod step4_assign_role_tests {
        use super::*;

        #[test]
        fn test_step4_assign_role_valid() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard.state.step = WizardStep::ModelSelection; // Set to ModelSelection first
            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();

            let result = wizard.step4_assign_role("architect", "claude-sonnet-4-5");
            assert!(result.is_ok());
            assert_eq!(
                wizard.state.role_assignments.get("architect"),
                Some(&"claude-sonnet-4-5".to_string())
            );
            // Step should have advanced to RoleAssignment
            assert_eq!(wizard.state.step, WizardStep::RoleAssignment);
        }

        #[test]
        fn test_step4_assign_role_invalid_role() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();

            let result = wizard.step4_assign_role("invalid_role", "claude-sonnet-4-5");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid role"));
        }

        #[test]
        fn test_step4_assign_role_model_not_selected() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let result = wizard.step4_assign_role("architect", "claude-sonnet-4-5");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not selected"));
        }

        #[test]
        fn test_step4_assign_role_updates_config() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();

            wizard
                .step4_assign_role("architect", "claude-sonnet-4-5")
                .unwrap();

            assert!(wizard
                .state
                .config
                .role_assignments
                .contains_key("architect"));
            let assignment = &wizard.state.config.role_assignments["architect"];
            assert_eq!(assignment.model_id, "claude-sonnet-4-5");
            assert_eq!(assignment.provider, "anthropic");
        }

        #[test]
        fn test_step4_assign_role_multiple_roles() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard
                .step3_select_model("Fast", "claude-haiku-4-5")
                .unwrap();
            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();

            wizard
                .step4_assign_role("scout", "claude-haiku-4-5")
                .unwrap();
            wizard
                .step4_assign_role("architect", "claude-sonnet-4-5")
                .unwrap();

            assert_eq!(wizard.state.role_assignments.len(), 2);
            assert_eq!(
                wizard.state.role_assignments.get("scout"),
                Some(&"claude-haiku-4-5".to_string())
            );
            assert_eq!(
                wizard.state.role_assignments.get("architect"),
                Some(&"claude-sonnet-4-5".to_string())
            );
        }

        #[test]
        fn test_step4_assign_role_with_openai_model() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard.step3_select_model("Balanced", "gpt-4o").unwrap();

            wizard.step4_assign_role("critic", "gpt-4o").unwrap();

            let assignment = &wizard.state.config.role_assignments["critic"];
            assert_eq!(assignment.provider, "openai");
        }
    }

    // get_suggested_models tests
    mod get_suggested_models_tests {
        use super::*;

        #[test]
        fn test_get_suggested_models_no_discovery() {
            let wizard = ConfigWizard::new();
            let models = wizard.get_suggested_models("Fast");
            assert!(models.is_empty());
        }

        #[test]
        fn test_get_suggested_models_fast_tier() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let models = wizard.get_suggested_models("Fast");
            // Should include claude-haiku (contains "haiku")
            assert!(models.contains(&"claude-haiku-4-5".to_string()));
        }

        #[test]
        fn test_get_suggested_models_balanced_tier() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let models = wizard.get_suggested_models("Balanced");
            // Balanced tier should return most models
            assert!(!models.is_empty());
        }

        #[test]
        fn test_get_suggested_models_reasoning_tier() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let models = wizard.get_suggested_models("Reasoning");
            // Should include claude-sonnet-4-5 (supports_thinking: true)
            assert!(models.contains(&"claude-sonnet-4-5".to_string()));
        }

        #[test]
        fn test_get_suggested_models_vision_tier() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let models = wizard.get_suggested_models("Vision");
            // Should include models that support images
            assert!(models.contains(&"claude-sonnet-4-5".to_string()));
            assert!(models.contains(&"gpt-4o".to_string()));
        }

        #[test]
        fn test_get_suggested_models_case_insensitive() {
            let mut wizard = ConfigWizard::new();
            wizard.state.discovery_result = Some(mock_discovery_result());

            let models_lower = wizard.get_suggested_models("fast");
            let models_upper = wizard.get_suggested_models("FAST");
            assert_eq!(models_lower, models_upper);
        }
    }

    // can_proceed tests
    mod can_proceed_tests {
        use super::*;

        #[test]
        fn test_can_proceed_initial_state() {
            let wizard = ConfigWizard::new();
            assert!(!wizard.can_proceed());
        }

        #[test]
        fn test_can_proceed_with_detection() {
            let mut wizard = ConfigWizard::new();
            wizard.state.pi_detection = Some(mock_detection());
            assert!(wizard.can_proceed());
        }

        #[test]
        fn test_can_proceed_model_selection() {
            let mut wizard = ConfigWizard::new();
            wizard.state.step = WizardStep::ModelSelection;
            wizard.state.pi_detection = Some(mock_detection());
            wizard.state.discovery_result = Some(mock_discovery_result());
            assert!(wizard.can_proceed());
        }

        #[test]
        fn test_can_proceed_role_assignment_needs_model() {
            let mut wizard = ConfigWizard::new();
            wizard.state.step = WizardStep::RoleAssignment;
            wizard.state.pi_detection = Some(mock_detection());
            wizard.state.discovery_result = Some(mock_discovery_result());
            assert!(!wizard.can_proceed());

            wizard
                .state
                .selected_models
                .insert("Balanced".to_string(), "claude-sonnet-4-5".to_string());
            assert!(wizard.can_proceed());
        }

        #[test]
        fn test_can_proceed_confirmation_needs_role() {
            let mut wizard = ConfigWizard::new();
            wizard.state.step = WizardStep::Confirmation;
            wizard.state.pi_detection = Some(mock_detection());
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard
                .state
                .selected_models
                .insert("Balanced".to_string(), "claude-sonnet-4-5".to_string());
            assert!(!wizard.can_proceed());

            wizard
                .state
                .role_assignments
                .insert("architect".to_string(), "claude-sonnet-4-5".to_string());
            assert!(wizard.can_proceed());
        }
    }

    // Navigation tests
    mod navigation_tests {
        use super::*;

        #[test]
        fn test_next_step_from_detection() {
            let mut wizard = ConfigWizard::new();
            wizard.next_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::ProviderReview);
        }

        #[test]
        fn test_next_step_from_complete_fails() {
            let mut wizard = ConfigWizard::new();
            wizard.state.step = WizardStep::Complete;
            let result = wizard.next_step();
            assert!(result.is_err());
        }

        #[test]
        fn test_prev_step_from_provider_review() {
            let mut wizard = ConfigWizard::new();
            wizard.state.step = WizardStep::ProviderReview;
            wizard.prev_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::Detection);
        }

        #[test]
        fn test_prev_step_from_detection_fails() {
            let mut wizard = ConfigWizard::new();
            let result = wizard.prev_step();
            assert!(result.is_err());
        }

        #[test]
        fn test_navigation_through_all_steps() {
            let mut wizard = ConfigWizard::new();

            assert_eq!(wizard.state.step, WizardStep::Detection);

            wizard.next_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::ProviderReview);

            wizard.next_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::ModelSelection);

            wizard.next_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::RoleAssignment);

            wizard.next_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::Confirmation);

            wizard.next_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::Complete);

            // Test going back
            wizard.prev_step().unwrap();
            assert_eq!(wizard.state.step, WizardStep::Confirmation);
        }
    }

    // Helper function tests
    mod helper_function_tests {
        use super::*;

        #[test]
        fn test_provider_display_name_anthropic() {
            assert_eq!(
                ConfigWizard::provider_display_name("anthropic"),
                "Anthropic"
            );
        }

        #[test]
        fn test_provider_display_name_openai() {
            assert_eq!(ConfigWizard::provider_display_name("openai"), "OpenAI");
        }

        #[test]
        fn test_provider_display_name_google() {
            assert_eq!(ConfigWizard::provider_display_name("google"), "Google");
        }

        #[test]
        fn test_provider_display_name_groq() {
            assert_eq!(ConfigWizard::provider_display_name("groq"), "Groq");
        }

        #[test]
        fn test_provider_display_name_openrouter() {
            assert_eq!(
                ConfigWizard::provider_display_name("openrouter"),
                "OpenRouter"
            );
        }

        #[test]
        fn test_provider_display_name_unknown() {
            assert_eq!(ConfigWizard::provider_display_name("unknown"), "Unknown");
        }

        #[test]
        fn test_parse_tier_reasoning() {
            assert_eq!(
                ConfigWizard::parse_tier("Reasoning").unwrap(),
                ModelTier::Reasoning
            );
        }

        #[test]
        fn test_parse_tier_fast() {
            assert_eq!(ConfigWizard::parse_tier("Fast").unwrap(), ModelTier::Fast);
        }

        #[test]
        fn test_parse_tier_balanced() {
            assert_eq!(
                ConfigWizard::parse_tier("Balanced").unwrap(),
                ModelTier::Balanced
            );
        }

        #[test]
        fn test_parse_tier_vision() {
            assert_eq!(
                ConfigWizard::parse_tier("Vision").unwrap(),
                ModelTier::Vision
            );
        }

        #[test]
        fn test_parse_tier_coding() {
            assert_eq!(
                ConfigWizard::parse_tier("Coding").unwrap(),
                ModelTier::Coding
            );
        }

        #[test]
        fn test_parse_tier_invalid() {
            assert!(ConfigWizard::parse_tier("Invalid").is_err());
        }
    }

    // State progression tests
    mod state_progression_tests {
        use super::*;

        #[test]
        fn test_full_wizard_flow_manual() {
            let mut wizard = ConfigWizard::new();

            // Initial state
            assert_eq!(wizard.state().step, WizardStep::Detection);
            assert!(!wizard.can_proceed());

            // Simulate detection
            wizard.state.pi_detection = Some(mock_detection());
            wizard.state.discovery_result = Some(mock_discovery_result());
            wizard.state.step = WizardStep::ProviderReview;

            assert!(wizard.can_proceed());

            // Select models
            wizard
                .step3_select_model("Fast", "claude-haiku-4-5")
                .unwrap();
            wizard
                .step3_select_model("Balanced", "claude-sonnet-4-5")
                .unwrap();

            assert_eq!(wizard.state.selected_models.len(), 2);

            // Assign roles
            wizard
                .step4_assign_role("scout", "claude-haiku-4-5")
                .unwrap();
            wizard
                .step4_assign_role("architect", "claude-sonnet-4-5")
                .unwrap();

            assert_eq!(wizard.state.role_assignments.len(), 2);

            // Verify can proceed to confirmation
            assert!(wizard.can_proceed());
        }

        #[test]
        fn test_wizard_preserves_config_from_input() {
            let mut config = PiMonoConfig::default();
            config.enabled = false;
            config.settings.timeout = 600;

            let wizard = ConfigWizard::from_config(config.clone());

            assert!(!wizard.config().enabled);
            assert_eq!(wizard.config().settings.timeout, 600);
        }
    }
}
