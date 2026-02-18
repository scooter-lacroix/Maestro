//! Frame-based protocol for WebSocket communication
//!
//! Based on Moltis protocol pattern with Request/Response/Event frames.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request frame from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestFrame {
    /// Frame type: "req"
    pub r#type: String,
    /// Unique request ID for correlation
    pub id: String,
    /// Method name to invoke
    pub method: String,
    /// Optional parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl RequestFrame {
    /// Create a new request frame
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            r#type: "req".to_string(),
            id: Uuid::new_v4().to_string(),
            method: method.into(),
            params,
        }
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Response frame to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFrame {
    /// Frame type: "res"
    pub r#type: String,
    /// Request ID being responded to
    pub id: String,
    /// Whether the request succeeded
    pub success: bool,
    /// Result payload (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Error code (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
}

impl ResponseFrame {
    /// Create a success response
    pub fn success(id: impl Into<String>, result: Option<serde_json::Value>) -> Self {
        Self {
            r#type: "res".to_string(),
            id: id.into(),
            success: true,
            result,
            error: None,
            error_code: None,
        }
    }

    /// Create an error response
    pub fn error(id: impl Into<String>, error: impl Into<String>, code: Option<i32>) -> Self {
        Self {
            r#type: "res".to_string(),
            id: id.into(),
            success: false,
            result: None,
            error: Some(error.into()),
            error_code: code,
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Event frame pushed to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFrame {
    /// Frame type: "event"
    pub r#type: String,
    /// Event name/type
    pub event: String,
    /// Event payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Sequence number for ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
}

impl EventFrame {
    /// Create a new event frame
    pub fn new(event: impl Into<String>, payload: Option<serde_json::Value>, seq: Option<u64>) -> Self {
        Self {
            r#type: "event".to_string(),
            event: event.into(),
            payload,
            seq,
            ts: Some(chrono::Utc::now().timestamp_millis()),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Error codes for protocol responses
pub mod error_codes {
    /// Invalid request format
    pub const INVALID_REQUEST: i32 = -32700;
    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;
    /// Rate limited
    pub const RATE_LIMITED: i32 = -32001;
    /// Unauthorized
    pub const UNAUTHORIZED: i32 = -32002;
    /// Timeout
    pub const TIMEOUT: i32 = -32003;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_frame_serialization() {
        let req = RequestFrame::new("ping", Some(serde_json::json!({"count": 1})));
        let json = req.to_json().unwrap();
        assert!(json.contains(r#""type":"req""#));
        assert!(json.contains("ping"));

        let parsed: RequestFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "ping");
    }

    #[test]
    fn test_response_frame_success() {
        let res = ResponseFrame::success("req-123", Some(serde_json::json!({"pong": true})));
        assert!(res.success);
        assert!(res.result.is_some());
    }

    #[test]
    fn test_response_frame_error() {
        let res = ResponseFrame::error("req-123", "Not found", Some(error_codes::METHOD_NOT_FOUND));
        assert!(!res.success);
        assert_eq!(res.error_code, Some(error_codes::METHOD_NOT_FOUND));
    }

    #[test]
    fn test_event_frame() {
        let event = EventFrame::new("tool.call", Some(serde_json::json!({"tool": "bash"})), Some(1));
        let json = event.to_json().unwrap();
        assert!(json.contains(r#""type":"event""#));
        assert!(json.contains("tool.call"));
    }
}
