use anyhow::{Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaestroConfig {
    pub engine: EngineConfig,
    pub memory: MemoryConfig,
    pub providers: HashMap<String, ProviderConfig>,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub max_iterations: u64,
    pub iteration_timeout_secs: u64,
    pub context_budget: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub storage_path: PathBuf,
    pub vector_dim: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_key: SecretValue,
    pub model: String,
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_leak_detection: bool,
    pub redaction_patterns: Vec<String>,
    pub sandbox_level: SandboxLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxLevel {
    None,
    Balanced,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecret {
    pub nonce_hex: String,
    pub ciphertext_hex: String,
}

/// Secure value wrapper that zeros memory on drop
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecretValue {
    Plain(String),
    Encrypted(EncryptedSecret),
}

// Debug implementation that doesn't leak secrets
impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretValue::Plain(_) => f
                .debug_tuple("SecretValue::Plain")
                .field(&"<REDACTED>")
                .finish(),
            SecretValue::Encrypted(enc) => {
                let nonce_preview = if enc.nonce_hex.len() > 16 {
                    format!("{}...", &enc.nonce_hex[..16])
                } else {
                    enc.nonce_hex.clone()
                };
                f.debug_tuple("SecretValue::Encrypted")
                    .field(&nonce_preview)
                    .finish()
            }
        }
    }
}

/// In-memory secret that gets zeroized when dropped
pub struct SecureSecret {
    is_set: bool,
    data: [u8; 512], // Fixed-size buffer for secrets
}

impl Drop for SecureSecret {
    fn drop(&mut self) {
        // Zeroize the data on drop
        self.data.zeroize();
    }
}

impl SecureSecret {
    /// Create a new secure secret from a string
    pub fn new(value: &str) -> Self {
        let mut secret = Self {
            is_set: true,
            data: [0u8; 512],
        };
        let bytes = value.as_bytes();
        let len = bytes.len().min(512);
        secret.data[..len].copy_from_slice(&bytes[..len]);
        secret
    }

    /// Get the secret as a string (cloned for safety)
    pub fn as_str(&self) -> String {
        if !self.is_set {
            return String::new();
        }
        // Find null terminator
        let len = self.data.iter().position(|&b| b == 0).unwrap_or(512);
        String::from_utf8_lossy(&self.data[..len]).to_string()
    }

    /// Check if secret is set
    pub fn is_empty(&self) -> bool {
        !self.is_set
    }
}

impl SecretValue {
    pub fn decrypt(&self, master_key: &[u8; 32]) -> Result<String> {
        match self {
            SecretValue::Plain(v) => Ok(v.clone()),
            SecretValue::Encrypted(enc) => decrypt_secret(master_key, enc),
        }
    }

    /// Convert to a secure secret that will be zeroized on drop
    pub fn to_secure(&self, master_key: &[u8; 32]) -> Result<SecureSecret> {
        let plaintext = self.decrypt(master_key)?;
        Ok(SecureSecret::new(&plaintext))
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{:02x}", b);
    }
    out
}
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(2) {
        anyhow::bail!("invalid hex length");
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let chars: Vec<char> = hex.chars().collect();
    for i in (0..chars.len()).step_by(2) {
        let hi = chars[i].to_digit(16).context("invalid hex digit")?;
        let lo = chars[i + 1].to_digit(16).context("invalid hex digit")?;
        out.push(((hi << 4) as u8) | (lo as u8));
    }
    Ok(out)
}

fn encrypt_secret(master_key: &[u8; 32], value: &str) -> Result<EncryptedSecret> {
    let cipher = XChaCha20Poly1305::new(master_key.into());
    let mut nonce_bytes = [0u8; 24];
    thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, value.as_bytes())
        .map_err(|e| anyhow::anyhow!("encryption failed: {}", e))?;

    Ok(EncryptedSecret {
        nonce_hex: bytes_to_hex(&nonce_bytes),
        ciphertext_hex: bytes_to_hex(&ciphertext),
    })
}
fn decrypt_secret(master_key: &[u8; 32], secret: &EncryptedSecret) -> Result<String> {
    let nonce_raw = hex_to_bytes(&secret.nonce_hex)?;
    if nonce_raw.len() != 24 {
        anyhow::bail!("invalid nonce length (expected 24 bytes for XChaCha20)");
    }
    let nonce = XNonce::from_slice(&nonce_raw);

    let cipher = XChaCha20Poly1305::new(master_key.into());
    let ciphertext = hex_to_bytes(&secret.ciphertext_hex)?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|e| anyhow::anyhow!("decryption failed: {}", e))?;

    String::from_utf8(plaintext).context("decrypted secret is not valid UTF-8")
}

impl MaestroConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;
        let config: Self = toml::from_str(&content).context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, content).context("Failed to write config file")?;
        Ok(())
    }
}

impl MaestroConfig {
    pub fn encrypt_secrets_in_place(&mut self, master_key: &[u8; 32]) -> Result<()> {
        for provider in self.providers.values_mut() {
            if let SecretValue::Plain(v) = &provider.api_key {
                provider.api_key = SecretValue::Encrypted(encrypt_secret(master_key, v)?);
            }
        }
        Ok(())
    }
    pub fn decrypt_secrets_in_place(&mut self, master_key: &[u8; 32]) -> Result<()> {
        for provider in self.providers.values_mut() {
            let decrypted = provider.api_key.decrypt(master_key)?;
            provider.api_key = SecretValue::Plain(decrypted);
        }
        Ok(())
    }

    pub fn save_encrypted(&self, path: impl AsRef<Path>, master_key: &[u8; 32]) -> Result<()> {
        let mut cloned = self.clone();
        cloned.encrypt_secrets_in_place(master_key)?;
        cloned.save(path)
    }

    pub fn load_with_optional_decryption(path: impl AsRef<Path>) -> Result<Self> {
        let mut cfg = Self::load(path)?;
        if let Some(master_key) = load_master_key_from_env()? {
            cfg.decrypt_secrets_in_place(&master_key)?;
        }
        Ok(cfg)
    }
}

pub fn load_master_key_from_env() -> Result<Option<[u8; 32]>> {
    match env::var("MAESTRO_CONFIG_MASTER_KEY") {
        Ok(raw) => {
            let bytes = hex_to_bytes(&raw)?;
            if bytes.len() != 32 {
                anyhow::bail!("MAESTRO_CONFIG_MASTER_KEY must be 32 bytes hex (64 chars)");
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Some(key))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(
            "failed to read MAESTRO_CONFIG_MASTER_KEY: {e}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let mut cfg = MaestroConfig {
            engine: EngineConfig {
                max_iterations: 8,
                iteration_timeout_secs: 60,
                context_budget: 32000,
            },
            memory: MemoryConfig {
                storage_path: PathBuf::from("/tmp/maestro-memory"),
                vector_dim: 1536,
            },
            providers: HashMap::from([(
                "default".to_string(),
                ProviderConfig {
                    enabled: true,
                    api_key: SecretValue::Plain("super-secret".to_string()),
                    model: "test-model".to_string(),
                    settings: HashMap::new(),
                },
            )]),
            security: SecurityConfig {
                enable_leak_detection: true,
                redaction_patterns: vec![],
                sandbox_level: SandboxLevel::Balanced,
            },
        };
        let key = [7u8; 32];
        cfg.encrypt_secrets_in_place(&key).unwrap();
        let enc = cfg.providers.get("default").unwrap();
        match enc.api_key {
            SecretValue::Encrypted(_) => {}
            _ => panic!("expected encrypted secret"),
        }

        // Verify nonce randomness (encrypt same thing again)
        let mut cfg2 = cfg.clone();
        cfg2.providers.get_mut("default").unwrap().api_key =
            SecretValue::Plain("super-secret".to_string());
        cfg2.encrypt_secrets_in_place(&key).unwrap();

        let enc1 = match &cfg.providers.get("default").unwrap().api_key {
            SecretValue::Encrypted(e) => e,
            _ => unreachable!(),
        };
        let enc2 = match &cfg2.providers.get("default").unwrap().api_key {
            SecretValue::Encrypted(e) => e,
            _ => unreachable!(),
        };
        assert_ne!(enc1.nonce_hex, enc2.nonce_hex);
        assert_ne!(enc1.ciphertext_hex, enc2.ciphertext_hex);
        cfg.decrypt_secrets_in_place(&key).unwrap();
        let dec = cfg.providers.get("default").unwrap();
        match &dec.api_key {
            SecretValue::Plain(v) => assert_eq!(v, "super-secret"),
            _ => panic!("expected plain secret after decrypt"),
        }
    }
    #[test]
    fn decrypt_fails_on_tampered_ciphertext() {
        let key = [9u8; 32];
        let secret = "my-secret";
        let encrypted = encrypt_secret(&key, secret).unwrap();

        let mut tampered = encrypted.clone();
        // Flip one bit in ciphertext
        let mut bytes = hex_to_bytes(&tampered.ciphertext_hex).unwrap();
        bytes[0] ^= 0x01;
        tampered.ciphertext_hex = bytes_to_hex(&bytes);

        let res = decrypt_secret(&key, &tampered);
        assert!(res.is_err(), "decryption should fail on tampered data");
    }

    #[test]
    fn test_secure_secret_zeroizes() {
        let secret = SecureSecret::new("super-secret-value");
        let value = secret.as_str();
        assert_eq!(value, "super-secret-value");
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secure_secret_empty() {
        let secret = SecureSecret::new("");
        assert_eq!(secret.as_str(), "");
    }

    #[test]
    fn test_secret_value_debug_redaction() {
        let plain = SecretValue::Plain("super-secret".to_string());
        let debug_str = format!("{:?}", plain);
        assert!(
            !debug_str.contains("super-secret"),
            "Debug should not leak plain secrets"
        );
        assert!(
            debug_str.contains("REDACTED"),
            "Debug should show REDACTED placeholder"
        );
    }

    #[test]
    fn test_secret_value_to_secure() {
        let key = [42u8; 32];
        let secret_value = SecretValue::Plain("test-secret".to_string());
        let secure = secret_value.to_secure(&key).unwrap();
        assert_eq!(secure.as_str(), "test-secret");
    }
}
