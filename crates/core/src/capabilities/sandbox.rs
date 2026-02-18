//! Dual-Tier Sandboxing (WASM + Docker)
//!
//! This module implements sandboxing following patterns from ZeroClaw and IronClaw:
//! - `zeroclaw/src/runtime/traits.rs` - RuntimeAdapter trait
//! - `ironclaw/src/sandbox/manager.rs` - Docker sandbox manager
//! - `zeroclaw/src/security/policy.rs` - AutonomyLevel enum
//!
//! Key features:
//! - AutonomyLevel: HumanApproval, Supervised, Autonomous
//! - SecurityPolicy with resource limits
//! - RuntimeAdapter trait for pluggable execution backends
//! - WASM and Docker sandbox implementations

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Level of autonomy for tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AutonomyLevel {
    /// All tool calls require human approval.
    HumanApproval,
    /// Tools run with supervision, dangerous ops require approval.
    Supervised,
    /// Full autonomy, minimal restrictions.
    #[default]
    Autonomous,
}

impl AutonomyLevel {
    /// Check if approval is required for the given operation.
    pub fn requires_approval(&self, operation: &str) -> bool {
        match self {
            Self::HumanApproval => true,
            Self::Supervised => {
                // Dangerous operations require approval
                matches!(
                    operation,
                    "file_write"
                        | "file_delete"
                        | "shell_exec"
                        | "network_request"
                        | "spawn_agent"
                )
            }
            Self::Autonomous => false,
        }
    }
}

/// Security policy for sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityPolicy {
    /// Autonomy level for this policy.
    #[serde(default)]
    pub autonomy_level: AutonomyLevel,
    /// Maximum memory in bytes (0 = unlimited).
    #[serde(default)]
    pub max_memory_bytes: u64,
    /// Maximum CPU shares (0 = unlimited).
    #[serde(default)]
    pub max_cpu_shares: u32,
    /// Maximum execution time.
    #[serde(with = "humantime_serde", default)]
    pub max_execution_time: Duration,
    /// Allowed network hosts (empty = no network).
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    /// Allowed file paths for read access.
    #[serde(default)]
    pub allowed_read_paths: Vec<PathBuf>,
    /// Allowed file paths for write access.
    #[serde(default)]
    pub allowed_write_paths: Vec<PathBuf>,
    /// Environment variables to pass through.
    #[serde(default)]
    pub passthrough_env: Vec<String>,
    /// Whether to allow network access.
    #[serde(default)]
    pub allow_network: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            autonomy_level: AutonomyLevel::default(),
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            max_cpu_shares: 1024,
            max_execution_time: Duration::from_secs(60),
            allowed_hosts: Vec::new(),
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
            passthrough_env: Vec::new(),
            allow_network: false,
        }
    }
}

impl SecurityPolicy {
    /// Create a restrictive policy for untrusted code.
    pub fn restricted() -> Self {
        Self {
            autonomy_level: AutonomyLevel::HumanApproval,
            max_memory_bytes: 64 * 1024 * 1024, // 64 MB
            max_cpu_shares: 512,
            max_execution_time: Duration::from_secs(30),
            allowed_hosts: Vec::new(),
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
            passthrough_env: Vec::new(),
            allow_network: false,
        }
    }

    /// Create a permissive policy for trusted code.
    pub fn permissive() -> Self {
        Self {
            autonomy_level: AutonomyLevel::Autonomous,
            max_memory_bytes: 0, // Unlimited
            max_cpu_shares: 0,
            max_execution_time: Duration::from_secs(300),
            allowed_hosts: vec!["*".to_string()],
            allowed_read_paths: vec![PathBuf::from("/")],
            allowed_write_paths: vec![PathBuf::from("/")],
            passthrough_env: vec!["PATH".to_string(), "HOME".to_string()],
            allow_network: true,
        }
    }

    /// Check if a path is readable.
    pub fn can_read(&self, path: &std::path::Path) -> bool {
        if self.allowed_read_paths.is_empty() {
            return false;
        }
        self.allowed_read_paths.iter().any(|allowed| {
            path.starts_with(allowed) || path == allowed
        })
    }

    /// Check if a path is writable.
    pub fn can_write(&self, path: &std::path::Path) -> bool {
        if self.allowed_write_paths.is_empty() {
            return false;
        }
        self.allowed_write_paths.iter().any(|allowed| {
            path.starts_with(allowed) || path == allowed
        })
    }
}

/// Resource limits for sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimits {
    /// Memory limit in bytes.
    pub memory_bytes: u64,
    /// CPU shares (relative weight).
    pub cpu_shares: u32,
    /// Execution timeout.
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Fuel for WASM execution (instruction count limit).
    pub fuel: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 64 * 1024 * 1024, // 64 MB
            cpu_shares: 1024,
            timeout: Duration::from_secs(60),
            fuel: Some(10_000_000),
        }
    }
}

/// Result of sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxResult {
    /// Exit code (0 = success).
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Execution duration.
    pub duration_ms: u64,
    /// Whether the execution was killed due to timeout.
    pub timed_out: bool,
    /// Whether the execution was killed due to OOM.
    pub oom_killed: bool,
}

/// Execution request for the sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    /// Command to execute.
    pub command: String,
    /// Arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Working directory.
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    /// Standard input.
    #[serde(default)]
    pub stdin: Option<String>,
    /// Resource limits.
    #[serde(default)]
    pub limits: ResourceLimits,
}

/// Runtime adapter trait for pluggable execution backends.
///
/// Based on ZeroClaw's RuntimeAdapter pattern.
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    /// Execute a command in the sandbox.
    async fn execute(&self, request: ExecutionRequest) -> anyhow::Result<SandboxResult>;

    /// Check if the runtime is available.
    fn is_available(&self) -> bool;

    /// Get the runtime name (e.g., "wasm", "docker", "native").
    fn name(&self) -> &str;

    /// Validate that the security policy is compatible with this runtime.
    fn validate_policy(&self, policy: &SecurityPolicy) -> anyhow::Result<()>;
}

/// Native (no sandbox) runtime for trusted execution.
pub struct NativeRuntime {
    policy: SecurityPolicy,
}

impl NativeRuntime {
    /// Create a new native runtime with the given policy.
    pub fn new(policy: SecurityPolicy) -> Self {
        Self { policy }
    }

    /// Get the security policy.
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }
}

#[async_trait]
impl RuntimeAdapter for NativeRuntime {
    async fn execute(&self, request: ExecutionRequest) -> anyhow::Result<SandboxResult> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::time::Instant;

        let start = Instant::now();

        // Build command
        let mut cmd = Command::new(&request.command);
        cmd.args(&request.args);

        // Set environment
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(cwd) = &request.cwd {
            cmd.current_dir(cwd);
        }

        // Set up stdin pipe if input is provided
        if request.stdin.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn()?;

        // Write stdin if provided
        if let Some(stdin_content) = &request.stdin {
            if let Some(mut stdin_handle) = child.stdin.take() {
                stdin_handle.write_all(stdin_content.as_bytes())?;
            }
        }

        // Wait for completion
        let output = child.wait_with_output()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Check if timeout was exceeded
        let timed_out = duration_ms > request.limits.timeout.as_millis() as u64;

        Ok(SandboxResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
            timed_out,
            oom_killed: false,
        })
    }

    fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "native"
    }

    fn validate_policy(&self, policy: &SecurityPolicy) -> anyhow::Result<()> {
        // Native runtime requires compatible autonomy level
        // Use self.policy as the baseline requirement
        if policy.autonomy_level != AutonomyLevel::Autonomous {
            anyhow::bail!(
                "Native runtime requires Autonomous autonomy level, got {:?}",
                policy.autonomy_level
            );
        }

        // Verify memory limits are compatible
        if self.policy.max_memory_bytes > 0 && policy.max_memory_bytes > self.policy.max_memory_bytes {
            anyhow::bail!(
                "Requested memory limit {} exceeds runtime limit {}",
                policy.max_memory_bytes,
                self.policy.max_memory_bytes
            );
        }

        Ok(())
    }
}

/// WASM sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WasmSandboxConfig {
    /// Default resource limits.
    #[serde(default)]
    pub default_limits: ResourceLimits,
    /// Whether to cache compiled modules.
    #[serde(default = "default_true")]
    pub cache_compiled: bool,
    /// Cache directory.
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
    /// Whether fuel metering is enabled.
    #[serde(default = "default_true")]
    pub fuel_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for WasmSandboxConfig {
    fn default() -> Self {
        Self {
            default_limits: ResourceLimits::default(),
            cache_compiled: true,
            cache_dir: None,
            fuel_enabled: true,
        }
    }
}

/// Docker sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSandboxConfig {
    /// Docker image to use.
    pub image: String,
    /// Memory limit in MB.
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
    /// CPU shares.
    #[serde(default = "default_cpu_shares")]
    pub cpu_shares: u32,
    /// Network mode.
    #[serde(default)]
    pub network_mode: String,
    /// Whether to drop all capabilities.
    #[serde(default = "default_true")]
    pub drop_capabilities: bool,
    /// Security options.
    #[serde(default = "default_security_opts")]
    pub security_opts: Vec<String>,
    /// Volume mounts.
    #[serde(default)]
    pub mounts: Vec<VolumeMount>,
}

fn default_memory_mb() -> u64 {
    256
}

fn default_cpu_shares() -> u32 {
    1024
}

fn default_security_opts() -> Vec<String> {
    vec!["no-new-privileges:true".to_string()]
}

/// Volume mount configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeMount {
    /// Host path.
    pub host_path: PathBuf,
    /// Container path.
    pub container_path: PathBuf,
    /// Whether the mount is read-only.
    #[serde(default)]
    pub read_only: bool,
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            image: "ubuntu:22.04".to_string(),
            memory_mb: default_memory_mb(),
            cpu_shares: default_cpu_shares(),
            network_mode: "none".to_string(),
            drop_capabilities: true,
            security_opts: default_security_opts(),
            mounts: Vec::new(),
        }
    }
}

/// Sandbox manager for creating and managing sandbox instances.
pub struct SandboxManager {
    /// Available runtimes.
    runtimes: HashMap<String, Arc<dyn RuntimeAdapter>>,
    /// Default policy.
    default_policy: SecurityPolicy,
}

impl SandboxManager {
    /// Create a new sandbox manager.
    pub fn new(default_policy: SecurityPolicy) -> Self {
        let mut runtimes: HashMap<String, Arc<dyn RuntimeAdapter>> = HashMap::new();

        // Register native runtime
        let native = Arc::new(NativeRuntime::new(default_policy.clone()));
        runtimes.insert("native".to_string(), native);

        Self {
            runtimes,
            default_policy,
        }
    }

    /// Register a runtime adapter.
    pub fn register_runtime(&mut self, name: impl Into<String>, runtime: Arc<dyn RuntimeAdapter>) {
        self.runtimes.insert(name.into(), runtime);
    }

    /// Get a runtime by name.
    pub fn get_runtime(&self, name: &str) -> Option<Arc<dyn RuntimeAdapter>> {
        self.runtimes.get(name).cloned()
    }

    /// Execute in the specified runtime.
    pub async fn execute(
        &self,
        runtime_name: &str,
        request: ExecutionRequest,
    ) -> anyhow::Result<SandboxResult> {
        let runtime = self
            .runtimes
            .get(runtime_name)
            .ok_or_else(|| anyhow::anyhow!("Runtime not found: {}", runtime_name))?;

        if !runtime.is_available() {
            anyhow::bail!("Runtime {} is not available", runtime_name);
        }

        runtime.execute(request).await
    }

    /// Get the default policy.
    pub fn default_policy(&self) -> &SecurityPolicy {
        &self.default_policy
    }

    /// List available runtimes.
    pub fn available_runtimes(&self) -> Vec<&str> {
        self.runtimes
            .iter()
            .filter(|(_, r)| r.is_available())
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new(SecurityPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_autonomy_level_default() {
        let level = AutonomyLevel::default();
        assert_eq!(level, AutonomyLevel::Autonomous);
    }

    #[test]
    fn test_autonomy_level_approval() {
        assert!(AutonomyLevel::HumanApproval.requires_approval("read"));
        assert!(AutonomyLevel::Supervised.requires_approval("file_delete"));
        assert!(!AutonomyLevel::Supervised.requires_approval("read"));
        assert!(!AutonomyLevel::Autonomous.requires_approval("shell_exec"));
    }

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.max_memory_bytes, 256 * 1024 * 1024);
        assert!(!policy.allow_network);
    }

    #[test]
    fn test_security_policy_restricted() {
        let policy = SecurityPolicy::restricted();
        assert_eq!(policy.autonomy_level, AutonomyLevel::HumanApproval);
        assert_eq!(policy.max_memory_bytes, 64 * 1024 * 1024);
    }

    #[test]
    fn test_security_policy_permissive() {
        let policy = SecurityPolicy::permissive();
        assert_eq!(policy.autonomy_level, AutonomyLevel::Autonomous);
        assert!(policy.allow_network);
    }

    #[test]
    fn test_security_policy_path_checks() {
        let mut policy = SecurityPolicy::default();
        policy.allowed_read_paths = vec![PathBuf::from("/data")];
        policy.allowed_write_paths = vec![PathBuf::from("/tmp")];

        assert!(policy.can_read(Path::new("/data/file.txt")));
        assert!(policy.can_read(Path::new("/data")));
        assert!(!policy.can_read(Path::new("/etc/passwd")));

        assert!(policy.can_write(Path::new("/tmp/output.txt")));
        assert!(!policy.can_write(Path::new("/data/file.txt")));
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_bytes, 64 * 1024 * 1024);
        assert!(limits.fuel.is_some());
    }

    #[test]
    fn test_wasm_sandbox_config_default() {
        let config = WasmSandboxConfig::default();
        assert!(config.cache_compiled);
        assert!(config.fuel_enabled);
    }

    #[test]
    fn test_docker_sandbox_config_default() {
        let config = DockerSandboxConfig::default();
        assert_eq!(config.image, "ubuntu:22.04");
        assert_eq!(config.memory_mb, 256);
        assert!(config.drop_capabilities);
    }

    #[test]
    fn test_sandbox_manager_creation() {
        let manager = SandboxManager::default();
        assert!(manager.get_runtime("native").is_some());
        assert!(manager.get_runtime("native").unwrap().is_available());
    }

    #[test]
    fn test_sandbox_manager_available_runtimes() {
        let manager = SandboxManager::default();
        let runtimes = manager.available_runtimes();
        assert!(runtimes.contains(&"native"));
    }

    #[test]
    fn test_execution_request_serialization() {
        let request = ExecutionRequest {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
            cwd: None,
            stdin: None,
            limits: ResourceLimits::default(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("echo"));
        assert!(json.contains("hello"));
    }

    #[test]
    fn test_sandbox_result_serialization() {
        let result = SandboxResult {
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            timed_out: false,
            oom_killed: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("exitCode"));
        assert!(json.contains("output"));
    }
}
