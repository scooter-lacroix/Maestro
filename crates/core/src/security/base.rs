use crate::config::SecurityConfig;
use crate::traits::{LeakCheckResult, Message, Observer};
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

pub struct SecurityObserver {
    enabled: bool,
    patterns: Vec<Regex>,
}

impl SecurityObserver {
    pub fn new(config: &SecurityConfig) -> Result<Self> {
        let mut patterns = Vec::new();
        for pattern in &config.redaction_patterns {
            patterns.push(Regex::new(pattern)?);
        }

        Ok(Self {
            enabled: config.enable_leak_detection,
            patterns,
        })
    }
}

#[async_trait]
impl Observer for SecurityObserver {
    async fn observe_tool_execution(
        &self,
        _tool_name: &str,
        input: &Value,
        output: &Result<Value>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Scan input
        let input_str = serde_json::to_string(input)?;
        let input_check = self.scan_for_leaks(&input_str).await?;
        if !input_check.is_safe {
            tracing::warn!(
                "Potential leak detected in tool input: {:?}",
                input_check.findings
            );
        }

        // Scan output if successful
        if let Ok(val) = output {
            let output_str = serde_json::to_string(val)?;
            let output_check = self.scan_for_leaks(&output_str).await?;
            if !output_check.is_safe {
                tracing::warn!(
                    "Potential leak detected in tool output: {:?}",
                    output_check.findings
                );
            }
        }

        Ok(())
    }

    async fn observe_message(&self, message: &Message) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let check = self.scan_for_leaks(&message.content).await?;
        if !check.is_safe {
            tracing::warn!("Potential leak detected in message: {:?}", check.findings);
        }

        Ok(())
    }

    async fn scan_for_leaks(&self, data: &str) -> Result<LeakCheckResult> {
        if !self.enabled {
            return Ok(LeakCheckResult {
                is_safe: true,
                findings: Vec::new(),
            });
        }

        let mut findings = Vec::new();
        for regex in &self.patterns {
            if regex.is_match(data) {
                findings.push(regex.as_str().to_string());
            }
        }

        Ok(LeakCheckResult {
            is_safe: findings.is_empty(),
            findings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SandboxLevel;

    #[tokio::test]
    async fn test_leak_detection() {
        let config = SecurityConfig {
            enable_leak_detection: true,
            redaction_patterns: vec![r"sk-[a-zA-Z0-9]{48}".to_string()],
            sandbox_level: SandboxLevel::None,
        };

        let observer = SecurityObserver::new(&config).unwrap();

        let safe_data = "This is safe data";
        let check = observer.scan_for_leaks(safe_data).await.unwrap();
        assert!(check.is_safe);

        let unsafe_data = "My key is sk-123456789012345678901234567890123456789012345678";
        let check = observer.scan_for_leaks(unsafe_data).await.unwrap();
        assert!(!check.is_safe);
        assert_eq!(check.findings.len(), 1);
    }
}
