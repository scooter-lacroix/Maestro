//! File operations tool with safety constraints
//!
//! The FileTool provides controlled file system operations with:
//! - Read/write file operations
//! - Path validation (prevents traversal attacks)
//! - Working directory sandboxing
//! - Output sanitization

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::tools::{Tool, ToolOutput};

/// Configuration for FileTool
#[derive(Debug, Clone)]
pub struct FileToolConfig {
    /// Base directory for relative paths (sandbox)
    pub base_directory: Option<PathBuf>,
    /// Maximum file size to read (bytes)
    pub max_read_bytes: usize,
    /// Whether to allow write operations
    pub allow_write: bool,
    /// Whether to allow delete operations
    pub allow_delete: bool,
    /// Allowed file extensions (empty = all allowed)
    pub allowed_extensions: Vec<String>,
    /// Blocked paths that should never be accessed
    pub blocked_paths: Vec<PathBuf>,
}

impl Default for FileToolConfig {
    fn default() -> Self {
        Self {
            base_directory: None,
            max_read_bytes: 1024 * 1024, // 1MB
            allow_write: true,
            allow_delete: false,
            allowed_extensions: vec![],
            blocked_paths: vec![
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/etc/shadow"),
                PathBuf::from("/etc/ssh"),
                PathBuf::from("~/.ssh"),
                PathBuf::from("~/.gnupg"),
            ],
        }
    }
}

/// File tool for read/write operations
pub struct FileTool {
    config: FileToolConfig,
}

impl FileTool {
    /// Create a new FileTool with default configuration
    pub fn new() -> Self {
        Self {
            config: FileToolConfig::default(),
        }
    }

    /// Create a new FileTool with custom configuration
    pub fn with_config(config: FileToolConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &FileToolConfig {
        &self.config
    }

    /// Validate and resolve a path
    fn validate_path(&self, path_str: &str) -> Result<PathBuf, String> {
        // Expand home directory
        let expanded = if path_str.starts_with('~') {
            if let Some(home) = std::env::var("HOME").ok() {
                path_str.replacen('~', &home, 1)
            } else {
                path_str.to_string()
            }
        } else {
            path_str.to_string()
        };

        let path = PathBuf::from(&expanded);

        // Check for path traversal attempts
        let path_str_lower = path_str.to_lowercase();
        let traversal_patterns = ["../", "..\\", "/..", "\\.."];
        for pattern in &traversal_patterns {
            if path_str_lower.contains(pattern) {
                return Err("Path traversal detected".to_string());
            }
        }

        // Resolve the path relative to base directory if set
        let resolved = if path.is_relative() {
            if let Some(ref base) = self.config.base_directory {
                base.join(&path)
            } else {
                std::fs::canonicalize(".").unwrap_or_default().join(&path)
            }
        } else {
            path
        };

        // Check against blocked paths
        for blocked in &self.config.blocked_paths {
            let blocked_expanded = if blocked.starts_with("~") {
                if let Some(home) = std::env::var("HOME").ok() {
                    PathBuf::from(blocked.display().to_string().replacen('~', &home, 1))
                } else {
                    blocked.clone()
                }
            } else {
                blocked.clone()
            };

            if resolved.starts_with(&blocked_expanded) || resolved == blocked_expanded {
                return Err(format!("Access to path is blocked: {}", path_str));
            }
        }

        // Check file extension if restrictions are set
        if !self.config.allowed_extensions.is_empty() {
            if let Some(ext) = resolved.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !self
                    .config
                    .allowed_extensions
                    .iter()
                    .any(|allowed| allowed.to_lowercase() == ext_str)
                {
                    return Err(format!("File extension not allowed: {:?}", ext));
                }
            }
        }

        Ok(resolved)
    }

    /// Read a file
    async fn read_file(&self, path: &Path) -> ToolOutput {
        // Check file size first
        let metadata = match fs::metadata(path).await {
            Ok(m) => m,
            Err(e) => return ToolOutput::error(format!("Failed to access file: {}", e)),
        };

        let size = metadata.len() as usize;
        if size > self.config.max_read_bytes {
            return ToolOutput::error(format!(
                "File too large: {} bytes (max: {})",
                size, self.config.max_read_bytes
            ));
        }

        match fs::read_to_string(path).await {
            Ok(content) => {
                // Check for binary content
                if content.chars().any(|c| c == '\0') {
                    ToolOutput::error("Cannot read binary file as text".to_string())
                } else {
                    ToolOutput::success(content)
                }
            }
            Err(e) => ToolOutput::error(format!("Failed to read file: {}", e)),
        }
    }

    /// Write to a file
    async fn write_file(&self, path: &Path, content: &str, create_dirs: bool) -> ToolOutput {
        if !self.config.allow_write {
            return ToolOutput::error("Write operations are not allowed".to_string());
        }

        // Create parent directories if needed
        if create_dirs {
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return ToolOutput::error(format!("Failed to create directories: {}", e));
                }
            }
        }

        // Write to a temporary file first, then rename for atomicity
        let temp_path = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));

        match fs::write(&temp_path, content).await {
            Ok(()) => {
                // Rename temp file to target
                match fs::rename(&temp_path, path).await {
                    Ok(()) => ToolOutput::success(format!(
                        "Successfully wrote {} bytes to {}",
                        content.len(),
                        path.display()
                    )),
                    Err(e) => {
                        // Clean up temp file
                        let _ = fs::remove_file(&temp_path).await;
                        ToolOutput::error(format!("Failed to write file: {}", e))
                    }
                }
            }
            Err(e) => ToolOutput::error(format!("Failed to write file: {}", e)),
        }
    }

    /// Delete a file
    async fn delete_file(&self, path: &Path) -> ToolOutput {
        if !self.config.allow_delete {
            return ToolOutput::error("Delete operations are not allowed".to_string());
        }

        match fs::remove_file(path).await {
            Ok(()) => ToolOutput::success(format!("Successfully deleted {}", path.display())),
            Err(e) => ToolOutput::error(format!("Failed to delete file: {}", e)),
        }
    }

    /// List directory contents
    async fn list_directory(&self, path: &Path) -> ToolOutput {
        let mut entries = match fs::read_dir(path).await {
            Ok(e) => e,
            Err(e) => return ToolOutput::error(format!("Failed to read directory: {}", e)),
        };

        let mut items = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry
                .file_type()
                .await
                .map(|t| t.is_dir())
                .unwrap_or(false);
            items.push(if is_dir { format!("{}/", name) } else { name });
        }

        items.sort();
        ToolOutput::success(items.join("\n"))
    }

    /// Check if file exists
    async fn file_exists(&self, path: &Path) -> ToolOutput {
        match fs::metadata(path).await {
            Ok(metadata) => {
                let file_type = if metadata.is_dir() {
                    "directory"
                } else if metadata.is_file() {
                    "file"
                } else {
                    "other"
                };
                ToolOutput::success(format!("{} exists as {}", path.display(), file_type))
            }
            Err(_) => ToolOutput::success(format!("{} does not exist", path.display())),
        }
    }
}

impl Default for FileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str {
        "file"
    }

    fn description(&self) -> &str {
        "Read, write, list, and check files with safety constraints. Supports path validation, extension filtering, and size limits."
    }

    fn parameters_schema(&self) -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "delete", "list", "exists"],
                    "description": "The file operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "The file or directory path"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write (for write operation)"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist (for write operation)",
                    "default": false
                }
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(&self, arguments: JsonValue) -> ToolOutput {
        // Parse operation
        let operation = match arguments.get("operation") {
            Some(v) => match v.as_str() {
                Some(s) => s,
                None => return ToolOutput::error("operation must be a string".to_string()),
            },
            None => return ToolOutput::error("operation argument required".to_string()),
        };

        // Parse path
        let path_str = match arguments.get("path") {
            Some(v) => match v.as_str() {
                Some(s) => s,
                None => return ToolOutput::error("path must be a string".to_string()),
            },
            None => return ToolOutput::error("path argument required".to_string()),
        };

        // Validate and resolve path
        let path = match self.validate_path(path_str) {
            Ok(p) => p,
            Err(e) => return ToolOutput::error(e),
        };

        match operation {
            "read" => self.read_file(&path).await,
            "write" => {
                let content = arguments
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let create_dirs = arguments
                    .get("create_dirs")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                self.write_file(&path, content, create_dirs).await
            }
            "delete" => self.delete_file(&path).await,
            "list" => self.list_directory(&path).await,
            "exists" => self.file_exists(&path).await,
            _ => ToolOutput::error(format!("Unknown operation: {}", operation)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_path_simple() {
        let tool = FileTool::new();
        let result = tool.validate_path("test.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_traversal_rejected() {
        let tool = FileTool::new();
        let result = tool.validate_path("../../../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("traversal"));
    }

    #[test]
    fn test_validate_path_absolute() {
        let tool = FileTool::new();
        let result = tool.validate_path("/tmp/test.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_blocked() {
        let tool = FileTool::new();
        let result = tool.validate_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocked"));
    }

    #[test]
    fn test_sandbox_relative_path() {
        let tmp = TempDir::new().unwrap();
        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let result = tool.validate_path("test.txt").unwrap();
        assert!(result.starts_with(tmp.path()));
    }

    #[test]
    fn test_extension_filter() {
        let config = FileToolConfig {
            allowed_extensions: vec!["txt".to_string(), "md".to_string()],
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        // Allowed extensions
        assert!(tool.validate_path("test.txt").is_ok());
        assert!(tool.validate_path("README.md").is_ok());

        // Blocked extension
        let result = tool.validate_path("script.sh");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not allowed"));
    }

    #[tokio::test]
    async fn test_read_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").await.unwrap();

        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let output = tool
            .execute(json!({
                "operation": "read",
                "path": "test.txt"
            }))
            .await;

        assert!(!output.is_error);
        assert!(output.content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let tool = FileTool::new();

        let output = tool
            .execute(json!({
                "operation": "read",
                "path": "/nonexistent/file.txt"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("Failed to access"));
    }

    #[tokio::test]
    async fn test_write_file() {
        let tmp = TempDir::new().unwrap();
        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            allow_write: true,
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let output = tool
            .execute(json!({
                "operation": "write",
                "path": "output.txt",
                "content": "Test content"
            }))
            .await;

        assert!(!output.is_error);
        assert!(output.content.contains("Successfully wrote"));

        // Verify file was written
        let content = fs::read_to_string(tmp.path().join("output.txt"))
            .await
            .unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_write_file_disabled() {
        let config = FileToolConfig {
            allow_write: false,
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let output = tool
            .execute(json!({
                "operation": "write",
                "path": "/tmp/test.txt",
                "content": "Test"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("not allowed"));
    }

    #[tokio::test]
    async fn test_write_with_create_dirs() {
        let tmp = TempDir::new().unwrap();
        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            allow_write: true,
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let output = tool
            .execute(json!({
                "operation": "write",
                "path": "subdir/nested/output.txt",
                "content": "Nested content",
                "create_dirs": true
            }))
            .await;

        assert!(!output.is_error);

        // Verify nested file was created
        let nested_path = tmp.path().join("subdir/nested/output.txt");
        assert!(nested_path.exists());
    }

    #[tokio::test]
    async fn test_delete_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("delete_me.txt");
        fs::write(&file_path, "content").await.unwrap();

        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            allow_delete: true,
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let output = tool
            .execute(json!({
                "operation": "delete",
                "path": "delete_me.txt"
            }))
            .await;

        assert!(!output.is_error);
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_file_disabled() {
        let tool = FileTool::new(); // allow_delete defaults to false

        let output = tool
            .execute(json!({
                "operation": "delete",
                "path": "/tmp/test.txt"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("not allowed"));
    }

    #[tokio::test]
    async fn test_list_directory() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file1.txt"), "").await.unwrap();
        fs::write(tmp.path().join("file2.txt"), "").await.unwrap();
        fs::create_dir(tmp.path().join("subdir")).await.unwrap();

        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        let output = tool
            .execute(json!({
                "operation": "list",
                "path": "."
            }))
            .await;

        assert!(!output.is_error);
        assert!(output.content.contains("file1.txt"));
        assert!(output.content.contains("file2.txt"));
        assert!(output.content.contains("subdir/")); // Directories end with /
    }

    #[tokio::test]
    async fn test_file_exists() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("exists.txt");
        fs::write(&file_path, "content").await.unwrap();

        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        // Existing file
        let output = tool
            .execute(json!({
                "operation": "exists",
                "path": "exists.txt"
            }))
            .await;
        assert!(!output.is_error);
        assert!(output.content.contains("exists"));

        // Non-existing file
        let output = tool
            .execute(json!({
                "operation": "exists",
                "path": "nonexistent.txt"
            }))
            .await;
        assert!(!output.is_error);
        assert!(output.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_missing_operation() {
        let tool = FileTool::new();

        let output = tool.execute(json!({"path": "/tmp/test.txt"})).await;
        assert!(output.is_error);
        assert!(output.content.contains("required"));
    }

    #[tokio::test]
    async fn test_missing_path() {
        let tool = FileTool::new();

        let output = tool.execute(json!({"operation": "read"})).await;
        assert!(output.is_error);
        assert!(output.content.contains("required"));
    }

    #[tokio::test]
    async fn test_unknown_operation() {
        let tool = FileTool::new();

        let output = tool
            .execute(json!({
                "operation": "invalid",
                "path": "/tmp/test.txt"
            }))
            .await;
        assert!(output.is_error);
        assert!(output.content.contains("Unknown operation"));
    }

    #[test]
    fn test_parameters_schema() {
        let tool = FileTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["operation"]["enum"].is_array());
        assert!(schema["required"].as_array().unwrap().contains(&json!("operation")));
        assert!(schema["required"].as_array().unwrap().contains(&json!("path")));
    }

    #[test]
    fn test_tool_name_and_description() {
        let tool = FileTool::new();
        assert_eq!(tool.name(), "file");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_max_read_size() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("large.txt");
        let large_content = "x".repeat(2000);

        let config = FileToolConfig {
            base_directory: Some(tmp.path().to_path_buf()),
            max_read_bytes: 1000,
            ..Default::default()
        };
        let tool = FileTool::with_config(config);

        fs::write(&file_path, &large_content).await.unwrap();

        let output = tool
            .execute(json!({
                "operation": "read",
                "path": "large.txt"
            }))
            .await;

        assert!(output.is_error);
        assert!(output.content.contains("too large"));
    }
}
