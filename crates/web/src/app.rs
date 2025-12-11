//! Leptos WebTransport Demo Application.

use crate::transport::{BidiStream, WebTransportClient};
use leptos::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen_futures::spawn_local;

/// Shared client state using Rc<RefCell<>> for non-Clone types
type SharedClient = Rc<RefCell<Option<WebTransportClient>>>;
type SharedStream = Rc<RefCell<Option<BidiStream>>>;

/// Parse a hex string to bytes
fn parse_hex(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim().replace(' ', "").replace(':', "");
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Main application component.
#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal("Disconnected".to_string());
    let (messages, set_messages) = signal(Vec::<String>::new());
    let (input, set_input) = signal(String::new());
    let (cert_hash, set_cert_hash) = signal(String::new());
    let (connected, set_connected) = signal(false);
    let (has_stream, set_has_stream) = signal(false);

    // Use Rc<RefCell> for non-Clone client and stream
    let client: SharedClient = Rc::new(RefCell::new(None));
    let stream: SharedStream = Rc::new(RefCell::new(None));

    // Connect handler
    let client_connect = Rc::clone(&client);
    let stream_connect = Rc::clone(&stream);
    let connect = move |_| {
        let client = Rc::clone(&client_connect);
        let stream = Rc::clone(&stream_connect);
        let hash_input = cert_hash.get();

        spawn_local(async move {
            set_status.set("Connecting...".to_string());

            // Parse cert hash if provided
            let cert_hash_bytes = if !hash_input.is_empty() {
                match parse_hex(&hash_input) {
                    Some(bytes) if bytes.len() == 32 => Some(bytes),
                    Some(_) => {
                        add_message(&set_messages, "✗ Certificate hash must be 32 bytes (64 hex chars)");
                        set_status.set("Connection failed".to_string());
                        return;
                    }
                    None => {
                        add_message(&set_messages, "✗ Invalid hex format for certificate hash");
                        set_status.set("Connection failed".to_string());
                        return;
                    }
                }
            } else {
                None
            };

            let result = WebTransportClient::connect(
                "https://127.0.0.1:4433/webtransport",
                // "https://localhost:4433/webtransport",
                cert_hash_bytes.as_deref(),
            ).await;

            match result {
                Ok(c) => {
                    add_message(&set_messages, "✓ Connected to server");
                    set_status.set("Connected".to_string());
                    set_connected.set(true);

                    // Store the client
                    *client.borrow_mut() = Some(c.clone());

                    // Open a bidirectional stream
                    match c.open_bidi_stream().await {
                        Ok(s) => {
                            add_message(&set_messages, "✓ Opened bidirectional stream");

                            // Store the stream before using it
                            *stream.borrow_mut() = Some(s.clone());
                            set_has_stream.set(true);

                            // Read welcome message
                            match s.recv().await {
                                Ok(data) => {
                                    let msg = String::from_utf8_lossy(&data);
                                    add_message(&set_messages, &format!("Server: {}", msg));
                                }
                                Err(e) => {
                                    add_message(
                                        &set_messages,
                                        &format!("Read error: {:?}", e),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            add_message(&set_messages, &format!("Stream error: {:?}", e));
                        }
                    }
                }
                Err(e) => {
                    set_status.set("Connection failed".to_string());
                    add_message(&set_messages, &format!("✗ Connection error: {:?}", e));
                }
            }
        });
    };

    // Send message handler
    let stream_send = Rc::clone(&stream);
    let send_message = move |_| {
        let msg = input.get();
        if msg.is_empty() {
            return;
        }
        set_input.set(String::new());
        
        // Clone the stream out of RefCell before the async block
        let stream_clone = stream_send.borrow().clone();

        spawn_local(async move {
            if let Some(s) = stream_clone {
                add_message(&set_messages, &format!("You: {}", msg));

                if let Err(e) = s.send(msg.as_bytes()).await {
                    add_message(&set_messages, &format!("Send error: {:?}", e));
                    return;
                }

                // Wait for echo response
                match s.recv().await {
                    Ok(data) => {
                        let response = String::from_utf8_lossy(&data);
                        add_message(&set_messages, &format!("Server: {}", response));
                    }
                    Err(e) => {
                        add_message(&set_messages, &format!("Recv error: {:?}", e));
                    }
                }
            } else {
                add_message(&set_messages, "Not connected");
            }
        });
    };
    let send_message_clone = send_message.clone();

    // Send datagram handler
    let client_datagram = Rc::clone(&client);
    let send_datagram = move |_| {
        // Clone the client out of RefCell before the async block
        let client_clone = client_datagram.borrow().clone();

        spawn_local(async move {
            if let Some(c) = client_clone {
                let data = b"Hello via datagram!";
                add_message(&set_messages, "Datagram sent: Hello via datagram!");

                if let Err(e) = c.send_datagram(data).await {
                    add_message(&set_messages, &format!("Datagram error: {:?}", e));
                    return;
                }

                // Try to receive datagram response
                match c.recv_datagram().await {
                    Ok(data) => {
                        let msg = String::from_utf8_lossy(&data);
                        add_message(&set_messages, &format!("Datagram received: {}", msg));
                    }
                    Err(e) => {
                        add_message(&set_messages, &format!("Datagram recv error: {:?}", e));
                    }
                }
            } else {
                add_message(&set_messages, "Not connected");
            }
        });
    };

    // Disconnect handler
    let client_disconnect = Rc::clone(&client);
    let stream_disconnect = Rc::clone(&stream);
    let disconnect = move |_| {
        if let Some(c) = client_disconnect.borrow().as_ref() {
            c.close();
        }
        *client_disconnect.borrow_mut() = None;
        *stream_disconnect.borrow_mut() = None;
        set_connected.set(false);
        set_has_stream.set(false);
        set_status.set("Disconnected".to_string());
        add_message(&set_messages, "Disconnected");
    };

    view! {
        <div class="container">
            <h1>"WebTransport Demo"</h1>

            <div class="status">
                <span class="label">"Status: "</span>
                <span class="value">{move || status.get()}</span>
            </div>

            <div class="cert-hash">
                <label>"Certificate SHA-256 Hash (from server output):"</label>
                <input
                    type="text"
                    placeholder="e.g. a1b2c3d4..."
                    prop:value=move || cert_hash.get()
                    on:input=move |e| set_cert_hash.set(event_target_value(&e))
                    disabled=move || connected.get()
                />
            </div>

            <div class="controls">
                <button on:click=connect disabled=move || connected.get()>
                    "Connect"
                </button>
                <button on:click=disconnect disabled=move || !connected.get()>
                    "Disconnect"
                </button>
                <button on:click=send_datagram disabled=move || !connected.get()>
                    "Send Datagram"
                </button>
            </div>

            <div class="input-row">
                <input
                    type="text"
                    placeholder="Type a message..."
                    prop:value=move || input.get()
                    on:input=move |e| set_input.set(event_target_value(&e))
                    on:keypress=move |e| {
                        if e.key() == "Enter" {
                            send_message_clone(());
                        }
                    }
                    disabled=move || !has_stream.get()
                />
                <button
                    on:click=move |_| send_message(())
                    disabled=move || !has_stream.get()
                >
                    "Send"
                </button>
            </div>

            <div class="messages">
                <h2>"Messages"</h2>
                <div class="message-list">
                    <For
                        each=move || messages.get().into_iter().enumerate()
                        key=|(i, _)| *i
                        children=|(_, msg)| view! {
                            <div class="message">{msg}</div>
                        }
                    />
                </div>
            </div>
        </div>
    }
}

fn add_message(set_messages: &WriteSignal<Vec<String>>, msg: &str) {
    set_messages.update(|msgs| msgs.push(msg.to_string()));
}
