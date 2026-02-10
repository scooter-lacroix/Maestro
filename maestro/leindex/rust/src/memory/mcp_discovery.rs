//! System MCP configuration discovery.
//!
//! Consolidates MCP server definitions from common CLI tool config locations
//! (Claude Code, Codex CLI, OpenCode, Amp, etc.) into a single set of server specs.

use crate::memory::models::McpTransport;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiscoveredMcpServer {
    pub name: String,
    pub transport: McpTransport,
    pub command: String,
    pub args: Vec<String>,
    pub env: serde_json::Value,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub source: String,
}

pub fn discover_system_mcp_servers() -> Vec<DiscoveredMcpServer> {
    let mut out: HashMap<String, DiscoveredMcpServer> = HashMap::new();

    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let candidates: Vec<(String, PathBuf)> = vec![
        (
            "claude_settings".to_string(),
            home.join(".claude/settings.json"),
        ),
        (
            "claude_config".to_string(),
            home.join(".claude/.claude.json"),
        ),
        (
            "claude_user_mcp".to_string(),
            home.join(".claude/.mcp.json"),
        ),
        (
            "claude_code_fallback".to_string(),
            home.join(".config/claude-code/mcp.json"),
        ),
        ("amp".to_string(), home.join(".config/amp/settings.json")),
        (
            "opencode".to_string(),
            home.join(".config/opencode/opencode.json"),
        ),
        ("cursor".to_string(), home.join(".cursor/mcp.json")),
        (
            "cursor_alt".to_string(),
            home.join(".config/cursor/mcp.json"),
        ),
        ("codex".to_string(), home.join(".codex/config.toml")),
        ("qwen".to_string(), home.join(".qwen/settings.json")),
        ("gemini".to_string(), home.join(".gemini/settings.json")),
    ];

    for (label, path) in candidates {
        if !path.exists() {
            continue;
        }

        let discovered = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            discover_from_toml_file(&label, &path).unwrap_or_default()
        } else {
            discover_from_json_file(&label, &path).unwrap_or_default()
        };

        for server in discovered {
            out.entry(server.name.clone()).or_insert(server);
        }
    }

    // Built-in fallbacks: add well-known local servers when installed even if no config exists.
    // This keeps the pool portable across fresh installs and avoids relying on user-specific
    // config files for common utilities.
    add_if_missing(
        &mut out,
        detect_stdio_server(
            "agent-browser",
            "agent-browser",
            vec!["server".to_string()],
            None,
            "builtin",
        ),
    );

    add_if_missing(
        &mut out,
        detect_stdio_server(
            "brave-search",
            "npx",
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search".to_string(),
            ],
            None,
            "builtin",
        ),
    );

    let mut list: Vec<_> = out.into_values().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

fn add_if_missing(
    map: &mut HashMap<String, DiscoveredMcpServer>,
    candidate: Option<DiscoveredMcpServer>,
) {
    if let Some(server) = candidate {
        map.entry(server.name.clone()).or_insert(server);
    }
}

fn detect_stdio_server(
    name: &str,
    command: &str,
    args: Vec<String>,
    cwd: Option<String>,
    source: &str,
) -> Option<DiscoveredMcpServer> {
    // Only add if the command is resolvable on PATH to avoid broken defaults.
    if !command_exists(command) {
        return None;
    }

    Some(DiscoveredMcpServer {
        name: name.to_string(),
        transport: McpTransport::Stdio,
        command: command.to_string(),
        args,
        env: serde_json::json!({}),
        cwd,
        url: None,
        headers: None,
        source: source.to_string(),
    })
}

fn command_exists(cmd: &str) -> bool {
    // If cmd includes a path separator, treat it as a path.
    if cmd.contains(std::path::MAIN_SEPARATOR) {
        return std::path::Path::new(cmd).exists();
    }

    // Otherwise, search PATH
    if let Some(paths) = std::env::var_os("PATH") {
        for p in std::env::split_paths(&paths) {
            let candidate = p.join(cmd);
            if candidate.exists() {
                return true;
            }
        }
    }
    false
}

pub fn discover_from_json_file(label: &str, path: &Path) -> Result<Vec<DiscoveredMcpServer>> {
    let text = std::fs::read_to_string(path)?;
    let root: serde_json::Value = serde_json::from_str(&text)?;

    // Common patterns:
    // - { "mcpServers": { "<name>": { command, args, env, type/http/url/headers } } }
    // - { "amp.mcpServers": { ... } }
    // - OpenCode: { "mcp": { "<name>": { type, command: [..], environment } } }
    let mut servers: Vec<(String, serde_json::Value)> = Vec::new();

    if let Some(map) = root.get("mcpServers").and_then(|v| v.as_object()) {
        servers.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    if let Some(map) = root.get("amp.mcpServers").and_then(|v| v.as_object()) {
        servers.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    if let Some(map) = root.get("mcp").and_then(|v| v.as_object()) {
        servers.extend(map.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    let mut out = Vec::new();
    for (name, cfg) in servers {
        if let Some(server) = parse_json_server(&name, &cfg, label) {
            out.push(server);
        }
    }
    Ok(out)
}

fn parse_json_server(
    name: &str,
    cfg: &serde_json::Value,
    source_label: &str,
) -> Option<DiscoveredMcpServer> {
    // HTTP-style server (Claude Code style)
    if cfg.get("type").and_then(|v| v.as_str()) == Some("http") || cfg.get("url").is_some() {
        let url = cfg
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if url.is_none() {
            return None;
        }

        let headers = cfg.get("headers").cloned();

        return Some(DiscoveredMcpServer {
            name: name.to_string(),
            transport: McpTransport::Http,
            command: "http".to_string(),
            args: Vec::new(),
            env: serde_json::json!({}),
            cwd: None,
            url,
            headers,
            source: source_label.to_string(),
        });
    }

    // OpenCode-style: command is an array
    if let Some(cmd_arr) = cfg.get("command").and_then(|v| v.as_array()) {
        let mut parts: Vec<String> = cmd_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if parts.is_empty() {
            return None;
        }
        let command = parts.remove(0);
        let args = parts;

        let env = cfg
            .get("environment")
            .cloned()
            .or_else(|| cfg.get("env").cloned())
            .unwrap_or_else(|| serde_json::json!({}));

        return Some(DiscoveredMcpServer {
            name: name.to_string(),
            transport: McpTransport::Stdio,
            command,
            args,
            env,
            cwd: cfg
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url: None,
            headers: None,
            source: source_label.to_string(),
        });
    }

    // Standard: command is a string
    let command = cfg.get("command").and_then(|v| v.as_str())?.to_string();
    let args: Vec<String> = cfg
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let env = cfg
        .get("env")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let cwd = cfg
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(DiscoveredMcpServer {
        name: name.to_string(),
        transport: McpTransport::Stdio,
        command,
        args,
        env,
        cwd,
        url: None,
        headers: None,
        source: source_label.to_string(),
    })
}

pub fn discover_from_toml_file(label: &str, path: &Path) -> Result<Vec<DiscoveredMcpServer>> {
    let text = std::fs::read_to_string(path)?;
    let root: toml::Value = toml::from_str(&text)?;

    let Some(servers) = root.get("mcp_servers").and_then(|v| v.as_table()) else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for (name, cfg) in servers {
        if let Some(server) = parse_toml_server(name, cfg, label) {
            out.push(server);
        }
    }
    Ok(out)
}

fn parse_toml_server(
    name: &str,
    cfg: &toml::Value,
    source_label: &str,
) -> Option<DiscoveredMcpServer> {
    let table = cfg.as_table()?;

    if table.get("transport").and_then(|v| v.as_str()) == Some("http") || table.get("url").is_some()
    {
        let url = table
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())?;
        // Headers can be specified as arbitrary top-level keys; keep what we can.
        let mut headers_obj = serde_json::Map::new();
        if let Some(auth) = table.get("Authorization").and_then(|v| v.as_str()) {
            headers_obj.insert(
                "Authorization".to_string(),
                serde_json::Value::String(auth.to_string()),
            );
        }

        return Some(DiscoveredMcpServer {
            name: name.to_string(),
            transport: McpTransport::Http,
            command: "http".to_string(),
            args: Vec::new(),
            env: serde_json::json!({}),
            cwd: None,
            url: Some(url),
            headers: if headers_obj.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(headers_obj))
            },
            source: source_label.to_string(),
        });
    }

    let command = table.get("command")?.as_str()?.to_string();
    let args: Vec<String> = table
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some(DiscoveredMcpServer {
        name: name.to_string(),
        transport: McpTransport::Stdio,
        command,
        args,
        env: serde_json::json!({}),
        cwd: table
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        url: None,
        headers: None,
        source: source_label.to_string(),
    })
}
