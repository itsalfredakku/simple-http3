//! Request handlers for REST and streaming endpoints.

use crate::router::RestResponse;
use bytes::Bytes;
use h3::server::RequestStream;
use http::{Request, Response, StatusCode};
use std::time::Duration;
use tracing::info;

// =============================================================================
// REST Handlers
// =============================================================================

/// Index handler.
pub async fn index(_req: Request<()>) -> RestResponse {
    RestResponse::text("Hello from HTTP/3!")
}

/// Health check handler.
pub async fn health(_req: Request<()>) -> RestResponse {
    RestResponse::json(r#"{"status": "healthy", "protocol": "h3"}"#)
}

/// JSON API example.
pub async fn api_info(_req: Request<()>) -> RestResponse {
    RestResponse::json(
        r#"{"name": "simple-http3", "version": "0.1.0", "endpoints": ["/", "/health", "/api/info", "/stream/time", "/stream/counter"]}"#,
    )
}

// =============================================================================
// Streaming Handlers
// =============================================================================

/// Server-Sent Events style: pushes current time every second for 5 iterations.
///
/// Demonstrates server-push pattern where client receives multiple data chunks
/// over a single stream.
pub async fn time_stream(
    _req: Request<()>,
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
) -> anyhow::Result<()> {
    // Send response headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(())?;

    stream.send_response(response).await?;

    // Push time updates
    for i in 1..=5 {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let event = format!("event: time\ndata: {}\nid: {}\n\n", now, i);

        info!("  Streaming chunk {}/5", i);
        stream.send_data(Bytes::from(event)).await?;

        if i < 5 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    // Signal end of stream
    stream.send_data(Bytes::from("event: done\ndata: stream complete\n\n")).await?;
    stream.finish().await?;

    info!("  Stream completed");
    Ok(())
}

/// Counter stream: demonstrates a simple counting stream.
pub async fn counter_stream(
    _req: Request<()>,
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
) -> anyhow::Result<()> {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-ndjson")
        .body(())?;

    stream.send_response(response).await?;

    for i in 1..=10 {
        let json = format!(r#"{{"count": {}, "timestamp": {}}}"#, i, chrono::Utc::now().timestamp());
        let line = format!("{}\n", json);

        stream.send_data(Bytes::from(line)).await?;

        if i < 10 {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    stream.finish().await?;
    info!("  Counter stream completed");
    Ok(())
}
