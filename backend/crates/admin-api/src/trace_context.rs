// Request tracing middleware — generates a unique trace_id per request,
// includes it in response headers and logs.

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response};
use axum::middleware::Next;
use std::time::Instant;
use tracing::info;
use uuid::Uuid;

/// Header name for trace ID propagation.
pub const TRACE_ID_HEADER: &str = "X-Trace-Id";

/// Extract or generate a trace ID from the request.
///
/// If the request has an `X-Trace-Id` header, use it. Otherwise generate a new UUID.
pub fn extract_trace_id(req: &Request<Body>) -> String {
    req.headers()
        .get(TRACE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Axum middleware layer that:
/// 1. Extracts or generates `trace_id`
/// 2. Logs request start (method, path, trace_id)
/// 3. Passes request through
/// 4. Logs request end (status, duration, trace_id)
/// 5. Adds `X-Trace-Id` to response headers
pub async fn trace_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let trace_id = extract_trace_id(&req);
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    info!(
        trace_id = %trace_id,
        method = %method,
        path = %path,
        "request started"
    );

    let mut response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status().as_u16();

    info!(
        trace_id = %trace_id,
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration.as_millis() as u64,
        "request completed"
    );

    // Add trace ID to response headers
    if let Ok(value) = HeaderValue::from_str(&trace_id) {
        response.headers_mut().insert(TRACE_ID_HEADER, value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::middleware;
    use axum::routing::get;
    use axum::Router;
    use axum::{body::Body, http::StatusCode};
    use tower::ServiceExt;

    async fn dummy_handler() -> &'static str {
        "ok"
    }

    #[test]
    fn test_extract_trace_id_generates_uuid() {
        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let trace_id = extract_trace_id(&req);
        // Should be a valid UUID v4 string (36 chars with hyphens)
        assert_eq!(trace_id.len(), 36);
        assert!(uuid::Uuid::parse_str(&trace_id).is_ok());
    }

    #[test]
    fn test_extract_trace_id_uses_existing() {
        let req = Request::builder()
            .uri("/test")
            .header(TRACE_ID_HEADER, "my-custom-trace-id")
            .body(Body::empty())
            .unwrap();

        let trace_id = extract_trace_id(&req);
        assert_eq!(trace_id, "my-custom-trace-id");
    }

    #[tokio::test]
    async fn test_trace_middleware() {
        let app = Router::new()
            .route("/test", get(dummy_handler))
            .layer(middleware::from_fn(trace_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("X-Trace-Id").is_some());

        // The trace ID in the response should be a valid UUID
        let trace_id = response.headers().get("X-Trace-Id").unwrap().to_str().unwrap();
        assert!(uuid::Uuid::parse_str(trace_id).is_ok());
    }
}
