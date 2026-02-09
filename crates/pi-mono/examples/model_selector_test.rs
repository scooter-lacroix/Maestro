use maestro_pi_mono::{ModelSelector, ModelConfig, ModelTier, ModelPreference, ProviderConfig};
use std::collections::HashMap;

fn main() {
    let mut providers = HashMap::new();
    providers.insert("anthropic".to_string(), ProviderConfig {
        display_name: "Anthropic".to_string(),
        is_configured: true,
        env_var: "ANTHROPIC_API_KEY".to_string(),
    });

    let config = ModelConfig {
        providers,
        model_preferences: vec![
            ModelPreference {
                model_id: "claude-sonnet-4-5".to_string(),
                provider: "anthropic".to_string(),
                tier: ModelTier::Balanced,
                is_default: true,
            },
        ],
        ..Default::default()
    };

    let selector = ModelSelector::new(&config);
    println!("ModelSelector created successfully!");

    let result = selector.select_by_tier(ModelTier::Balanced);
    println!("Selected model: {:?}", result);
}
