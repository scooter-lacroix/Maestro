// MCP Persistence System Design

## Requirements Analysis

Current state:
- `McpManager` stores server configs and tokens in memory only
- No persistence across gateway restarts
- No workspace integration
- Configs are registered at runtime but not saved

Proposed minimal design:

### 1. Workspace MCP Configuration Structure

Create `workspace/mcp/servers.toml` with format:

```toml
# MCP Server Configurations
[[servers]]
name = "github"
url = "http://localhost:8080"
requires_auth = true

[[servers]]
name = "git-scm"
command = "git-mcp-server"
args = ["--port", "8081"]
requires_auth = false

# Stored authentication tokens
[tokens]
github = "secret-token"
```

### 2. MCP Persistence Manager

```rust
/// Manages MCP server persistence to workspace
pub struct McpPersistenceManager {
    /// Path to workspace MCP directory
    workspace_path: PathBuf,
    /// Path to servers.toml file
    config_path: PathBuf,
}

impl McpPersistenceManager {
    /// Create new persistence manager for workspace
    pub fn new(workspace_path: &Path) -> Self {
        let mcp_dir = workspace_path.join("mcp");
        let config_path = mcp_dir.join("servers.toml");
        Self {
            workspace_path: mcp_dir,
            config_path,
        }
    }

    /// Load MCP servers from workspace config
    pub fn load_servers(&self) -> Result<Vec<McpServerConfig>, ConfigError> {
        if !self.config_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.config_path)?;
        let toml: TomlConfig = toml::from_str(&content)?;
        Ok(toml.servers)
    }

    /// Load stored auth tokens from workspace config
    pub fn load_tokens(&self) -> Result<HashMap<String, AuthToken>, ConfigError> {
        if !self.config_path.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(&self.config_path)?;
        let toml: TomlConfig = toml::from_str(&content)?;
        Ok(toml.tokens.unwrap_or_default())
    }

    /// Save MCP servers to workspace config
    pub fn save_servers(&self, servers: &[McpServerConfig]) -> Result<() , ConfigError> {
        // Ensure directory exists
        std::fs::create_dir_all(&self.workspace_path)?;

        // Load existing config to preserve tokens
        let mut toml = if self.config_path.exists() {
            toml::from_str(&std::fs::read_to_string(&self.config_path)?)?
        } else {
            TomlConfig::default()
        };

        toml.servers = servers.to_vec();
        
        let content = toml::to_string(&toml)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Save auth token for a server
    pub fn save_token(&self, server_name: &str, token: &AuthToken) -> Result<() , ConfigError> {
        // Load existing config
        let mut toml = if self.config_path.exists() {
            toml::from_str(&std::fs::read_to_string(&self.config_path)?)?
        } else {
            TomlConfig::default()
        };

        if toml.tokens.is_none() {
            toml.tokens = Some(HashMap::new());
        }
        
        toml.tokens.as_mut().unwrap().insert(server_name.to_string(), token.clone());
        
        let content = toml::to_string(&toml)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Remove auth token for a server
    pub fn remove_token(&self, server_name: &str) -> Result<() , ConfigError> {
        // Load existing config
        let mut toml = if self.config_path.exists() {
            toml::from_str(&std::fs::read_to_string(&self.config_path)?)?
        } else {
            TomlConfig::default()
        };

        if let Some(tokens) = &mut toml.tokens {
            tokens.remove(server_name);
        }
        
        let content = toml::to_string(&toml)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }
}

/// TOML configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlConfig {
    #[serde(default)]
    servers: Vec<McpServerConfig>,
    #[serde(default)]
    tokens: Option<HashMap<String, AuthToken>>,
}

impl Default for TomlConfig {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            tokens: None,
        }
    }
}
```

### 3. Enhanced MCP Manager Integration

```rust
/// Enhanced MCP manager with persistence
pub struct EnhancedMcpManager {
    /// Inner MCP manager (from maestro-core)
    inner: McpManager,
    /// Persistence manager for workspace storage
    persistence: McpPersistenceManager,
}

impl EnhancedMcpManager {
    /// Create new enhanced MCP manager with workspace path
    pub fn new(workspace_path: &Path) -> Self {
        Self {
            inner: McpManager::new(),
            persistence: McpPersistenceManager::new(workspace_path),
        }
    }

    /// Initialize from workspace - load configs and tokens
    pub async fn initialize_from_workspace(&self) -> Result<() , ConfigError> {
        // Load servers from workspace
        let servers = self.persistence.load_servers()?;
        for server in servers {
            self.inner.register_server(server).await;
        }

        // Load tokens from workspace
        let tokens = self.persistence.load_tokens()?;
        for (server_name, token) in tokens {
            self.inner.set_auth_token(server_name, token).await;
        }

        Ok(())
    }

    /// Register server with persistence
    pub async fn register_server(&self, config: McpServerConfig) -> Result<() , ConfigError> {
        self.inner.register_server(config.clone()).await;
        self.save_servers().await?;
        Ok(())
    }

    /// Update server with persistence
    pub async fn update_server(&self, config: McpServerConfig) -> Result<bool, ConfigError> {
        let updated = self.inner.update_server(config.clone()).await;
        if updated {
            self.save_servers().await?;
        }
        Ok(updated)
    }

    /// Remove server with persistence
    pub async fn remove_server(&self, server_name: &str) -> Result<bool, ConfigError> {
        let removed = self.inner.remove_server(server_name).await;
        if removed {
            self.save_servers().await?;
            self.persistence.remove_token(server_name)?;
        }
        Ok(removed)
    }

    /// Set auth token with persistence
    pub async fn set_auth_token(&self, server_name: &str, token: AuthToken) -> Result<bool, ConfigError> {
        let success = self.inner.set_auth_token(server_name, token.clone()).await;
        if success {
            self.persistence.save_token(server_name, &token)?;
        }
        Ok(success)
    }

    /// Clear auth token with persistence
    pub async fn clear_auth_token(&self, server_name: &str) -> Result<bool, ConfigError> {
        let success = self.inner.get_auth_token(server_name).await.is_some();
        if success {
            self.inner.clear_auth_token_for_server(server_name).await;
            self.persistence.remove_token(server_name)?;
        }
        Ok(success)
    }

    /// Save current servers to workspace
    async fn save_servers(&self) -> Result<() , ConfigError> {
        let servers = self.inner.registered_server_configs().await;
        self.persistence.save_servers(&servers)?;
        Ok(())
    }
}
```

### 4. GatewayState Integration

Replace the simple `mcp_manager: McpManager` with:

```rust
pub struct GatewayState {
    // ... existing fields ...
    
    /// Enhanced MCP manager with persistence
    mcp_manager: EnhancedMcpManager,
    
    // ... existing fields ...
}

impl GatewayState {
    /// Create new gateway state with workspace path
    pub fn with_workspace(workspace_path: &Path) -> Self {
        let mcp_manager = EnhancedMcpManager::new(workspace_path);
        // Initialize MCP from workspace
        let _ = mcp_manager.initialize_from_workspace().await;
        
        Self {
            // ... existing fields ...
            mcp_manager,
            // ... existing fields ...
        }
    }
}
```

### 5. API Endpoints

Add to agent types:

```rust
/// Request to list MCP servers with persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpListRequest {
    /// Include auth token status
    #[serde(default)]
    pub include_tokens: bool,
}

/// Request to save MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSaveRequest {
    pub config: McpServerConfig,
}

/// Request to update MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpUpdateRequest {
    pub config: McpServerConfig,
}

/// Request to remove MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRemoveRequest {
    pub server_name: String,
}

/// Request to set MCP auth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTokenSetRequest {
    pub server_name: String,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<GatewayAuthTokenType>,
}

/// Request to clear MCP auth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTokenClearRequest {
    pub server_name: String,
}
```

This design provides:
- Workspace-based MCP server persistence
- Auth token storage with MCP servers
- Automatic hydration on gateway startup
- Local storage without new services
- Backward compatibility with existing MCP manager API
- Minimal changes to existing code structure