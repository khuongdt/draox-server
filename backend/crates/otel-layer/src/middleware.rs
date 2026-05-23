use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Response, StatusCode},
    middleware::Next,
};
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status as OtelStatus, Tracer},
};
use opentelemetry_semantic_conventions::trace::{HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, URL_PATH};
use tracing::warn;

/// Axum middleware that creates a server-side span for every HTTP request.
///
/// Extracts W3C `traceparent`/`tracestate` from incoming headers for
/// distributed-trace context propagation.
pub async fn trace_request(req: Request<Body>, next: Next) -> Response<Body> {
    let tracer = global::tracer("draox-http");
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract parent context from W3C traceparent header if present
    let parent_cx = extract_context(req.headers());

    let mut span = tracer
        .span_builder(format!("{method} {path}"))
        .with_kind(SpanKind::Server)
        .with_attributes(vec![
            opentelemetry::KeyValue::new(HTTP_REQUEST_METHOD, method.clone()),
            opentelemetry::KeyValue::new(URL_PATH, path.clone()),
        ])
        .start_with_context(&tracer, &parent_cx);

    let response = next.run(req).await;
    let status = response.status().as_u16() as i64;

    span.set_attribute(opentelemetry::KeyValue::new(HTTP_RESPONSE_STATUS_CODE, status));
    if response.status().is_server_error() {
        span.set_status(OtelStatus::Error {
            description: std::borrow::Cow::Owned(format!("HTTP {status}")),
        });
    }
    span.end();
    response
}

fn extract_context(headers: &HeaderMap) -> opentelemetry::Context {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    struct HeaderExtractor<'a>(&'a HeaderMap);
    impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| v.to_str().ok())
        }
        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|k| k.as_str()).collect()
        }
    }

    let propagator = TraceContextPropagator::new();
    propagator.extract(&HeaderExtractor(headers))
}

/// Inject the current span context into outgoing HTTP headers (for upstream calls).
pub fn inject_context(headers: &mut HeaderMap) {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    struct HeaderInjector<'a>(&'a mut HeaderMap);
    impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            if let (Ok(k), Ok(v)) = (
                axum::http::HeaderName::try_from(key),
                axum::http::HeaderValue::try_from(value),
            ) {
                self.0.insert(k, v);
            }
        }
    }

    let propagator = TraceContextPropagator::new();
    let cx = opentelemetry::Context::current();
    propagator.inject_context(&cx, &mut HeaderInjector(headers));
}
