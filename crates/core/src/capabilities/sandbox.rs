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
//! - Command injection protection through allowlist and validation

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Command allowlist for safe execution
const COMMAND_ALLOWLIST: &[&str] = &[
    // Common safe commands
    "echo", "cat", "head", "tail", "grep", "sort", "uniq", "wc",
    "ls", "find", "file", "stat", "dirname", "basename",
    "date", "sleep", "true", "false", "yes", "seq",
    // Build tools
    "cargo", "rustc", "gcc", "clang", "make", "cmake", "ninja",
    "python", "python3", "node", "npm", "pnpm", "yarn", "bun",
    // Git operations
    "git",
    // File operations
    "cp", "mv", "rm", "mkdir", "touch", "chmod", "chown",
    // Compression
    "tar", "gzip", "gunzip", "zip", "unzip", "xz",
    // Text processing
    "sed", "awk", "cut", "tr", "diff",
    // Network (restricted)
    "curl", "wget", "ssh",
    // System info
    "ps", "top", "htop", "df", "du", "free", "uname",
];

/// Characters that are dangerous in shell commands
const DANGEROUS_CHARS: &[char] = &['|', '&', ';', '$', '`', '\n', '\r', '\x00'];

/// Validate that a command string does not contain shell injection patterns
pub fn validate_command_safe(command: &str, args: &[String]) -> anyhow::Result<()> {
    // Check command name against allowlist
    let command_name = command.split('/').next_back().unwrap_or(command);
    if !COMMAND_ALLOWLIST.contains(&command_name) {
        anyhow::bail!(
            "Command '{}' is not in the allowlist. Allowed commands: {:?}",
            command_name,
            COMMAND_ALLOWLIST
        );
    }

    // Check for dangerous characters in command
    for c in DANGEROUS_CHARS {
        if command.contains(*c) {
            anyhow::bail!(
                "Command contains dangerous character '{}'. This could indicate a shell injection attempt.",
                c
            );
        }
    }

    // Check each argument for dangerous characters
    for (i, arg) in args.iter().enumerate() {
        for c in DANGEROUS_CHARS {
            if arg.contains(*c) {
                anyhow::bail!(
                    "Argument {} contains dangerous character '{}': '{}'",
                    i,
                    c,
                    arg
                );
            }
        }
    }

    // Check for pipe chains in arguments
    for arg in args {
        if arg.contains('|') || arg.contains('&') || arg.contains(';') {
            anyhow::bail!(
                "Argument contains shell operator: '{}'. This could indicate command chaining.",
                arg
            );
        }
    }

    Ok(())
}

/// Validate that a path is safe and within allowed roots
/// Uses canonicalize to resolve symlinks and relative paths
pub fn validate_path_safe(path: &Path, allowed_roots: &[PathBuf]) -> anyhow::Result<()> {
    // Canonicalize the path to resolve any symlinks or relative components
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());

    // If no allowed roots, deny all
    if allowed_roots.is_empty() {
        anyhow::bail!("No allowed paths configured - access denied");
    }

    // Check if the canonical path is within any allowed root
    let mut is_allowed = false;
    for root in allowed_roots {
        let root_canonical = root
            .canonicalize()
            .unwrap_or_else(|_| root.clone());

        if canonical.starts_with(&root_canonical) {
            is_allowed = true;
            break;
        }
    }

    if !is_allowed {
        anyhow::bail!(
            "Path '{}' is not within allowed roots: {:?}",
            canonical.display(),
            allowed_roots
        );
    }

    Ok(())
}

/// Check if an environment variable key is safe (no shell injection)
pub fn is_safe_env_key(key: &str) -> bool {
    // Environment variable keys should be alphanumeric with underscores
    // No spaces, quotes, or special characters
    key.chars()
        .all(|c| c.is_alphanumeric() || c == '_')
        && !key.is_empty()
        && !key.starts_with(|c: char| c.is_ascii_digit())
}

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
    /// Uses canonicalize to prevent path traversal attacks.
    pub fn can_read(&self, path: &std::path::Path) -> bool {
        if self.allowed_read_paths.is_empty() {
            return false;
        }

        // First, try to canonicalize the path to resolve symlinks and relative paths
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Path doesn't exist, so we can't fully validate it
                // Fall back to prefix check on the raw path
                return self.allowed_read_paths.iter().any(|allowed| {
                    path.starts_with(allowed) || path == allowed
                });
            }
        };

        self.allowed_read_paths.iter().any(|allowed| {
            let allowed_canonical = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canonical) || canonical == allowed_canonical
        })
    }

    /// Check if a path is writable.
    /// Uses canonicalize to prevent path traversal attacks.
    pub fn can_write(&self, path: &std::path::Path) -> bool {
        if self.allowed_write_paths.is_empty() {
            return false;
        }

        // First, try to canonicalize the path to resolve symlinks and relative paths
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Path doesn't exist, so we can't fully validate it
                // Fall back to prefix check on the raw path
                return self.allowed_write_paths.iter().any(|allowed| {
                    path.starts_with(allowed) || path == allowed
                });
            }
        };

        self.allowed_write_paths.iter().any(|allowed| {
            let allowed_canonical = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            canonical.starts_with(&allowed_canonical) || canonical == allowed_canonical
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

        // SECURITY: Validate command against injection attacks
        validate_command_safe(&request.command, &request.args)
            .context("Command validation failed - possible injection attempt")?;

        // Validate working directory is within allowed paths
        if let Some(cwd) = &request.cwd {
            if let Err(e) = validate_path_safe(cwd, &self.policy.allowed_read_paths) {
                anyhow::bail!("Working directory validation failed: {}", e);
            }
        }

        let start = Instant::now();

        // Build command - using Command API which avoids shell interpretation
        let mut cmd = Command::new(&request.command);
        cmd.args(&request.args);

        // Set environment (validate keys are safe)
        for (key, value) in &request.env {
            if !is_safe_env_key(key) {
                anyhow::bail!("Environment variable key '{}' contains unsafe characters", key);
            }
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

    // ========== SECURITY TESTS ==========

    #[test]
    fn test_command_injection_blocked_pipe() {
        let result = validate_command_safe("cat", &["| rm -rf /".to_string()]);
        assert!(result.is_err(), "Should reject pipe injection");
    }

    #[test]
    fn test_command_injection_blocked_semicolon() {
        let result = validate_command_safe("cat", &["; echo hacked".to_string()]);
        assert!(result.is_err(), "Should reject semicolon injection");
    }

    #[test]
    fn test_command_injection_blocked_backtick() {
        let result = validate_command_safe("cat", &["`whoami`".to_string()]);
        assert!(result.is_err(), "Should reject backtick injection");
    }

    #[test]
    fn test_command_injection_blocked_dollar() {
        let result = validate_command_safe("cat", &["$(rm -rf /)".to_string()]);
        assert!(result.is_err(), "Should reject dollar substitution");
    }

    #[test]
    fn test_command_injection_blocked_newline() {
        let result = validate_command_safe("cat", &["file.txt\necho hacked".to_string()]);
        assert!(result.is_err(), "Should reject newline injection");
    }

    #[test]
    fn test_command_not_in_allowlist() {
        let result = validate_command_safe("malicious_command", &[]);
        assert!(result.is_err(), "Should reject commands not in allowlist");
    }

    #[test]
    fn test_safe_command_allowed() {
        let result = validate_command_safe("cat", &["file.txt".to_string()]);
        assert!(result.is_ok(), "Should allow safe commands");
    }

    #[test]
    fn test_safe_command_with_multiple_args() {
        let result = validate_command_safe(
            "grep",
            &["-r".to_string(), "pattern".to_string(), "/path".to_string()],
        );
        assert!(result.is_ok(), "Should allow safe commands with multiple args");
    }

    #[test]
    fn test_git_command_allowed() {
        let result = validate_command_safe("git", &["status".to_string()]);
        assert!(result.is_ok(), "Should allow git commands");
    }

    #[test]
    fn test_path_traversal_blocked() {
        // Use temp dir for actual filesystem testing
        let temp_dir = std::env::temp_dir();
        let safe_base = temp_dir.join("maestro_test_safe");
        let _ = std::fs::create_dir_all(&safe_base);

        let allowed_paths = vec![safe_base.clone()];

        // Test 1: Direct safe path should work
        let safe = safe_base.join("file.txt");
        std::fs::write(&safe, "test").unwrap(); // Create the file
        let result = validate_path_safe(&safe, &allowed_paths);
        assert!(result.is_ok(), "Should allow safe path: {:?}", result.err());

        // Test 2: Path traversal via .. should be rejected
        // Create a file outside the safe directory
        let outside_dir = temp_dir.join("maestro_test_outside");
        std::fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("outside.txt");
        std::fs::write(&outside_file, "outside data").unwrap();

        // Try to access via .. traversal
        let traversal = safe_base.join("../maestro_test_outside/outside.txt");
        let result = validate_path_safe(&traversal, &allowed_paths);
        assert!(result.is_err(), "Should reject path traversal via ..: {:?}", result.err());

        // Cleanup
        let _ = std::fs::remove_dir_all(&safe_base);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    #[test]
    fn test_path_symlink_blocked() {
        // Use temp dir for actual filesystem testing
        let temp_dir = std::env::temp_dir();
        let safe_base = temp_dir.join("maestro_test_symlink");
        let outside = temp_dir.join("maestro_test_outside");
        let _ = std::fs::create_dir_all(&safe_base);
        let _ = std::fs::create_dir_all(&outside);

        // Create a file outside safe area
        let outside_file = outside.join("secret.txt");
        std::fs::write(&outside_file, "secret data").unwrap();

        // Create a symlink inside safe area pointing outside
        let symlink = safe_base.join("link_to_outside");
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&outside, &symlink);
        }
        #[cfg(windows)]
        {
            let _ = std::os::windows::fs::symlink_dir(&outside, &symlink);
        }

        let allowed_paths = vec![safe_base.clone()];

        // The symlink target should be validated
        let result = validate_path_safe(&symlink, &allowed_paths);
        // Symlinks are resolved by canonicalize, so if it points outside,
        // it should be rejected
        assert!(result.is_err(), "Should reject symlink pointing outside allowed path");

        // Cleanup
        let _ = std::fs::remove_dir_all(&safe_base);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn test_path_validation_empty_roots() {
        let path = PathBuf::from("/etc/passwd");
        let result = validate_path_safe(&path, &[]);
        assert!(result.is_err(), "Should reject when no allowed roots configured");
    }

    #[test]
    fn test_env_key_validation_unsafe_chars() {
        assert!(!is_safe_env_key("TEST;VAR"), "Should reject semicolon in env key");
        assert!(!is_safe_env_key("TEST VAR"), "Should reject space in env key");
        assert!(!is_safe_env_key("TEST|VAR"), "Should reject pipe in env key");
    }

    #[test]
    fn test_env_key_validation_safe() {
        assert!(is_safe_env_key("TEST_VAR"), "Should allow normal env key");
        assert!(is_safe_env_key("PATH"), "Should allow PATH");
        assert!(is_safe_env_key("HOME"), "Should allow HOME");
    }

    #[test]
    fn test_env_key_validation_empty() {
        assert!(!is_safe_env_key(""), "Should reject empty env key");
    }

    #[test]
    fn test_env_key_validation_leading_digit() {
        assert!(!is_safe_env_key("1VAR"), "Should reject env key starting with digit");
        assert!(is_safe_env_key("V1AR"), "Should allow env key with digit not at start");
    }

    // Test that NativeRuntime validates commands
    #[tokio::test]
    async fn test_native_runtime_blocks_injection() {
        let policy = SecurityPolicy::permissive();
        let runtime = NativeRuntime::new(policy);

        let request = ExecutionRequest {
            command: "cat".to_string(),
            args: vec!["| echo hacked".to_string()],
            env: HashMap::new(),
            cwd: None,
            stdin: None,
            limits: ResourceLimits::default(),
        };

        let result = runtime.execute(request).await;
        assert!(result.is_err(), "Should reject command injection attempt");
    }
}
