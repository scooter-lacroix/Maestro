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
    /// Whether to approve or deny (accepts 'behavior' or legacy 'approved')
    #[serde(alias = "approved")]
    pub behavior: DecisionBehavior,
    /// Optional annotations/feedback if denied
    /// Accepts Vec<Annotation> or String (legacy format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    /// Legacy feedback field (converted to annotations)
    #[serde(skip_serializing_if = "Option::is_none", alias = "feedback")]
    pub feedback: Option<String>,
    /// Optional autonomy mode change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomy_mode: Option<AutonomyMode>,
}

/// Decision behavior - allow or deny
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionBehavior {
    /// Approve and proceed
    Allow,
    /// Deny with annotations for remediation
    Deny,
}

impl<'de> Deserialize<'de> for DecisionBehavior {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct DecisionBehaviorVisitor;

        impl<'de> Visitor<'de> for DecisionBehaviorVisitor {
            type Value = DecisionBehavior;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string 'allow'/'deny' or boolean true/false")
            }

            fn visit_str<E>(self, value: &str) -> Result<DecisionBehavior, E>
            where
                E: de::Error,
            {
                match value.to_lowercase().as_str() {
                    "allow" => Ok(DecisionBehavior::Allow),
                    "deny" => Ok(DecisionBehavior::Deny),
                    _ => Err(de::Error::unknown_variant(value, &["allow", "deny"])),
                }
            }

            fn visit_bool<E>(self, value: bool) -> Result<DecisionBehavior, E>
            where
                E: de::Error,
            {
                if value {
                    Ok(DecisionBehavior::Allow)
                } else {
                    Ok(DecisionBehavior::Deny)
                }
            }
        }

        deserializer.deserialize_any(DecisionBehaviorVisitor)
    }
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
            feedback: None,
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
                end: Position {
                    line: 2,
                    column: 10,
                },
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
