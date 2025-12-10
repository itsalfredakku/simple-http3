//! Common utilities shared between HTTP/3 server and client.
//!
//! This crate provides:
//! - TLS certificate generation and handling
//! - Configuration types
//! - Common error types

pub mod config;
pub mod tls;

pub use config::{ClientConfig, ServerConfig};
pub use tls::{generate_self_signed_cert, CertificateChain, InsecureCertVerifier};
