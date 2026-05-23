use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider as SdkTracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use thiserror::Error;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::config::OtelConfig;

#[derive(Debug, Error)]
pub enum TracerError {
    #[error("OTLP exporter error: {0}")]
    OtlpSetup(String),
    #[error("tracing subscriber init error: {0}")]
    SubscriberInit(String),
}

/// Install the global tracing subscriber with an OpenTelemetry layer.
/// Must be called once at server startup before any `tracing::info!` etc.
pub fn init_tracer(cfg: &OtelConfig) -> Result<opentelemetry_sdk::trace::Tracer, TracerError> {
    let resource = Resource::new(vec![
        opentelemetry::KeyValue::new(SERVICE_NAME, cfg.service_name.clone()),
        opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    let sampler = if (cfg.sample_rate - 1.0).abs() < f64::EPSILON {
        Sampler::AlwaysOn
    } else {
        Sampler::TraceIdRatioBased(cfg.sample_rate)
    };

    // opentelemetry-otlp 0.27: new_pipeline/new_exporter removed; use SpanExporter::builder()
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(cfg.endpoint.clone())
        .build()
        .map_err(|e| TracerError::OtlpSetup(e.to_string()))?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_sampler(sampler)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer(cfg.service_name.clone());

    let otel_layer = OpenTelemetryLayer::new(tracer.clone());
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .try_init()
        .map_err(|e: tracing_subscriber::util::TryInitError| TracerError::SubscriberInit(e.to_string()))?;

    Ok(tracer)
}

/// Flush pending spans and shut down the global tracer. Call on server shutdown.
pub fn shutdown_tracer() {
    opentelemetry::global::shutdown_tracer_provider();
}
