use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub editor: String,
    pub install_path: String,
    pub theme: String,
    pub selected_tools: Vec<String>,
    /// When true, uses terminal's background (transparent) instead of theme's background
    /// while keeping the theme's color scheme for text and accents
    pub transparent: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: "hx".to_string(),
            install_path: "~/.maestro".to_string(),
            theme: "catppuccin-mocha".to_string(),
            selected_tools: Vec::new(),
            transparent: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("maestro").join("config.toml");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(config_dir) = dirs::config_dir() {
            let maestro_conf = config_dir.join("maestro");
            if !maestro_conf.exists() {
                fs::create_dir_all(&maestro_conf)?;
            }
            let config_path = maestro_conf.join("config.toml");
            let toml_string = toml::to_string(self)?;
            fs::write(config_path, toml_string)?;
        }
        Ok(())
    }
}
