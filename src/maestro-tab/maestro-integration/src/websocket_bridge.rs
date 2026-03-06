//! WebSocket to PTY Bridge for Maestro-tab integration
//!
//! This module provides the bridge between MaestroTabMultiplexer and
//! the tab-daemon via WebSocket, handling PTY I/O messages.

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tab_api::config::DaemonConfig;
use tab_api::pty::{PtyWebsocketRequest, PtyWebsocketResponse};
use tab_api::tab::TabMetadata;
use tab_websocket::WebsocketConnection;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

/// Bridge between WebSocket and PTY operations
pub struct WebSocketPtyBridge {
    /// Channel for outgoing PTY requests
    request_tx: mpsc::Sender<PtyWebsocketRequest>,
    /// Channel for incoming PTY responses
    response_rx: Arc<Mutex<mpsc::Receiver<PtyWebsocketResponse>>>,
    /// Current tab metadata
    metadata: Arc<Mutex<Option<TabMetadata>>>,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Handle to the reader task
    _reader_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Handle to the writer task
    _writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl WebSocketPtyBridge {
    /// Create a new bridge with the given channels (for testing/advanced use)
    pub fn new(
        request_tx: mpsc::Sender<PtyWebsocketRequest>,
        response_rx: mpsc::Receiver<PtyWebsocketResponse>,
    ) -> Self {
        Self {
            request_tx,
            response_rx: Arc::new(Mutex::new(response_rx)),
            metadata: Arc::new(Mutex::new(None)),
            shutdown_tx: None,
            _reader_handle: Arc::new(Mutex::new(None)),
            _writer_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to tab-daemon and create a new WebSocket PTY bridge
    ///
    /// This establishes a WebSocket connection to the daemon's /pty endpoint
    /// and spawns reader/writer tasks for bidirectional communication.
    pub async fn connect(daemon_config: &DaemonConfig) -> Result<Self> {
        let ws_url = format!("ws://127.0.0.1:{}/pty", daemon_config.port);
        tracing::info!("Connecting to tab-daemon PTY endpoint at {}", ws_url);

        // Establish authenticated WebSocket connection
        let websocket = tab_websocket::connect_authorized(ws_url, daemon_config.auth_token.clone())
            .await
            .context("Failed to connect to tab-daemon PTY endpoint")?;

        tracing::info!("WebSocket connection established to tab-daemon");

        // Create channels for bidirectional communication
        let (request_tx, request_rx) = mpsc::channel::<PtyWebsocketRequest>(64);
        let (response_tx, response_rx) = mpsc::channel::<PtyWebsocketResponse>(64);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let (_shutdown_tx2, shutdown_rx2) = mpsc::channel::<()>(1);

        // Split the WebSocket for concurrent read/write
        let (ws_write, ws_read) = websocket.split();

        // Spawn reader task
        let reader_handle = tokio::spawn({
            let response_tx = response_tx.clone();
            async move {
                if let Err(e) = run_ws_reader_boxed(ws_read, response_tx, shutdown_rx).await {
                    tracing::error!("WebSocket reader task error: {}", e);
                }
            }
        });

        // Spawn writer task
        let writer_handle = tokio::spawn({
            let request_rx = Arc::new(Mutex::new(request_rx));
            async move {
                if let Err(e) = run_ws_writer_boxed(ws_write, request_rx, shutdown_rx2).await {
                    tracing::error!("WebSocket writer task error: {}", e);
                }
            }
        });

        Ok(Self {
            request_tx,
            response_rx: Arc::new(Mutex::new(response_rx)),
            metadata: Arc::new(Mutex::new(None)),
            shutdown_tx: Some(shutdown_tx),
            _reader_handle: Arc::new(Mutex::new(Some(reader_handle))),
            _writer_handle: Arc::new(Mutex::new(Some(writer_handle))),
        })
    }

    /// Initialize the PTY with tab metadata
    pub async fn init(&self, metadata: TabMetadata) -> Result<()> {
        let request = PtyWebsocketRequest::Init(metadata.clone());
        self.request_tx
            .send(request)
            .await
            .context("Failed to send PTY init request")?;

        // Store metadata
        let mut meta = self.metadata.lock().await;
        *meta = Some(metadata);

        Ok(())
    }

    /// Send input to the PTY
    pub async fn send_input(&self, data: Vec<u8>) -> Result<()> {
        use tab_api::chunk::InputChunk;
        let chunk = InputChunk { data };
        let request = PtyWebsocketRequest::Input(chunk);
        self.request_tx
            .send(request)
            .await
            .context("Failed to send PTY input")?;
        Ok(())
    }

    /// Resize the PTY
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let request = PtyWebsocketRequest::Resize((cols, rows));
        self.request_tx
            .send(request)
            .await
            .context("Failed to send PTY resize request")?;
        Ok(())
    }

    /// Terminate the PTY
    pub async fn terminate(&self) -> Result<()> {
        let request = PtyWebsocketRequest::Terminate;
        self.request_tx
            .send(request)
            .await
            .context("Failed to send PTY terminate request")?;
        Ok(())
    }

    /// Receive the next response from the PTY
    pub async fn recv_response(&self) -> Option<PtyWebsocketResponse> {
        let mut rx = self.response_rx.lock().await;
        rx.recv().await
    }

    /// Get the current tab metadata
    pub async fn metadata(&self) -> Option<TabMetadata> {
        let meta = self.metadata.lock().await;
        meta.clone()
    }

    /// Shutdown the bridge gracefully
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down WebSocket PTY bridge");

        // Send shutdown signal to tasks
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }

        // Close request channel to signal writer task to stop
        // (channel is closed when all senders are dropped)

        Ok(())
    }

    /// Run the PTY event loop, forwarding output to the provided callback
    ///
    /// This method runs indefinitely until the PTY is stopped or an error occurs.
    /// The callback receives raw PTY output bytes.
    pub async fn run_pty_loop<F>(&self, mut output_callback: F) -> Result<()>
    where
        F: FnMut(Vec<u8>) + Send + 'static,
    {
        tracing::info!("Starting PTY event loop");

        loop {
            match self.recv_response().await {
                Some(PtyWebsocketResponse::Started(metadata)) => {
                    tracing::info!(
                        "PTY started for tab: {} (id: {:?})",
                        metadata.name,
                        metadata.id
                    );
                    // Update metadata
                    let mut meta = self.metadata.lock().await;
                    *meta = Some(metadata);
                }
                Some(PtyWebsocketResponse::Output(chunk)) => {
                    tracing::trace!("PTY output: {} bytes", chunk.data.len());
                    output_callback(chunk.data);
                }
                Some(PtyWebsocketResponse::Stopped) => {
                    tracing::info!("PTY stopped, ending event loop");
                    break;
                }
                None => {
                    tracing::debug!("PTY response channel closed");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get a clone of the request sender
    pub fn request_sender(&self) -> mpsc::Sender<PtyWebsocketRequest> {
        self.request_tx.clone()
    }
}

/// Type alias for the WebSocket read half
pub type WsReadHalf = futures::stream::SplitStream<WebsocketConnection>;

/// Type alias for the WebSocket write half
pub type WsWriteHalf = futures::stream::SplitSink<WebsocketConnection, tungstenite::Message>;

/// WebSocket reader task - reads from WebSocket and forwards to response channel
async fn run_ws_reader_boxed(
    mut ws_read: WsReadHalf,
    response_tx: mpsc::Sender<PtyWebsocketResponse>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> Result<()> {
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                tracing::debug!("WebSocket reader received shutdown signal");
                break;
            }
            // Read from WebSocket
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(message)) => {
                        // Decode the message
                        match tab_websocket::decode::<PtyWebsocketResponse>(Ok(message)) {
                            Ok(response) => {
                                tracing::trace!("Received PTY response: {:?}", response);
                                if response_tx.send(response).await.is_err() {
                                    tracing::error!("Failed to send response to channel, receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to decode WebSocket message: {}", e);
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket read error: {}", e);
                        break;
                    }
                    None => {
                        tracing::debug!("WebSocket stream closed");
                        break;
                    }
                }
            }
        }
    }

    // Signal that the PTY has stopped
    let _ = response_tx.send(PtyWebsocketResponse::Stopped).await;
    Ok(())
}

/// WebSocket writer task - reads from request channel and writes to WebSocket
async fn run_ws_writer_boxed(
    mut ws_write: WsWriteHalf,
    request_rx: Arc<Mutex<mpsc::Receiver<PtyWebsocketRequest>>>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> Result<()> {
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                tracing::debug!("WebSocket writer received shutdown signal");
                break;
            }
            // Read from request channel
            request = async {
                let mut rx = request_rx.lock().await;
                rx.recv().await
            } => {
                match request {
                    Some(req) => {
                        tracing::trace!("Sending PTY request: {:?}", req);
                        // Encode and send
                        match tab_websocket::encode(req) {
                            Ok(message) => {
                                if let Err(e) = ws_write.send(message).await {
                                    tracing::error!("Failed to write to WebSocket: {}", e);
                                    break;
                                }
                                // Flush the sink
                                if let Err(e) = ws_write.flush().await {
                                    tracing::error!("Failed to flush WebSocket: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to encode PTY request: {}", e);
                            }
                        }
                    }
                    None => {
                        tracing::debug!("Request channel closed, shutting down writer");
                        break;
                    }
                }
            }
        }
    }

    // Try to close the WebSocket gracefully
    let _ = ws_write.close().await;
    Ok(())
}

/// Handle incoming PTY responses (legacy callback-based handler)
pub async fn handle_pty_responses(
    bridge: Arc<WebSocketPtyBridge>,
    mut output_callback: impl FnMut(Vec<u8>) + Send + 'static,
) -> Result<()> {
    loop {
        match bridge.recv_response().await {
            Some(PtyWebsocketResponse::Started(metadata)) => {
                tracing::info!("PTY started for tab: {}", metadata.name);
            }
            Some(PtyWebsocketResponse::Output(chunk)) => {
                output_callback(chunk.data);
            }
            Some(PtyWebsocketResponse::Stopped) => {
                tracing::info!("PTY stopped");
                break;
            }
            None => {
                tracing::debug!("PTY response channel closed");
                break;
            }
        }
    }
    Ok(())
}

/// Encode a PTY request for WebSocket transmission
pub fn encode_request(request: &PtyWebsocketRequest) -> Result<Vec<u8>> {
    Ok(bincode::serialize(request)?)
}

/// Decode a PTY response from WebSocket
pub fn decode_response(data: &[u8]) -> Result<PtyWebsocketResponse> {
    Ok(bincode::deserialize(data)?)
}

/// Builder for creating a WebSocket PTY bridge with custom configuration
pub struct WebSocketPtyBridgeBuilder {
    buffer_size: usize,
}

impl WebSocketPtyBridgeBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self { buffer_size: 64 }
    }

    /// Set the channel buffer size (default: 64)
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Build and connect to the daemon
    pub async fn connect(self, daemon_config: &DaemonConfig) -> Result<WebSocketPtyBridge> {
        // For now, delegate to the standard connect method
        // Future enhancement: use buffer_size for custom channel sizes
        WebSocketPtyBridge::connect(daemon_config).await
    }
}

impl Default for WebSocketPtyBridgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_request() {
        let request = PtyWebsocketRequest::Resize((80, 24));
        let encoded = encode_request(&request).unwrap();
        let decoded: PtyWebsocketRequest = bincode::deserialize(&encoded).unwrap();

        match decoded {
            PtyWebsocketRequest::Resize((cols, rows)) => {
                assert_eq!(cols, 80);
                assert_eq!(rows, 24);
            }
            _ => panic!("Wrong request type"),
        }
    }

    #[test]
    fn test_encode_decode_response() {
        use tab_api::chunk::OutputChunk;
        use tab_api::tab::TabId;

        let response = PtyWebsocketResponse::Output(OutputChunk {
            index: 0,
            data: vec![1, 2, 3],
        });
        let encoded = bincode::serialize(&response).unwrap();
        let decoded: PtyWebsocketResponse = bincode::deserialize(&encoded).unwrap();

        match decoded {
            PtyWebsocketResponse::Output(chunk) => {
                assert_eq!(chunk.data, vec![1, 2, 3]);
            }
            _ => panic!("Wrong response type"),
        }
    }

    // Note: Async tests with channels require proper tokio runtime setup
    // These are tested in integration tests with actual daemon connection
    // The unit tests below verify the basic struct creation

    #[test]
    fn test_bridge_builder_default() {
        let builder = WebSocketPtyBridgeBuilder::default();
        assert_eq!(builder.buffer_size, 64);
    }

    #[test]
    fn test_bridge_builder_custom_buffer() {
        let builder = WebSocketPtyBridgeBuilder::new().buffer_size(128);
        assert_eq!(builder.buffer_size, 128);
    }

    #[test]
    fn test_type_aliases_exist() {
        // Verify type aliases compile
        fn _check_types() {
            let _: Option<WsReadHalf> = None;
            let _: Option<WsWriteHalf> = None;
        }
    }
}
