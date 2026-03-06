//! Integration tests for model configuration public API
//!
//! These tests verify that the model configuration types are properly
//! re-exported and can be used through the public API.

use maestro_pi_mono::{
    ExecutionSettings, ModelConfig, ModelPreference, ModelTier, ProviderConfig, RoleAssignment,
};

#[test]
fn test_model_tier_public_api() {
    let tier = ModelTier::Balanced;
    assert_eq!(tier, ModelTier::Balanced);
}

#[test]
fn test_model_preference_public_api() {
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
fn test_provider_config_public_api() {
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
fn test_role_assignment_public_api() {
    let role = RoleAssignment {
        model_id: "claude-sonnet-4-5".to_string(),
        provider: "anthropic".to_string(),
        fallback_models: Some(vec!["gpt-4o-mini".to_string()]),
        use_reasoning: Some(true),
    };
    assert_eq!(role.model_id, "claude-sonnet-4-5");
    assert_eq!(role.use_reasoning, Some(true));
}

#[test]
fn test_execution_settings_public_api() {
    let settings = ExecutionSettings::default();
    assert_eq!(settings.timeout, 300);
    assert_eq!(settings.parallel_limit, 4);
    assert!(settings.chain_mode);
    assert!(settings.streaming);
}

#[test]
fn test_model_config_public_api() {
    let config = ModelConfig::default();
    assert!(config.enabled);
    assert_eq!(config.version, "1.0");
}

#[test]
fn test_complete_configuration_example() {
    // This test demonstrates creating a complete configuration
    // matching the YAML spec from the requirements

    use std::collections::HashMap;

    // Create providers
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

    // Create model preferences
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

    // Create role assignments
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

    // Create settings
    let settings = ExecutionSettings {
        timeout: 300,
        parallel_limit: 4,
        chain_mode: true,
        streaming: true,
    };

    // Create the full configuration
    let config = ModelConfig {
        version: "1.0".to_string(),
        enabled: true,
        path: Some("/usr/local/bin/pi".to_string()),
        version_info: Some("0.49.3".to_string()),
        providers,
        model_preferences,
        role_assignments,
        settings,
    };

    // Verify all fields
    assert_eq!(config.version, "1.0");
    assert!(config.enabled);
    assert_eq!(config.path.unwrap(), "/usr/local/bin/pi");
    assert_eq!(config.version_info.unwrap(), "0.49.3");
    assert_eq!(config.providers.len(), 5);
    assert_eq!(config.model_preferences.len(), 2);
    assert_eq!(config.role_assignments.len(), 4);
    assert_eq!(config.settings.timeout, 300);
    assert_eq!(config.settings.parallel_limit, 4);
    assert!(config.settings.chain_mode);
    assert!(config.settings.streaming);

    // Verify provider configuration status
    assert!(config.providers["anthropic"].is_configured);
    assert!(config.providers["openai"].is_configured);
    assert!(!config.providers["google"].is_configured);
    assert!(!config.providers["groq"].is_configured);
    assert!(!config.providers["openrouter"].is_configured);
}
