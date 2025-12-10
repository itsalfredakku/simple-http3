# HTTP/3 Server & Client

A simple HTTP/3 server and client implementation using QUIC (Quinn) and the h3 crate in Rust.

## Features

- HTTP/3 over QUIC protocol
- Self-signed TLS certificates (auto-generated)
- Multiple endpoint routing (`/`, `/test`, `/health`)

## Building

```bash
cargo build --release
```

## Running

### Start the Server

```bash
cargo run -p server --release
```

Or run the binary directly:

```bash
./target/release/server
```

The server listens on `127.0.0.1:4433`.

### Run the Client

In a separate terminal:

```bash
cargo run -p client --release
```

Or run the binary directly:

```bash
./target/release/client
```

The client makes requests to `/`, `/test`, `/health`, and `/unknown` endpoints.

## Project Structure

```
├── Cargo.toml              # Workspace configuration
├── crates/
│   ├── server/             # HTTP/3 server
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── client/             # HTTP/3 client
│       ├── Cargo.toml
│       └── src/main.rs
```

## Dependencies

- **quinn** - QUIC implementation
- **h3 / h3-quinn** - HTTP/3 protocol
- **rustls** - TLS with AWS LC crypto provider
- **tokio** - Async runtime
