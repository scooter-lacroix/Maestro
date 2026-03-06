//! Transparency support via OSC 111 sequences for foot terminal

use anyhow::Result;
use std::io::Write;

/// OSC 111 transparency sequence prefix
pub const OSC_111_PREFIX: &str = "\x1b]111;";

/// OSC 111 transparency sequence suffix
pub const OSC_111_SUFFIX: &str = "\x07";

/// Transparency configuration
#[derive(Debug, Clone, Copy)]
pub struct TransparencyConfig {
    /// Alpha value (0-255, where 0 is fully transparent and 255 is fully opaque)
    pub alpha: u8,
    /// Whether transparency is enabled
    pub enabled: bool,
}

impl Default for TransparencyConfig {
    fn default() -> Self {
        Self {
            alpha: 200, // ~78% opacity
            enabled: true,
        }
    }
}

impl TransparencyConfig {
    /// Create a new transparency config with the specified alpha
    pub fn new(alpha: u8) -> Self {
        Self {
            alpha,
            enabled: true,
        }
    }

    /// Create a disabled transparency config
    pub fn disabled() -> Self {
        Self {
            alpha: 255,
            enabled: false,
        }
    }

    /// Generate the OSC 111 sequence for this config
    pub fn to_sequence(&self) -> String {
        if self.enabled {
            format!("{}{}{}", OSC_111_PREFIX, self.alpha, OSC_111_SUFFIX)
        } else {
            format!("{}{}", OSC_111_PREFIX, OSC_111_SUFFIX)
        }
    }
}

/// Generate OSC 111 transparency sequence for a given alpha value (0-255)
pub fn transparency_sequence(alpha: u8) -> String {
    format!("{}{}{}", OSC_111_PREFIX, alpha, OSC_111_SUFFIX)
}

/// Reset transparency to terminal default
pub fn reset_transparency_sequence() -> String {
    format!("{}{}", OSC_111_PREFIX, OSC_111_SUFFIX)
}

/// Apply transparency directly to the terminal
///
/// This writes the OSC 111 sequence directly to /dev/tty,
/// bypassing any multiplexer buffering.
pub fn apply_transparency(alpha: u8) -> Result<()> {
    let sequence = transparency_sequence(alpha);
    write_to_tty(&sequence)
}

/// Reset transparency on the terminal
pub fn reset_transparency() -> Result<()> {
    let sequence = reset_transparency_sequence();
    write_to_tty(&sequence)
}

/// Write a string directly to /dev/tty
fn write_to_tty(s: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .or_else(|_| {
            // Fallback: try stdout
            std::fs::OpenOptions::new().write(true).open("/dev/stdout")
        })?;

    file.write_all(s.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Shell hook script for fish shell to maintain transparency
pub fn fish_transparency_hook(alpha: u8) -> String {
    format!(
        r#"# Maestro transparency hook for fish shell
function __maestro_transparency --on-event fish_prompt
    echo -n "{}"
end
"#,
        transparency_sequence(alpha)
    )
}

/// Shell hook script for bash to maintain transparency
pub fn bash_transparency_hook(alpha: u8) -> String {
    format!(
        r#"# Maestro transparency hook for bash
__maestro_transparency() {{
    echo -n "{}"
}}
PROMPT_COMMAND="__maestro_transparency${{PROMPT_COMMAND:+; $PROMPT_COMMAND}}"
"#,
        transparency_sequence(alpha)
    )
}

/// Shell hook script for zsh to maintain transparency
pub fn zsh_transparency_hook(alpha: u8) -> String {
    format!(
        r#"# Maestro transparency hook for zsh
__maestro_transparency() {{
    echo -n "{}"
}}
precmd_functions+=(__maestro_transparency)
"#,
        transparency_sequence(alpha)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transparency_config_default() {
        let config = TransparencyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.alpha, 200);
    }

    #[test]
    fn test_transparency_sequence_format() {
        let seq = transparency_sequence(128);
        assert!(seq.starts_with("\x1b]111;"));
        assert!(seq.ends_with("\x07"));
        assert!(seq.contains("128"));
    }

    #[test]
    fn test_reset_sequence() {
        let seq = reset_transparency_sequence();
        assert_eq!(seq, "\x1b]111;\x07");
    }

    #[test]
    fn test_shell_hooks() {
        let alpha = 180;
        let expected = transparency_sequence(alpha);

        let fish = fish_transparency_hook(alpha);
        assert!(fish.contains(&expected));
        assert!(fish.contains("fish_prompt"));

        let bash = bash_transparency_hook(alpha);
        assert!(bash.contains(&expected));
        assert!(bash.contains("PROMPT_COMMAND"));

        let zsh = zsh_transparency_hook(alpha);
        assert!(zsh.contains(&expected));
        assert!(zsh.contains("precmd_functions"));
    }
}
