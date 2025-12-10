# Simple HTTP/3

A minimal, extensible HTTP/3 server and client implementation in Rust — perfect as a boilerplate for experimentation or learning.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## What is HTTP/3?

HTTP/3 is the latest version of the HTTP protocol, built on top of **QUIC** instead of TCP. Key benefits:

- **Faster connections** — 0-RTT and 1-RTT handshakes (vs TCP+TLS 3-RTT)
- **No head-of-line blocking** — streams are independent
- **Connection migration** — seamlessly switch networks (WiFi → cellular)
- **Built-in encryption** — TLS 1.3 is mandatory

```
┌─────────────────────────────────────────┐
│              Application                │
├─────────────────────────────────────────┤
│                HTTP/3                   │
├─────────────────────────────────────────┤
│                 QUIC                    │
├─────────────────────────────────────────┤
│               UDP + TLS                 │
└─────────────────────────────────────────┘
```

## Features

- ✅ HTTP/3 over QUIC (using Quinn + h3)
- ✅ Self-signed TLS certificates (auto-generated)
- ✅ Extensible routing system
- ✅ Modular architecture for easy customization
- ✅ Shared `common` crate for reusable code

## Project Structure

```
simple-http3/
├── Cargo.toml                 # Workspace configuration
├── crates/
│   ├── common/                # Shared utilities
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs         # Re-exports
│   │       ├── config.rs      # Server/Client configuration
│   │       └── tls.rs         # TLS utilities & cert generation
│   ├── server/                # HTTP/3 server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs        # Entry point & route definitions
│   │       ├── router.rs      # Simple path-based router
│   │       └── server.rs      # Server implementation
│   └── client/                # HTTP/3 client
│       ├── Cargo.toml
│       └── src/
│           └── main.rs        # Client implementation
└── README.md
```

## Quick Start

### Build

```bash
cargo build --release
```

### Run the Server

```bash
cargo run -p server --release
```

Or:
```bash
./target/release/server
```

Output:
```
INFO  server > HTTP/3 server listening on 127.0.0.1:4433
INFO  server > Available routes: ["/", "/test", "/health", "/json"]
```

### Run the Client

In another terminal:

```bash
cargo run -p client --release
```

Or:
```bash
./target/release/client
```

## Extending the Server

### Adding Routes

Edit `crates/server/src/main.rs`:

```rust
let router = router::Router::new()
    .route("/", |_req| async { "Hello from HTTP/3!" })
    .route("/api/users", |_req| async { 
        r#"[{"id": 1, "name": "Alice"}]"# 
    })
    .route("/health", |_req| async { "OK" });
```

### Custom Configuration

```rust
use common::ServerConfig;

let config = ServerConfig::new("0.0.0.0:443".parse()?)
    .with_hostnames(vec!["example.com".to_string()])
    .with_idle_timeout(60);
```

### Adding Middleware (Future)

The router can be extended with middleware patterns:

```rust
// Example pattern for future extension
router
    .middleware(logging_middleware)
    .middleware(auth_middleware)
    .route("/protected", handler)
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [quinn](https://crates.io/crates/quinn) | 0.11 | QUIC transport |
| [h3](https://crates.io/crates/h3) | 0.0.8 | HTTP/3 protocol |
| [h3-quinn](https://crates.io/crates/h3-quinn) | 0.0.10 | h3 + Quinn integration |
| [rustls](https://crates.io/crates/rustls) | 0.23 | TLS with AWS LC crypto |
| [tokio](https://crates.io/crates/tokio) | 1.x | Async runtime |

## Alternative QUIC/HTTP3 Libraries

| Library | Maintainer | Notes |
|---------|------------|-------|
| **quinn + h3** (this project) | Community | Pure Rust, will integrate with hyper |
| [s2n-quic](https://crates.io/crates/s2n-quic) | AWS | Production-ready, requires Linux 5.0+ |
| [quiche](https://crates.io/crates/quiche) | Cloudflare | Powers Cloudflare edge, uses BoringSSL |

## Testing with curl

Once [curl with HTTP/3](https://curl.se/docs/http3.html) is available:

```bash
curl --http3 https://localhost:4433/
curl --http3 https://localhost:4433/health
```

## License

MIT
