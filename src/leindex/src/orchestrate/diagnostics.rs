//! LSP Diagnostic Integration for Orchestrate
//!
//! Provides edit detection and LSP diagnostic validation after agent edits.
//! Ensures agents return error-free code by checking diagnostics before
//! task completion.

use crate::memory::lsp_manager::LspType;
use crate::orchestrate::lsp_client::path_to_file_uri;
use crate::orchestrate::model::LspDiagnosticConfig;
use anyhow::{Context, Result};
use ignore::{Walk, WalkBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

/// LSP diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<lsp_types::DiagnosticSeverity> for DiagnosticSeverity {
    fn from(severity: lsp_types::DiagnosticSeverity) -> Self {
        match severity {
            lsp_types::DiagnosticSeverity::ERROR => Self::Error,
            lsp_types::DiagnosticSeverity::WARNING => Self::Warning,
            lsp_types::DiagnosticSeverity::INFORMATION => Self::Info,
            lsp_types::DiagnosticSeverity::HINT => Self::Hint,
            _ => Self::Info,
        }
    }
}

/// LSP diagnostic from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// File URI
    pub uri: String,
    /// Range in file
    pub range: lsp_types::Range,
    /// Severity
    pub severity: DiagnosticSeverity,
    /// Diagnostic message
    pub message: String,
    /// Source (e.g., "rust-analyzer")
    pub source: Option<String>,
    /// Related diagnostic codes
    pub code: Option<lsp_types::NumberOrString>,
}

impl Diagnostic {
    /// Format diagnostic for display
    pub fn format(&self) -> String {
        let start = &self.range.start;
        let severity = match self.severity {
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Warning => "WARN",
            DiagnosticSeverity::Info => "INFO",
            DiagnosticSeverity::Hint => "HINT",
        };
        let source = self.source.as_deref().unwrap_or("lsp");
        format!(
            "{}:{}:{}: {} [{}]: {}",
            self.uri,
            start.line + 1,
            start.character + 1,
            severity,
            source,
            self.message
        )
    }
}

/// Snapshot of file state for edit detection
#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub modified: SystemTime,
    #[allow(dead_code)]
    pub size: u64,
}

impl FileSnapshot {
    pub fn from_path(path: &Path) -> Result<Self> {
        let meta =
            std::fs::metadata(path).with_context(|| format!("Failed to metadata: {:?}", path))?;
        Ok(Self {
            path: path.to_path_buf(),
            modified: meta
                .modified()
                .with_context(|| format!("Failed to get modified time: {:?}", path))?,
            size: meta.len(),
        })
    }
}

/// Edit detection tracker
#[derive(Debug, Clone)]
pub struct EditTracker {
    /// Working directory
    working_dir: PathBuf,
    /// File snapshots before agent run (path -> modified time)
    before_snapshots: HashMap<PathBuf, SystemTime>,
    /// Files modified during agent run
    modified_files: Vec<PathBuf>,
}

impl EditTracker {
    /// Create a new edit tracker
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            before_snapshots: HashMap::new(),
            modified_files: Vec::new(),
        }
    }

    /// Capture file state before agent run
    pub fn capture_before(&mut self, config: &LspDiagnosticConfig) -> Result<()> {
        self.before_snapshots.clear();

        // Walk the working directory and capture file states
        let walk = self.build_walker(config)?;

        for entry in walk {
            let entry = entry?;
            let path = entry.path().to_path_buf();

            // Only track source files
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if self.is_source_file(&ext_str) {
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if let Ok(modified) = meta.modified() {
                            self.before_snapshots.insert(path.clone(), modified);
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "Captured {} files for edit detection",
            self.before_snapshots.len()
        );

        Ok(())
    }

    /// Detect files modified after agent run
    pub fn detect_edits(&mut self, config: &LspDiagnosticConfig) -> Result<Vec<PathBuf>> {
        self.modified_files.clear();

        let walk = self.build_walker(config)?;

        for entry in walk {
            let entry = entry?;
            let path = entry.path().to_path_buf();

            // Check if this file was tracked before
            if let Some(before_modified) = self.before_snapshots.get(&path) {
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(after_modified) = meta.modified() {
                        if &after_modified > before_modified {
                            self.modified_files.push(path);
                        }
                    }
                }
            }
        }

        tracing::debug!("Detected {} modified files", self.modified_files.len());

        Ok(self.modified_files.clone())
    }

    /// Get modified files
    pub fn modified_files(&self) -> &[PathBuf] {
        &self.modified_files
    }

    /// Build a walker respecting config
    fn build_walker(&self, config: &LspDiagnosticConfig) -> Result<Walk> {
        let mut builder = WalkBuilder::new(&self.working_dir);

        // Add exclude dirs
        for dir in &config.exclude_dirs {
            let dir_pattern = dir.clone();
            builder.filter_entry(move |entry| {
                // Skip hidden directories and our exclude patterns
                if let Some(ft) = entry.file_type() {
                    if ft.is_dir() {
                        let name = entry.file_name().to_string_lossy();
                        // Skip hidden dirs
                        if name.starts_with('.') {
                            return false;
                        }
                        // Skip exclude patterns
                        if name == dir_pattern {
                            return false;
                        }
                    }
                }
                true
            });
        }

        // Add gitignore support
        builder.add_custom_ignore_filename(".maestroignore");
        builder.add_custom_ignore_filename(".gitignore");

        // Build the walker
        Ok(builder.build())
    }

    /// Check if extension is a source file
    fn is_source_file(&self, ext: &str) -> bool {
        matches!(
            ext,
            "rs" | "py"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "go"
                | "java"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "rb"
                | "php"
                | "swift"
                | "kt"
                | "kts"
        )
    }
}

/// Diagnostic validation result
#[derive(Debug, Clone)]
pub struct DiagnosticValidation {
    /// Diagnostics found
    pub diagnostics: Vec<Diagnostic>,
    /// Whether validation passed (no errors or warnings based on config)
    pub passed: bool,
    /// Error message if validation failed
    pub error_message: Option<String>,
}

/// Validate LSP diagnostics for a file
///
/// This function:
/// 1. Determines the appropriate LSP for the file type
/// 2. Connects to the LSP via Unix socket proxy
/// 3. Sends didChange notification
/// 4. Requests diagnostics
/// 5. Returns validation result
///
/// If LSP communication fails, returns a passing validation to avoid
/// blocking the agent loop on LSP issues (errors are logged).
pub async fn validate_diagnostics(
    session_id: &str,
    file_path: &Path,
    config: &LspDiagnosticConfig,
) -> Result<DiagnosticValidation> {
    use super::lsp_client::create_proxy_client;

    let file_uri = path_to_file_uri(file_path)?;

    tracing::debug!("Validating diagnostics for: {}", file_uri);

    // Determine the LSP type from the file extension
    let lsp_type = detect_lsp_type_for_file(file_path);
    let lsp_type = match lsp_type {
        Some(lsp) => lsp,
        None => {
            tracing::debug!("No LSP configured for file: {:?}", file_path);
            return Ok(DiagnosticValidation {
                diagnostics: Vec::new(),
                passed: true,
                error_message: None,
            });
        }
    };

    // Create an LSP client connected via proxy
    let client = match create_proxy_client(session_id, lsp_type).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "Failed to create LSP client for {:?}: {}, skipping diagnostics",
                file_path,
                e
            );
            // Don't fail the iteration if LSP client creation fails
            return Ok(DiagnosticValidation {
                diagnostics: Vec::new(),
                passed: true,
                error_message: None,
            });
        }
    };

    // Validate diagnostics via the LSP
    match client
        .validate_diagnostics(file_path, config.timeout_secs)
        .await
    {
        Ok(validation) => {
            if !validation.passed {
                tracing::warn!(
                    "LSP diagnostics failed for {}: {} diagnostics found",
                    file_uri,
                    validation.diagnostics.len()
                );
            }
            Ok(validation)
        }
        Err(e) => {
            tracing::warn!(
                "LSP diagnostics communication error for {:?}: {}, skipping",
                file_path,
                e
            );
            // Don't fail the iteration if LSP communication fails
            Ok(DiagnosticValidation {
                diagnostics: Vec::new(),
                passed: true,
                error_message: None,
            })
        }
    }
}

/// Detect which LSP should handle a file based on extension
fn detect_lsp_type_for_file(file_path: &Path) -> Option<LspType> {
    let ext = file_path.extension()?.to_string_lossy();
    match ext.as_ref() {
        "rs" => Some(LspType::Rust),
        "py" => Some(LspType::Python),
        "ts" | "tsx" | "js" | "jsx" | "mjs" => Some(LspType::TypeScript),
        _ => None,
    }
}

/// Validate diagnostics for multiple files with timeout
pub async fn validate_diagnostics_batch(
    session_id: &str,
    files: &[PathBuf],
    config: &LspDiagnosticConfig,
) -> Result<DiagnosticValidation> {
    let mut all_diagnostics = Vec::new();
    let duration = Duration::from_secs(config.timeout_secs);

    for file_path in files {
        let validation = timeout(
            duration,
            validate_diagnostics(session_id, file_path, config),
        )
        .await
        .with_context(|| {
            format!(
                "Diagnostic timeout for {:?} after {}s",
                file_path, config.timeout_secs
            )
        })??;

        all_diagnostics.extend(validation.diagnostics);

        // Early exit if errors found and fail_on_errors is true
        if config.fail_on_errors {
            let has_errors = all_diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error);
            if has_errors {
                return Ok(DiagnosticValidation {
                    passed: false,
                    error_message: Some(format_diagnostics(&all_diagnostics, config)),
                    diagnostics: all_diagnostics,
                });
            }
        }
    }

    // Check for warnings if fail_on_warnings is true
    let passed = if config.fail_on_errors {
        !all_diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    } else if config.fail_on_warnings {
        !all_diagnostics.iter().any(|d| {
            matches!(
                d.severity,
                DiagnosticSeverity::Error | DiagnosticSeverity::Warning
            )
        })
    } else {
        true
    };

    Ok(DiagnosticValidation {
        passed,
        error_message: if !passed {
            Some(format_diagnostics(&all_diagnostics, config))
        } else {
            None
        },
        diagnostics: all_diagnostics,
    })
}

/// Format diagnostics for display
pub fn format_diagnostics(diagnostics: &[Diagnostic], config: &LspDiagnosticConfig) -> String {
    let count = diagnostics.len();
    let to_show = diagnostics.iter().take(config.max_diagnostics);

    let mut output = format!("LSP Diagnostics ({} found):\n", count);

    for diag in to_show {
        output.push_str(&diag.format());
        output.push('\n');
    }

    if count > config.max_diagnostics {
        output.push_str(&format!(
            "... and {} more (max {} shown)\n",
            count - config.max_diagnostics,
            config.max_diagnostics
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_file_uri() {
        let path = Path::new("/tmp/test.rs");
        let uri = path_to_file_uri(path).unwrap();
        assert_eq!(uri, "file:///tmp/test.rs");
    }

    #[test]
    fn test_diagnostic_format() {
        let diag = Diagnostic {
            uri: "file:///tmp/test.rs".to_string(),
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 5,
                },
                end: lsp_types::Position {
                    line: 10,
                    character: 15,
                },
            },
            severity: DiagnosticSeverity::Error,
            message: "expected type, found `()`".to_string(),
            source: Some("rust-analyzer".to_string()),
            code: None,
        };

        let formatted = diag.format();
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("rust-analyzer"));
        assert!(formatted.contains("expected type"));
        assert!(formatted.contains(":11:6")); // line 10 -> 11 (1-indexed)
    }

    #[test]
    fn test_edit_tracker() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        // Write test file
        std::fs::write(&test_file, "fn main() {}").unwrap();

        let mut tracker = EditTracker::new(temp_dir.path().to_path_buf());
        let config = LspDiagnosticConfig::default();

        // Capture before
        tracker.capture_before(&config).unwrap();

        // Ensure at least 10ms delay to guarantee mtime changes on most filesystems
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Modify file
        std::fs::write(&test_file, "fn main() { println!(\"hi\"); }").unwrap();

        // Ensure the change propagates
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Detect edits
        let edits = tracker.detect_edits(&config).unwrap();

        assert!(!edits.is_empty());
        assert!(edits.contains(&test_file));
    }

    #[test]
    fn test_lsp_diagnostic_config_default() {
        let config = LspDiagnosticConfig::default();
        assert!(config.enabled);
        assert!(config.fail_on_errors);
        assert!(!config.fail_on_warnings);
        assert!(config.include_patterns.contains(&"**/*.rs".to_string()));
        assert!(config.exclude_dirs.contains(&"target".to_string()));
    }
}
