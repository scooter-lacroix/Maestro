//! Shared API Response Types
//!
//! Common response types used across all API handlers.

use axum::{http::StatusCode, Json};
use serde::Serialize;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                success: true,
                data: Some(data),
                error: None,
            }),
        )
    }

    pub fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<Self>) {
        (
            status,
            Json(Self {
                success: false,
                data: None,
                error: Some(msg.to_string()),
            }),
        )
    }
}
