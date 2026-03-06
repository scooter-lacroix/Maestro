//! SecurityPolicy Bridge for Tool Execution
//!
//! This module provides integration between maestro-claw tools and maestro-core's
//! SecurityPolicy for sandboxing and approval flows.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value as JsonValue;

use maestro_core::capabilities::sandbox::{
    validate_command_safe, validate_path_safe, AutonomyLevel, ExecutionRequest, NativeRuntime,
    ResourceLimits, RuntimeAdapter, SandboxManager, SandboxResult, SecurityPolicy,
};

use crate::tools::{Tool, ToolOutput};

/// Error from security policy enforcement
#[derive(Debug, Clone, thiserror::Error)]
pub enum SecurityPolicyError {
    /// Operation requires approval
    #[error("Operation '{operation}' requires approval at {level:?} autonomy level")]
    ApprovalRequired {
        operation: String,
        level: AutonomyLevel,
    },

    /// Path access denied
    #[error("Path access denied: {path} - {reason}")]
    PathAccessDenied { path: String, reason: String },

    /// Command not allowed
    #[error("Command not allowed: {command} - {reason}")]
    CommandNotAllowed { command: String, reason: String },

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {reason}")]
    ResourceLimitExceeded { reason: String },

    /// Sandbox execution failed
    #[error("Sandbox execution failed: {0}")]
    ExecutionFailed(String),

    /// Policy validation failed
    #[error("Policy validation failed: {0}")]
    ValidationFailed(String),
}

/// Bridge between maestro-claw tools and maestro-core SecurityPolicy
///
/// This struct wraps tool execution with security policy enforcement:
/// - Checks autonomy level for approval requirements
/// - Validates file paths against allowed roots
/// - Validates commands against allowlist
/// - Optionally executes commands in a sandbox
#[derive(Clone)]
pub struct SecurityPolicyBridge {
    /// The security policy to enforce
    policy: SecurityPolicy,
    /// Optional sandbox manager for isolated execution
    sandbox_manager: Option<Arc<SandboxManager>>,
    /// Approval callback for HumanApproval level
    approval_callback: Option<Arc<dyn ApprovalCallback + Send + Sync>>,
}

/// Callback trait for approval requests
#[async_trait::async_trait]
pub trait ApprovalCallback: Send + Sync {
    /// Request approval for an operation
    /// Returns true if approved, false if denied
    async fn request_approval(&self, operation: &str, details: &JsonValue) -> bool;
}

impl SecurityPolicyBridge {
    /// Create a new security policy bridge with the given policy
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            sandbox_manager: None,
            approval_callback: None,
        }
    }

    /// Create a bridge with a sandbox manager
    pub fn with_sandbox(policy: SecurityPolicy, sandbox: Arc<SandboxManager>) -> Self {
        Self {
            policy,
            sandbox_manager: Some(sandbox),
            approval_callback: None,
        }
    }

    /// Set the approval callback for HumanApproval level
    pub fn with_approval_callback(
        mut self,
        callback: Arc<dyn ApprovalCallback + Send + Sync>,
    ) -> Self {
        self.approval_callback = Some(callback);
        self
    }

    /// Get the current security policy
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Check if an operation requires approval
    pub fn requires_approval(&self, operation: &str) -> bool {
        self.policy.autonomy_level.requires_approval(operation)
    }

    /// Validate a file path for read access
    pub fn validate_read_path(&self, path: &std::path::Path) -> Result<(), SecurityPolicyError> {
        validate_path_safe(path, &self.policy.allowed_read_paths).map_err(|e| {
            SecurityPolicyError::PathAccessDenied {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        })
    }

    /// Validate a file path for write access
    pub fn validate_write_path(&self, path: &std::path::Path) -> Result<(), SecurityPolicyError> {
        validate_path_safe(path, &self.policy.allowed_write_paths).map_err(|e| {
            SecurityPolicyError::PathAccessDenied {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        })
    }

    /// Validate a command for execution
    pub fn validate_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<(), SecurityPolicyError> {
        validate_command_safe(command, args).map_err(|e| SecurityPolicyError::CommandNotAllowed {
            command: command.to_string(),
            reason: e.to_string(),
        })
    }

    /// Request approval for an operation (if needed)
    pub async fn request_approval(
        &self,
        operation: &str,
        details: &JsonValue,
    ) -> Result<(), SecurityPolicyError> {
        if !self.requires_approval(operation) {
            return Ok(());
        }

        if let Some(callback) = &self.approval_callback {
            let approved = callback.request_approval(operation, details).await;
            if approved {
                Ok(())
            } else {
                Err(SecurityPolicyError::ApprovalRequired {
                    operation: operation.to_string(),
                    level: self.policy.autonomy_level,
                })
            }
        } else {
            // No callback registered - deny by default at HumanApproval level
            Err(SecurityPolicyError::ApprovalRequired {
                operation: operation.to_string(),
                level: self.policy.autonomy_level,
            })
        }
    }

    fn approval_operation(tool_name: &str, arguments: &JsonValue) -> String {
        match tool_name {
            "shell" => "shell_exec".to_string(),
            "file" => match arguments
                .get("operation")
                .and_then(|value| value.as_str())
                .unwrap_or("read")
            {
                "write" => "file_write".to_string(),
                "delete" => "file_delete".to_string(),
                _ => "file_read".to_string(),
            },
            "cron_add" | "cron_remove" => "file_write".to_string(),
            name if name.starts_with("mcp__") => "network_request".to_string(),
            other => other.to_string(),
        }
    }

    /// Execute a command in the sandbox (if configured) or natively
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<String>,
        env: std::collections::HashMap<String, String>,
        cwd: Option<PathBuf>,
        stdin: Option<String>,
    ) -> Result<SandboxResult, SecurityPolicyError> {
        // Validate command first
        self.validate_command(&command, &args)?;

        // Validate working directory
        if let Some(ref wd) = cwd {
            self.validate_read_path(wd)?;
        }

        // Check if approval is required
        let details = serde_json::json!({
            "command": command,
            "args": args,
            "cwd": cwd,
        });
        self.request_approval("shell_exec", &details).await?;

        let request = ExecutionRequest {
            command,
            args,
            env,
            cwd,
            stdin,
            limits: ResourceLimits::default(),
        };

        if let Some(sandbox) = &self.sandbox_manager {
            sandbox
                .execute("native", request)
                .await
                .map_err(|e| SecurityPolicyError::ExecutionFailed(e.to_string()))
        } else {
            // Execute natively without sandbox
            // This is only safe because we've already validated the command
            let runtime = NativeRuntime::new(self.policy.clone());
            <NativeRuntime as RuntimeAdapter>::execute(&runtime, request)
                .await
                .map_err(|e| SecurityPolicyError::ExecutionFailed(e.to_string()))
        }
    }

    /// Wrap a tool with security policy enforcement
    pub fn wrap_tool<T: Tool + 'static>(self, tool: T) -> SecuredTool<T> {
        SecuredTool {
            inner: tool,
            bridge: self,
        }
    }

    /// Wrap a shared trait-object tool with security policy enforcement.
    pub fn wrap_shared_tool(self, tool: Arc<dyn Tool>) -> Arc<dyn Tool> {
        Arc::new(SharedSecuredTool {
            inner: tool,
            bridge: self,
        })
    }
}

/// A tool wrapped with security policy enforcement
pub struct SecuredTool<T: Tool> {
    inner: T,
    bridge: SecurityPolicyBridge,
}

impl<T: Tool> SecuredTool<T> {
    /// Get the inner tool
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get the security bridge
    pub fn bridge(&self) -> &SecurityPolicyBridge {
        &self.bridge
    }
}

#[async_trait::async_trait]
impl<T: Tool + Send + Sync> Tool for SecuredTool<T> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> JsonValue {
        self.inner.parameters_schema()
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        let operation = SecurityPolicyBridge::approval_operation(self.name(), &arguments);
        let approval_details = serde_json::json!({
            "tool_name": self.name(),
            "arguments": arguments,
        });
        match self
            .bridge
            .request_approval(&operation, &approval_details)
            .await
        {
            Ok(()) => {}
            Err(e) => return ToolOutput::error(e.to_string()),
        }

        // Execute the inner tool
        self.inner
            .execute(approval_details["arguments"].clone())
            .await
    }
}

/// Trait-object variant of `SecuredTool` for shared registries.
pub struct SharedSecuredTool {
    inner: Arc<dyn Tool>,
    bridge: SecurityPolicyBridge,
}

#[async_trait::async_trait]
impl Tool for SharedSecuredTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> JsonValue {
        self.inner.parameters_schema()
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        let operation = SecurityPolicyBridge::approval_operation(self.name(), &arguments);
        let approval_details = serde_json::json!({
            "tool_name": self.name(),
            "arguments": arguments,
        });
        match self
            .bridge
            .request_approval(&operation, &approval_details)
            .await
        {
            Ok(()) => {
                self.inner
                    .execute(approval_details["arguments"].clone())
                    .await
            }
            Err(e) => ToolOutput::error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy() -> SecurityPolicy {
        let mut policy = SecurityPolicy::default();
        policy.allowed_read_paths = vec![std::path::PathBuf::from("/tmp")];
        policy.allowed_write_paths = vec![std::path::PathBuf::from("/tmp")];
        policy
    }

    #[test]
    fn test_bridge_creation() {
        let policy = create_test_policy();
        let bridge = SecurityPolicyBridge::new(policy);
        assert!(!bridge.requires_approval("read"));
    }

    #[test]
    fn test_human_approval_level() {
        let mut policy = SecurityPolicy::restricted();
        policy.allowed_read_paths = vec![std::path::PathBuf::from("/tmp")];

        let bridge = SecurityPolicyBridge::new(policy);
        assert!(bridge.requires_approval("read")); // HumanApproval requires approval for everything
    }

    #[test]
    fn test_supervised_level() {
        let mut policy = SecurityPolicy::default();
        policy.autonomy_level = AutonomyLevel::Supervised;
        policy.allowed_read_paths = vec![std::path::PathBuf::from("/tmp")];

        let bridge = SecurityPolicyBridge::new(policy);
        assert!(!bridge.requires_approval("read")); // Safe operation
        assert!(bridge.requires_approval("shell_exec")); // Dangerous operation
    }

    #[test]
    fn test_autonomous_level() {
        let policy = SecurityPolicy::permissive();
        let bridge = SecurityPolicyBridge::new(policy);
        assert!(!bridge.requires_approval("shell_exec")); // No approval needed
    }

    #[test]
    fn test_validate_command_safe() {
        let policy = create_test_policy();
        let bridge = SecurityPolicyBridge::new(policy);

        // Safe command
        assert!(bridge
            .validate_command("echo", &["hello".to_string()])
            .is_ok());

        // Unsafe command (injection attempt)
        assert!(bridge
            .validate_command("echo", &["| rm -rf /".to_string()])
            .is_err());
    }

    #[test]
    fn test_validate_path_safe() {
        // Path within allowed roots - need to test with existing paths
        let temp_dir = std::env::temp_dir();
        let allowed_path = temp_dir.join("maestro_test_safe");
        let _ = std::fs::create_dir_all(&allowed_path);

        let mut policy = SecurityPolicy::default();
        policy.allowed_read_paths = vec![allowed_path.clone()];
        let bridge = SecurityPolicyBridge::new(policy);

        let safe_file = allowed_path.join("test.txt");
        std::fs::write(&safe_file, "test").unwrap();
        assert!(bridge.validate_read_path(&safe_file).is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&allowed_path);
    }

    #[tokio::test]
    async fn test_approval_denied_without_callback() {
        let mut policy = SecurityPolicy::restricted();
        policy.allowed_read_paths = vec![std::path::PathBuf::from("/tmp")];

        let bridge = SecurityPolicyBridge::new(policy);

        let result = bridge
            .request_approval("shell_exec", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    struct MockApprovalCallback {
        approve: bool,
    }

    #[async_trait::async_trait]
    impl ApprovalCallback for MockApprovalCallback {
        async fn request_approval(&self, _operation: &str, _details: &JsonValue) -> bool {
            self.approve
        }
    }

    #[tokio::test]
    async fn test_approval_with_callback() {
        let mut policy = SecurityPolicy::restricted();
        policy.allowed_read_paths = vec![std::path::PathBuf::from("/tmp")];

        let callback = Arc::new(MockApprovalCallback { approve: true });
        let bridge = SecurityPolicyBridge::new(policy).with_approval_callback(callback);

        let result = bridge
            .request_approval("shell_exec", &serde_json::json!({}))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_approval_denied_by_callback() {
        let mut policy = SecurityPolicy::restricted();
        policy.allowed_read_paths = vec![std::path::PathBuf::from("/tmp")];

        let callback = Arc::new(MockApprovalCallback { approve: false });
        let bridge = SecurityPolicyBridge::new(policy).with_approval_callback(callback);

        let result = bridge
            .request_approval("shell_exec", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_approval_operation_mapping() {
        assert_eq!(
            SecurityPolicyBridge::approval_operation("shell", &serde_json::json!({})),
            "shell_exec"
        );
        assert_eq!(
            SecurityPolicyBridge::approval_operation(
                "file",
                &serde_json::json!({"operation": "write"})
            ),
            "file_write"
        );
        assert_eq!(
            SecurityPolicyBridge::approval_operation("mcp__github__issues", &serde_json::json!({})),
            "network_request"
        );
    }
}
