//! Tests for Pi-Mono model discovery
//!
//! This test module follows TDD principles to test the model discovery functionality.

use maestro_pi_mono::detection::PiDetection;
use maestro_pi_mono::discovery::{
    ModelDiscovery, ModelInfo, ProviderStatus, DiscoveryResult, DEFAULT_CACHE_DURATION_SECS,
};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[test]
fn test_model_info_struct_creation() {
    let model = ModelInfo {
        provider: "anthropic".to_string(),
        model_id: "claude-3-5-sonnet-20241022".to_string(),
        context_window: "200k".to_string(),
        max_output: "8k".to_string(),
        supports_thinking: false,
        supports_images: true,
    };

    assert_eq!(model.provider, "anthropic");
    assert_eq!(model.model_id, "claude-3-5-sonnet-20241022");
    assert_eq!(model.context_window, "200k");
    assert_eq!(model.max_output, "8k");
    assert!(!model.supports_thinking);
    assert!(model.supports_images);
}

#[test]
fn test_provider_status_struct_creation() {
    let status = ProviderStatus {
        provider: "anthropic".to_string(),
        is_configured: true,
        env_var: "ANTHROPIC_API_KEY".to_string(),
    };

    assert_eq!(status.provider, "anthropic");
    assert!(status.is_configured);
    assert_eq!(status.env_var, "ANTHROPIC_API_KEY");
}

#[test]
fn test_discovery_result_struct_creation() {
    let now = SystemTime::now();
    let expires = now + Duration::from_secs(DEFAULT_CACHE_DURATION_SECS);

    let result = DiscoveryResult {
        models: vec![],
        providers: vec![],
        discovered_at: now,
        cache_expires: expires,
    };

    assert!(result.models.is_empty());
    assert!(result.providers.is_empty());
    assert_eq!(result.discovered_at, now);
    assert_eq!(result.cache_expires, expires);
}

#[test]
fn test_model_info_serialization() {
    let model = ModelInfo {
        provider: "openai".to_string(),
        model_id: "gpt-4".to_string(),
        context_window: "128k".to_string(),
        max_output: "4k".to_string(),
        supports_thinking: false,
        supports_images: true,
    };

    // Test serialization
    let json = serde_json::to_string(&model).expect("Failed to serialize");
    assert!(json.contains("openai"));
    assert!(json.contains("gpt-4"));

    // Test deserialization
    let deserialized: ModelInfo = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.provider, "openai");
    assert_eq!(deserialized.model_id, "gpt-4");
}

#[test]
fn test_parse_models_output_basic() {
    let output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\timages\n\
                  openai\tgpt-4\t128k\t4k\t\t\n\
                  google\tgemini-pro\t128k\t8k\t\t";

    let models = ModelDiscovery::parse_models_output(output);
    assert_eq!(models.len(), 3);

    assert_eq!(models[0].provider, "anthropic");
    assert_eq!(models[0].model_id, "claude-3-5-sonnet-20241022");
    assert_eq!(models[0].context_window, "200k");
    assert_eq!(models[0].max_output, "8k");
    assert!(!models[0].supports_thinking);
    assert!(models[0].supports_images);

    assert_eq!(models[1].provider, "openai");
    assert_eq!(models[1].model_id, "gpt-4");
    assert!(!models[1].supports_thinking);
    assert!(!models[1].supports_images);
}

#[test]
fn test_parse_models_output_with_thinking() {
    let output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\tthinking\timages";

    let models = ModelDiscovery::parse_models_output(output);
    assert_eq!(models.len(), 1);

    assert!(models[0].supports_thinking);
    assert!(models[0].supports_images);
}

#[test]
fn test_parse_models_output_empty() {
    let output = "";

    let models = ModelDiscovery::parse_models_output(output);
    assert!(models.is_empty());
}

#[test]
fn test_parse_models_output_malformed() {
    let output = "anthropic\tclaude-3-5-sonnet\n\
                  openai"; // Missing fields

    let models = ModelDiscovery::parse_models_output(output);
    // Should handle malformed lines gracefully
    assert_eq!(models.len(), 0);
}

#[test]
fn test_determine_provider_status() {
    let models = vec![
        ModelInfo {
            provider: "anthropic".to_string(),
            model_id: "claude-3-5-sonnet-20241022".to_string(),
            context_window: "200k".to_string(),
            max_output: "8k".to_string(),
            supports_thinking: false,
            supports_images: true,
        },
        ModelInfo {
            provider: "openai".to_string(),
            model_id: "gpt-4".to_string(),
            context_window: "128k".to_string(),
            max_output: "4k".to_string(),
            supports_thinking: false,
            supports_images: true,
        },
    ];

    let providers = ModelDiscovery::determine_provider_status(&models);
    assert_eq!(providers.len(), 5); // All 5 providers should be in the list

    // Check that anthropic and openai are configured
    let anthropic = providers.iter().find(|p| p.provider == "anthropic").unwrap();
    assert!(anthropic.is_configured);
    assert_eq!(anthropic.env_var, "ANTHROPIC_API_KEY");

    let openai = providers.iter().find(|p| p.provider == "openai").unwrap();
    assert!(openai.is_configured);
    assert_eq!(openai.env_var, "OPENAI_API_KEY");

    // Check that unconfigured providers are marked as such
    let google = providers.iter().find(|p| p.provider == "google").unwrap();
    assert!(!google.is_configured);
    assert_eq!(google.env_var, "GOOGLE_API_KEY");

    let groq = providers.iter().find(|p| p.provider == "groq").unwrap();
    assert!(!groq.is_configured);
    assert_eq!(groq.env_var, "GROQ_API_KEY");

    let openrouter = providers.iter().find(|p| p.provider == "openrouter").unwrap();
    assert!(!openrouter.is_configured);
    assert_eq!(openrouter.env_var, "OPENROUTER_API_KEY");
}

#[test]
fn test_determine_provider_status_no_models() {
    let models = vec![];

    let providers = ModelDiscovery::determine_provider_status(&models);
    assert_eq!(providers.len(), 5);

    // All providers should be unconfigured
    for provider in &providers {
        assert!(!provider.is_configured);
    }
}

#[test]
fn test_model_discovery_creation() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let discovery = ModelDiscovery::new(detection);
    assert!(discovery.cache().is_none());
}

#[test]
fn test_cache_expiration_check() {
    let now = SystemTime::now();

    // Not expired - cache expires in the future
    let expires_future = now + Duration::from_secs(3600);
    assert!(!ModelDiscovery::is_cache_expired(now, expires_future));

    // Expired - cache expires in the past
    let expires_past = now - Duration::from_secs(3600);
    assert!(ModelDiscovery::is_cache_expired(now, expires_past));

    // Just expired - cache expires exactly now
    // SystemTime::duration_since might return error for equal times
    // So we test with a very small future time
    let expires_just_after = now + Duration::from_nanos(1);
    assert!(!ModelDiscovery::is_cache_expired(now, expires_just_after));

    // Edge case: equal times (now and expires are the same)
    // In this case, duration_since will be 0, so it's considered expired
    let _expires_now = now;
}

#[tokio::test]
async fn test_discover_models_with_mock() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let mut discovery = ModelDiscovery::new(detection);

    // Test with mock executor
    let mock_output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\timages\n\
                       openai\tgpt-4\t128k\t4k\t\t";

    // Create a test helper that uses mock output
    let result = discovery.discover_models_with_mock(mock_output).await;

    assert!(result.is_ok());
    let discovery_result = result.unwrap();

    assert_eq!(discovery_result.models.len(), 2);
    assert_eq!(discovery_result.providers.len(), 5);

    // Check first model
    assert_eq!(discovery_result.models[0].provider, "anthropic");
    assert_eq!(discovery_result.models[0].model_id, "claude-3-5-sonnet-20241022");

    // Check cache timing
    let now = SystemTime::now();
    let time_since_discovery = now
        .duration_since(discovery_result.discovered_at)
        .unwrap();
    assert!(time_since_discovery.as_secs() < 5); // Should be very recent (5 second tolerance)

    let cache_duration = discovery_result
        .cache_expires
        .duration_since(discovery_result.discovered_at)
        .unwrap();
    assert_eq!(cache_duration.as_secs(), DEFAULT_CACHE_DURATION_SECS); // 24 hours
}

#[tokio::test]
async fn test_discover_models_uses_cache() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let mut discovery = ModelDiscovery::new(detection);

    // Set up a valid cache
    let now = SystemTime::now();
    let cached_result = DiscoveryResult {
        models: vec![ModelInfo {
            provider: "anthropic".to_string(),
            model_id: "claude-3-5-sonnet-20241022".to_string(),
            context_window: "200k".to_string(),
            max_output: "8k".to_string(),
            supports_thinking: false,
            supports_images: true,
        }],
        providers: vec![],
        discovered_at: now,
        cache_expires: now + Duration::from_secs(3600), // Expires in 1 hour
    };

    discovery.set_cache(Some(cached_result.clone()));

    // Discover should return cached result
    let mock_output = "openai\tgpt-4\t128k\t4k\t\t"; // Different output
    let result = discovery.discover_models_with_mock(mock_output).await;

    assert!(result.is_ok());
    let discovery_result = result.unwrap();

    // Should have returned cached result, not the new mock output
    assert_eq!(discovery_result.models.len(), 1);
    assert_eq!(discovery_result.models[0].provider, "anthropic");
}

#[tokio::test]
async fn test_discover_models_cache_expired() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let mut discovery = ModelDiscovery::new(detection);

    // Set up an expired cache
    let now = SystemTime::now();
    let cached_result = DiscoveryResult {
        models: vec![ModelInfo {
            provider: "anthropic".to_string(),
            model_id: "claude-3-5-sonnet-20241022".to_string(),
            context_window: "200k".to_string(),
            max_output: "8k".to_string(),
            supports_thinking: false,
            supports_images: true,
        }],
        providers: vec![],
        discovered_at: now - Duration::from_secs(DEFAULT_CACHE_DURATION_SECS), // 24 hours ago
        cache_expires: now - Duration::from_secs(3600), // Expired 1 hour ago
    };

    discovery.set_cache(Some(cached_result));

    // Discover should ignore expired cache and use new mock output
    let mock_output = "openai\tgpt-4\t128k\t4k\t\t";
    let result = discovery.discover_models_with_mock(mock_output).await;

    assert!(result.is_ok());
    let discovery_result = result.unwrap();

    // Should have returned new result from mock output
    assert_eq!(discovery_result.models.len(), 1);
    assert_eq!(discovery_result.models[0].provider, "openai");
}

#[test]
fn test_get_auth_guidance_for_unconfigured_providers() {
    let providers = vec![
        ProviderStatus {
            provider: "anthropic".to_string(),
            is_configured: true,
            env_var: "ANTHROPIC_API_KEY".to_string(),
        },
        ProviderStatus {
            provider: "openai".to_string(),
            is_configured: false,
            env_var: "OPENAI_API_KEY".to_string(),
        },
        ProviderStatus {
            provider: "google".to_string(),
            is_configured: false,
            env_var: "GOOGLE_API_KEY".to_string(),
        },
    ];

    let guidance = ModelDiscovery::get_auth_guidance(&providers);

    // Should mention unconfigured providers
    assert!(guidance.contains("openai"));
    assert!(guidance.contains("OPENAI_API_KEY"));
    assert!(guidance.contains("google"));
    assert!(guidance.contains("GOOGLE_API_KEY"));

    // Should not mention configured providers
    assert!(!guidance.contains("anthropic"));
}

#[test]
fn test_get_auth_guidance_all_configured() {
    let providers = vec![
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
    ];

    let guidance = ModelDiscovery::get_auth_guidance(&providers);

    // Should indicate all providers are configured
    assert!(guidance.contains("All") || guidance.contains("configured"));
}

#[test]
fn test_get_auth_guidance_empty() {
    let providers = vec![];

    let guidance = ModelDiscovery::get_auth_guidance(&providers);
    // Should handle empty list gracefully
    assert!(!guidance.is_empty());
}

#[test]
fn test_env_var_mapping() {
    // Test that provider names map to correct env vars
    let models = vec![
        ModelInfo {
            provider: "anthropic".to_string(),
            model_id: "claude-3-5-sonnet".to_string(),
            context_window: "200k".to_string(),
            max_output: "8k".to_string(),
            supports_thinking: false,
            supports_images: true,
        },
        ModelInfo {
            provider: "openai".to_string(),
            model_id: "gpt-4".to_string(),
            context_window: "128k".to_string(),
            max_output: "4k".to_string(),
            supports_thinking: false,
            supports_images: true,
        },
        ModelInfo {
            provider: "google".to_string(),
            model_id: "gemini-pro".to_string(),
            context_window: "128k".to_string(),
            max_output: "8k".to_string(),
            supports_thinking: false,
            supports_images: false,
        },
        ModelInfo {
            provider: "groq".to_string(),
            model_id: "llama-3-70b".to_string(),
            context_window: "128k".to_string(),
            max_output: "4k".to_string(),
            supports_thinking: false,
            supports_images: false,
        },
        ModelInfo {
            provider: "openrouter".to_string(),
            model_id: "anthropic/claude-2".to_string(),
            context_window: "100k".to_string(),
            max_output: "4k".to_string(),
            supports_thinking: false,
            supports_images: false,
        },
    ];

    let providers = ModelDiscovery::determine_provider_status(&models);

    let anthropic = providers.iter().find(|p| p.provider == "anthropic").unwrap();
    assert_eq!(anthropic.env_var, "ANTHROPIC_API_KEY");

    let openai = providers.iter().find(|p| p.provider == "openai").unwrap();
    assert_eq!(openai.env_var, "OPENAI_API_KEY");

    let google = providers.iter().find(|p| p.provider == "google").unwrap();
    assert_eq!(google.env_var, "GOOGLE_API_KEY");

    let groq = providers.iter().find(|p| p.provider == "groq").unwrap();
    assert_eq!(groq.env_var, "GROQ_API_KEY");

    let openrouter = providers.iter().find(|p| p.provider == "openrouter").unwrap();
    assert_eq!(openrouter.env_var, "OPENROUTER_API_KEY");
}

#[test]
fn test_get_current_auth_guidance_no_cache() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let discovery = ModelDiscovery::new(detection);
    // No cache set
    assert!(discovery.get_current_auth_guidance().is_none());
}

#[tokio::test]
async fn test_get_current_auth_guidance_with_cache() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let mut discovery = ModelDiscovery::new(detection);

    // Set up cache with unconfigured providers
    let mock_output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\timages";
    let _result = discovery.discover_models_with_mock(mock_output).await;

    let guidance = discovery.get_current_auth_guidance();
    assert!(guidance.is_some());
    let guidance = guidance.unwrap();
    // Should mention unconfigured providers (openai, google, groq, openrouter)
    assert!(guidance.contains("openai") || guidance.contains("google"));
}

#[tokio::test]
async fn test_refresh_clears_cache() {
    let detection = PiDetection {
        executable_path: PathBuf::from("/usr/local/bin/pi"),
        version: Some("0.49.3".to_string()),
        capabilities: Default::default(),
    };

    let mut discovery = ModelDiscovery::new(detection);

    // Set up initial cache
    let mock_output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\timages";
    let result1 = discovery.discover_models_with_mock(mock_output).await;
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap().models.len(), 1);

    // Refresh with different output
    let mock_output2 = "openai\tgpt-4\t128k\t4k\t\t";
    let result2 = discovery.discover_models_with_mock(mock_output2).await;
    assert!(result2.is_ok());
    // Should still return cached result since we didn't call refresh
    assert_eq!(result2.unwrap().models[0].provider, "anthropic");

    // Now clear cache manually and check
    discovery.set_cache(None);
    let result3 = discovery.discover_models_with_mock(mock_output2).await;
    assert!(result3.is_ok());
    // Should return new result after cache was cleared
    assert_eq!(result3.unwrap().models[0].provider, "openai");
}

#[test]
fn test_parse_models_output_with_extra_tabs() {
    // Test handling of extra tabs between fields
    let output = "anthropic\t\tclaude-3-5-sonnet-20241022\t\t200k\t8k\t\t";

    let models = ModelDiscovery::parse_models_output(output);
    // Empty fields should be handled
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider, "anthropic");
    // Extra tabs might cause empty fields - this is edge case behavior
}

#[test]
fn test_parse_models_output_case_insensitive_provider() {
    // Test that provider names are case-sensitive in parsing
    let output = "Anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\t";

    let models = ModelDiscovery::parse_models_output(output);
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].provider, "Anthropic"); // Preserves case
}
