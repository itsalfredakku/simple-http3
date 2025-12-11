//! WebTransport session handling.
//!
//! WebTransport provides bidirectional streams and datagrams over QUIC,
//! accessible from browsers via the WebTransport API.

use bytes::Bytes;
use h3::quic::BidiStream;
use h3_webtransport::server::{AcceptedBi, WebTransportSession};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error, info};

/// Handle a WebTransport session.
///
/// This demonstrates:
/// - Server-initiated bidirectional stream
/// - Echo for client-initiated streams
/// - Datagram echo
pub async fn handle_session(
    session: WebTransportSession<h3_quinn::Connection, Bytes>,
) -> anyhow::Result<()> {
    let session_id = session.session_id();
    info!("WebTransport session established: {:?}", session_id);

    // Open a server-initiated bidirectional stream to send a welcome message
    let welcome_stream = session.open_bi(session_id).await?;
    tokio::spawn(async move {
        if let Err(e) = send_welcome(welcome_stream).await {
            debug!("Welcome stream error: {:?}", e);
        }
    });

    // Set up datagram handlers
    let mut datagram_reader = session.datagram_reader();
    let mut datagram_sender = session.datagram_sender();

    loop {
        tokio::select! {
            // Handle incoming datagrams (echo them back)
            datagram = datagram_reader.read_datagram() => {
                match datagram {
                    Ok(datagram) => {
                        let payload = datagram.into_payload();
                        debug!("Received datagram: {} bytes", payload.len());
                        if let Err(e) = datagram_sender.send_datagram(payload) {
                            error!("Failed to send datagram: {:?}", e);
                        }
                    }
                    Err(e) => {
                        debug!("Datagram reader error: {:?}", e);
                        break;
                    }
                }
            }

            // Handle incoming unidirectional streams
            uni_stream = session.accept_uni() => {
                match uni_stream {
                    Ok(Some((id, recv_stream))) => {
                        debug!("Accepted uni stream: {:?}", id);
                        // Open a uni stream back to echo
                        match session.open_uni(id).await {
                            Ok(send_stream) => {
                                tokio::spawn(async move {
                                    if let Err(e) = echo_uni(send_stream, recv_stream).await {
                                        debug!("Uni stream echo error: {:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Failed to open uni stream: {:?}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("No more uni streams");
                        break;
                    }
                    Err(e) => {
                        debug!("Uni stream accept error: {:?}", e);
                        break;
                    }
                }
            }

            // Handle incoming bidirectional streams
            bidi_stream = session.accept_bi() => {
                match bidi_stream {
                    Ok(Some(accepted)) => {
                        match accepted {
                            AcceptedBi::BidiStream(id, stream) => {
                                debug!("Accepted bidi stream: {:?}", id);
                                let (send, recv) = BidiStream::split(stream);
                                tokio::spawn(async move {
                                    if let Err(e) = echo_bidi(send, recv).await {
                                        debug!("Bidi stream echo error: {:?}", e);
                                    }
                                });
                            }
                            AcceptedBi::Request(req, stream) => {
                                // Additional HTTP/3 request within session
                                debug!("Received HTTP request in session: {:?}", req.uri());
                                drop(stream);
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("No more bidi streams");
                        break;
                    }
                    Err(e) => {
                        debug!("Bidi stream accept error: {:?}", e);
                        break;
                    }
                }
            }

            else => {
                break;
            }
        }
    }

    info!("WebTransport session ended: {:?}", session_id);
    Ok(())
}

/// Send a welcome message on a server-initiated stream.
async fn send_welcome<S>(mut stream: S) -> anyhow::Result<()>
where
    S: AsyncWriteExt + AsyncReadExt + Unpin,
{
    info!("Sending welcome message");

    // Send welcome
    let welcome = b"Welcome to HTTP/3 WebTransport server!";
    stream.write_all(welcome).await?;
    stream.shutdown().await?;

    // Read client response
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;

    if !response.is_empty() {
        info!("Client responded: {:?}", String::from_utf8_lossy(&response));
    }

    Ok(())
}

/// Echo data on a unidirectional stream pair.
async fn echo_uni<S, R>(mut send: S, mut recv: R) -> anyhow::Result<()>
where
    S: AsyncWriteExt + Unpin,
    R: AsyncReadExt + Unpin,
{
    let mut buf = Vec::new();
    recv.read_to_end(&mut buf).await?;

    debug!("Echoing {} bytes on uni stream", buf.len());

    // Send back in chunks to demonstrate streaming
    for chunk in buf.chunks(64) {
        tokio::time::sleep(Duration::from_millis(10)).await;
        send.write_all(chunk).await?;
    }

    Ok(())
}

/// Echo data on a bidirectional stream.
/// Reads messages incrementally and echoes them back immediately.
async fn echo_bidi<S, R>(mut send: S, mut recv: R) -> anyhow::Result<()>
where
    S: AsyncWriteExt + Unpin,
    R: AsyncReadExt + Unpin,
{
    let mut buf = [0u8; 4096];
    
    loop {
        match recv.read(&mut buf).await {
            Ok(0) => {
                // Stream closed
                debug!("Bidi stream closed by client");
                break;
            }
            Ok(n) => {
                let data = &buf[..n];
                debug!("Echoing {} bytes on bidi stream: {:?}", n, String::from_utf8_lossy(data));
                
                // Echo back immediately with prefix
                send.write_all(b"[echo] ").await?;
                send.write_all(data).await?;
                send.flush().await?;
            }
            Err(e) => {
                debug!("Bidi stream read error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
