//! WebTransport wrapper for browser API using JavaScript interop.
//!
//! Since WebTransport is a relatively new API, we use direct JS interop
//! rather than web-sys bindings which may not be complete.

use js_sys::{Array, Object, Promise, Uint8Array};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

// Import WebTransport from JavaScript
#[wasm_bindgen]
extern "C" {
    /// WebTransport interface
    #[wasm_bindgen(js_name = WebTransport)]
    pub type WebTransport;

    #[wasm_bindgen(constructor, js_class = "WebTransport")]
    pub fn new(url: &str) -> WebTransport;

    #[wasm_bindgen(constructor, js_class = "WebTransport")]
    pub fn new_with_options(url: &str, options: &Object) -> WebTransport;

    #[wasm_bindgen(method, getter)]
    pub fn ready(this: &WebTransport) -> Promise;

    #[wasm_bindgen(method, getter)]
    pub fn closed(this: &WebTransport) -> Promise;

    #[wasm_bindgen(method, getter)]
    pub fn datagrams(this: &WebTransport) -> DatagramDuplex;

    #[wasm_bindgen(method, js_name = createBidirectionalStream)]
    pub fn create_bidirectional_stream(this: &WebTransport) -> Promise;

    #[wasm_bindgen(method)]
    pub fn close(this: &WebTransport);

    /// WebTransportBidirectionalStream interface
    #[wasm_bindgen(js_name = WebTransportBidirectionalStream)]
    pub type BidiStreamJs;

    #[wasm_bindgen(method, getter)]
    pub fn readable(this: &BidiStreamJs) -> ReadableStream;

    #[wasm_bindgen(method, getter)]
    pub fn writable(this: &BidiStreamJs) -> WritableStream;

    /// WebTransportDatagramDuplexStream
    pub type DatagramDuplex;

    #[wasm_bindgen(method, getter)]
    pub fn readable(this: &DatagramDuplex) -> ReadableStream;

    #[wasm_bindgen(method, getter)]
    pub fn writable(this: &DatagramDuplex) -> WritableStream;

    /// ReadableStream
    pub type ReadableStream;

    #[wasm_bindgen(method, js_name = getReader)]
    pub fn get_reader(this: &ReadableStream) -> ReadableStreamReader;

    /// ReadableStreamDefaultReader
    pub type ReadableStreamReader;

    #[wasm_bindgen(method)]
    pub fn read(this: &ReadableStreamReader) -> Promise;

    #[wasm_bindgen(method, js_name = releaseLock)]
    pub fn release_lock(this: &ReadableStreamReader);

    /// WritableStream
    pub type WritableStream;

    #[wasm_bindgen(method, js_name = getWriter)]
    pub fn get_writer(this: &WritableStream) -> WritableStreamWriter;

    #[wasm_bindgen(method)]
    pub fn close(this: &WritableStream) -> Promise;

    /// WritableStreamDefaultWriter
    pub type WritableStreamWriter;

    #[wasm_bindgen(method)]
    pub fn write(this: &WritableStreamWriter, chunk: &JsValue) -> Promise;

    #[wasm_bindgen(method, js_name = releaseLock)]
    pub fn release_lock(this: &WritableStreamWriter);

    #[wasm_bindgen(method)]
    pub fn close(this: &WritableStreamWriter) -> Promise;
}

/// WebTransport client wrapper.
pub struct WebTransportClient {
    transport: Rc<WebTransport>,
}

impl Clone for WebTransportClient {
    fn clone(&self) -> Self {
        Self {
            transport: Rc::clone(&self.transport),
        }
    }
}

impl WebTransportClient {
    /// Connect to a WebTransport server.
    /// 
    /// If `cert_hash` is provided (SHA-256 hash of the server certificate),
    /// it will be used to allow self-signed certificates.
    pub async fn connect(url: &str, cert_hash: Option<&[u8]>) -> Result<Self, JsValue> {
        let transport = if let Some(hash) = cert_hash {
            // Create options with serverCertificateHashes for self-signed certs
            let options = Object::new();
            let hashes = Array::new();
            
            let hash_obj = Object::new();
            js_sys::Reflect::set(&hash_obj, &"algorithm".into(), &"sha-256".into())?;
            
            let hash_array = Uint8Array::from(hash);
            js_sys::Reflect::set(&hash_obj, &"value".into(), &hash_array.buffer())?;
            
            hashes.push(&hash_obj);
            js_sys::Reflect::set(&options, &"serverCertificateHashes".into(), &hashes)?;
            
            WebTransport::new_with_options(url, &options)
        } else {
            WebTransport::new(url)
        };

        // Wait for the connection to be ready
        JsFuture::from(transport.ready()).await?;

        Ok(Self {
            transport: Rc::new(transport),
        })
    }

    /// Open a bidirectional stream.
    pub async fn open_bidi_stream(&self) -> Result<BidiStream, JsValue> {
        let promise = self.transport.create_bidirectional_stream();
        let stream: BidiStreamJs = JsFuture::from(promise).await?.dyn_into()?;
        Ok(BidiStream::new(stream))
    }

    /// Send a datagram.
    pub async fn send_datagram(&self, data: &[u8]) -> Result<(), JsValue> {
        let datagrams = self.transport.datagrams();
        let writable = datagrams.writable();
        let writer = writable.get_writer();

        let array = Uint8Array::from(data);
        JsFuture::from(writer.write(&array.into())).await?;
        writer.release_lock();

        Ok(())
    }

    /// Receive a datagram.
    pub async fn recv_datagram(&self) -> Result<Vec<u8>, JsValue> {
        let datagrams = self.transport.datagrams();
        let readable = datagrams.readable();
        let reader = readable.get_reader();

        let result = JsFuture::from(reader.read()).await?;
        reader.release_lock();

        let value = js_sys::Reflect::get(&result, &"value".into())?;
        if value.is_undefined() {
            return Err(JsValue::from_str("Stream closed"));
        }

        let array: Uint8Array = value.dyn_into()?;
        Ok(array.to_vec())
    }

    /// Close the transport.
    pub fn close(&self) {
        self.transport.close();
    }
}

/// Bidirectional stream wrapper.
pub struct BidiStream {
    stream: Rc<BidiStreamJs>,
}

impl Clone for BidiStream {
    fn clone(&self) -> Self {
        Self {
            stream: Rc::clone(&self.stream),
        }
    }
}

impl BidiStream {
    fn new(stream: BidiStreamJs) -> Self {
        Self {
            stream: Rc::new(stream),
        }
    }

    /// Send data on the stream.
    pub async fn send(&self, data: &[u8]) -> Result<(), JsValue> {
        let writable = self.stream.writable();
        let writer = writable.get_writer();

        let array = Uint8Array::from(data);
        JsFuture::from(writer.write(&array.into())).await?;
        writer.release_lock();

        Ok(())
    }

    /// Receive data from the stream.
    pub async fn recv(&self) -> Result<Vec<u8>, JsValue> {
        let readable = self.stream.readable();
        let reader = readable.get_reader();

        let result = JsFuture::from(reader.read()).await?;
        reader.release_lock();

        let done = js_sys::Reflect::get(&result, &"done".into())?;
        if done.as_bool().unwrap_or(false) {
            return Err(JsValue::from_str("Stream closed"));
        }

        let value = js_sys::Reflect::get(&result, &"value".into())?;
        let array: Uint8Array = value.dyn_into()?;
        Ok(array.to_vec())
    }

    /// Close the send side of the stream.
    pub async fn close_send(&self) -> Result<(), JsValue> {
        let writable = self.stream.writable();
        let writer = writable.get_writer();
        JsFuture::from(writer.close()).await?;
        Ok(())
    }
}

