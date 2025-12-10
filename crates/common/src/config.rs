//! Configuration types for server and client.

use std::net::SocketAddr;

/// Server configuration options.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the server to.
    pub bind_addr: SocketAddr,
    /// Hostnames for the self-signed certificate.
    pub cert_hostnames: Vec<String>,
    /// Idle timeout in seconds.
    pub idle_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:4433".parse().unwrap(),
            cert_hostnames: vec!["localhost".to_string()],
            idle_timeout_secs: 30,
        }
    }
}

impl ServerConfig {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            ..Default::default()
        }
    }

    pub fn with_hostnames(mut self, hostnames: Vec<String>) -> Self {
        self.cert_hostnames = hostnames;
        self
    }

    pub fn with_idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }
}

/// Client configuration options.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server address to connect to.
    pub server_addr: SocketAddr,
    /// Server name for TLS (SNI).
    pub server_name: String,
    /// Whether to skip certificate verification (for self-signed certs).
    pub insecure: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:4433".parse().unwrap(),
            server_name: "localhost".to_string(),
            insecure: true,
        }
    }
}

impl ClientConfig {
    pub fn new(server_addr: SocketAddr, server_name: impl Into<String>) -> Self {
        Self {
            server_addr,
            server_name: server_name.into(),
            ..Default::default()
        }
    }

    pub fn secure(mut self) -> Self {
        self.insecure = false;
        self
    }
}
