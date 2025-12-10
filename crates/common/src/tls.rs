//! TLS certificate utilities.

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::sync::Arc;

/// A certificate chain with its private key.
pub struct CertificateChain {
    pub cert_chain: Vec<CertificateDer<'static>>,
    pub private_key: PrivateKeyDer<'static>,
}

/// Generate a self-signed certificate for the given hostnames.
///
/// # Example
/// ```
/// use common::tls::generate_self_signed_cert;
///
/// let cert = generate_self_signed_cert(&["localhost".to_string()]).unwrap();
/// ```
pub fn generate_self_signed_cert(hostnames: &[String]) -> anyhow::Result<CertificateChain> {
    let cert = rcgen::generate_simple_self_signed(hostnames.to_vec())?;
    let private_key = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let cert_chain = vec![cert.cert.der().clone()];

    Ok(CertificateChain {
        cert_chain,
        private_key,
    })
}

/// Certificate verifier that skips verification (for development/testing only).
///
/// # Warning
/// This should NEVER be used in production. It accepts any certificate,
/// making the connection vulnerable to MITM attacks.
#[derive(Debug)]
pub struct InsecureCertVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureCertVerifier {
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

/// Create an insecure certificate verifier wrapped in Arc.
pub fn insecure_verifier() -> Arc<InsecureCertVerifier> {
    Arc::new(InsecureCertVerifier)
}
