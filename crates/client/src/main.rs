//! HTTP/3 Client
//!
//! A simple HTTP/3 client demonstrating:
//! - QUIC transport with Quinn
//! - HTTP/3 protocol handling with h3
//! - Self-signed certificate handling

use bytes::Buf;
use common::{tls::insecure_verifier, ClientConfig};
use http::{Request, Uri};
use quinn::Endpoint;
use rustls::ClientConfig as TlsClientConfig;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

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
        // Note: For production with real certificates, you'd need to configure
        // proper certificate verification. This example uses insecure mode.
        tracing::warn!("Secure mode requested but using insecure verifier for demo");
    }
    
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
    ));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    let conn = endpoint
        .connect(config.server_addr, &config.server_name)?
        .await?;

    info!("Connected to server at {}", config.server_addr);

    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await?;

    // Spawn connection driver
    tokio::spawn(async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    // Make requests to different endpoints
    let paths = vec!["/", "/test", "/health", "/json", "/unknown"];

    for path in paths {
        let uri: Uri = format!("https://{}:{}{}", config.server_name, config.server_addr.port(), path).parse()?;
        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .body(())?;

        info!("GET {}", path);
        let mut stream = send_request.send_request(req).await?;
        stream.finish().await?;

        let response = stream.recv_response().await?;
        info!("  Status: {}", response.status());

        // Read response body
        let mut body = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await? {
            while chunk.has_remaining() {
                let bytes = chunk.chunk();
                body.extend_from_slice(bytes);
                chunk.advance(bytes.len());
            }
        }
        let body_str = String::from_utf8_lossy(&body);
        info!("  Body: {}", body_str);
        println!();
    }

    Ok(())
}
