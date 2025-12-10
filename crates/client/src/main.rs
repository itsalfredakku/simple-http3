//! HTTP/3 Client
//!
//! Demonstrates:
//! - REST-style requests (request/response)
//! - Streaming requests (receiving multiple chunks)
//! - Graceful connection shutdown

use bytes::Buf;
use common::{tls::insecure_verifier, ClientConfig};
use http::{Request, Uri};
use quinn::Endpoint;
use rustls::ClientConfig as TlsClientConfig;
use std::sync::Arc;
use tracing::{info, warn};

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

    // Configure the client
    let config = ClientConfig::default();

    // Create client TLS config
    let mut tls_config = TlsClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(insecure_verifier())
        .with_no_client_auth();

    if !config.insecure {
        warn!("Secure mode requested but using insecure verifier for demo");
    }

    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
    ));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    info!("Connecting to {}...", config.server_addr);

    let conn = endpoint
        .connect(config.server_addr, &config.server_name)?
        .await?;

    info!("Connected!\n");

    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await?;

    // Spawn connection driver
    let driver_handle = tokio::spawn(async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx)).await
    });

    // =========================================================================
    // REST Requests
    // =========================================================================
    info!("=== REST Requests ===\n");

    let rest_paths = vec!["/", "/health", "/api/info", "/not-found"];

    for path in rest_paths {
        let uri: Uri = format!(
            "https://{}:{}{}",
            config.server_name,
            config.server_addr.port(),
            path
        )
        .parse()?;

        let req = Request::builder().method("GET").uri(uri).body(())?;

        info!("GET {}", path);
        let mut stream = send_request.send_request(req).await?;
        stream.finish().await?;

        let response = stream.recv_response().await?;
        info!("  Status: {}", response.status());

        // Read response body
        let body = read_body(&mut stream).await?;
        info!("  Body: {}\n", body);
    }

    // =========================================================================
    // Streaming Request
    // =========================================================================
    info!("=== Streaming Request ===\n");

    let uri: Uri = format!(
        "https://{}:{}/stream/time",
        config.server_name,
        config.server_addr.port()
    )
    .parse()?;

    let req = Request::builder().method("GET").uri(uri).body(())?;

    info!("GET /stream/time (SSE stream)");
    let mut stream = send_request.send_request(req).await?;
    stream.finish().await?;

    let response = stream.recv_response().await?;
    info!("  Status: {}", response.status());
    info!("  Content-Type: {:?}", response.headers().get("content-type"));
    info!("  Receiving chunks:");

    // Read streaming chunks as they arrive
    while let Some(mut chunk) = stream.recv_data().await? {
        while chunk.has_remaining() {
            let bytes = chunk.chunk();
            let text = String::from_utf8_lossy(bytes);
            // Print each line
            for line in text.lines() {
                if !line.is_empty() {
                    info!("    {}", line);
                }
            }
            chunk.advance(bytes.len());
        }
    }
    info!("");

    // =========================================================================
    // Clean Shutdown
    // =========================================================================
    info!("=== Closing Connection ===");

    // Drop send_request to signal we're done sending
    drop(send_request);

    // Wait for driver to finish (handles GOAWAY)
    let _ = driver_handle.await;

    // Wait for endpoint to be fully idle
    endpoint.wait_idle().await;

    info!("Connection closed cleanly");

    Ok(())
}

/// Read the entire response body into a string.
async fn read_body<S, B>(stream: &mut h3::client::RequestStream<S, B>) -> anyhow::Result<String>
where
    S: h3::quic::RecvStream,
    B: bytes::Buf,
{
    let mut body = Vec::new();
    while let Some(mut chunk) = stream.recv_data().await? {
        while chunk.has_remaining() {
            let bytes = chunk.chunk();
            body.extend_from_slice(bytes);
            chunk.advance(bytes.len());
        }
    }
    Ok(String::from_utf8_lossy(&body).to_string())
}
