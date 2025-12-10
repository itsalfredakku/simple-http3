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
- ✅ REST-style request/response handlers
- ✅ Server-Sent Events (SSE) streaming
- ✅ Self-signed TLS certificates (auto-generated)
- ✅ Graceful connection handling
- ✅ Modular architecture for easy customization

## Quick Start

### Build

```bash
cargo build --release
```

### Run the Server

```bash
./target/release/server
```

### Run the Client

In another terminal:

```bash
./target/release/client
```

## Sample Output

### Server

```
$ ./target/release/server
INFO Starting HTTP/3 server
INFO HTTP/3 server listening on 127.0.0.1:4433
INFO Routes: ["/api/info", "/health", "/stream/counter", "/", "/stream/time"]
INFO GET /
INFO GET /health
INFO GET /api/info
INFO GET /not-found
INFO GET /stream/time
INFO   Streaming chunk 1/5
INFO   Streaming chunk 2/5
INFO   Streaming chunk 3/5
INFO   Streaming chunk 4/5
INFO   Streaming chunk 5/5
INFO   Stream completed
```

### Client

```
$ ./target/release/client
INFO Connecting to 127.0.0.1:4433...
INFO Connected!

INFO === REST Requests ===

INFO GET /
INFO   Status: 200 OK
INFO   Body: Hello from HTTP/3!

INFO GET /health
INFO   Status: 200 OK
INFO   Body: {"status": "healthy", "protocol": "h3"}

INFO GET /api/info
INFO   Status: 200 OK
INFO   Body: {"name": "simple-http3", "version": "0.1.0", "endpoints": [...]}

INFO GET /not-found
INFO   Status: 404 Not Found
INFO   Body: {"error": "Not Found"}

INFO === Streaming Request ===

INFO GET /stream/time (SSE stream)
INFO   Status: 200 OK
INFO   Content-Type: Some("text/event-stream")
INFO   Receiving chunks:
INFO     event: time
INFO     data: 2025-12-10 14:29:25 UTC
INFO     id: 1
INFO     event: time
INFO     data: 2025-12-10 14:29:26 UTC
INFO     id: 2
      ... (1 second intervals)
INFO     event: done
INFO     data: stream complete

INFO === Closing Connection ===
INFO Connection closed cleanly
```

## Project Structure

```
simple-http3/
├── Cargo.toml                 # Workspace configuration
├── crates/
│   ├── common/                # Shared utilities
│   │   └── src/
│   │       ├── lib.rs         # Re-exports
│   │       ├── config.rs      # Server/Client configuration
│   │       └── tls.rs         # TLS & cert generation
│   ├── server/                # HTTP/3 server
│   │   └── src/
│   │       ├── main.rs        # Entry point & routes
│   │       ├── handlers.rs    # REST & streaming handlers
│   │       ├── router.rs      # Path-based router
│   │       └── server.rs      # Server implementation
│   └── client/                # HTTP/3 client
│       └── src/
│           └── main.rs        # Client implementation
└── README.md
```

## API Endpoints

| Endpoint | Type | Description |
|----------|------|-------------|
| `GET /` | REST | Hello message |
| `GET /health` | REST | Health check (JSON) |
| `GET /api/info` | REST | API information |
| `GET /stream/time` | SSE | Pushes time every second (5x) |
| `GET /stream/counter` | Stream | Counter with JSON lines |

## Extending the Server

### Adding REST Routes

```rust
let router = Router::new()
    .route("/", handlers::index)
    .route("/api/users", |_req| async {
        RestResponse::json(r#"[{"id": 1, "name": "Alice"}]"#)
    });
```

### Adding Streaming Routes

```rust
let router = Router::new()
    .stream("/stream/events", |_req, mut stream| async move {
        stream.send_response(response).await?;
        for i in 1..=10 {
            stream.send_data(Bytes::from(format!("event {}\n", i))).await?;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        stream.finish().await?;
        Ok(())
    });
```

### Custom Configuration

```rust
use common::ServerConfig;

let config = ServerConfig::new("0.0.0.0:443".parse()?)
    .with_hostnames(vec!["example.com".to_string()])
    .with_idle_timeout(60);
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
