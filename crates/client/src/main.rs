use bytes::Buf;
use http::{Request, Uri};
use quinn::Endpoint;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
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

    // Create client TLS config that accepts any certificate (for self-signed certs)
    let mut tls_config = TlsClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    tls_config.alpn_protocols = vec![b"h3".to_vec()];

    let client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
    ));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    let server_addr = "127.0.0.1:4433".parse()?;
    let conn = endpoint.connect(server_addr, "localhost")?.await?;

    info!("Connected to server");

    let quinn_conn = h3_quinn::Connection::new(conn);
    let (mut driver, mut send_request) = h3::client::new(quinn_conn).await?;

    // Spawn connection driver
    tokio::spawn(async move {
        futures::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    // Make requests to different endpoints
    let paths = vec!["/", "/test", "/health", "/unknown"];

    for path in paths {
        let uri: Uri = format!("https://localhost:4433{}", path).parse()?;
        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .body(())?;

        info!("Sending request to {}", path);
        let mut stream = send_request.send_request(req).await?;
        stream.finish().await?;

        let response = stream.recv_response().await?;
        info!("Response status: {}", response.status());

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
        info!("Response body: {}", body_str);
        println!("---");
    }

    Ok(())
}

/// Custom certificate verifier that skips verification (for self-signed certs)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
