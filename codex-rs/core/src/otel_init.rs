use crate::config::Config;
use codex_otel::OtelProvider;
use std::error::Error;

/// Build an OpenTelemetry provider from the app Config.
///
/// Returns `None` when OTEL export is disabled.
pub fn build_provider(
    _config: &Config,
    _service_version: &str,
    _service_name_override: Option<&str>,
    _default_analytics_enabled: bool,
) -> Result<Option<OtelProvider>, Box<dyn Error>> {
    Ok(None)
}

/// Filter predicate for exporting only Codex-owned events via OTEL.
/// Keeps events that originated from codex_otel module
pub fn codex_export_filter(meta: &tracing::Metadata<'_>) -> bool {
    let _ = meta;
    false
}

pub fn record_process_start(_otel: Option<&OtelProvider>, _originator: &str) {}

pub fn install_sqlite_telemetry(_otel: Option<&OtelProvider>, _originator: &str) {}
