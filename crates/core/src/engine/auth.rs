//! Auth Interrupt/Resume Flow
//!
//! This module implements IronClaw-style auth interruption and resume:
//! - Auth-required tool detection
//! - Thread state transition to AwaitingAuth
//! - Token submission and validation
//! - Execution resume after successful auth
//!
//! Based on IronClaw patterns from `analysis_foundation_20260217.md`:
//! - `src/agent/agent_loop.rs:process_auth_token`
//! - `src/agent/agent_loop.rs:process_approval` (auth-detection branch)

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Type of authentication token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthTokenType {
    /// Bearer token (Authorization header)
    Bearer,
    /// API key (X-API-Key header or similar)
    ApiKey,
    /// OAuth token (full OAuth flow)
    OAuth,
}

impl AuthTokenType {
    /// Returns the string identifier for this token type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bearer => "bearer",
            Self::ApiKey => "api_key",
            Self::OAuth => "oauth",
        }
    }
}

/// An authentication token.
///
/// Tokens are stored securely and redacted in debug output.
#[derive(Clone)]
pub struct AuthToken {
    value: String,
    token_type: AuthTokenType,
}

impl AuthToken {
    /// Create a new auth token.
    pub fn new(value: impl Into<String>, token_type: AuthTokenType) -> Self {
        Self {
            value: value.into(),
            token_type,
        }
    }

    /// Get the token value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the token type.
    pub fn token_type(&self) -> AuthTokenType {
        self.token_type
    }
}

// Redact token value in debug output for security
impl std::fmt::Debug for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthToken")
            .field("value", &"[REDACTED]")
            .field("token_type", &self.token_type)
            .finish()
    }
}

/// An authentication request from a tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRequest {
    request_id: String,
    tool_name: String,
    message: String,
    metadata: HashMap<String, String>,
}

impl AuthRequest {
    /// Create a new auth request with a unique ID.
    pub fn new(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.into(),
            message: message.into(),
            metadata: HashMap::new(),
        }
    }

    /// Get the unique request ID.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get the tool name that requires auth.
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Get the human-readable auth message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Set metadata for this auth request.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata from this auth request.
    pub fn metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Result of a token submission.
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// Authentication successful, returns the validated token.
    Success(AuthToken),
    /// Token validation failed.
    InvalidToken(String),
    /// No pending auth request.
    NoPendingRequest,
    /// Auth request was cancelled.
    Cancelled,
}

/// Type alias for token validator function.
pub type TokenValidator = Box<dyn Fn(&AuthToken) -> Result<(), String> + Send + Sync>;

/// Manager for authentication interrupts and resume flow.
///
/// This manager handles:
/// - Tracking pending auth requests
/// - Validating submitted tokens
/// - Coordinating auth state with the session state machine
///
/// Thread-safe: uses Arc<RwLock<>> for interior mutability.
pub struct AuthManager {
    pending_request: Arc<RwLock<Option<AuthRequest>>>,
    validator: Arc<RwLock<Option<TokenValidator>>>,
}

impl AuthManager {
    /// Create a new auth manager.
    pub fn new() -> Self {
        Self {
            pending_request: Arc::new(RwLock::new(None)),
            validator: Arc::new(RwLock::new(None)),
        }
    }

    /// Set a custom token validator.
    ///
    /// The validator should return `Ok(())` for valid tokens,
    /// or `Err(reason)` for invalid tokens.
    pub fn set_token_validator<F>(&mut self, validator: F)
    where
        F: Fn(&AuthToken) -> Result<(), String> + Send + Sync + 'static,
    {
        let mut v = self.validator.write().unwrap();
        *v = Some(Box::new(validator));
    }

    /// Request authentication for a tool.
    ///
    /// This should be called when a tool requires authentication.
    /// Returns the auth request for the caller to present to the user.
    pub fn request_auth(
        &mut self,
        tool_name: impl Into<String>,
        message: impl Into<String>,
    ) -> AuthRequest {
        let request = AuthRequest::new(tool_name, message);
        let mut pending = self.pending_request.write().unwrap();
        *pending = Some(request.clone());
        request
    }

    /// Check if there's a pending auth request.
    pub fn has_pending_request(&self) -> bool {
        let pending = self.pending_request.read().unwrap();
        pending.is_some()
    }

    /// Get the pending auth request, if any.
    pub fn pending_request(&self) -> Option<AuthRequest> {
        let pending = self.pending_request.read().unwrap();
        pending.clone()
    }

    /// Submit a token for authentication.
    ///
    /// This validates the token and returns the result.
    /// On success, the pending request is cleared.
    pub fn submit_token(&mut self, token: AuthToken) -> AuthResult {
        let pending = self.pending_request.read().unwrap();
        let Some(_request) = pending.as_ref() else {
            return AuthResult::NoPendingRequest;
        };

        // Basic validation: token must not be empty
        if token.value().is_empty() {
            return AuthResult::InvalidToken("Token value cannot be empty".to_string());
        }

        // Run custom validator if set
        let validator = self.validator.read().unwrap();
        if let Some(ref v) = *validator {
            if let Err(reason) = v(&token) {
                return AuthResult::InvalidToken(reason);
            }
        }

        // Success - clear the pending request
        drop(pending);
        let mut pending = self.pending_request.write().unwrap();
        *pending = None;

        AuthResult::Success(token)
    }

    /// Cancel the pending auth request.
    pub fn cancel_request(&mut self) {
        let mut pending = self.pending_request.write().unwrap();
        *pending = None;
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_token_new() {
        let token = AuthToken::new("test-value", AuthTokenType::Bearer);
        assert_eq!(token.value(), "test-value");
        assert_eq!(token.token_type(), AuthTokenType::Bearer);
    }

    #[test]
    fn test_auth_request_new() {
        let request = AuthRequest::new("tool1", "Please authenticate");
        assert_eq!(request.tool_name(), "tool1");
        assert_eq!(request.message(), "Please authenticate");
        assert!(!request.request_id().is_empty());
    }

    #[test]
    fn test_auth_manager_new() {
        let manager = AuthManager::new();
        assert!(!manager.has_pending_request());
    }

    #[test]
    fn test_auth_manager_request_auth() {
        let mut manager = AuthManager::new();
        manager.request_auth("tool1", "Please authenticate");
        assert!(manager.has_pending_request());
    }

    #[test]
    fn test_auth_manager_submit_token_no_request() {
        let mut manager = AuthManager::new();
        let token = AuthToken::new("test", AuthTokenType::Bearer);
        let result = manager.submit_token(token);
        assert!(matches!(result, AuthResult::NoPendingRequest));
    }

    #[test]
    fn test_auth_manager_submit_token_empty() {
        let mut manager = AuthManager::new();
        manager.request_auth("tool1", "Please authenticate");

        let token = AuthToken::new("", AuthTokenType::Bearer);
        let result = manager.submit_token(token);
        assert!(matches!(result, AuthResult::InvalidToken(_)));
        assert!(manager.has_pending_request());
    }

    #[test]
    fn test_auth_manager_submit_token_success() {
        let mut manager = AuthManager::new();
        manager.request_auth("tool1", "Please authenticate");

        let token = AuthToken::new("valid-token", AuthTokenType::Bearer);
        let result = manager.submit_token(token);
        assert!(matches!(result, AuthResult::Success(_)));
        assert!(!manager.has_pending_request());
    }

    #[test]
    fn test_auth_manager_cancel() {
        let mut manager = AuthManager::new();
        manager.request_auth("tool1", "Please authenticate");
        assert!(manager.has_pending_request());

        manager.cancel_request();
        assert!(!manager.has_pending_request());
    }
}
