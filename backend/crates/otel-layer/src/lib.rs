pub mod config;
pub mod metrics;
pub mod middleware;
pub mod tracer;

pub use config::OtelConfig;
pub use metrics::ServerMetrics;
pub use middleware::{inject_context, trace_request};
pub use tracer::{init_tracer, shutdown_tracer, TracerError};
