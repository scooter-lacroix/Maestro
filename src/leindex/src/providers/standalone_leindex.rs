use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;

use crate::provider_boundary::{
    AnalysisProvider, AnalysisProviderKind, ProviderDiagnostic, ProviderDoctorReport,
    ProviderMcpConfig, ProviderStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeIndexInstallMethod {
    Cargo,
    InstallScript,
    PyPiBootstrap,
}

impl LeIndexInstallMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::InstallScript => "install-script",
            Self::PyPiBootstrap => "pypi-bootstrap",
        }
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            Self::Cargo => "cargo install --force leindex",
            Self::InstallScript => {
                "curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/LeIndex/master/install.sh -o install-leindex.sh && bash install-leindex.sh"
            }
            Self::PyPiBootstrap => "pip install leindex && leindex --version",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StandaloneLeIndexProvider {
    binary: PathBuf,
}

impl Default for StandaloneLeIndexProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl StandaloneLeIndexProvider {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("leindex"),
        }
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn supported_install_methods() -> [LeIndexInstallMethod; 3] {
        [
            LeIndexInstallMethod::Cargo,
            LeIndexInstallMethod::InstallScript,
            LeIndexInstallMethod::PyPiBootstrap,
        ]
    }

    pub fn detect() -> Result<Option<Self>> {
        let provider = Self::new();
        match provider.capture_output(["--version"])? {
            Some(_) => Ok(Some(provider)),
            None => Ok(None),
        }
    }

    pub fn direct_stdio_config(&self, include_stdio_type: bool) -> serde_json::Value {
        let mut payload = serde_json::json!({
            "command": self.binary.to_string_lossy().to_string(),
            "args": ["mcp"],
        });
        if include_stdio_type {
            payload["type"] = serde_json::Value::String("stdio".to_string());
        }
        payload
    }

    pub fn capability_snapshot(&self) -> Result<BTreeMap<String, String>> {
        let mut snapshot = BTreeMap::new();
        if let Some(version) = self.capture_output(["--version"])? {
            snapshot.insert(
                "version".to_string(),
                version.lines().next().unwrap_or("").trim().to_string(),
            );
        }
        if let Some(help) = self.capture_output(["--help"])? {
            let command_count = [
                "index",
                "search",
                "analyze",
                "phase",
                "diagnostics",
                "serve",
                "mcp",
                "dashboard",
            ]
            .into_iter()
            .filter(|command| help.contains(command))
            .count();
            snapshot.insert("command_count".to_string(), command_count.to_string());
        }
        snapshot.insert("delivery".to_string(), "direct-mcp".to_string());
        Ok(snapshot)
    }

    pub fn health_report_sync(&self, project_root: &Path) -> Result<ProviderDoctorReport> {
        let mut diagnostics = Vec::new();
        let mut warnings = Vec::new();
        let mut recommended_actions = Vec::new();
        let mut status = ProviderStatus::Healthy;

        match self.capture_output(["--version"])? {
            Some(version) => diagnostics.push(self.diagnostic(
                ProviderStatus::Healthy,
                format!(
                    "Version detected: {}",
                    version.lines().next().unwrap_or(version.as_str())
                ),
                ["version"],
            )),
            None => {
                status = ProviderStatus::Missing;
                diagnostics.push(self.diagnostic(
                    ProviderStatus::Missing,
                    "LeIndex binary not found on PATH",
                    ["version"],
                ));
                recommended_actions.extend(
                    Self::supported_install_methods()
                        .into_iter()
                        .map(|method| method.install_hint().to_string()),
                );
            }
        }

        if status == ProviderStatus::Healthy {
            match self.capture_output(["--help"])? {
                Some(output)
                    if ["index", "search", "analyze", "phase", "mcp"]
                        .into_iter()
                        .all(|command| output.contains(command)) =>
                {
                    diagnostics.push(self.diagnostic(
                        ProviderStatus::Healthy,
                        "Command surface includes index/search/analyze/phase/mcp",
                        ["commands", "index", "search", "analyze", "phase", "mcp"],
                    ))
                }
                Some(_) => {
                    status = ProviderStatus::Degraded;
                    diagnostics.push(self.diagnostic(
                        ProviderStatus::Degraded,
                        "LeIndex responded, but expected managed-session commands were not visible",
                        ["commands"],
                    ));
                }
                None => {
                    status = ProviderStatus::Degraded;
                    diagnostics.push(self.diagnostic(
                        ProviderStatus::Degraded,
                        "Failed to read LeIndex command surface",
                        ["commands"],
                    ));
                }
            }

            match self.capture_output(["analyze", "--help"])? {
                Some(_) => diagnostics.push(self.diagnostic(
                    ProviderStatus::Healthy,
                    format!(
                        "Analyze entrypoint is callable for project root {}",
                        project_root.display()
                    ),
                    ["analyze", "project-root"],
                )),
                None => {
                    status = ProviderStatus::Degraded;
                    diagnostics.push(self.diagnostic(
                        ProviderStatus::Degraded,
                        "Analyze entrypoint is not callable",
                        ["analyze", "project-root"],
                    ));
                }
            }

            match self.capture_output(["mcp", "--help"])? {
                Some(_) => diagnostics.push(self.diagnostic(
                    ProviderStatus::Healthy,
                    "MCP entrypoint is callable",
                    ["mcp"],
                )),
                None => {
                    status = ProviderStatus::Degraded;
                    warnings.push("`leindex mcp --help` did not succeed".to_string());
                }
            }
        }

        Ok(ProviderDoctorReport {
            subject: "standalone-leindex".to_string(),
            status,
            diagnostics,
            warnings,
            recommended_actions,
        })
    }

    pub fn diagnostic_snapshot(&self, project_root: &Path) -> Result<ProviderDiagnostic> {
        let report = self.health_report_sync(project_root)?;
        Ok(report.diagnostics.into_iter().next().unwrap_or_else(|| {
            self.diagnostic(ProviderStatus::Missing, "No diagnostics produced", ["mcp"])
        }))
    }

    fn capture_output<I, S>(&self, args: I) -> Result<Option<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        match Command::new(&self.binary).args(args).output() {
            Ok(output) if output.status.success() => Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )),
            Ok(_) => Ok(None),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn diagnostic(
        &self,
        status: ProviderStatus,
        detail: impl Into<String>,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> ProviderDiagnostic {
        ProviderDiagnostic {
            provider_name: "leindex".to_string(),
            provider_kind: "analysis".to_string(),
            status,
            executable: Some(self.binary.to_string_lossy().to_string()),
            version: None,
            source: Some("standalone".to_string()),
            detail: detail.into(),
            capabilities: capabilities.into_iter().map(Into::into).collect(),
            checked_at: Utc::now(),
        }
    }
}

#[async_trait]
impl AnalysisProvider for StandaloneLeIndexProvider {
    fn kind(&self) -> AnalysisProviderKind {
        AnalysisProviderKind::StandaloneLeindex
    }

    fn name(&self) -> &str {
        "leindex"
    }

    fn executable_hint(&self) -> &str {
        self.binary.to_str().unwrap_or("leindex")
    }

    async fn validate_health(&self, project_root: &Path) -> Result<ProviderDoctorReport> {
        self.health_report_sync(project_root)
    }

    async fn build_mcp_config(
        &self,
        _project_root: &Path,
        _session_id: &str,
    ) -> Result<ProviderMcpConfig> {
        Ok(ProviderMcpConfig {
            direct_servers: BTreeMap::from([(
                "leindex".to_string(),
                self.direct_stdio_config(true),
            )]),
            pooled_servers: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_stdio_config_points_to_standalone_binary() {
        let provider = StandaloneLeIndexProvider::new();
        let config = provider.direct_stdio_config(true);
        assert_eq!(config["command"], "leindex");
        assert_eq!(config["args"], serde_json::json!(["mcp"]));
        assert_eq!(config["type"], "stdio");
    }

    #[test]
    fn supported_install_methods_cover_required_paths() {
        let methods = StandaloneLeIndexProvider::supported_install_methods();
        assert_eq!(methods.len(), 3);
        assert!(methods
            .iter()
            .any(|method| *method == LeIndexInstallMethod::Cargo));
        assert!(methods
            .iter()
            .any(|method| *method == LeIndexInstallMethod::InstallScript));
        assert!(methods
            .iter()
            .any(|method| *method == LeIndexInstallMethod::PyPiBootstrap));
    }
}
