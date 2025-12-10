//! Router for HTTP/3 requests with REST and streaming support.

use bytes::Bytes;
use h3::server::RequestStream;
use http::Request;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Response type for REST handlers.
pub struct RestResponse {
    pub body: String,
    pub content_type: &'static str,
}

impl RestResponse {
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            content_type: "text/plain",
        }
    }

    pub fn json(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            content_type: "application/json",
        }
    }
}

/// A boxed async REST handler function.
pub type BoxedRestHandler = Arc<
    dyn Fn(Request<()>) -> Pin<Box<dyn Future<Output = RestResponse> + Send>> + Send + Sync,
>;

/// A boxed async stream handler function.
pub type BoxedStreamHandler = Arc<
    dyn Fn(
            Request<()>,
            RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Handler type enum.
pub enum Handler {
    Rest(BoxedRestHandler),
    Stream(BoxedStreamHandler),
}

/// A path-based router supporting REST and streaming handlers.
pub struct Router {
    routes: HashMap<String, Handler>,
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Add a REST route (request/response pattern).
    ///
    /// # Example
    /// ```ignore
    /// router.route("/api/users", |_req| async {
    ///     RestResponse::json(r#"[{"id": 1}]"#)
    /// })
    /// ```
    pub fn route<F, Fut>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<()>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RestResponse> + Send + 'static,
    {
        let handler = Arc::new(move |req: Request<()>| {
            let fut = handler(req);
            Box::pin(fut) as Pin<Box<dyn Future<Output = RestResponse> + Send>>
        });
        self.routes.insert(path.to_string(), Handler::Rest(handler));
        self
    }

    /// Add a streaming route (handler manages the stream directly).
    ///
    /// # Example
    /// ```ignore
    /// router.stream("/stream/events", |req, stream| async move {
    ///     // Send multiple chunks over time
    ///     Ok(())
    /// })
    /// ```
    pub fn stream<F, Fut>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<()>, RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |req: Request<()>, stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>| {
                let fut = handler(req, stream);
                Box::pin(fut) as Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>
            },
        );
        self.routes
            .insert(path.to_string(), Handler::Stream(handler));
        self
    }

    /// Get handler for a path.
    pub fn get(&self, path: &str) -> Option<&Handler> {
        self.routes.get(path)
    }

    /// Check if path exists.
    #[allow(dead_code)]
    pub fn contains(&self, path: &str) -> bool {
        self.routes.contains_key(path)
    }

    /// List all registered routes.
    pub fn routes(&self) -> Vec<&str> {
        self.routes.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
