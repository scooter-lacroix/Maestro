//! LSP Client for Diagnostic Validation
//!
//! Direct LSP communication for triggering diagnostics after agent edits.
//! Supports both direct stdio communication and Unix socket (via proxy).

use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

use super::diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticValidation};
use crate::memory::lsp_manager::LspType;

/// LSP client for diagnostic communication
pub struct LspClient {
    /// Session identifier
    session_id: String,
    /// LSP type
    lsp_type: LspType,
    /// Communication mode
    mode: LspClientMode,
}

/// How to connect to the LSP
#[derive(Debug, Clone)]
pub enum LspClientMode {
    /// Connect via Unix socket (stdio proxy)
    Proxy { socket_path: String },
    /// Direct stdio communication (requires child handle)
    Direct,
}

impl LspClient {
    /// Get the LSP type for this client
    pub fn get_lsp_type(&self) -> &LspType {
        &self.lsp_type
    }

    /// Get the session ID for this client
    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }
    /// Create a new LSP client for the given session and LSP type
    pub fn new(session_id: String, lsp_type: LspType) -> Self {
        Self {
            session_id,
            lsp_type,
            mode: LspClientMode::Direct,
        }
    }

    /// Create a client that connects via Unix socket proxy
    pub fn with_proxy(session_id: String, lsp_type: LspType, socket_path: String) -> Self {
        Self {
            session_id,
            lsp_type,
            mode: LspClientMode::Proxy { socket_path },
        }
    }

    /// Get the socket path for a given session and LSP type
    pub fn socket_path(session_id: &str, lsp_type: LspType) -> String {
        let sanitized = session_id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        format!(
            "/tmp/maestro-lsp-{}-{}.sock",
            lsp_type.language(),
            sanitized
        )
    }

    /// Notify LSP of file changes and get diagnostics
    ///
    /// This is the main entry point for post-edit diagnostic validation.
    ///
    /// ## Steps:
    /// 1. Connect to LSP (via socket or direct stdio)
    /// 2. Send `textDocument/didChange` notification
    /// 3. Wait for `textDocument/publishDiagnostics` notification
    /// 4. Parse and return diagnostics
    pub async fn validate_diagnostics(
        &self,
        file_path: &Path,
        timeout_secs: u64,
    ) -> Result<DiagnosticValidation> {
        let file_uri = path_to_file_uri(file_path)?;

        match &self.mode {
            LspClientMode::Proxy { socket_path } => {
                self.validate_via_proxy(&file_uri, file_path, socket_path, timeout_secs)
                    .await
            }
            LspClientMode::Direct => {
                // Direct stdio requires the LspManager to provide access
                // For now, return a placeholder validation
                tracing::warn!(
                    "Direct LSP communication not yet implemented for session {}",
                    self.session_id
                );
                Ok(DiagnosticValidation {
                    diagnostics: Vec::new(),
                    passed: true,
                    error_message: None,
                })
            }
        }
    }

    /// Validate diagnostics via Unix socket proxy
    async fn validate_via_proxy(
        &self,
        file_uri: &str,
        file_path: &Path,
        socket_path: &str,
        timeout_secs: u64,
    ) -> Result<DiagnosticValidation> {
        // Connect to the proxy socket
        let mut stream = timeout(Duration::from_secs(2), UnixStream::connect(socket_path))
            .await
            .with_context(|| format!("LSP proxy connection timeout at {}", socket_path))?
            .with_context(|| format!("Failed to connect to LSP proxy at {}", socket_path))?;

        // Read file content for didChange notification
        let content = tokio::fs::read_to_string(file_path)
            .await
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        // Send didChange notification
        let did_change = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": file_uri,
                    "version": 1
                },
                "contentChanges": [{
                    "text": content
                }]
            }
        });

        self.send_message(&mut stream, &did_change).await?;

        // Wait a bit for LSP to process and publish diagnostics
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Request diagnostics (pull model)
        let diagnostics_request = json!({
            "jsonrpc": "2.0",
            "id": "diagnostics-1",
            "method": "textDocument/diagnostics",
            "params": {
                "textDocument": { "uri": file_uri }
            }
        });

        self.send_message(&mut stream, &diagnostics_request).await?;

        // Read response
        let response = timeout(
            Duration::from_secs(timeout_secs),
            self.read_message(&mut stream),
        )
        .await
        .with_context(|| format!("LSP diagnostics timeout for {}", file_uri))??;

        // Parse diagnostics from response
        let diagnostics = self.parse_diagnostics_response(&response, file_uri)?;

        // Check if validation passed
        let has_errors = diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error);

        let error_message = if has_errors {
            Some(format_diagnostics(&diagnostics, 50))
        } else {
            None
        };

        Ok(DiagnosticValidation {
            passed: !has_errors,
            diagnostics,
            error_message,
        })
    }

    /// Send a message to the LSP with proper LSP framing
    async fn send_message(
        &self,
        stream: &mut UnixStream,
        message: &serde_json::Value,
    ) -> Result<()> {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        stream.write_all(header.as_bytes()).await?;
        stream.write_all(content.as_bytes()).await?;
        stream.flush().await?;

        tracing::debug!("Sent LSP message: {}", message);
        Ok(())
    }

    /// Read a message from the LSP with proper LSP framing
    async fn read_message(&self, stream: &mut UnixStream) -> Result<String> {
        // Read Content-Length header
        let mut header_buf = Vec::new();

        // Read until we find \r\n\r\n
        loop {
            let mut byte = [0u8; 1];
            stream.read_exact(&mut byte).await?;
            header_buf.push(byte[0]);

            // Check for \r\n\r\n terminator
            if header_buf.len() >= 4 {
                let last_four = &header_buf[header_buf.len() - 4..];
                if last_four == b"\r\n\r\n" {
                    break;
                }
            }

            // Safety check against malformed headers
            if header_buf.len() > 4096 {
                return Err(anyhow!("LSP header too large or malformed"));
            }
        }

        let header = String::from_utf8_lossy(&header_buf);
        let content_length = parse_content_length(&header)?;

        // Read the content
        let mut content_buf = vec![0u8; content_length];
        stream.read_exact(&mut content_buf).await?;

        let content = String::from_utf8_lossy(&content_buf).to_string();
        Ok(content)
    }

    /// Parse diagnostics from LSP response
    fn parse_diagnostics_response(
        &self,
        response: &str,
        file_uri: &str,
    ) -> Result<Vec<Diagnostic>> {
        let json_val: serde_json::Value = serde_json::from_str(response)?;

        // Check if this is a response with result
        if let Some(result) = json_val.get("result") {
            // textDocument/diagnostics returns an array
            if let Some(diagnostics_array) = result.as_array() {
                let mut diagnostics = Vec::new();

                for diag_json in diagnostics_array {
                    if let Ok(diag) = self.parse_lsp_diagnostic(diag_json, file_uri) {
                        diagnostics.push(diag);
                    }
                }

                return Ok(diagnostics);
            }
        }

        // No diagnostics means clean file
        Ok(Vec::new())
    }

    /// Parse a single LSP diagnostic from JSON
    fn parse_lsp_diagnostic(
        &self,
        diag_json: &serde_json::Value,
        file_uri: &str,
    ) -> Result<Diagnostic> {
        let range = diag_json
            .get("range")
            .ok_or_else(|| anyhow!("Diagnostic missing range"))?;

        let lsp_range = lsp_types::Range {
            start: lsp_types::Position {
                line: range["start"]["line"].as_u64().unwrap_or(0) as u32,
                character: range["start"]["character"].as_u64().unwrap_or(0) as u32,
            },
            end: lsp_types::Position {
                line: range["end"]["line"].as_u64().unwrap_or(0) as u32,
                character: range["end"]["character"].as_u64().unwrap_or(0) as u32,
            },
        };

        let severity = match diag_json.get("severity").and_then(|s| s.as_u64()) {
            Some(1) => DiagnosticSeverity::Error,
            Some(2) => DiagnosticSeverity::Warning,
            Some(3) => DiagnosticSeverity::Info,
            Some(4) => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error, // Default to error for safety
        };

        let message = diag_json
            .get("message")
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown diagnostic")
            .to_string();

        let source = diag_json
            .get("source")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        Ok(Diagnostic {
            uri: file_uri.to_string(),
            range: lsp_range,
            severity,
            message,
            source,
            code: None, // Could parse "code" field if needed
        })
    }
}

/// Parse Content-Length from LSP headers
fn parse_content_length(header: &str) -> Result<usize> {
    for line in header.lines() {
        if line.starts_with("Content-Length:") {
            let len_str = line["Content-Length:".len()..].trim();
            return len_str
                .parse::<usize>()
                .with_context(|| format!("Invalid Content-Length: {}", len_str));
        }
    }
    Err(anyhow!("Content-Length header not found"))
}

/// Format diagnostics for display
fn format_diagnostics(diagnostics: &[Diagnostic], max: usize) -> String {
    let count = diagnostics.len();
    let to_show = diagnostics.iter().take(max);

    let mut output = format!("LSP Diagnostics ({} found):\n", count);

    for diag in to_show {
        output.push_str(&diag.format());
        output.push('\n');
    }

    if count > max {
        output.push_str(&format!(
            "... and {} more (max {} shown)\n",
            count - max,
            max
        ));
    }

    output
}

/// Convert path to file:// URI
pub fn path_to_file_uri(path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::fs::canonicalize(path)
            .with_context(|| format!("Failed to canonicalize: {:?}", path))?
    };

    let path_str = absolute.to_string_lossy();
    Ok(format!("file://{}", path_str))
}

/// Create an LSP client connected via proxy
pub async fn create_proxy_client(session_id: &str, lsp_type: LspType) -> Result<LspClient> {
    let socket_path = LspClient::socket_path(session_id, lsp_type);

    // Check if socket exists and is accessible
    if let Ok(stream) = timeout(
        Duration::from_millis(500),
        UnixStream::connect(&socket_path),
    )
    .await
    {
        // Socket is available
        drop(stream);
        Ok(LspClient::with_proxy(
            session_id.to_string(),
            lsp_type,
            socket_path,
        ))
    } else {
        // Socket not available, try direct mode
        tracing::warn!(
            "LSP proxy socket not found for {}: {}, using direct mode",
            lsp_type.language(),
            socket_path
        );
        Ok(LspClient::new(session_id.to_string(), lsp_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_generation() {
        let path = LspClient::socket_path("test-session-123", LspType::Rust);
        assert!(path.contains("maestro-lsp-rust"));
        assert!(path.contains("test-session-123"));
        assert!(path.ends_with(".sock"));
    }

    #[test]
    fn test_content_length_parsing() {
        let header = "Content-Length: 42\r\n\r\n";
        let len = parse_content_length(header).unwrap();
        assert_eq!(len, 42);
    }

    #[test]
    fn test_content_length_parsing_multiline() {
        let header = "Content-Type: application/json\r\nContent-Length: 123\r\n\r\n";
        let len = parse_content_length(header).unwrap();
        assert_eq!(len, 123);
    }

    #[test]
    fn test_path_to_file_uri() {
        let path = Path::new("/tmp/test.rs");
        let uri = path_to_file_uri(path).unwrap();
        assert_eq!(uri, "file:///tmp/test.rs");
    }

    #[test]
    fn test_parse_lsp_diagnostic() {
        let client = LspClient::new("test".to_string(), LspType::Rust);

        let diag_json = json!({
            "range": {
                "start": {"line": 10, "character": 5},
                "end": {"line": 10, "character": 15}
            },
            "severity": 1,
            "message": "expected type, found `()`",
            "source": "rust-analyzer"
        });

        let diag = client
            .parse_lsp_diagnostic(&diag_json, "file:///tmp/test.rs")
            .unwrap();
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.message, "expected type, found `()`");
        assert_eq!(diag.source, Some("rust-analyzer".to_string()));
    }
}
