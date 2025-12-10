//! Simple router for HTTP/3 requests.

use http::Request;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A boxed async handler function.
pub type BoxedHandler = Arc<
    dyn Fn(Request<()>) -> Pin<Box<dyn Future<Output = &'static str> + Send>> + Send + Sync,
>;

/// A simple path-based router.
pub struct Router {
    routes: HashMap<String, BoxedHandler>,
    not_found: &'static str,
}

impl Router {
    /// Create a new router.
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            not_found: "Not Found",
        }
    }

    /// Add a route with an async handler.
    ///
    /// # Example
    /// ```ignore
    /// let router = Router::new()
    ///     .route("/", |_req| async { "Hello!" })
    ///     .route("/api", |_req| async { r#"{"ok": true}"# });
    /// ```
    pub fn route<F, Fut>(mut self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<()>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = &'static str> + Send + 'static,
    {
        let handler = Arc::new(move |req: Request<()>| {
            let fut = handler(req);
            Box::pin(fut) as Pin<Box<dyn Future<Output = &'static str> + Send>>
        });
        self.routes.insert(path.to_string(), handler);
        self
    }

    /// Set the default response for unmatched routes.
    pub fn not_found(mut self, response: &'static str) -> Self {
        self.not_found = response;
        self
    }

    /// Handle a request and return the response body.
    pub async fn handle(&self, req: Request<()>) -> (&'static str, &'static str) {
        let path = req.uri().path();
        
        if let Some(handler) = self.routes.get(path) {
            let body = handler(req).await;
            let content_type = if body.starts_with('{') || body.starts_with('[') {
                "application/json"
            } else {
                "text/plain"
            };
            (body, content_type)
        } else {
            (self.not_found, "text/plain")
        }
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
