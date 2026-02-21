//! Tool-call parsing with robust fallbacks.
//!
//! This module implements ZeroClaw-style tool-call parsing with:
//! - Structured tool calls (native JSON format)
//! - Tagged-text fallback parsing
//! - Malformed input handling with detailed errors
//! - Alias tag rejection (strict word boundary matching)
//! - Strict non-cross-match between native and text fallback paths
//!
//! Based on ZeroClaw patterns from `analysis_foundation_20260217.md`:
//! - `src/agent/loop_.rs:parse_tool_calls`
//! - `src/agent/loop_.rs:parse_tool_calls_from_json_value`

use serde_json::Value;

/// Parsed intent from LLM response content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// A plain text response without tool calls.
    Response,
    /// A tool call invocation.
    ToolCall(ToolCall),
}

/// A parsed tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Unique request identifier for this tool call.
    pub request_id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// Tool arguments as JSON value.
    pub arguments: Value,
}

/// Error type for tool-call parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseIntentError {
    /// The tool call structure is malformed.
    MalformedToolCall(String),
}

impl std::fmt::Display for ParseIntentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedToolCall(msg) => write!(f, "malformed tool call: {msg}"),
        }
    }
}

impl std::error::Error for ParseIntentError {}

/// Parse LLM response content to extract tool calls or text response.
///
/// This function implements ZeroClaw's dual-path parsing:
/// 1. **Native path**: Structured JSON with `tool_calls` key
/// 2. **Fallback path**: Tagged-text format `<tool name="...">...</tool>` (if enabled)
///
/// The two paths are strictly separated - no cross-matching behavior.
///
/// # Arguments
/// * `content` - The LLM response content to parse
/// * `enable_text_fallback` - Whether to enable tagged-text fallback parsing
///
/// # Returns
/// * `Ok(Intent::Response)` - No tool calls detected
/// * `Ok(Intent::ToolCall(...))` - Valid tool call parsed
/// * `Err(ParseIntentError)` - Malformed tool call detected
pub fn parse_intent(content: &str, enable_text_fallback: bool) -> Result<Intent, ParseIntentError> {
    let trimmed = content.trim();

    // Early return for empty content
    if trimmed.is_empty() {
        return Ok(Intent::Response);
    }

    // Path 1: Try native structured tool calls
    if let Some(intent) = parse_native_tool_call(trimmed)? {
        return Ok(intent);
    }

    // Path 2: Try text fallback (if enabled and native path failed)
    if enable_text_fallback {
        if let Some(intent) = parse_text_fallback(trimmed)? {
            return Ok(intent);
        }
    }

    // No tool calls detected - plain response
    Ok(Intent::Response)
}

/// Parse native structured JSON tool calls.
///
/// Expects format: `{"tool_calls":[{"request_id":"...","name":"...","arguments":...}]}`
///
/// Returns `Ok(None)` if the content is not native JSON tool call format.
/// Returns `Err` if the content appears to be native JSON but is malformed.
fn parse_native_tool_call(content: &str) -> Result<Option<Intent>, ParseIntentError> {
    // Quick check: must start with '{' for JSON
    if !content.starts_with('{') {
        return Ok(None);
    }

    // Attempt JSON parsing
    let root = match serde_json::from_str::<Value>(content) {
        Ok(value) => value,
        Err(_) => {
            // Only error if this looks like it was intended as a tool call payload
            if is_likely_native_tool_call_payload(content) {
                return Err(ParseIntentError::MalformedToolCall(
                    "tool_calls payload is not valid JSON".to_string(),
                ));
            }
            return Ok(None);
        }
    };

    parse_native_root(root)
}

/// Check if content appears to be a native tool call payload.
///
/// This detects malformed JSON that was intended to be a tool call.
fn is_likely_native_tool_call_payload(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{') && trimmed.contains("tool_calls")
}

/// Parse native JSON root object for tool_calls array.
fn parse_native_root(root: Value) -> Result<Option<Intent>, ParseIntentError> {
    let Some(tool_calls_value) = root.get("tool_calls") else {
        return Ok(None);
    };

    let tool_calls = tool_calls_value.as_array().ok_or_else(|| {
        ParseIntentError::MalformedToolCall("tool_calls must be an array when present".to_string())
    })?;

    if tool_calls.is_empty() {
        return Err(ParseIntentError::MalformedToolCall(
            "tool_calls array must contain at least one call".to_string(),
        ));
    }

    if tool_calls.len() > 1 {
        return Err(ParseIntentError::MalformedToolCall(
            "tool_calls array must contain exactly one call; multiple calls are not supported"
                .to_string(),
        ));
    }

    let first = tool_calls.first().unwrap(); // Safe: we just checked len > 0
    let tool_call = parse_tool_call_value(first)?;
    Ok(Some(Intent::ToolCall(tool_call)))
}

/// Parse a single tool call from JSON value.
fn parse_tool_call_value(raw: &Value) -> Result<ToolCall, ParseIntentError> {
    // Extract and validate request_id
    let request_id = raw
        .get("request_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ParseIntentError::MalformedToolCall(
                "tool call is missing non-empty request_id".to_string(),
            )
        })
        .map(|s| s.to_string())?;

    // Extract and validate name
    let name = raw
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ParseIntentError::MalformedToolCall("tool call is missing non-empty name".to_string())
        })
        .map(|s| s.to_string())?;

    // Extract and validate arguments
    let arguments_value = raw.get("arguments").ok_or_else(|| {
        ParseIntentError::MalformedToolCall("tool call is missing arguments".to_string())
    })?;

    let arguments = parse_arguments(arguments_value)?;

    Ok(ToolCall {
        request_id,
        name,
        arguments,
    })
}

/// Parse arguments value (can be string-encoded JSON or direct JSON value).
///
/// For string arguments:
/// - First attempts to parse as JSON (for encoded objects/arrays)
/// - If JSON parsing fails, passes through as raw string (for primitive values)
///
/// For non-string arguments: passes through directly.
fn parse_arguments(arguments: &Value) -> Result<Value, ParseIntentError> {
    match arguments {
        // String arguments may be JSON-encoded OR primitive string values
        Value::String(payload) => {
            // Try to parse as JSON first (for encoded objects/arrays)
            match serde_json::from_str::<Value>(payload) {
                Ok(parsed) => Ok(parsed),
                // If not valid JSON, pass through as raw string (primitive value)
                Err(_) => Ok(arguments.clone()),
            }
        }
        // Direct JSON values are passed through
        Value::Object(_) | Value::Array(_) | Value::Null | Value::Bool(_) | Value::Number(_) => {
            Ok(arguments.clone())
        }
    }
}

/// Parse tagged-text fallback format: `<tool name="...">...</tool>`
///
/// This implements ZeroClaw's prompt-tag fallback for providers without native tools.
///
/// Behavior:
/// - Returns `Ok(Some(Intent))` if a valid tool call is parsed
/// - Returns `Err` if content looks like a malformed tool call (has structural issues)
/// - Returns `Ok(None)` if no tool call pattern is detected (plain text)
///
/// The distinction between "error" and "plain text":
/// - Well-formed opening tag + JSON payload + no closing tag → Error (malformed tool call)
/// - Text mentioning `<tool>` but no proper structure → Plain text (Response)
pub fn parse_text_fallback(content: &str) -> Result<Option<Intent>, ParseIntentError> {
    let start = match find_tool_tag_start(content) {
        Some(index) => index,
        None => return Ok(None),
    };

    let remaining = &content[start..];

    // Find closing '>' of opening tag
    let open_end = match remaining.find('>') {
        Some(pos) => pos,
        None => return Ok(None), // Incomplete tag - treat as plain text
    };

    let open_tag = &remaining[..=open_end];

    // Check if this looks like a well-formed tool call opening
    // If it has a name attribute or looks like `<tool>` without attributes
    // Account for optional whitespace around '=' (e.g., "name = ")
    let has_name_attr = open_tag.contains("name=") || open_tag.contains("name ");
    let looks_like_tool_call = has_name_attr || open_tag.ends_with("<tool>");

    // Extract tool name from attribute (if present)
    let name = extract_tag_attribute(open_tag, "name");

    // Find closing tag
    let close_tag = "</tool>";
    let close_index = remaining.find(close_tag);

    match (looks_like_tool_call, close_index, name) {
        (true, Some(close_pos), Some(name)) => {
            // Complete structure - parse payload
            let payload = remaining[(open_end + 1)..close_pos].trim();
            let arguments = serde_json::from_str::<Value>(payload).map_err(|_| {
                ParseIntentError::MalformedToolCall(
                    "text tool payload is not valid JSON".to_string(),
                )
            })?;

            Ok(Some(Intent::ToolCall(ToolCall {
                request_id: name.clone(),
                name,
                arguments,
            })))
        }
        (true, None, Some(_)) => {
            // Has opening tag with name, but no closing tag
            // Check if there's JSON-like content after the opening tag
            let after_open = &remaining[(open_end + 1)..];
            let trimmed = after_open.trim();

            // If it starts with `{` or `[`, it looks like an incomplete tool call
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                return Err(ParseIntentError::MalformedToolCall(
                    "text tool tag is missing closing </tool>".to_string(),
                ));
            }

            // Otherwise, treat as plain text
            Ok(None)
        }
        (true, _, None) => {
            // Looks like a tool call but missing name attribute
            // Check if there's a closing tag and JSON payload
            if let Some(close_pos) = close_index {
                let payload = remaining[(open_end + 1)..close_pos].trim();
                if payload.starts_with('{') || payload.starts_with('[') {
                    return Err(ParseIntentError::MalformedToolCall(
                        "text tool tag is missing name attribute".to_string(),
                    ));
                }
            }
            Ok(None)
        }
        _ => Ok(None), // Not a tool call pattern
    }
}

/// Find the start position of a `<tool` tag with strict word boundary matching.
///
/// This implements alias tag rejection: matches like "toolkit", "toolbox" are NOT matched.
/// Only "<tool" followed by '>' or whitespace is considered a valid tag start.
///
/// # Examples
/// - `"<tool name=\"test\">"` → Some(0)
/// - `"This <tool name=\"test\">"` → Some(5)
/// - `"This is a toolkit"` → None (alias rejected)
pub fn find_tool_tag_start(content: &str) -> Option<usize> {
    for (index, _) in content.match_indices("<tool") {
        let next = content[(index + "<tool".len())..].chars().next();

        // Strict word boundary: must be followed by '>' or whitespace
        match next {
            Some('>') => return Some(index),
            Some(ch) if ch.is_whitespace() => return Some(index),
            None => return Some(index),
            Some(_) => continue, // Alias tag like "toolkit" - reject
        }
    }

    None
}

/// Extract an attribute value from an HTML-like tag string.
///
/// Handles optional whitespace around the `=` sign (e.g., `name="val"`, `name = "val"`).
///
/// # Arguments
/// * `tag` - The tag string (e.g., `<tool name="search" other="value">`)
/// * `attribute` - The attribute name to extract
///
/// # Returns
/// * `Some(value)` - Attribute value if found and non-empty
/// * `None` - Attribute not found or empty
pub fn extract_tag_attribute(tag: &str, attribute: &str) -> Option<String> {
    // Build pattern: attribute name followed by optional whitespace, '=', optional whitespace, and '"'
    // We'll search for the attribute name first, then parse the value manually
    let attr_start = tag.find(attribute)?;
    let after_attr = &tag[attr_start + attribute.len()..];

    // Skip whitespace after attribute name
    let after_whitespace = after_attr.trim_start();

    // Check for '='
    if !after_whitespace.starts_with('=') {
        return None;
    }

    // Skip '=' and any following whitespace
    let after_equals = after_whitespace[1..].trim_start();

    // Expect opening quote
    if !after_equals.starts_with('"') {
        return None;
    }

    // Find the closing quote
    let value_start = 1; // After opening quote
    let value_end = after_equals[value_start..].find('"')?;
    let value = &after_equals[value_start..value_start + value_end];

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_intent_empty() {
        assert_eq!(parse_intent("", false), Ok(Intent::Response));
    }

    #[test]
    fn test_parse_intent_plain_text() {
        assert_eq!(parse_intent("Hello, world!", false), Ok(Intent::Response));
    }

    #[test]
    fn test_parse_native_tool_call_valid() {
        let content = r#"{"tool_calls":[{"request_id":"test-123","name":"search","arguments":{"query":"test"}}]}"#;
        assert!(matches!(
            parse_intent(content, false),
            Ok(Intent::ToolCall(_))
        ));
    }

    #[test]
    fn test_parse_text_fallback_valid() {
        let content = r#"<tool name="search">{"query":"test"}</tool>"#;
        assert!(matches!(
            parse_intent(content, true),
            Ok(Intent::ToolCall(_))
        ));
    }

    #[test]
    fn test_alias_tag_rejection() {
        let content = "This is a toolkit component";
        assert_eq!(find_tool_tag_start(content), None);
    }
}
