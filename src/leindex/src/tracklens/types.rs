// TrackLens Core Types
//
// This module defines the core types used across TrackLens:
// - Review modes (review, walkthrough, code-review)
// - Decisions (approve, deny with annotations)
// - Annotations (comments, feedback)
// - Autonomy modes (full-auto, semi-auto, checkpoint)

use serde::{Deserialize, Serialize};

// ─── Review Mode ─────────────────────────────────────────────────────────────

/// Review mode for TrackLens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewMode {
    /// Plan/spec/walkthrough review mode
    Review,
    /// Code review mode (git diff)
    CodeReview,
    /// Annotate mode (arbitrary markdown)
    Annotate,
}

// ─── Decision & Behavior ───────────────────────────────────────────────────────

/// User decision from TrackLens review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackLensDecision {
    /// Whether to approve or deny
    pub behavior: DecisionBehavior,
    /// Optional annotations if denied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    /// Optional autonomy mode change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomy_mode: Option<AutonomyMode>,
}

/// Decision behavior - allow or deny
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionBehavior {
    /// Approve and proceed
    Allow,
    /// Deny with annotations for remediation
    Deny,
}

// ─── Annotation ───────────────────────────────────────────────────────────────

/// Annotation on a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// Unique ID for this annotation
    pub id: String,
    /// Text selection for the annotation
    pub selection: TextSelection,
    /// Annotation content
    pub content: AnnotationContent,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Text selection range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSelection {
    /// Start position (line, column)
    pub start: Position,
    /// End position (line, column)
    pub end: Position,
    /// Selected text
    pub text: String,
}

/// Position in a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

/// Annotation content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationContent {
    /// Comment text
    pub comment: String,
    /// Severity (info, warning, error)
    pub severity: AnnotationSeverity,
}

/// Annotation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error (blocking)
    Error,
}

// ─── Autonomy Mode ────────────────────────────────────────────────────────────

/// Autonomy mode for Conductor (mapped from legacy permissions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutonomyMode {
    /// Full automatic - bypass all checkpoints
    FullAuto,
    /// Semi-automatic - accept edits but checkpoint on major decisions
    SemiAuto,
    /// Checkpoint - require approval for all decisions
    Checkpoint,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_serialization() {
        let decision = TrackLensDecision {
            behavior: DecisionBehavior::Allow,
            annotations: None,
            autonomy_mode: None,
        };

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"behavior\":\"allow\""));
    }

    #[test]
    fn test_annotation_with_selection() {
        let annotation = Annotation {
            id: "test-1".to_string(),
            selection: TextSelection {
                start: Position { line: 1, column: 0 },
                end: Position { line: 2, column: 10 },
                text: "selected text".to_string(),
            },
            content: AnnotationContent {
                comment: "Test comment".to_string(),
                severity: AnnotationSeverity::Warning,
            },
            timestamp: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&annotation).unwrap();
        assert!(json.contains("\"severity\":\"warning\""));
    }
}
