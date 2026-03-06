use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use tracing::warn;

use maestro_claw::config::Config as ClawConfig;
use maestro_claw::integration::ApprovalCallback;
use maestro_claw::{
    agent_loop, build_default_hook_system, build_default_tool_registry_with_extras, AgentConfig,
    AnthropicConfig, AnthropicProvider, OpenAIConfig, OpenAIProvider, ProviderAdapter,
    SecurityPolicyBridge, Tool as ClawTool, ToolOutput,
};
use maestro_core::traits::Tool as CoreTool;
use maestro_core::{
    ApprovalDecision, AuthToken, ChannelType, McpManagerSnapshot, McpServerConfig, McpToolBridge,
    SecurityPolicy,
};

use crate::agent::{
    AgentExecuteRequest, AgentExecuteResponse, AgentStatusResponse, ApprovalDecisionRequest,
    ApprovalDecisionResponse, ApprovalQueueResponse, GatewayAuthTokenType, McpServerInfo,
    McpServerListResponse, McpServerRegisterRequest, McpServerRegisterResponse,
    McpServerRemoveResponse, PairingInitiateRequest, PairingInitiateResponse, PairingListResponse,
    PairingVerifyRequest, PairingVerifyResponse, PendingPairingInfo, PendingToolAuth,
    PendingToolAuthResponse, SessionCreateRequest, SessionCreateResponse, SessionInfo,
    SessionListResponse, TokenListResponse, TokenRevokeResponse,
};
use crate::protocol::ResponseFrame;
use crate::state::{event_scope_guards, scopes, AuthContext, GatewayState, StoredAgentSession};

pub(crate) enum McpConnectOutcome {
    Connected,
    AuthRequired(PendingToolAuth),
}

const WORKSPACE_MCP_SERVERS_FILE: &str = "mcp/servers.toml";

#[derive(Debug, Clone)]
pub(crate) struct GatewayRuntimeError {
    status: StatusCode,
    message: String,
}

impl GatewayRuntimeError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub(crate) fn to_ws_response(&self, request_id: &str) -> ResponseFrame {
        ResponseFrame::error(
            request_id,
            self.message.clone(),
            Some(i32::from(self.status.as_u16())),
        )
    }
}

impl From<String> for GatewayRuntimeError {
    fn from(value: String) -> Self {
        Self::internal(value)
    }
}

impl std::fmt::Display for GatewayRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GatewayRuntimeError {}

impl IntoResponse for GatewayRuntimeError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.message,
            })),
        )
            .into_response()
    }
}

fn auth_error(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
}

pub(crate) fn verify_agent_auth(
    state: &GatewayState,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> Result<AuthContext, Response> {
    verify_agent_auth_scoped(state, headers, query_token, None)
}

pub(crate) fn verify_agent_auth_scoped(
    state: &GatewayState,
    headers: &HeaderMap,
    query_token: Option<&str>,
    required_scope: Option<&str>,
) -> Result<AuthContext, Response> {
    let provided = extract_bearer(headers)
        .or_else(|| extract_api_key(headers))
        .or(query_token);
    let auth_disabled = state.config.agent_api_key.is_none() && state.issued_tokens.is_empty();

    if auth_disabled {
        return Ok(AuthContext::master());
    }

    let Some(token) = provided else {
        return Err(auth_error(
            "Authorization: Bearer <token> header or api_key query parameter required",
        ));
    };

    let Some(context) = state.validate_token(token) else {
        return Err(auth_error("Invalid or expired token"));
    };

    if let Some(scope) = required_scope {
        if !context.has_scope(scope) {
            return Err(auth_error("Token lacks the required scope"));
        }
    }

    Ok(context)
}

pub(crate) fn parse_requested_scopes(scopes_param: Option<&str>) -> HashSet<String> {
    let mut scopes = HashSet::new();
    if let Some(param) = scopes_param {
        for scope in param
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            scopes.insert(scope.to_string());
        }
    }

    if scopes.is_empty() {
        scopes.insert(scopes::APPROVALS.to_string());
        scopes.insert(scopes::SESSIONS.to_string());
        scopes.insert(scopes::TOOLS.to_string());
        scopes.insert(scopes::CRON.to_string());
        scopes.insert(scopes::SYSTEM.to_string());
    }

    scopes
}

pub(crate) fn parse_event_scopes(scopes_param: Option<&str>) -> HashSet<String> {
    parse_requested_scopes(scopes_param)
}

fn normalize_gateway_scopes(scopes: impl IntoIterator<Item = String>) -> HashSet<String> {
    let allowed = [
        scopes::APPROVALS,
        scopes::SESSIONS,
        scopes::TOOLS,
        scopes::CRON,
        scopes::SYSTEM,
    ];
    let mut normalized = HashSet::new();
    for scope in scopes {
        if allowed.contains(&scope.as_str()) {
            normalized.insert(scope);
        }
    }
    if normalized.is_empty() {
        normalized.extend(allowed.into_iter().map(str::to_string));
    }
    normalized
}

pub(crate) fn event_visible_to_scopes(event_name: &str, allowed_scopes: &HashSet<String>) -> bool {
    let guards = event_scope_guards();
    match guards.get(event_name) {
        Some(required) => required.iter().any(|scope| allowed_scopes.contains(*scope)),
        None => {
            allowed_scopes.contains(scopes::SYSTEM) || allowed_scopes.contains(scopes::SESSIONS)
        }
    }
}

pub(crate) fn event_visible(
    event: &crate::protocol::EventFrame,
    allowed_scopes: &HashSet<String>,
) -> bool {
    event_visible_to_scopes(&event.event, allowed_scopes)
}

fn workspace_mcp_servers_path(state: &GatewayState) -> Option<PathBuf> {
    state
        .config
        .workspace_path
        .as_ref()
        .map(|workspace| workspace.join(WORKSPACE_MCP_SERVERS_FILE))
}

fn sync_gateway_auth_tokens_from_snapshot(state: &GatewayState, snapshot: &McpManagerSnapshot) {
    state.clear_all_auth_tokens();
    for server in &snapshot.servers {
        if let Some(auth_token) = server.auth_token.clone() {
            if let Ok(auth_token) = auth_token.into_auth_token() {
                state.store_auth_token_for_server(server.config.name.clone(), auth_token);
            }
        }
    }
}

async fn persist_workspace_mcp_servers(
    state: &Arc<GatewayState>,
) -> Result<(), GatewayRuntimeError> {
    let Some(path) = workspace_mcp_servers_path(state) else {
        return Ok(());
    };

    let snapshot = state.mcp_manager.snapshot().await;
    let content = toml::to_string_pretty(&snapshot)
        .map_err(|error| GatewayRuntimeError::internal(error.to_string()))?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| GatewayRuntimeError::internal(error.to_string()))?;
    }
    tokio::fs::write(&path, content)
        .await
        .map_err(|error| GatewayRuntimeError::internal(error.to_string()))?;

    sync_gateway_auth_tokens_from_snapshot(state, &snapshot);
    Ok(())
}

pub(crate) async fn hydrate_workspace_mcp_servers(state: &Arc<GatewayState>) -> anyhow::Result<()> {
    let Some(path) = workspace_mcp_servers_path(state) else {
        return Ok(());
    };
    if !tokio::fs::try_exists(&path).await? {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(&path).await?;
    let snapshot: McpManagerSnapshot = toml::from_str(&content)?;
    state.mcp_manager.hydrate_snapshot(snapshot.clone()).await?;
    sync_gateway_auth_tokens_from_snapshot(state, &snapshot);
    Ok(())
}

pub(crate) fn initiate_pairing(
    state: &Arc<GatewayState>,
    req: &PairingInitiateRequest,
) -> PairingInitiateResponse {
    let scopes = normalize_gateway_scopes(req.parse_scopes().into_iter());
    let pairing = state.create_pairing(req.device_name.clone(), scopes);
    PairingInitiateResponse {
        code: pairing.code,
        expires_at: pairing.expires_at,
        challenge_id: pairing.challenge_id,
        message: "Pairing initiated. Verify this code to receive a scoped access token."
            .to_string(),
    }
}

pub(crate) fn verify_pairing_code(
    state: &Arc<GatewayState>,
    req: &PairingVerifyRequest,
) -> Result<PairingVerifyResponse, GatewayRuntimeError> {
    let token = state
        .verify_pairing(&req.code, req.ttl_seconds.unwrap_or(86_400))
        .ok_or_else(|| GatewayRuntimeError::bad_request("Invalid or expired pairing code"))?;
    let created = create_session(
        state,
        &SessionCreateRequest {
            metadata: None,
            provider: state.config.default_llm_provider.clone(),
            model: state.config.default_model.clone(),
        },
    );

    let mut scopes: Vec<_> = token.scopes.iter().cloned().collect();
    scopes.sort();

    Ok(PairingVerifyResponse {
        access_token: token.token,
        token_id: token.token_id,
        token_type: "issued".to_string(),
        scopes,
        device_name: token.device_name,
        expires_at: token.expires_at,
        session_id: created.session_id,
    })
}

pub(crate) fn list_pairings(state: &Arc<GatewayState>) -> PairingListResponse {
    let pending = state
        .list_pending_pairings()
        .into_iter()
        .map(|pairing| {
            let mut scopes: Vec<_> = pairing.scopes.into_iter().collect();
            scopes.sort();
            PendingPairingInfo {
                code: pairing.code,
                device_name: pairing.device_name,
                scopes,
                expires_at: pairing.expires_at,
                created_at: pairing.created_at,
            }
        })
        .collect();
    PairingListResponse::new(pending)
}

pub(crate) fn list_tokens(state: &Arc<GatewayState>) -> TokenListResponse {
    TokenListResponse::new(state.list_tokens())
}

pub(crate) fn revoke_token(
    state: &Arc<GatewayState>,
    token_id: &str,
) -> Result<TokenRevokeResponse, GatewayRuntimeError> {
    if !state.revoke_token_by_id(token_id) {
        return Err(GatewayRuntimeError::not_found(format!(
            "Token not found: {}",
            token_id
        )));
    }
    Ok(TokenRevokeResponse { revoked: true })
}

pub(crate) fn list_sessions(state: &Arc<GatewayState>) -> SessionListResponse {
    let sessions: Vec<SessionInfo> = state
        .session_store
        .iter()
        .map(|entry| entry.value().info.clone())
        .collect();
    SessionListResponse::from_sessions(sessions)
}

pub(crate) fn create_session(
    state: &Arc<GatewayState>,
    req: &SessionCreateRequest,
) -> SessionCreateResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let thread = maestro_claw::Thread::new(session_id.clone());

    state.session_store.insert(
        session_id.clone(),
        StoredAgentSession {
            info: SessionInfo {
                id: session_id.clone(),
                thread_count: 1,
                turn_count: 0,
                created_at: created_at.clone(),
                status: "idle".to_string(),
            },
            thread,
            provider: req.provider.clone(),
            model: req.model.clone().unwrap_or_default(),
            pending_approval_id: None,
            pending_auth_id: None,
        },
    );

    SessionCreateResponse {
        session_id,
        created_at,
    }
}

pub(crate) fn get_session_info(state: &Arc<GatewayState>, session_id: &str) -> Option<SessionInfo> {
    state
        .session_store
        .get(session_id)
        .map(|entry| entry.value().info.clone())
}

pub(crate) fn delete_session(state: &Arc<GatewayState>, session_id: &str) -> bool {
    state.remove_session(session_id)
}

pub(crate) fn agent_status(state: &Arc<GatewayState>) -> AgentStatusResponse {
    AgentStatusResponse {
        status: if state.active_run_count() > 0 {
            "busy".to_string()
        } else {
            "idle".to_string()
        },
        sessions: state.session_store.len(),
        active_runs: state.active_run_count(),
        pending_approvals: state.pending_approval_count(),
        pending_auth: state.pending_tool_auth_count(),
    }
}

fn managed_server_info(
    server: maestro_core::ManagedServer,
    pending_auth: &[PendingToolAuth],
) -> McpServerInfo {
    let maestro_core::ManagedServer {
        config,
        state,
        connected_at,
        last_error,
        has_auth_token,
        auth_token_type,
        auth_updated_at,
        tools_count,
    } = server;
    let pending_auth = pending_auth
        .iter()
        .find(|auth| auth.server_name == config.name)
        .cloned();

    McpServerInfo {
        name: config.name,
        url: config.url,
        command: config.command,
        requires_auth: config.requires_auth,
        has_oauth: config.oauth_config.is_some(),
        state: state.as_str().to_string(),
        connected: state == maestro_core::ServerState::Connected,
        has_auth_token,
        auth_token_type,
        auth_updated_at,
        connected_at,
        last_error,
        tools_count,
        pending_auth,
    }
}

pub(crate) async fn list_mcp_servers(state: &Arc<GatewayState>) -> McpServerListResponse {
    let pending_auth = state.list_pending_tool_auth();
    let mut servers: Vec<_> = state
        .mcp_manager
        .list_managed_servers()
        .await
        .into_iter()
        .map(|server| managed_server_info(server, &pending_auth))
        .collect();
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    McpServerListResponse {
        count: servers.len(),
        servers,
    }
}

pub(crate) async fn register_or_update_mcp_server(
    state: &Arc<GatewayState>,
    req: &McpServerRegisterRequest,
) -> Result<McpServerRegisterResponse, GatewayRuntimeError> {
    if req.url.is_none() && req.command.is_none() {
        return Err(GatewayRuntimeError::bad_request(
            "MCP server must provide either a url or command",
        ));
    }

    let config = McpServerConfig {
        name: req.name.clone(),
        url: req.url.clone(),
        command: req.command.clone(),
        args: req.args.clone(),
        env: req.env.clone(),
        requires_auth: req.requires_auth,
        oauth_config: req.oauth_config.clone(),
    };
    let updated = state.mcp_manager.get_config(&req.name).await.is_some();
    let previous_snapshot = state.mcp_manager.snapshot().await;

    if updated {
        state.mcp_manager.update_server(config).await;
    } else {
        state.mcp_manager.register_server(config).await;
    }

    if let Err(error) = persist_workspace_mcp_servers(state).await {
        state
            .mcp_manager
            .hydrate_snapshot(previous_snapshot.clone())
            .await
            .map_err(|hydrate_error| GatewayRuntimeError::internal(hydrate_error.to_string()))?;
        sync_gateway_auth_tokens_from_snapshot(state, &previous_snapshot);
        return Err(error);
    }

    Ok(McpServerRegisterResponse {
        name: req.name.clone(),
        registered: true,
        updated,
    })
}

pub(crate) async fn remove_mcp_server(
    state: &Arc<GatewayState>,
    server_name: &str,
) -> Result<McpServerRemoveResponse, GatewayRuntimeError> {
    let previous_snapshot = state.mcp_manager.snapshot().await;
    if !state.mcp_manager.remove_server(server_name).await {
        return Err(GatewayRuntimeError::bad_request(format!(
            "Unable to remove MCP server '{}'",
            server_name
        )));
    }
    if let Err(error) = persist_workspace_mcp_servers(state).await {
        state
            .mcp_manager
            .hydrate_snapshot(previous_snapshot.clone())
            .await
            .map_err(|hydrate_error| GatewayRuntimeError::internal(hydrate_error.to_string()))?;
        sync_gateway_auth_tokens_from_snapshot(state, &previous_snapshot);
        return Err(error);
    }
    state.clear_tool_auth_state(server_name);
    Ok(McpServerRemoveResponse {
        name: server_name.to_string(),
        removed: true,
    })
}

pub(crate) async fn disconnect_mcp_server(
    state: &Arc<GatewayState>,
    server_name: &str,
) -> Result<(), GatewayRuntimeError> {
    state
        .mcp_manager
        .disconnect(server_name)
        .await
        .map_err(|error| GatewayRuntimeError::bad_request(error.to_string()))
}

fn build_agent_provider(
    config: &crate::state::GatewayConfig,
    req: &AgentExecuteRequest,
) -> Result<Arc<dyn maestro_claw::agent::Provider>, GatewayRuntimeError> {
    let provider_name = req
        .provider
        .as_deref()
        .unwrap_or(config.default_llm_provider.as_str());

    match provider_name {
        "openai" => {
            let api_key = config.openai_api_key.as_deref().ok_or_else(|| {
                GatewayRuntimeError::service_unavailable(
                    "OpenAI API key not configured (set openai_api_key)",
                )
            })?;
            let model = req
                .model
                .as_deref()
                .or(config.default_model.as_deref())
                .unwrap_or("gpt-4o");
            let inner =
                OpenAIProvider::new(OpenAIConfig::new(api_key.to_string(), model.to_string()))
                    .map_err(|err| GatewayRuntimeError::internal(err.to_string()))?;
            Ok(Arc::new(ProviderAdapter::new(Arc::new(inner))))
        }
        "anthropic" => {
            let api_key = config.anthropic_api_key.as_deref().ok_or_else(|| {
                GatewayRuntimeError::service_unavailable(
                    "Anthropic API key not configured (set anthropic_api_key)",
                )
            })?;
            let model = req
                .model
                .as_deref()
                .or(config.default_model.as_deref())
                .unwrap_or("claude-3-5-sonnet-20241022");
            let inner = AnthropicProvider::new(AnthropicConfig::new(
                api_key.to_string(),
                model.to_string(),
            ))
            .map_err(|err| GatewayRuntimeError::internal(err.to_string()))?;
            Ok(Arc::new(ProviderAdapter::new(Arc::new(inner))))
        }
        unknown => Err(GatewayRuntimeError::bad_request(format!(
            "Unknown provider '{}'. Supported: openai, anthropic",
            unknown
        ))),
    }
}

fn gateway_workspace(policy: &SecurityPolicy) -> PathBuf {
    policy
        .allowed_write_paths
        .first()
        .cloned()
        .or_else(|| policy.allowed_read_paths.first().cloned())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn build_gateway_runtime_config(state: &GatewayState, provider_name: &str) -> ClawConfig {
    let mut config = ClawConfig::default();
    config.primary_tool = provider_name.to_string();
    config.workspace_dir = gateway_workspace(state.sandbox_manager.default_policy());
    // Keep the full tool surface available, then let the security bridge
    // enforce approval/autonomy policy inside the tool loop.
    config.autonomy.level = "autonomous".to_string();
    config.autonomy.workspace_only = true;
    config
}

fn build_gateway_security_policy(
    state: &GatewayState,
    workspace_dir: &std::path::Path,
) -> SecurityPolicy {
    let mut policy = state.sandbox_manager.default_policy().clone();
    if policy.allowed_read_paths.is_empty() {
        policy.allowed_read_paths = vec![workspace_dir.to_path_buf()];
    }
    if policy.allowed_write_paths.is_empty() {
        policy.allowed_write_paths = vec![workspace_dir.to_path_buf()];
    }
    policy
}

fn prompt_preview(prompt: &str) -> String {
    const PREVIEW_CHARS: usize = 97;
    if prompt.chars().count() > PREVIEW_CHARS {
        let truncated: String = prompt.chars().take(PREVIEW_CHARS).collect();
        format!("{truncated}...")
    } else {
        prompt.to_string()
    }
}

fn approval_decision_outcome(decision: Option<ApprovalDecision>) -> Option<bool> {
    match decision {
        Some(ApprovalDecision::Approve | ApprovalDecision::Always) => Some(true),
        Some(ApprovalDecision::Reject) => Some(false),
        None => None,
    }
}

struct GatewayApprovalCallback {
    state: Arc<GatewayState>,
    session_id: String,
    thread_id: String,
}

#[async_trait::async_trait]
impl ApprovalCallback for GatewayApprovalCallback {
    async fn request_approval(&self, operation: &str, details: &serde_json::Value) -> bool {
        let tool_name = details
            .get("tool_name")
            .and_then(|value| value.as_str())
            .unwrap_or(operation);

        if self
            .state
            .approval_manager
            .should_auto_approve(operation, ChannelType::Cli)
        {
            return true;
        }

        let approval = self.state.enqueue_approval(
            &self.session_id,
            &self.thread_id,
            tool_name,
            operation,
            details
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        );

        let Some(mut receiver) = self
            .state
            .subscribe_approval_resolution(&approval.request_id)
        else {
            self.state.expire_approval(&approval.request_id);
            return false;
        };

        if let Some(approved) = approval_decision_outcome(*receiver.borrow()) {
            return approved;
        }

        let wait = tokio::time::timeout(Duration::from_secs(900), receiver.changed()).await;
        match wait {
            Ok(Ok(())) => approval_decision_outcome(*receiver.borrow()).unwrap_or(false),
            _ => {
                self.state.expire_approval(&approval.request_id);
                false
            }
        }
    }
}

struct GatewayMcpTool {
    inner: McpToolBridge,
}

impl GatewayMcpTool {
    fn new(inner: McpToolBridge) -> Self {
        Self { inner }
    }
}

fn json_value_to_content(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text,
        other => serde_json::to_string_pretty(&other).unwrap_or_else(|_| other.to_string()),
    }
}

#[async_trait::async_trait]
impl ClawTool for GatewayMcpTool {
    fn name(&self) -> &str {
        CoreTool::name(&self.inner)
    }

    fn description(&self) -> &str {
        CoreTool::description(&self.inner)
    }

    fn parameters_schema(&self) -> serde_json::Value {
        CoreTool::input_schema(&self.inner)
    }

    async fn execute(&self, arguments: serde_json::Value) -> ToolOutput {
        match CoreTool::execute(&self.inner, arguments).await {
            Ok(value) => ToolOutput::success(json_value_to_content(value)),
            Err(error) => ToolOutput::error(error.to_string()),
        }
    }
}

async fn build_extra_mcp_tools(state: &Arc<GatewayState>) -> Vec<Arc<dyn ClawTool>> {
    let mut tools: Vec<Arc<dyn ClawTool>> = Vec::new();
    for server_name in state.mcp_manager.connected_servers().await {
        match state.mcp_manager.create_tool_bridges(&server_name).await {
            Ok(bridges) => {
                for bridge in bridges {
                    tools.push(Arc::new(GatewayMcpTool::new(bridge)) as Arc<dyn ClawTool>);
                }
            }
            Err(error) => {
                warn!(
                    server_name = %server_name,
                    error = %error,
                    "Failed to build MCP tool bridges"
                );
            }
        }
    }
    tools
}

pub(crate) async fn execute_agent_request(
    state: Arc<GatewayState>,
    req: AgentExecuteRequest,
) -> Result<AgentExecuteResponse, GatewayRuntimeError> {
    state.gc_sessions();

    let mut effective_req = req.clone();

    let session_id = if let Some(session_id) = req.session_id.clone() {
        if let Some(entry) = state.session_store.get(&session_id) {
            if effective_req.provider.is_none() && !entry.provider.is_empty() {
                effective_req.provider = Some(entry.provider.clone());
            }
            if effective_req.model.is_none() && !entry.model.is_empty() {
                effective_req.model = Some(entry.model.clone());
            }
            session_id
        } else {
            return Err(GatewayRuntimeError::not_found("Session not found"));
        }
    } else {
        create_session(
            &state,
            &SessionCreateRequest {
                metadata: None,
                provider: req
                    .provider
                    .clone()
                    .unwrap_or_else(|| state.config.default_llm_provider.clone()),
                model: req.model.clone(),
            },
        )
        .session_id
    };

    let provider = build_agent_provider(&state.config, &effective_req)?;

    let mut thread = state
        .session_store
        .get(&session_id)
        .map(|entry| entry.thread.clone())
        .ok_or_else(|| GatewayRuntimeError::not_found("Session not found"))?;
    thread.add_turn(maestro_claw::Turn::new(
        maestro_claw::TurnRole::User,
        effective_req.prompt.clone(),
    ));

    let thread_id = thread.id().to_string();
    let provider_name = effective_req
        .provider
        .clone()
        .unwrap_or_else(|| state.config.default_llm_provider.clone());

    state.set_session_status(&session_id, "active");
    state.add_active_run();
    state.broadcast(
        "agent.execute.started",
        serde_json::json!({
            "session_id": session_id,
            "thread_id": thread_id,
            "prompt_preview": prompt_preview(&effective_req.prompt),
            "provider": provider_name,
        }),
    );

    let runtime_config = build_gateway_runtime_config(&state, &provider_name);
    let security_bridge = SecurityPolicyBridge::new(build_gateway_security_policy(
        &state,
        &runtime_config.workspace_dir,
    ))
    .with_approval_callback(Arc::new(GatewayApprovalCallback {
        state: state.clone(),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
    }));
    let extra_tools = build_extra_mcp_tools(&state).await;
    let tools = build_default_tool_registry_with_extras(
        &runtime_config,
        extra_tools,
        Some(security_bridge),
    );
    let hooks = build_default_hook_system(&provider_name);
    let config = AgentConfig::default()
        .with_max_turns(effective_req.max_turns.unwrap_or(20))
        .with_timeout(state.config.request_timeout_secs);

    let result = agent_loop(&mut thread, provider, tools, hooks, config).await;
    state.remove_active_run();

    if let Some(mut entry) = state.session_store.get_mut(&session_id) {
        entry.thread = thread.clone();
        entry.info.turn_count = thread.turn_count();
        entry.info.thread_count = 1;
        if entry.provider.is_empty() {
            entry.provider = provider_name.clone();
        }
        if entry.model.is_empty() {
            entry.model = effective_req.model.clone().unwrap_or_default();
        }
    }

    match result {
        Ok(agent_result) => {
            state.set_session_status(&session_id, "idle");
            state.broadcast(
                "agent.execute.completed",
                serde_json::json!({
                    "session_id": session_id,
                    "thread_id": thread_id,
                    "turns_used": agent_result.total_turns,
                    "tool_calls": agent_result.tool_calls_executed,
                    "completed_normally": agent_result.completed_normally,
                }),
            );

            Ok(AgentExecuteResponse {
                session_id,
                thread_id,
                content: agent_result.content().to_string(),
                turns_used: agent_result.total_turns,
                tool_calls: agent_result.tool_calls_executed,
                completed_normally: agent_result.completed_normally,
                termination_reason: if !agent_result.completed_normally {
                    Some(agent_result.termination_reason)
                } else {
                    None
                },
            })
        }
        Err(error) => {
            if state
                .session_store
                .get(&session_id)
                .map(|entry| entry.info.status == "active")
                .unwrap_or(false)
            {
                state.set_session_status(&session_id, "error");
            }
            Err(GatewayRuntimeError::internal(error.to_string()))
        }
    }
}

pub(crate) async fn connect_mcp_server_for_session(
    state: Arc<GatewayState>,
    server_name: &str,
    session_id: Option<&str>,
) -> Result<McpConnectOutcome, GatewayRuntimeError> {
    let config = state.mcp_manager.get_config(server_name).await;
    if let Some(config) = config {
        if config.requires_auth
            && state
                .mcp_manager
                .get_auth_token(server_name)
                .await
                .is_none()
        {
            let auth = state.enqueue_tool_auth(
                server_name,
                session_id,
                if config.oauth_config.is_some() {
                    GatewayAuthTokenType::Oauth
                } else {
                    GatewayAuthTokenType::Bearer
                },
                format!("Authenticate MCP server '{}'", server_name),
                config.oauth_config,
            );
            return Ok(McpConnectOutcome::AuthRequired(auth));
        }
    } else {
        return Err(GatewayRuntimeError::not_found(format!(
            "MCP server not found: {}",
            server_name
        )));
    }

    state
        .mcp_manager
        .connect(server_name)
        .await
        .map_err(|error| GatewayRuntimeError::bad_request(error.to_string()))?;

    let _ = state.mark_tool_auth_connected_for_server(server_name);

    Ok(McpConnectOutcome::Connected)
}

pub(crate) async fn connect_mcp_server(
    state: Arc<GatewayState>,
    server_name: &str,
) -> Result<McpConnectOutcome, GatewayRuntimeError> {
    connect_mcp_server_for_session(state, server_name, None).await
}

pub(crate) async fn submit_mcp_auth(
    state: &Arc<GatewayState>,
    request_id: &str,
    token: AuthToken,
) -> Result<crate::agent::McpAuthSubmitResponse, GatewayRuntimeError> {
    let previous_snapshot = state.mcp_manager.snapshot().await;
    let auth = state
        .submit_tool_auth_token(request_id, token.clone())
        .map_err(GatewayRuntimeError::not_found)?;
    if !state
        .mcp_manager
        .set_auth_token(&auth.server_name, token)
        .await
    {
        state.clear_tool_auth_state(&auth.server_name);
        return Err(GatewayRuntimeError::not_found(format!(
            "MCP server not found: {}",
            auth.server_name
        )));
    }
    if let Err(error) = persist_workspace_mcp_servers(state).await {
        state
            .mcp_manager
            .hydrate_snapshot(previous_snapshot.clone())
            .await
            .map_err(|hydrate_error| GatewayRuntimeError::internal(hydrate_error.to_string()))?;
        sync_gateway_auth_tokens_from_snapshot(state, &previous_snapshot);
        return Err(error);
    }
    state
        .mcp_manager
        .connect(&auth.server_name)
        .await
        .map_err(|error| {
            let message = error.to_string();
            let _ = state.mark_tool_auth_failed_for_server(&auth.server_name, &message);
            GatewayRuntimeError::bad_request(message)
        })?;
    let auth = state
        .mark_tool_auth_connected(request_id)
        .map_err(GatewayRuntimeError::not_found)?;
    let _ = state.mark_tool_auth_connected_for_server(&auth.server_name);
    Ok(crate::agent::McpAuthSubmitResponse {
        auth,
        connected: true,
    })
}

pub(crate) fn approval_queue(state: &Arc<GatewayState>) -> ApprovalQueueResponse {
    ApprovalQueueResponse::new(state.list_pending_approvals())
}

pub(crate) fn resolve_approval_request(
    state: &Arc<GatewayState>,
    request_id: &str,
    request: &ApprovalDecisionRequest,
) -> Result<ApprovalDecisionResponse, GatewayRuntimeError> {
    let approval = state
        .resolve_approval(request_id, request.decision.into())
        .map_err(GatewayRuntimeError::not_found)?;
    Ok(ApprovalDecisionResponse { approval })
}

pub(crate) fn pending_tool_auth(state: &Arc<GatewayState>) -> PendingToolAuthResponse {
    PendingToolAuthResponse::new(state.list_pending_tool_auth())
}

pub(crate) fn gateway_auth_token(
    token: String,
    token_type: Option<GatewayAuthTokenType>,
) -> AuthToken {
    let token_type = token_type.unwrap_or(GatewayAuthTokenType::Bearer);
    AuthToken::new(token, token_type.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn approval_decision_outcome_handles_pre_resolved_state() {
        assert_eq!(
            approval_decision_outcome(Some(ApprovalDecision::Approve)),
            Some(true)
        );
        assert_eq!(
            approval_decision_outcome(Some(ApprovalDecision::Always)),
            Some(true)
        );
        assert_eq!(
            approval_decision_outcome(Some(ApprovalDecision::Reject)),
            Some(false)
        );
        assert_eq!(approval_decision_outcome(None), None);
    }

    #[test]
    fn verify_agent_auth_accepts_bearer_api_key_and_query_token() {
        let mut config = crate::state::GatewayConfig::default();
        config.agent_api_key = Some("master-token".to_string());
        let state = GatewayState::with_config(config);

        let mut bearer = HeaderMap::new();
        bearer.insert(
            "Authorization",
            HeaderValue::from_static("Bearer master-token"),
        );
        assert!(verify_agent_auth(&state, &bearer, None).is_ok());

        let mut api_key = HeaderMap::new();
        api_key.insert("X-API-Key", HeaderValue::from_static("master-token"));
        assert!(verify_agent_auth(&state, &api_key, None).is_ok());

        let query_headers = HeaderMap::new();
        assert!(verify_agent_auth(&state, &query_headers, Some("master-token")).is_ok());
    }

    #[test]
    fn verify_agent_auth_enforces_required_scope_for_issued_tokens() {
        let state = GatewayState::new();
        let mut granted = HashSet::new();
        granted.insert(scopes::SESSIONS.to_string());
        let token = state.issue_token(granted, 3600, Some("device".into()));

        let headers = HeaderMap::new();
        let auth =
            verify_agent_auth_scoped(&state, &headers, Some(&token.token), Some(scopes::SESSIONS));
        assert!(auth.is_ok());

        let denied = verify_agent_auth_scoped(
            &state,
            &headers,
            Some(&token.token),
            Some(scopes::APPROVALS),
        );
        assert!(denied.is_err());
    }

    #[test]
    fn pairing_flow_creates_scoped_token_and_session() {
        let state = Arc::new(GatewayState::new());
        let initiated = initiate_pairing(
            &state,
            &PairingInitiateRequest {
                device_name: Some("local-cli".into()),
                scopes: Some(json!(["sessions", "tools", "ignored"])),
            },
        );

        assert_eq!(initiated.code.len(), 6);

        let verified = verify_pairing_code(
            &state,
            &PairingVerifyRequest {
                code: initiated.code,
                ttl_seconds: Some(600),
            },
        )
        .expect("pairing should verify");

        assert_eq!(verified.device_name.as_deref(), Some("local-cli"));
        assert!(!verified.token_id.is_empty());
        assert!(verified.scopes.contains(&scopes::SESSIONS.to_string()));
        assert!(verified.scopes.contains(&scopes::TOOLS.to_string()));
        assert!(
            state.session_store.get(&verified.session_id).is_some(),
            "pairing should create a session"
        );
    }

    #[test]
    fn token_inventory_can_be_listed_and_revoked_by_id() {
        let state = Arc::new(GatewayState::new());
        let mut scopes = HashSet::new();
        scopes.insert(scopes::SESSIONS.to_string());

        let issued = state.issue_token(scopes, 600, Some("desktop".into()));
        let listed = list_tokens(&state);

        assert_eq!(listed.total, 1);
        assert_eq!(listed.tokens[0].token_id, issued.token_id);
        assert_eq!(listed.tokens[0].device_name.as_deref(), Some("desktop"));

        let revoked = revoke_token(&state, &issued.token_id).expect("token should revoke");
        assert!(revoked.revoked);
        assert!(state.validate_token(&issued.token).is_none());
    }

    #[tokio::test]
    async fn workspace_mcp_servers_are_persisted_and_hydrated() {
        let temp = tempdir().unwrap();
        let workspace = temp.path().to_path_buf();

        let mut config = crate::state::GatewayConfig::default();
        config.workspace_path = Some(workspace.clone());
        let state = Arc::new(GatewayState::with_config(config));

        let register = McpServerRegisterRequest {
            name: "github".to_string(),
            url: Some("http://localhost:8080".to_string()),
            command: None,
            args: vec!["serve".to_string()],
            env: std::collections::HashMap::new(),
            requires_auth: true,
            oauth_config: None,
        };
        register_or_update_mcp_server(&state, &register)
            .await
            .expect("server should persist");

        let auth = state.enqueue_tool_auth(
            "github",
            None,
            GatewayAuthTokenType::Bearer,
            "Authenticate GitHub MCP",
            None,
        );
        submit_mcp_auth(
            &state,
            &auth.request_id,
            AuthToken::new("secret-token", maestro_core::AuthTokenType::Bearer),
        )
        .await
        .expect("auth should persist");

        let persisted = tokio::fs::read_to_string(workspace.join(WORKSPACE_MCP_SERVERS_FILE))
            .await
            .expect("persisted servers file");
        assert!(persisted.contains("github"));
        assert!(persisted.contains("secret-token"));

        let mut restored_config = crate::state::GatewayConfig::default();
        restored_config.workspace_path = Some(workspace);
        let restored = Arc::new(GatewayState::with_config(restored_config));
        hydrate_workspace_mcp_servers(&restored)
            .await
            .expect("workspace hydrate should succeed");

        let hydrated = restored
            .mcp_manager
            .get_config("github")
            .await
            .expect("hydrated config");
        assert_eq!(hydrated.url.as_deref(), Some("http://localhost:8080"));
        assert_eq!(
            restored
                .mcp_manager
                .get_auth_token("github")
                .await
                .expect("hydrated auth token")
                .value(),
            "secret-token"
        );
        assert_eq!(
            restored
                .auth_token_for_server("github")
                .expect("gateway auth token")
                .value(),
            "secret-token"
        );
    }

    #[tokio::test]
    async fn submit_mcp_auth_stores_token_in_gateway_and_manager() {
        let state = Arc::new(GatewayState::new());
        state
            .mcp_manager
            .register_server(McpServerConfig {
                name: "github".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: std::collections::HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let created = create_session(
            &state,
            &SessionCreateRequest {
                metadata: None,
                provider: "openai".to_string(),
                model: None,
            },
        );

        let pending =
            connect_mcp_server_for_session(state.clone(), "github", Some(&created.session_id))
                .await
                .expect("connect should request auth");
        let auth = match pending {
            McpConnectOutcome::AuthRequired(auth) => auth,
            McpConnectOutcome::Connected => panic!("auth should be required"),
        };

        let response = submit_mcp_auth(
            &state,
            &auth.request_id,
            AuthToken::new("secret-token", maestro_core::AuthTokenType::Bearer),
        )
        .await
        .expect("auth submission should connect");

        assert!(response.connected);
        assert_eq!(
            state.auth_token_for_server("github").unwrap().value(),
            "secret-token"
        );
        assert_eq!(
            state
                .mcp_manager
                .get_auth_token("github")
                .await
                .expect("manager token")
                .value(),
            "secret-token"
        );
        let session = state
            .session_store
            .get(&created.session_id)
            .expect("session");
        assert!(session.pending_auth_id.is_none());
        assert_eq!(session.info.status, "active");
    }

    #[tokio::test]
    async fn submit_mcp_auth_clears_sibling_pending_requests_for_same_server() {
        let state = Arc::new(GatewayState::new());
        state
            .mcp_manager
            .register_server(McpServerConfig {
                name: "github".to_string(),
                url: Some("http://localhost:8080".to_string()),
                command: None,
                args: vec![],
                env: std::collections::HashMap::new(),
                requires_auth: true,
                oauth_config: None,
            })
            .await;

        let session_a = create_session(
            &state,
            &SessionCreateRequest {
                metadata: None,
                provider: "openai".to_string(),
                model: None,
            },
        );
        let session_b = create_session(
            &state,
            &SessionCreateRequest {
                metadata: None,
                provider: "openai".to_string(),
                model: None,
            },
        );

        let auth_a = match connect_mcp_server_for_session(
            state.clone(),
            "github",
            Some(&session_a.session_id),
        )
        .await
        .expect("session a should request auth")
        {
            McpConnectOutcome::AuthRequired(auth) => auth,
            McpConnectOutcome::Connected => panic!("auth should be required"),
        };
        let auth_b = match connect_mcp_server_for_session(
            state.clone(),
            "github",
            Some(&session_b.session_id),
        )
        .await
        .expect("session b should request auth")
        {
            McpConnectOutcome::AuthRequired(auth) => auth,
            McpConnectOutcome::Connected => panic!("auth should be required"),
        };

        submit_mcp_auth(
            &state,
            &auth_a.request_id,
            AuthToken::new("secret-token", maestro_core::AuthTokenType::Bearer),
        )
        .await
        .expect("auth submission should connect");

        assert_eq!(state.pending_tool_auth_count(), 0);
        assert!(state.pending_tool_auth.get(&auth_a.request_id).is_some());
        assert!(state.pending_tool_auth.get(&auth_b.request_id).is_some());
        assert!(state
            .session_store
            .get(&session_a.session_id)
            .expect("session a")
            .pending_auth_id
            .is_none());
        assert!(state
            .session_store
            .get(&session_b.session_id)
            .expect("session b")
            .pending_auth_id
            .is_none());
    }
}
