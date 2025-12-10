//! HTTP/3 server implementation.

use crate::router::{Handler, Router};
use bytes::Bytes;
use common::{tls::generate_self_signed_cert, ServerConfig};
use h3::server::RequestStream;
use http::{Request, Response, StatusCode};
use quinn::{Endpoint, ServerConfig as QuinnServerConfig};
use rustls::ServerConfig as TlsServerConfig;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Run the HTTP/3 server with the given configuration and router.
pub async fn run(config: ServerConfig, router: Router) -> anyhow::Result<()> {
    let cert = generate_self_signed_cert(&config.cert_hostnames)?;

    let mut tls_config = TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain, cert.private_key)?;
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let mut server_config = QuinnServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?,
    ));

    // Set transport config for idle timeout
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.max_idle_timeout(Some(
        std::time::Duration::from_secs(config.idle_timeout_secs)
            .try_into()
            .unwrap(),
    ));
    server_config.transport_config(Arc::new(transport_config));

    let endpoint = Endpoint::server(server_config, config.bind_addr)?;
    let router = Arc::new(router);

    info!("HTTP/3 server listening on {}", config.bind_addr);
    info!("Routes: {:?}", router.routes());

    while let Some(incoming) = endpoint.accept().await {
        let router = Arc::clone(&router);

        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    let remote = conn.remote_address();
                    debug!("New connection from {}", remote);

                    if let Err(e) = handle_connection(conn, router).await {
                        error!("Connection error from {}: {:?}", remote, e);
                    }
                }
                Err(e) => {
                    error!("Failed to accept connection: {:?}", e);
                }
            }
        });
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Connection, router: Arc<Router>) -> anyhow::Result<()> {
    let remote = conn.remote_address();
    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await?;

    loop {
        match h3_conn.accept().await {
            Ok(Some(req_resolver)) => {
                let router = Arc::clone(&router);
                tokio::spawn(async move {
                    let (req, stream) = match req_resolver.resolve_request().await {
                        Ok(resolved) => resolved,
                        Err(e) => {
                            error!("Failed to resolve request: {:?}", e);
                            return;
                        }
                    };
                    if let Err(e) = handle_request(req, stream, &router).await {
                        debug!("Request handling ended: {:?}", e);
                    }
                });
            }
            Ok(None) => {
                // Client closed connection gracefully (GOAWAY)
                debug!("Connection closed by client: {}", remote);
                break;
            }
            Err(e) => {
                // Check error type
                let err_str = format!("{:?}", e);
                if err_str.contains("Timeout") {
                    debug!("Connection timed out: {}", remote);
                } else if err_str.contains("H3_NO_ERROR") || err_str.contains("ApplicationClose") {
                    // H3_NO_ERROR is a graceful close initiated by client
                    debug!("Connection closed gracefully: {}", remote);
                } else if err_str.contains("Reset") || err_str.contains("Closed") {
                    debug!("Connection reset: {}", remote);
                } else {
                    error!("Connection error from {}: {:?}", remote, e);
                }
                break;
            }
        }
    }

    Ok(())
}

async fn handle_request(
    req: Request<()>,
    stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    router: &Router,
) -> anyhow::Result<()> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    info!("{} {}", method, path);

    match router.get(&path) {
        Some(Handler::Rest(handler)) => {
            handle_rest_request(req, stream, handler).await?;
        }
        Some(Handler::Stream(handler)) => {
            // Stream handler takes ownership and manages the stream
            handler(req, stream).await?;
        }
        None => {
            handle_not_found(stream).await?;
        }
    }

    Ok(())
}

async fn handle_rest_request(
    req: Request<()>,
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    handler: &crate::router::BoxedRestHandler,
) -> anyhow::Result<()> {
    let resp = handler(req).await;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", resp.content_type)
        .header("content-length", resp.body.len())
        .body(())?;

    stream.send_response(response).await?;
    stream.send_data(Bytes::from(resp.body)).await?;
    stream.finish().await?;

    Ok(())
}

async fn handle_not_found(
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
) -> anyhow::Result<()> {
    let body = r#"{"error": "Not Found"}"#;

    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "application/json")
        .header("content-length", body.len())
        .body(())?;

    stream.send_response(response).await?;
    stream.send_data(Bytes::from(body)).await?;
    stream.finish().await?;

    Ok(())
}
