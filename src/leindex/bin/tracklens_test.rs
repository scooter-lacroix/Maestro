/// Quick launcher to test TrackLens UI
use leindex_core::tracklens::{
    ReviewContent, ReviewMetadata, ReviewMode, ServerConfig, TrackLensServer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig {
        port: 3847,
        host: "127.0.0.1".to_string(),
        open_browser: false,
    };
    let server = TrackLensServer::new(config);
    let url = server.start().await?;

    println!("TrackLens running at: {url}");

    // Set sample content
    server.set_content(ReviewContent {
        mode: ReviewMode::Review,
        content: "# Test Review\n\nThis is a test to verify the TrackLens UI loads correctly.\n\n## Checklist\n- [ ] UI renders\n- [ ] Content visible\n- [ ] Approve/Deny buttons work".to_string(),
        metadata: ReviewMetadata {
            track_id: Some("test-verify".to_string()),
            document_type: "markdown".to_string(),
            origin: "cli-test".to_string(),
        },
    })?;

    // Wait for client ready
    let ready = server
        .wait_for_client_ready(std::time::Duration::from_secs(15))
        .await;
    if ready.is_ok() {
        println!("✓ Client UI reported ready!");
    } else {
        println!("⚠ Client UI did not report ready within 15s");
    }

    println!("Waiting for decision (approve/deny in browser)...");
    let decision = server.wait_for_decision().await?;
    println!("Decision: {:?}", decision.behavior);

    Ok(())
}
