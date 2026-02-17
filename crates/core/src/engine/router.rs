use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub request_id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Response,
    ToolCall(ToolCall),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteErrorKind {
    MalformedToolCall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteError {
    pub kind: RouteErrorKind,
    pub message: String,
}

impl RouteError {
    fn malformed(message: impl Into<String>) -> Self {
        Self {
            kind: RouteErrorKind::MalformedToolCall,
            message: message.into(),
        }
    }
}

pub fn parse_intent(content: &str, enable_text_fallback: bool) -> Result<Intent, RouteError> {
    let trimmed = content.trim();

    if let Some(intent) = parse_native_tool_call(trimmed)? {
        return Ok(intent);
    }

    if enable_text_fallback {
        if let Some(intent) = parse_text_fallback(trimmed)? {
            return Ok(intent);
        }
    }

    Ok(Intent::Response)
}

fn parse_native_tool_call(content: &str) -> Result<Option<Intent>, RouteError> {
    if content.is_empty() {
        return Ok(None);
    }

    match serde_json::from_str::<Value>(content) {
        Ok(root) => parse_native_root(root),
        Err(_) => {
            if is_likely_native_tool_call_payload(content) {
                return Err(RouteError::malformed(
                    "tool_calls payload is not valid JSON",
                ));
            }
            Ok(None)
        }
    }
}

fn is_likely_native_tool_call_payload(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{') && trimmed.contains("tool_calls")
}

fn parse_native_root(root: Value) -> Result<Option<Intent>, RouteError> {
    let Some(tool_calls_value) = root.get("tool_calls") else {
        return Ok(None);
    };

    let tool_calls = tool_calls_value
        .as_array()
        .ok_or_else(|| RouteError::malformed("tool_calls must be an array when present"))?;

    if tool_calls.len() > 1 {
        return Err(RouteError::malformed(
            "tool_calls array must contain exactly one call; multiple calls are not supported",
        ));
    }

    let first = tool_calls
        .first()
        .ok_or_else(|| RouteError::malformed("tool_calls array must contain at least one call"))?;

    let tool_call = parse_tool_call_value(first)?;
    Ok(Some(Intent::ToolCall(tool_call)))
}

fn parse_tool_call_value(raw: &Value) -> Result<ToolCall, RouteError> {
    let request_id = raw
        .get("request_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RouteError::malformed("tool call is missing non-empty request_id"))?
        .to_string();

    let name = raw
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RouteError::malformed("tool call is missing non-empty name"))?
        .to_string();

    let arguments_value = raw
        .get("arguments")
        .ok_or_else(|| RouteError::malformed("tool call is missing arguments"))?;

    let arguments = parse_arguments(arguments_value)?;

    Ok(ToolCall {
        request_id,
        name,
        arguments,
    })
}

fn parse_arguments(arguments: &Value) -> Result<Value, RouteError> {
    match arguments {
        Value::String(payload) => serde_json::from_str(payload)
            .map_err(|_| RouteError::malformed("tool call arguments string is not valid JSON")),
        Value::Object(_) | Value::Array(_) | Value::Null | Value::Bool(_) | Value::Number(_) => {
            Ok(arguments.clone())
        }
    }
}

fn parse_text_fallback(content: &str) -> Result<Option<Intent>, RouteError> {
    let start = match find_tool_tag_start(content) {
        Some(index) => index,
        None => return Ok(None),
    };

    let remaining = &content[start..];
    let open_end = remaining
        .find('>')
        .ok_or_else(|| RouteError::malformed("text tool tag is missing closing '>'"))?;
    let open_tag = &remaining[..=open_end];

    let name = extract_tag_attribute(open_tag, "name")
        .ok_or_else(|| RouteError::malformed("text tool tag is missing name attribute"))?;

    let close_tag = "</tool>";
    let close_index = remaining
        .find(close_tag)
        .ok_or_else(|| RouteError::malformed("text tool tag is missing closing </tool>"))?;

    let payload = remaining[(open_end + 1)..close_index].trim();
    let arguments = serde_json::from_str::<Value>(payload)
        .map_err(|_| RouteError::malformed("text tool payload is not valid JSON"))?;

    Ok(Some(Intent::ToolCall(ToolCall {
        request_id: name.clone(),
        name,
        arguments,
    })))
}

fn find_tool_tag_start(content: &str) -> Option<usize> {
    for (index, _) in content.match_indices("<tool") {
        let next = content[(index + "<tool".len())..].chars().next();
        match next {
            Some('>') => return Some(index),
            Some(ch) if ch.is_whitespace() => return Some(index),
            None => return Some(index),
            Some(_) => continue,
        }
    }

    None
}

fn extract_tag_attribute(tag: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let tail = &tag[start..];
    let end = tail.find('"')?;
    let value = &tail[..end];
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
