//! HTTP/3 Server
//!
//! A modular HTTP/3 server demonstrating:
//! - REST-style request/response handlers
//! - Server-Sent Events (SSE) streaming
//! - WebTransport bidirectional streams and datagrams
//! - QUIC transport with Quinn
//! - Self-signed TLS certificates

mod handlers;
mod router;
mod server;
mod webtransport;

use common::ServerConfig;
use router::Router;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    // Install the AWS LC crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    // Configure the server
    let config = ServerConfig::default()
        .with_hostnames(vec![
            "localhost".to_string(),
            "127.0.0.1".to_string(),
        ])
        .with_idle_timeout(10); // 10 seconds for demo

    info!("Starting HTTP/3 server");

    // Create router with REST and streaming routes
    let router = Router::new()
        // REST endpoints (request → response → done)
        .route("/", handlers::index)
        .route("/health", handlers::health)
        .route("/api/info", handlers::api_info)
        // Streaming endpoints (server pushes multiple chunks)
        .stream("/stream/time", handlers::time_stream)
        .stream("/stream/counter", handlers::counter_stream);

    // Start the server
    server::run(config, router).await
}
