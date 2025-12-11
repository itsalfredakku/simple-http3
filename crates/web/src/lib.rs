//! WebTransport Client Library for Browser
//!
//! This crate provides a Leptos-based WASM application that connects
//! to an HTTP/3 server using the WebTransport API.

mod app;
mod transport;

pub use app::App;

use wasm_bindgen::prelude::*;

/// Entry point for the WASM application
#[wasm_bindgen(start)]
pub fn main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Mount the Leptos app
    leptos::mount::mount_to_body(App);
}
