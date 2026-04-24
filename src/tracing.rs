//! OpenTelemetry tracing module.
//!
//! Provides distributed tracing instrumentation for the orchestrator framework.

use opentelemetry::global;
use opentelemetry::trace::TraceError;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Initializes OpenTelemetry tracing with OTLP exporter.
///
/// # Arguments
/// * `endpoint` - OTLP endpoint URL (e.g., "http://localhost:4317")
/// * `service_name` - Name of the service for tracing
///
/// # Returns
/// Result indicating success or error
pub fn init_tracing(endpoint: &str, service_name: &str) -> Result<(), TraceError> {
    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .build_span_exporter()?;

    // Create tracer provider with OTLP exporter
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                opentelemetry::KeyValue::new("service.name", service_name.to_string()),
                opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            ])),
        )
        .build();

    // Set global tracer provider
    global::set_tracer_provider(provider.clone());

    // Create tracing layer with OpenTelemetry
    let telemetry_layer =
        tracing_opentelemetry::layer().with_tracer(provider.tracer(service_name.to_string()));

    // Initialize subscriber with both console and telemetry layers
    let env_filter = EnvFilter::from_default_env()
        .add_directive("rust_orchestrator=debug".parse().unwrap())
        .add_directive("chromiumoxide=off".parse().unwrap());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Initializes tracing with console output only (no OpenTelemetry).
///
/// Use this when OTLP endpoint is not available.
pub fn init_console_tracing() {
    let env_filter = EnvFilter::from_default_env()
        .add_directive("rust_orchestrator=debug".parse().unwrap())
        .add_directive("chromiumoxide=off".parse().unwrap());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Shuts down OpenTelemetry tracing provider.
///
/// Should be called before application exit to flush any pending spans.
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_console_tracing() {
        // This test verifies that console tracing initialization doesn't panic
        // It will actually initialize the global subscriber, which may affect other tests
        // In a real test suite, you'd want to use a test isolation mechanism
        init_console_tracing();
        // If we get here without panicking, the test passes
    }

    #[test]
    fn test_shutdown_tracing() {
        // This test verifies that shutdown doesn't panic
        // It's safe to call even if tracing wasn't initialized
        shutdown_tracing();
    }

    #[test]
    fn test_shutdown_tracing_idempotent() {
        // Multiple shutdowns should not panic
        shutdown_tracing();
        shutdown_tracing();
        shutdown_tracing();
    }
}
