//! Model discovery for Pi-Mono CLI
//!
//! This module provides functionality for discovering available models
//! and provider authentication status.

use crate::detection::PiDetection;
use crate::error::{DetectionError, Error, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tracing::warn;

/// Default cache duration for model discovery results (24 hours in seconds)
pub const DEFAULT_CACHE_DURATION_SECS: u64 = 86400;

/// Information about a single model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: String,
    pub model_id: String,
    pub context_window: String,
    pub max_output: String,
    pub supports_thinking: bool,
    pub supports_images: bool,
}

/// Provider authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub is_configured: bool,
    pub env_var: String,
}

/// Model discovery result with cache info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub models: Vec<ModelInfo>,
    pub providers: Vec<ProviderStatus>,
    pub discovered_at: SystemTime,
    pub cache_expires: SystemTime,
}

/// Model discovery service
pub struct ModelDiscovery {
    pi_detection: PiDetection,
    cache: Option<DiscoveryResult>,
}

impl ModelDiscovery {
    /// Create a new model discovery service
    pub fn new(pi_detection: PiDetection) -> Self {
        Self {
            pi_detection,
            cache: None,
        }
    }

    /// Get the current cache (if any)
    pub fn cache(&self) -> Option<&DiscoveryResult> {
        self.cache.as_ref()
    }

    /// Set the cache (for testing purposes)
    pub fn set_cache(&mut self, cache: Option<DiscoveryResult>) {
        self.cache = cache;
    }

    /// Check if cache is expired
    ///
    /// # Logic Explanation
    ///
    /// The cache expiration logic uses the following rules:
    ///
    /// 1. **Valid Cache**: `expires` is in the future relative to `now`
    ///    - `duration_since(now)` succeeds (returns `Ok(Duration)`)
    ///    - Duration is positive (non-zero)
    ///    - Returns `false` (cache is still valid)
    ///
    /// 2. **Expired Cache**: `expires` is in the past relative to `now`
    ///    - `duration_since(now)` fails (returns `Err`)
    ///    - This happens when `now > expires`
    ///    - Returns `true` (cache is expired)
    ///
    /// 3. **Edge Case - Exactly at expiration**: `expires` equals `now`
    ///    - `duration_since(now)` succeeds with zero duration
    ///    - `is_zero()` returns `true`
    ///    - Returns `true` (cache is expired)
    ///
    /// # Returns
    ///
    /// * `false` if the cache is still valid (expires time is in the future)
    /// * `true` if the cache has expired (expires time is in the past or equals now)
    pub fn is_cache_expired(now: SystemTime, expires: SystemTime) -> bool {
        // Attempt to calculate the duration from now to expires
        match expires.duration_since(now) {
            Ok(time_until_expiry) => {
                // Success: expires is in the future or equal to now
                // Cache is expired only when there's NO time remaining (duration is exactly zero)
                // A positive duration means the cache is still valid
                time_until_expiry.is_zero()
            }
            Err(_) => {
                // Error: now is after expires (expires is in the past)
                // This happens when SystemTime arithmetic underflows because now > expires
                // Cache is definitely expired
                true
            }
        }
    }

    /// Parse models output from `pi --list-models`
    ///
    /// Expected format: tab-separated values
    /// provider\tmodel_id\tcontext_window\tmax_output\tthinking\timages
    pub fn parse_models_output(output: &str) -> Vec<ModelInfo> {
        output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() < 4 {
                    // Malformed line - skip it
                    return None;
                }

                Some(ModelInfo {
                    provider: parts[0].to_string(),
                    model_id: parts[1].to_string(),
                    context_window: parts[2].to_string(),
                    max_output: parts[3].to_string(),
                    supports_thinking: parts.get(4).map(|&s| !s.is_empty()).unwrap_or(false),
                    supports_images: parts.get(5).map(|&s| !s.is_empty()).unwrap_or(false),
                })
            })
            .collect()
    }

    /// Determine provider status based on discovered models
    pub fn determine_provider_status(models: &[ModelInfo]) -> Vec<ProviderStatus> {
        // All supported providers with their env vars
        let all_providers = Self::get_providers_with_env_vars();

        // Find which providers have at least one model (case-insensitive)
        let configured_providers: std::collections::HashSet<String> =
            models.iter().map(|m| m.provider.to_lowercase()).collect();

        all_providers
            .into_iter()
            .map(|(provider, env_var)| {
                let is_configured = configured_providers.contains(&provider.to_lowercase());
                ProviderStatus {
                    provider,
                    is_configured,
                    env_var,
                }
            })
            .collect()
    }

    /// Get all supported providers with their environment variable names
    fn get_providers_with_env_vars() -> Vec<(String, String)> {
        vec![
            ("anthropic".to_string(), "ANTHROPIC_API_KEY".to_string()),
            ("openai".to_string(), "OPENAI_API_KEY".to_string()),
            ("google".to_string(), "GOOGLE_API_KEY".to_string()),
            ("groq".to_string(), "GROQ_API_KEY".to_string()),
            ("openrouter".to_string(), "OPENROUTER_API_KEY".to_string()),
        ]
    }

    /// Get authentication guidance for unconfigured providers
    pub fn get_auth_guidance(providers: &[ProviderStatus]) -> String {
        let unconfigured: Vec<&ProviderStatus> =
            providers.iter().filter(|p| !p.is_configured).collect();

        if unconfigured.is_empty() {
            return "All providers are configured.".to_string();
        }

        let mut guidance = String::from("Unconfigured providers:\n");
        for provider in unconfigured {
            guidance.push_str(&format!(
                "  - {}: Set {} environment variable\n",
                provider.provider, provider.env_var
            ));
        }

        guidance
    }

    /// Discover available models (with mock executor for testing)
    pub async fn discover_models_with_mock(
        &mut self,
        mock_output: &str,
    ) -> Result<DiscoveryResult> {
        let now = SystemTime::now();

        // Check if we have a valid cache
        if let Some(cached) = &self.cache {
            if !Self::is_cache_expired(now, cached.cache_expires) {
                return Ok(cached.clone());
            }
        }

        // Parse mock output
        let models = Self::parse_models_output(mock_output);

        // Warn if output was successful but no models were found
        if !mock_output.is_empty() && models.is_empty() {
            warn!(
                "pi --list-models returned output but no models were parsed. Output: {:?}",
                mock_output
            );
        }

        let providers = Self::determine_provider_status(&models);

        // Use checked_add to prevent potential overflow (though unlikely with 24-hour duration)
        let cache_expires = now
            .checked_add(Duration::from_secs(DEFAULT_CACHE_DURATION_SECS))
            .expect("Cache duration overflow - system time is too far in the future");

        let result = DiscoveryResult {
            discovered_at: now,
            cache_expires,
            models,
            providers,
        };

        self.cache = Some(result.clone());
        Ok(result)
    }

    /// Discover available models by executing `pi --list-models`
    pub async fn discover_models(&mut self) -> Result<DiscoveryResult> {
        let now = SystemTime::now();

        // Check if we have a valid cache
        if let Some(cached) = &self.cache {
            if !Self::is_cache_expired(now, cached.cache_expires) {
                return Ok(cached.clone());
            }
        }

        // Execute pi --list-models
        let executable_path = self.pi_detection.executable_path.clone();
        let executable_path_str = format!("{:?}", executable_path);

        let output = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::task::spawn_blocking(move || {
                Command::new(&executable_path).arg("--list-models").output()
            }),
        )
        .await
        .map_err(|_| {
            Error::Detection(DetectionError::ExecutionFailed {
                command: executable_path_str.clone(),
                reason: "Command timed out after 10 seconds".to_string(),
            })
        })?
        .map_err(|e| {
            Error::Detection(DetectionError::ExecutionFailed {
                command: executable_path_str.clone(),
                reason: format!("Task join failed: {}", e),
            })
        })?
        .map_err(|e| {
            Error::Detection(DetectionError::ExecutionFailed {
                command: executable_path_str,
                reason: format!("Failed to execute: {}", e),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Detection(DetectionError::ExecutionFailed {
                command: format!("{:?} --list-models", self.pi_detection.executable_path),
                reason: stderr.to_string(),
            }));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the output
        let models = Self::parse_models_output(&stdout);

        // Warn if command succeeded but no models were found
        if !stdout.is_empty() && models.is_empty() {
            warn!(
                "pi --list-models returned success but no models were parsed. Output: {:?}",
                &stdout[..stdout.len().min(200)]
            );
        }

        let providers = Self::determine_provider_status(&models);

        // Use checked_add to prevent potential overflow (though unlikely with 24-hour duration)
        let cache_expires = now
            .checked_add(Duration::from_secs(DEFAULT_CACHE_DURATION_SECS))
            .expect("Cache duration overflow - system time is too far in the future");

        let result = DiscoveryResult {
            discovered_at: now,
            cache_expires,
            models,
            providers,
        };

        self.cache = Some(result.clone());
        Ok(result)
    }

    /// Force a refresh of the model discovery, bypassing cache
    pub async fn refresh(&mut self) -> Result<DiscoveryResult> {
        // Clear the cache
        self.cache = None;
        // Perform discovery
        self.discover_models().await
    }

    /// Get authentication guidance for current providers
    pub fn get_current_auth_guidance(&self) -> Option<String> {
        self.cache
            .as_ref()
            .map(|result| Self::get_auth_guidance(&result.providers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_models_output_basic() {
        let output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\timages\n\
                      openai\tgpt-4\t128k\t4k\t\t";

        let models = ModelDiscovery::parse_models_output(output);
        assert_eq!(models.len(), 2);

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
                      openai";

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
        assert_eq!(providers.len(), 5);

        let anthropic = providers
            .iter()
            .find(|p| p.provider == "anthropic")
            .unwrap();
        assert!(anthropic.is_configured);
        assert_eq!(anthropic.env_var, "ANTHROPIC_API_KEY");

        let openai = providers.iter().find(|p| p.provider == "openai").unwrap();
        assert!(openai.is_configured);
        assert_eq!(openai.env_var, "OPENAI_API_KEY");

        let google = providers.iter().find(|p| p.provider == "google").unwrap();
        assert!(!google.is_configured);
        assert_eq!(google.env_var, "GOOGLE_API_KEY");
    }

    #[test]
    fn test_determine_provider_status_no_models() {
        let models = vec![];

        let providers = ModelDiscovery::determine_provider_status(&models);
        assert_eq!(providers.len(), 5);

        for provider in &providers {
            assert!(!provider.is_configured);
        }
    }

    #[test]
    fn test_get_auth_guidance() {
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

        assert!(guidance.contains("openai"));
        assert!(guidance.contains("OPENAI_API_KEY"));
        assert!(guidance.contains("google"));
        assert!(guidance.contains("GOOGLE_API_KEY"));
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

        assert!(guidance.contains("All") || guidance.contains("configured"));
    }

    #[test]
    fn test_is_cache_expired() {
        let now = SystemTime::now();

        // Not expired
        let expires_future = now + Duration::from_secs(3600);
        assert!(!ModelDiscovery::is_cache_expired(now, expires_future));

        // Expired
        let expires_past = now - Duration::from_secs(3600);
        assert!(ModelDiscovery::is_cache_expired(now, expires_past));
    }

    #[tokio::test]
    async fn test_discover_models_with_mock() {
        let detection = PiDetection {
            executable_path: PathBuf::from("/usr/local/bin/pi"),
            version: Some("0.49.3".to_string()),
            capabilities: Default::default(),
        };

        let mut discovery = ModelDiscovery::new(detection);

        let mock_output = "anthropic\tclaude-3-5-sonnet-20241022\t200k\t8k\t\timages\n\
                           openai\tgpt-4\t128k\t4k\t\t";

        let result = discovery.discover_models_with_mock(mock_output).await;

        assert!(result.is_ok());
        let discovery_result = result.unwrap();

        assert_eq!(discovery_result.models.len(), 2);
        assert_eq!(discovery_result.providers.len(), 5);

        // Check cache timing
        let cache_duration = discovery_result
            .cache_expires
            .duration_since(discovery_result.discovered_at)
            .unwrap();
        assert_eq!(cache_duration.as_secs(), 86400);
    }

    #[tokio::test]
    async fn test_discover_models_uses_cache() {
        let detection = PiDetection {
            executable_path: PathBuf::from("/usr/local/bin/pi"),
            version: Some("0.49.3".to_string()),
            capabilities: Default::default(),
        };

        let mut discovery = ModelDiscovery::new(detection);

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
            cache_expires: now + Duration::from_secs(3600),
        };

        discovery.set_cache(Some(cached_result.clone()));

        let mock_output = "openai\tgpt-4\t128k\t4k\t\t";
        let result = discovery.discover_models_with_mock(mock_output).await;

        assert!(result.is_ok());
        let discovery_result = result.unwrap();

        // Should have returned cached result
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
            discovered_at: now - Duration::from_secs(86400),
            cache_expires: now - Duration::from_secs(3600),
        };

        discovery.set_cache(Some(cached_result));

        let mock_output = "openai\tgpt-4\t128k\t4k\t\t";
        let result = discovery.discover_models_with_mock(mock_output).await;

        assert!(result.is_ok());
        let discovery_result = result.unwrap();

        // Should have returned new result from mock output
        assert_eq!(discovery_result.models.len(), 1);
        assert_eq!(discovery_result.models[0].provider, "openai");
    }
}
