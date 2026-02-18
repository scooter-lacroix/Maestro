//! Maestro Gateway binary
//!
//! Run the Maestro web gateway server.

use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use maestro_gateway::{server, state::GatewayConfig};

#[derive(Parser, Debug)]
#[command(name = "maestro-gateway")]
#[command(about = "Maestro Web Gateway with SSE/WebSocket streaming")]
struct Args {
    /// Bind address
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Maximum connections
    #[arg(long, default_value_t = 100)]
    max_connections: usize,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 30)]
    timeout: u64,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("maestro_gateway={}", args.log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = GatewayConfig {
        bind_address: args.bind,
        port: args.port,
        max_connections: args.max_connections,
        request_timeout_secs: args.timeout,
        ..Default::default()
    };

    info!(
        "Starting Maestro Gateway on {}:{}",
        config.bind_address, config.port
    );

    // Run with shutdown signal
    tokio::select! {
        result = server::run(config) => {
            if let Err(e) = result {
                eprintln!("Gateway error: {}", e);
                return Err(e);
            }
        }
        _ = server::shutdown_signal() => {
            info!("Shutting down gateway...");
        }
    }

    info!("Gateway stopped");
    Ok(())
}
