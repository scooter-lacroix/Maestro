//! Server-Sent Events (SSE) for real-time updates
//!
//! Provides an alternative to WebSocket for clients that prefer SSE.

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
};
use futures_util::stream::{self, Stream};
use tokio_stream::StreamExt as _;
use tracing::debug;

use crate::protocol::EventFrame;
use crate::state::GatewayState;

/// SSE endpoint for event streaming
pub async fn sse_handler(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Query(query): Query<crate::ws::WsQuery>,
) -> Response {
    let query_token = query.api_key.as_deref().or(query.access_token.as_deref());
    let auth = match crate::agent_runtime::verify_agent_auth(&state, &headers, query_token) {
        Ok(auth) => auth,
        Err(error) => return error.into_response(),
    };

    debug!("SSE connection established");
    let requested_scopes = crate::agent_runtime::parse_event_scopes(query.scopes.as_deref());
    let scopes = auth.intersect_scopes(&requested_scopes);

    // Subscribe to broadcast events
    let mut event_rx = state.event_bus.subscribe();

    // Create a stream that converts EventFrames to SSE Events
    let stream = async_stream::stream! {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if !crate::agent_runtime::event_visible(&event, &scopes) {
                        continue;
                    }
                    // Convert EventFrame to SSE Event
                    let sse_event = event_to_sse(&event);
                    yield Ok::<Event, Infallible>(sse_event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    debug!("SSE broadcast channel closed");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    debug!("SSE client lagged by {} messages", n);
                    // Continue, don't break
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// Convert an EventFrame to an SSE Event
fn event_to_sse(frame: &EventFrame) -> Event {
    let mut event = Event::default().event(&frame.event);

    if let Some(payload) = &frame.payload {
        if let Ok(json) = serde_json::to_string(payload) {
            event = event.data(json);
        }
    }

    if let Some(seq) = frame.seq {
        event = event.id(seq.to_string());
    }

    event
}

/// SSE endpoint for specific event types
pub async fn sse_events_handler(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Query(query): Query<crate::ws::WsQuery>,
    axum::extract::Path(event_types): axum::extract::Path<String>,
) -> Response {
    let query_token = query.api_key.as_deref().or(query.access_token.as_deref());
    let auth = match crate::agent_runtime::verify_agent_auth(&state, &headers, query_token) {
        Ok(auth) => auth,
        Err(error) => return error.into_response(),
    };

    // Parse and own the event types
    let types: Vec<String> = event_types.split(',').map(|s| s.to_string()).collect();
    debug!("SSE filtered connection for events: {:?}", types);
    let requested_scopes = crate::agent_runtime::parse_event_scopes(query.scopes.as_deref());
    let scopes = auth.intersect_scopes(&requested_scopes);

    let mut event_rx = state.event_bus.subscribe();

    let stream = async_stream::stream! {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if !crate::agent_runtime::event_visible(&event, &scopes) {
                        continue;
                    }
                    // Filter by event type
                    if types.is_empty() || types.iter().any(|t| event.event == *t || event.event.starts_with(t)) {
                        let sse_event = event_to_sse(&event);
                        yield Ok::<Event, Infallible>(sse_event);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // Continue
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// Heartbeat SSE endpoint for connection testing
pub async fn sse_heartbeat() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| {
        Ok(Event::default()
            .event("heartbeat")
            .data(chrono::Utc::now().to_rfc3339()))
    })
    .throttle(std::time::Duration::from_secs(30));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::EventFrame;

    #[test]
    fn test_event_to_sse() {
        let frame = EventFrame::new(
            "test.event",
            Some(serde_json::json!({"foo": "bar"})),
            Some(42),
        );

        let sse = event_to_sse(&frame);

        // Event should have the event name set
        // Note: Event fields are private, we can only test that conversion doesn't panic
        let _ = sse;
    }
}
