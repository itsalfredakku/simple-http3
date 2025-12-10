//! HTTP/3 Server
//!
//! A modular HTTP/3 server implementation demonstrating:
//! - QUIC transport with Quinn
//! - HTTP/3 protocol handling with h3
//! - Extensible routing
//! - Self-signed TLS certificates

mod router;
mod server;

use common::ServerConfig;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Install the AWS LC crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    // Configure the server
    let config = ServerConfig::default()
        .with_hostnames(vec!["localhost".to_string()])
        .with_idle_timeout(30);

    info!("Starting HTTP/3 server on {}", config.bind_addr);

    // Create router with routes
    let router = router::Router::new()
        .route("/", |_req| async { "Hello from HTTP/3!" })
        .route("/test", |_req| async { "Hello from HTTP/3 test endpoint" })
        .route("/health", |_req| async { "OK" })
        .route("/json", |_req| async { r#"{"status": "ok", "protocol": "h3"}"# });

    // Start the server
    server::run(config, router).await
}
