use bytes::Bytes;
use h3::server::RequestStream;
use http::{Request, Response, StatusCode};
use quinn::{Endpoint, ServerConfig};
use rustls::pki_types::PrivateKeyDer;
use rustls::ServerConfig as TlsServerConfig;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Install the AWS LC crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let cert = generate_self_signed_cert()?;
    let mut tls_config = TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain, cert.private_key)?;
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?,
    ));

    let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;

    info!("HTTP/3 server listening on 127.0.0.1:4433");

    while let Some(conn) = endpoint.accept().await {
        let conn = conn.await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(conn).await {
                error!("Connection error: {:?}", e);
            }
        });
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Connection) -> anyhow::Result<()> {
    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
        .await
        .unwrap();

    loop {
        match h3_conn.accept().await {
            Ok(Some(req_resolver)) => {
                tokio::spawn(async move {
                    let (req, stream) = match req_resolver.resolve_request().await {
                        Ok(resolved) => resolved,
                        Err(e) => {
                            error!("Failed to resolve request: {:?}", e);
                            return;
                        }
                    };
                    if let Err(e) = handle_request(req, stream).await {
                        error!("Request error: {:?}", e);
                    }
                });
            }
            Ok(None) => {
                // Connection closed gracefully
                break;
            }
            Err(e) => {
                // Timeout is expected when client disconnects - check via Debug string
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
) -> anyhow::Result<()> {
    info!(
        "Got request for path: {}, protocol: {:?}",
        req.uri().path(),
        req.version()
    );

    let response_body = match req.uri().path() {
        "/" => "hello from http3",
        "/test" => "hello from http3 test endpoint",
        "/health" => "hello from http3 health check",
        _ => "hello from http3 - unknown endpoint",
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain")
        .body(())?;

    stream.send_response(response).await?;
    stream.send_data(Bytes::from(response_body)).await?;
    stream.finish().await?;

    Ok(())
}

struct CertificateChain {
    cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>,
}

fn generate_self_signed_cert() -> anyhow::Result<CertificateChain> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let private_key = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let cert_chain = vec![cert.cert.der().clone()];

    Ok(CertificateChain {
        cert_chain,
        private_key,
    })
}
