// Example demonstrating the config I/O API
use maestro_pi_mono::{
    config_dir, config_path, default_config, ensure_config_dir, validate_config_basic,
};

fn main() {
    println!("=== Pi-Mono Config I/O Demo ===\n");

    // Get config directory
    let dir = config_dir().unwrap();
    println!("Config directory: {}", dir.display());

    // Get config path
    let path = config_path().unwrap();
    println!("Config path: {}", path.display());

    // Ensure config directory exists
    let ensured = ensure_config_dir().unwrap();
    println!("Ensured directory: {}", ensured.display());

    // Create default config
    let config = default_config();
    println!("\nDefault config:");
    println!("  Version: {}", config.version);
    println!("  Enabled: {}", config.enabled);
    println!("  Providers: {}", config.providers.len());
    println!("  Timeout: {}s", config.settings.timeout);

    // Validate config
    match validate_config_basic(&config) {
        Ok(_) => println!("\nConfig validation: PASSED"),
        Err(e) => println!("\nConfig validation: FAILED - {}", e),
    }

    // List providers
    println!("\nConfigured providers:");
    for (name, provider) in &config.providers {
        println!("  - {} ({})", provider.display_name, name);
        println!("    Env var: {}", provider.env_var);
        println!("    Configured: {}", provider.is_configured);
    }

    println!("\n=== Demo Complete ===");
}
