//! HTTP/3 server implementation.

use crate::router::Router;
use bytes::Bytes;
use common::{tls::generate_self_signed_cert, ServerConfig};
use h3::server::RequestStream;
use http::{Request, Response, StatusCode};
use quinn::{Endpoint, ServerConfig as QuinnServerConfig};
use rustls::ServerConfig as TlsServerConfig;
use std::sync::Arc;
use tracing::{error, info};

/// Run the HTTP/3 server with the given configuration and router.
pub async fn run(config: ServerConfig, router: Router) -> anyhow::Result<()> {
    let cert = generate_self_signed_cert(&config.cert_hostnames)?;
    
    let mut tls_config = TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain, cert.private_key)?;
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let server_config = QuinnServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?,
    ));

    let endpoint = Endpoint::server(server_config, config.bind_addr)?;
    let router = Arc::new(router);

    info!("HTTP/3 server listening on {}", config.bind_addr);
    info!("Available routes: {:?}", router.routes());

    while let Some(conn) = endpoint.accept().await {
        let conn = conn.await?;
        let router = Arc::clone(&router);
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(conn, router).await {
                error!("Connection error: {:?}", e);
            }
        });
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Connection, router: Arc<Router>) -> anyhow::Result<()> {
    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
        .await
        .unwrap();

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
                        error!("Request error: {:?}", e);
                    }
                });
            }
            Ok(None) => {
                // Connection closed gracefully
                break;
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("Timeout") {
                    info!("Connection closed due to idle timeout");
                } else {
                    error!("Error accepting request: {:?}", e);
                }
                break;
            }
        }
    }

    Ok(())
}

async fn handle_request(
    req: Request<()>,
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    router: &Router,
) -> anyhow::Result<()> {
    info!(
        "{} {} - {:?}",
        req.method(),
        req.uri().path(),
        req.version()
    );

    let (body, content_type) = router.handle(req).await;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .body(())?;

    stream.send_response(response).await?;
    stream.send_data(Bytes::from(body)).await?;
    stream.finish().await?;

    Ok(())
}
