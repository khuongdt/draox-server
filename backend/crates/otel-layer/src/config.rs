use serde::{Deserialize, Serialize};

/// OpenTelemetry exporter endpoint protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExporterProtocol {
    /// gRPC OTLP (default, port 4317)
    Grpc,
    /// HTTP/JSON OTLP (port 4318)
    Http,
    /// Jaeger-native Thrift (legacy)
    Jaeger,
}

impl Default for ExporterProtocol {
    fn default() -> Self {
        Self::Grpc
    }
}

/// Configuration block for OpenTelemetry (maps to `[otel]` in config.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelConfig {
    /// Whether OpenTelemetry is enabled.
    #[serde(default = "OtelConfig::default_enabled")]
    pub enabled: bool,
    /// OTLP collector endpoint, e.g. `http://localhost:4317`.
    #[serde(default = "OtelConfig::default_endpoint")]
    pub endpoint: String,
    /// Service name reported in every span.
    #[serde(default = "OtelConfig::default_service_name")]
    pub service_name: String,
    /// Tail-based sampling rate [0.0, 1.0]. 1.0 = sample all.
    #[serde(default = "OtelConfig::default_sample_rate")]
    pub sample_rate: f64,
    /// Exporter protocol.
    #[serde(default)]
    pub protocol: ExporterProtocol,
    /// Whether to propagate W3C `traceparent` / `tracestate` headers.
    #[serde(default = "OtelConfig::default_propagate")]
    pub propagate_context: bool,
}

impl OtelConfig {
    fn default_enabled() -> bool { false }
    fn default_endpoint() -> String { "http://localhost:4317".into() }
    fn default_service_name() -> String { "draox-server".into() }
    fn default_sample_rate() -> f64 { 1.0 }
    fn default_propagate() -> bool { true }
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: Self::default_endpoint(),
            service_name: Self::default_service_name(),
            sample_rate: Self::default_sample_rate(),
            protocol: ExporterProtocol::default(),
            propagate_context: true,
        }
    }
}
