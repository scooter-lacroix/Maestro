//! MaestroClaw HTTP gateway.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use crate::config::Config;

const DEFAULT_GATEWAY_HOST: &str = "127.0.0.1";
const MAX_BODY_SIZE: usize = 65_536;
const REQUEST_TIMEOUT_SECS: u64 = 30;
const WEBHOOK_SECRET_HEADER: &str = "x-maestroclaw-secret";

#[derive(Clone)]
struct GatewayState {
    config: Arc<Config>,
}

pub async fn run_gateway(config: Config) -> Result<()> {
    let host = gateway_host(&config);
    let addr = format!("{host}:{}", config.gateway.port);

    crate::health::mark_component_ok("gateway");

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .route("/webhook", post(webhook_handler))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ))
        .with_state(GatewayState {
            config: Arc::new(config),
        });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("gateway listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn gateway_host(config: &Config) -> String {
    let host = config.gateway.host.trim();
    if host.is_empty() {
        DEFAULT_GATEWAY_HOST.to_string()
    } else {
        host.to_string()
    }
}

async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "service": "maestroclaw",
        })),
    )
}

async fn status_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(crate::health::snapshot_json()))
}

#[derive(Debug, serde::Deserialize)]
struct WebhookPayload {
    message: String,
}

async fn webhook_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    match validate_webhook_secret(&headers, &state.config) {
        Ok(()) => {}
        Err(WebhookAuthError::NotConfigured) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "error": "webhook secret not configured",
                })),
            );
        }
        Err(WebhookAuthError::Invalid) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "success": false,
                    "error": "missing or invalid webhook secret",
                })),
            );
        }
    }

    if payload.message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": "empty message",
            })),
        );
    }

    match crate::agent::run_prompt(&state.config, payload.message, 600).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "response": result.content(),
            })),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": error.to_string(),
            })),
        ),
    }
}

fn configured_webhook_secret(config: &Config) -> Option<&str> {
    config
        .channels
        .webhook
        .as_ref()
        .and_then(|webhook| webhook.secret.as_deref())
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
}

enum WebhookAuthError {
    NotConfigured,
    Invalid,
}

fn validate_webhook_secret(
    headers: &HeaderMap,
    config: &Config,
) -> std::result::Result<(), WebhookAuthError> {
    let expected = configured_webhook_secret(config).ok_or(WebhookAuthError::NotConfigured)?;
    let provided = headers
        .get(WEBHOOK_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim);

    if provided == Some(expected) {
        Ok(())
    } else {
        Err(WebhookAuthError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn health_returns_ok() {
        let response = health_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn status_returns_ok() {
        let response = status_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn webhook_rejects_empty_messages() {
        let mut config = Config::default();
        config.channels.webhook = Some(crate::config::schema::WebhookConfig {
            secret: Some("secret".into()),
        });
        let state = GatewayState {
            config: Arc::new(config),
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            WEBHOOK_SECRET_HEADER,
            axum::http::HeaderValue::from_static("secret"),
        );
        let response = webhook_handler(
            State(state),
            headers,
            Json(WebhookPayload {
                message: "   ".into(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn gateway_host_defaults_when_config_has_no_host() {
        let mut config = Config::default();
        config.gateway.host.clear();
        assert_eq!(gateway_host(&config), DEFAULT_GATEWAY_HOST);
    }

    #[tokio::test]
    async fn webhook_requires_configured_secret() {
        let state = GatewayState {
            config: Arc::new(Config::default()),
        };
        let response = webhook_handler(
            State(state),
            HeaderMap::new(),
            Json(WebhookPayload {
                message: "hello".into(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn webhook_rejects_invalid_secret() {
        let mut config = Config::default();
        config.channels.webhook = Some(crate::config::schema::WebhookConfig {
            secret: Some("secret".into()),
        });
        let state = GatewayState {
            config: Arc::new(config),
        };

        let response = webhook_handler(
            State(state),
            HeaderMap::new(),
            Json(WebhookPayload {
                message: "hello".into(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
