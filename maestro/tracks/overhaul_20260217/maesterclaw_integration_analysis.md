# MaesterClaw Integration Analysis

## Overview

This document catalogs the capabilities and architectural patterns to be integrated into MaesterClaw - Maestro's unified AI agent framework.

---

## 1. CHANNEL SYSTEM

### 1.1 Trait Architecture

**Core Channel Trait:**
```rust
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool;
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()>;
    async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()>;
}
```

**Advanced Channel Plugin System:**
```rust
pub trait ChannelPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn start_account(&mut self, account_id: &str, config: serde_json::Value) -> Result<()>;
    async fn stop_account(&mut self, account_id: &str) -> Result<()>;
    fn outbound(&self) -> Option<&dyn ChannelOutbound>;
    fn status(&self) -> Option<&dyn ChannelStatus>;
}

pub trait ChannelOutbound: Send + Sync {
    async fn send_text(&self, account_id: &str, to: &str, text: &str, reply_to: Option<&str>) -> Result<()>;
    async fn send_media(&self, account_id: &str, to: &str, payload: &ReplyPayload, reply_to: Option<&str>) -> Result<()>;
    async fn send_typing(&self, account_id: &str, to: &str) -> Result<()>;
    async fn send_location(&self, account_id: &str, to: &str, lat: f64, lng: f64, title: Option<&str>, reply_to: Option<&str>) -> Result<()>;
}

pub trait ChannelStreamOutbound: Send + Sync {
    async fn send_stream(&self, account_id: &str, to: &str, reply_to: Option<&str>, stream: StreamReceiver) -> Result<()>;
    async fn is_stream_enabled(&self, account_id: &str) -> bool;
}
```

### 1.2 Supported Platforms

| Platform | Type | Features |
|----------|------|----------|
| Telegram | Full | Text, Media, Streaming, Voice STT, Location |
| Discord | Full | Text, Media |
| Slack | Full | Text, Media |
| WhatsApp | Full | Text, Media, Webhook signature verification |
| Matrix | Full | Text |
| IRC | Basic | Text |
| Lark | Full | Text |
| iMessage | Full | Text |
| Signal | Full | Text |
| QQ | Full | Text |
| DingTalk | Full | Text |
| Email | Full | Text with subject |
| CLI | Basic | Text (stdin/stdout) |

### 1.3 Data Structures

```rust
pub struct ChannelMessage {
    pub id: String,
    pub sender: String,
    pub reply_target: String,
    pub content: String,
    pub channel: String,
    pub timestamp: u64,
}

pub struct SendMessage {
    pub content: String,
    pub recipient: String,
    pub subject: Option<String>,
}

pub struct ChannelReplyTarget {
    pub channel_type: ChannelType,
    pub account_id: String,
    pub chat_id: String,
    pub message_id: Option<String>,
}

pub struct ChannelAttachment {
    pub media_type: String,  // MIME type
    pub data: Vec<u8>,
}

pub enum ChannelMessageKind {
    Text, Voice, Audio, Photo, Document, Video, Location, Other,
}
```

### 1.4 Event System

```rust
pub enum ChannelEvent {
    InboundMessage {
        channel_type: ChannelType,
        account_id: String,
        peer_id: String,
        username: Option<String>,
        sender_name: Option<String>,
        message_count: Option<i64>,
        access_granted: bool,
    },
    AccountDisabled { channel_type, account_id, reason },
    OtpChallenge { channel_type, account_id, peer_id, username, code, expires_at },
    OtpResolved { channel_type, account_id, peer_id, username, resolution },
}

pub trait ChannelEventSink: Send + Sync {
    async fn emit(&self, event: ChannelEvent);
    async fn dispatch_to_chat(&self, text: &str, reply_to: ChannelReplyTarget, meta: ChannelMessageMeta);
    async fn dispatch_command(&self, command: &str, reply_to: ChannelReplyTarget) -> Result<String>;
    async fn request_disable_account(&self, channel_type: &str, account_id: &str, reason: &str);
    async fn transcribe_voice(&self, audio_data: &[u8], format: &str) -> Result<String>;
    async fn update_location(&self, reply_to: &ChannelReplyTarget, lat: f64, lng: f64) -> bool;
}
```

---

## 2. GATEWAY ARCHITECTURE

### 2.1 HTTP/WebSocket Server

**Endpoints:**
- `GET /health` - Health check (public)
- `POST /pair` - Pairing exchange (X-Pairing-Code header)
- `POST /webhook` - Main message webhook
- `GET /whatsapp` - Meta webhook verification
- `POST /whatsapp` - WhatsApp message webhook
- `WebSocket /ws` - Full-duplex communication

**Security Features:**
- Request body limit: 64KB
- Request timeout: 30s
- Rate limiting: Sliding window per IP
- Idempotency key support (X-Idempotency-Key)
- HMAC signature verification for webhooks

### 2.2 Protocol Frame Types

```rust
pub enum GatewayFrame {
    Request(RequestFrame),
    Response(ResponseFrame),
    Event(EventFrame),
}

pub struct RequestFrame {
    pub id: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

pub struct ResponseFrame {
    pub id: String,
    pub ok: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<ErrorShape>,
}

pub struct EventFrame {
    pub kind: String,
    pub payload: serde_json::Value,
    pub seq: u64,
}
```

### 2.3 Connection Lifecycle

1. WebSocket upgrade with optional auth headers
2. Handshake: Client sends `connect` request with protocol version
3. Auth validation (password/API key/local check)
4. Server sends `hello-ok` with capabilities
5. Message loop: request/response dispatch
6. Cleanup on disconnect

### 2.4 Authentication Methods

```rust
pub enum AuthMethod {
    Password,
    Passkey,   // WebAuthn
    ApiKey,
    Loopback,  // Localhost auto-auth
}

pub struct AuthIdentity {
    pub method: AuthMethod,
}

// API Key with scopes
pub const VALID_SCOPES: &[&str] = &[
    "operator.admin",
    "operator.read",
    "operator.write",
    "operator.approvals",
    "operator.pairing",
];
```

### 2.5 Gateway State

```rust
pub struct GatewayState {
    pub version: String,
    pub hostname: String,
    pub credential_store: Option<Arc<CredentialStore>>,
    pub auth: ResolvedAuth,  // Legacy env-var auth
    // Client registry, node registry, etc.
}
```

---

## 3. CRON/SCHEDULING SYSTEM

### 3.1 Schedule Types

```rust
pub enum Schedule {
    Cron { expr: String, tz: Option<String> },
    At { at: DateTime<Utc> },         // One-shot at specific time
    Every { every_ms: u64 },          // Fixed interval
}

pub enum CronSchedule {
    At { at_ms: u64 },
    Every { every_ms: u64, anchor_ms: Option<u64> },
    Cron { expr: String, tz: Option<String> },
}
```

### 3.2 Job Types

```rust
pub enum JobType {
    Shell,  // Execute shell command
    Agent,  // Run agent prompt
}

pub enum CronPayload {
    SystemEvent { text: String },
    AgentTurn {
        message: String,
        model: Option<String>,
        timeout_secs: Option<u64>,
        deliver: bool,              // Send result to channel
        channel: Option<String>,
        to: Option<String>,
    },
}

pub enum SessionTarget {
    Main,       // Inject into main conversation
    Isolated,   // Throwaway session (default)
    Named(String),  // Persistent named session
}
```

### 3.3 Job Structure

```rust
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub delete_after_run: bool,
    pub schedule: CronSchedule,
    pub payload: CronPayload,
    pub session_target: SessionTarget,
    pub state: CronJobState,
    pub sandbox: CronSandboxConfig,
    pub system: bool,  // Hidden from normal UI
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

pub struct CronJobState {
    pub next_run_at_ms: Option<u64>,
    pub running_at_ms: Option<u64>,
    pub last_run_at_ms: Option<u64>,
    pub last_status: Option<RunStatus>,
    pub last_error: Option<String>,
    pub last_duration_ms: Option<u64>,
}

pub struct CronSandboxConfig {
    pub enabled: bool,        // Default: true
    pub image: Option<String>, // Override sandbox image
}
```

### 3.4 Store Trait

```rust
pub trait CronStore: Send + Sync {
    async fn load_jobs(&self) -> Result<Vec<CronJob>>;
    async fn save_job(&self, job: &CronJob) -> Result<()>;
    async fn delete_job(&self, id: &str) -> Result<()>;
    async fn update_job(&self, job: &CronJob) -> Result<()>;
    async fn append_run(&self, job_id: &str, run: &CronRunRecord) -> Result<()>;
    async fn get_runs(&self, job_id: &str, limit: usize) -> Result<Vec<CronRunRecord>>;
}
```

### 3.5 Store Implementations

- **SQLite Store** - Persistent, production use
- **File Store** - JSON file based
- **Memory Store** - Testing/ephemeral

---

## 4. MEMORY SYSTEM

### 4.1 Memory Trait

```rust
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> Result<()>;
    async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>>;
    async fn list(&self, category: Option<&MemoryCategory>, session_id: Option<&str>) -> Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> Result<bool>;
    async fn count(&self) -> Result<usize>;
    async fn health_check(&self) -> bool;
}
```

### 4.2 Memory Categories

```rust
pub enum MemoryCategory {
    Core,          // Long-term facts, preferences, decisions
    Daily,         // Daily session logs
    Conversation,  // Conversation context
    Custom(String),
}
```

### 4.3 Memory Store Trait (Advanced)

```rust
pub trait MemoryStore: Send + Sync {
    // Files
    async fn upsert_file(&self, file: &FileRow) -> Result<()>;
    async fn get_file(&self, path: &str) -> Result<Option<FileRow>>;
    async fn delete_file(&self, path: &str) -> Result<()>;
    async fn list_files(&self) -> Result<Vec<FileRow>>;

    // Chunks
    async fn upsert_chunks(&self, chunks: &[ChunkRow]) -> Result<()>;
    async fn get_chunks_for_file(&self, path: &str) -> Result<Vec<ChunkRow>>;
    async fn delete_chunks_for_file(&self, path: &str) -> Result<()>;

    // Embedding Cache
    async fn get_cached_embedding(&self, provider: &str, model: &str, hash: &str) -> Result<Option<Vec<f32>>>;
    async fn put_cached_embedding(&self, provider: &str, model: &str, key: &str, hash: &str, embedding: &[f32]) -> Result<()>;

    // Search
    async fn vector_search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
    async fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
}
```

### 4.4 Backend Types

| Backend | Description | Features |
|---------|-------------|----------|
| SQLite | Primary backend | Vector search, hybrid search, embeddings |
| Lucid | External CLI bridge | Sync with local lucid-memory CLI |
| Markdown | Human-readable files | Simple, no dependencies |
| None | Disabled | No persistence |

### 4.5 Vector Operations

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32;
pub fn vec_to_bytes(v: &[f32]) -> Vec<u8>;
pub fn bytes_to_vec(bytes: &[u8]) -> Vec<f32>;

pub fn hybrid_merge(
    vector_results: &[(String, f32)],
    keyword_results: &[(String, f32)],
    vector_weight: f32,
    keyword_weight: f32,
    limit: usize,
) -> Vec<ScoredResult>;
```

---

## 5. AGENT/PROVIDER SYSTEM

### 5.1 Provider Trait

```rust
pub trait Provider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;
    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload;

    async fn simple_chat(&self, message: &str, model: &str, temperature: f64) -> Result<String>;
    async fn chat_with_system(&self, system: Option<&str>, message: &str, model: &str, temperature: f64) -> Result<String>;
    async fn chat_with_history(&self, messages: &[ChatMessage], model: &str, temperature: f64) -> Result<String>;
    async fn chat(&self, request: ChatRequest<'_>, model: &str, temperature: f64) -> Result<ChatResponse>;

    fn supports_native_tools(&self) -> bool;
    fn supports_streaming(&self) -> bool;

    async fn chat_with_tools(&self, messages: &[ChatMessage], tools: &[serde_json::Value], model: &str, temperature: f64) -> Result<ChatResponse>;
    async fn warmup(&self) -> Result<()>;

    // Streaming
    fn stream_chat_with_system(&self, system: Option<&str>, message: &str, model: &str, temperature: f64, options: StreamOptions) -> BoxStream<'static, StreamResult<StreamChunk>>;
    fn stream_chat_with_history(&self, messages: &[ChatMessage], model: &str, temperature: f64, options: StreamOptions) -> BoxStream<'static, StreamResult<StreamChunk>>;
}
```

### 5.2 Provider Capabilities

```rust
pub struct ProviderCapabilities {
    pub native_tool_calling: bool,
}

pub enum ToolsPayload {
    Gemini { function_declarations: Vec<serde_json::Value> },
    Anthropic { tools: Vec<serde_json::Value> },
    OpenAI { tools: Vec<serde_json::Value> },
    PromptGuided { instructions: String },
}
```

### 5.3 Supported Providers

| Provider | Native Tools | Streaming | Notes |
|----------|-------------|-----------|-------|
| OpenRouter | Yes | Yes | Primary router |
| Anthropic | Yes | Yes | Claude models |
| OpenAI | Yes | Yes | GPT models |
| Ollama | Yes | Yes | Local models |
| Gemini | Yes | Yes | Google models |
| Groq | Yes | Yes | Fast inference |
| Mistral | Yes | Yes | |
| xAI/Grok | Yes | Yes | |
| DeepSeek | Yes | Yes | |
| Together AI | Yes | Yes | |
| Fireworks AI | Yes | Yes | |
| Perplexity | Yes | Yes | |
| Cohere | Yes | Yes | |
| GitHub Copilot | Limited | Yes | OAuth flow |
| LM Studio | Yes | Yes | Local |
| NVIDIA NIM | Yes | Yes | |
| Custom URL | Varies | Varies | BYO endpoint |

### 5.4 Resilient Provider Chain

```rust
pub struct ReliabilityConfig {
    pub provider_retries: u32,
    pub provider_backoff_ms: u64,
    pub fallback_providers: Vec<String>,
    pub api_keys: Vec<String>,
    pub model_fallbacks: HashMap<String, String>,
    pub channel_initial_backoff_secs: u64,
    pub channel_max_backoff_secs: u64,
    pub scheduler_poll_secs: u64,
    pub scheduler_retries: u32,
}

pub fn create_resilient_provider(
    primary: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    reliability: &ReliabilityConfig,
) -> Result<Box<dyn Provider>>;

pub fn create_routed_provider(
    primary: &str,
    reliability: &ReliabilityConfig,
    model_routes: &[ModelRouteConfig],
    default_model: &str,
) -> Result<Box<dyn Provider>>;
```

### 5.5 Message Types

```rust
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub struct ChatResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

pub struct StreamChunk {
    pub delta: String,
    pub is_final: bool,
    pub token_count: usize,
}
```

---

## 6. SECURITY/SANDBOX SYSTEM

### 6.1 Autonomy Levels

```rust
pub enum AutonomyLevel {
    ReadOnly,    // Can observe but not act
    Supervised,  // Acts but requires approval for risky operations (default)
    Full,        // Autonomous within policy bounds
}
```

### 6.2 Security Policy

```rust
pub struct SecurityPolicy {
    pub autonomy: AutonomyLevel,
    pub workspace_dir: PathBuf,
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub max_actions_per_hour: u32,
    pub max_cost_per_day_cents: u32,
    pub require_approval_for_medium_risk: bool,
    pub block_high_risk_commands: bool,
    pub tracker: ActionTracker,
}
```

### 6.3 Command Risk Classification

```rust
pub enum CommandRiskLevel {
    Low,     // Read-only, safe
    Medium,  // State-changing (git commit, npm install, touch, mkdir)
    High,    // Destructive (rm, sudo, curl, wget, ssh, chmod, chown)
}
```

### 6.4 Runtime Adapter

```rust
pub trait RuntimeAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn has_shell_access(&self) -> bool;
    fn has_filesystem_access(&self) -> bool;
    fn storage_path(&self) -> PathBuf;
    fn supports_long_running(&self) -> bool;
    fn memory_budget(&self) -> u64;
    fn build_shell_command(&self, command: &str, workspace_dir: &Path) -> Result<tokio::process::Command>;
}
```

**Runtime Implementations:**
- **Native** - Direct execution
- **Docker** - Containerized execution
- **WASM** - Sandboxed WebAssembly

### 6.5 Audit Logging

```rust
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_id: String,
    pub event_type: AuditEventType,
    pub actor: Option<Actor>,
    pub action: Option<Action>,
    pub result: Option<ExecutionResult>,
    pub security: SecurityContext,
}

pub enum AuditEventType {
    CommandExecution,
    FileAccess,
    ConfigChange,
    AuthSuccess,
    AuthFailure,
    PolicyViolation,
    SecurityEvent,
}
```

---

## 7. AUTHENTICATION SYSTEM

### 7.1 Credential Store

```rust
pub struct CredentialStore {
    pool: SqlitePool,
    setup_complete: AtomicBool,
    auth_disabled: AtomicBool,
}

impl CredentialStore {
    // Password
    async fn set_initial_password(&self, password: &str) -> Result<()>;
    async fn verify_password(&self, password: &str) -> Result<bool>;
    async fn change_password(&self, current: &str, new: &str) -> Result<()>;

    // Sessions
    async fn create_session(&self) -> Result<String>;  // 30-day expiry
    async fn validate_session(&self, token: &str) -> Result<bool>;
    async fn delete_session(&self, token: &str) -> Result<()>;

    // API Keys
    async fn create_api_key(&self, label: &str, scopes: Option<&[String]>) -> Result<(i64, String)>;
    async fn verify_api_key(&self, raw_key: &str) -> Result<Option<ApiKeyVerification>>;
    async fn revoke_api_key(&self, key_id: i64) -> Result<()>;
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyEntry>>;

    // Passkeys (WebAuthn)
    async fn store_passkey(&self, credential_id: &[u8], name: &str, data: &[u8]) -> Result<i64>;
    async fn list_passkeys(&self) -> Result<Vec<PasskeyEntry>>;
    async fn remove_passkey(&self, id: i64) -> Result<()>;

    // Environment Variables
    async fn set_env_var(&self, key: &str, value: &str) -> Result<i64>;
    async fn list_env_vars(&self) -> Result<Vec<EnvVarEntry>>;
    async fn delete_env_var(&self, id: i64) -> Result<Option<String>>;
}
```

### 7.2 Auth Middleware Flow

1. Check if credential store has setup complete
2. If API key provided, verify and extract scopes
3. If password provided, verify against Argon2 hash
4. If no auth configured and local connection, allow access
5. If remote and no auth, require onboarding

### 7.3 WebAuthn/Passkey Support

- Stores credential IDs and passkey data blobs
- Supports multiple passkeys per user
- Enables passwordless authentication

---

## 8. CONFIGURATION SCHEMA

### 8.1 Top-Level Structure

```rust
pub struct Config {
    pub workspace_dir: PathBuf,
    pub config_path: PathBuf,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,

    pub observability: ObservabilityConfig,
    pub autonomy: AutonomyConfig,
    pub runtime: RuntimeConfig,
    pub reliability: ReliabilityConfig,
    pub scheduler: SchedulerConfig,
    pub agent: AgentConfig,
    pub model_routes: Vec<ModelRouteConfig>,
    pub heartbeat: HeartbeatConfig,
    pub cron: CronConfig,
    pub channels_config: ChannelsConfig,
    pub memory: MemoryConfig,
    pub tunnel: TunnelConfig,
    pub gateway: GatewayConfig,
    pub composio: ComposioConfig,
    pub secrets: SecretsConfig,
    pub browser: BrowserConfig,
    pub http_request: HttpRequestConfig,
    pub identity: IdentityConfig,
    pub cost: CostConfig,
    pub peripherals: PeripheralsConfig,
    pub agents: HashMap<String, DelegateAgentConfig>,
    pub hardware: HardwareConfig,
}
```

### 8.2 Gateway Configuration

```rust
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub require_pairing: bool,
    pub paired_tokens: Vec<String>,
    pub allow_public_bind: bool,
    pub pair_rate_limit_per_minute: u32,
    pub webhook_rate_limit_per_minute: u32,
    pub idempotency_ttl_secs: u64,
}
```

---

## 9. INTEGRATION PATTERNS

### 9.1 Channel-to-Agent Flow

1. Channel receives inbound message
2. ChannelEventSink.dispatch_to_chat() called
3. Gateway routes to main session
4. Agent processes with tools
5. Response broadcast via WebSocket
6. Channel outbound sends reply

### 9.2 Cron-to-Agent Flow

1. Scheduler polls due jobs
2. CronPayload::AgentTurn extracted
3. Isolated or named session created
4. Agent executes with optional sandbox
5. Result delivered to channel if configured
6. Run history recorded

### 9.3 Memory Integration

1. Agent requests context
2. Memory loader fetches relevant entries
3. Hybrid search (vector + keyword) performed
4. Context injected into system prompt
5. New memories stored after response

### 9.4 Provider Chain

1. Primary provider selected
2. Request with retry/backoff
3. On failure, fallback provider tried
4. Model fallbacks applied if needed
5. Response returned or error raised

---

## 10. MAESTRO INTEGRATION STATUS

### Already Implemented (maestro-core)

| Capability | Module | Status |
|------------|--------|--------|
| Channel Trait | `crates/core/src/channel/mod.rs` | ✅ Done |
| Telegram Channel | `crates/core/src/channel/telegram.rs` | ✅ Done |
| Track Integration | `crates/core/src/integration/mod.rs` | ✅ Done |
| Security Policy | `crates/core/src/security/` | ✅ Done |
| Memory Traits | `crates/core/src/memory/` | ✅ Done |
| Provider Traits | `crates/core/src/traits.rs` | ✅ Done |

### Already Implemented (maestro-gateway)

| Capability | Module | Status |
|------------|--------|--------|
| Frame Protocol | `crates/gateway/src/protocol.rs` | ✅ Done |
| Rate Limiting | `crates/gateway/src/rate_limit.rs` | ✅ Done |
| WebSocket Handler | `crates/gateway/src/ws.rs` | ✅ Done |
| SSE Streaming | `crates/gateway/src/sse.rs` | ✅ Done |
| REST Routes | `crates/gateway/src/routes.rs` | ✅ Done |
| Dashboard API | Gateway routes | ✅ Done |

### Pending Implementation

| Capability | Priority | Notes |
|------------|----------|-------|
| Cron Job Store | High | SQLite-based job persistence |
| Cron Scheduler Service | High | Tick-based polling |
| Channel Registry | Medium | Dynamic channel loading |
| Multi-account Channels | Medium | Multiple accounts per platform |
| Credential Store | Medium | Password/API key/passkey storage |
| Provider Router | Medium | Fallback chain with retries |
| Audit Logger | Low | Security event logging |
| Docker Runtime | Low | Containerized execution |
| WASM Runtime | Low | Sandboxed execution |
