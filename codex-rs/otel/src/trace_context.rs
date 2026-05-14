use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::sync::RwLock;

use codex_protocol::protocol::W3cTraceContext;
use opentelemetry::Context;
use opentelemetry::trace::TraceState;
use tracing::Span;

// Trace context propagation can happen outside the provider object, so configured
// tracestate lives beside the process-global tracer provider.
static TRACESTATE_ENTRIES: OnceLock<RwLock<BTreeMap<String, BTreeMap<String, String>>>> =
    OnceLock::new();

pub fn current_span_w3c_trace_context() -> Option<W3cTraceContext> {
    let _ = Span::current();
    None
}

pub fn span_w3c_trace_context(_span: &Span) -> Option<W3cTraceContext> {
    None
}

pub(crate) fn set_tracestate_entries(
    entries: BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_tracestate_entries(&entries)?;
    let mut guard = tracestate_entries()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = entries;
    Ok(())
}

pub fn current_span_trace_id() -> Option<String> {
    let _ = Span::current();
    None
}

pub fn context_from_w3c_trace_context(_trace: &W3cTraceContext) -> Option<Context> {
    None
}

pub fn set_parent_from_w3c_trace_context(_span: &Span, _trace: &W3cTraceContext) -> bool {
    false
}

pub fn set_parent_from_context(_span: &Span, _context: Context) {}

pub fn traceparent_context_from_env() -> Option<Context> {
    None
}

fn tracestate_entries() -> &'static RwLock<BTreeMap<String, BTreeMap<String, String>>> {
    TRACESTATE_ENTRIES.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// Validates configured tracestate members before they are propagated in W3C trace context.
pub fn validate_tracestate_entries(
    entries: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Reject malformed entries before installing them so propagated trace
    // context remains acceptable to other W3C Trace Context extractors. The
    // SDK validates member keys and list structure, but configured member
    // fields are joined into header values here and need stricter validation.
    let entries = entries
        .iter()
        .map(|(key, fields)| encode_tracestate_member_fields(key, fields))
        .collect::<Result<Vec<_>, _>>()?;
    TraceState::from_key_value(
        entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str())),
    )
    .map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid configured tracestate: {err}"),
        )
    })?;
    Ok(())
}

/// Validates one configured tracestate member and its encoded field value.
pub fn validate_tracestate_member(
    member_key: &str,
    fields: &BTreeMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (key, value) = encode_tracestate_member_fields(member_key, fields)?;
    TraceState::from_key_value([(key.as_str(), value.as_str())]).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid configured tracestate: {err}"),
        )
    })?;
    Ok(())
}

fn encode_tracestate_member_fields(
    member_key: &str,
    fields: &BTreeMap<String, String>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Configured fields are encoded into one opaque tracestate member value.
    // Validate both the field grammar and the final header value so malformed
    // config cannot produce propagated trace context that downstream W3C
    // extractors reject.
    let mut encoded = Vec::with_capacity(fields.len());
    for (field_key, value) in fields {
        if !is_configured_tracestate_field_key(field_key) {
            return Err(invalid_tracestate_config(format!(
                "invalid configured tracestate field key {member_key}.{field_key}"
            )));
        }
        if !is_configured_tracestate_field_value(value) {
            return Err(invalid_tracestate_config(format!(
                "invalid configured tracestate value for {member_key}.{field_key}"
            )));
        }
        encoded.push(format!("{field_key}:{value}"));
    }
    let value = encoded.join(";");
    if !is_header_safe_tracestate_member_value(&value) {
        return Err(invalid_tracestate_config(format!(
            "invalid configured tracestate value for {member_key}"
        )));
    }
    Ok((member_key.to_string(), value))
}

fn is_configured_tracestate_field_key(field_key: &str) -> bool {
    !field_key.is_empty()
        && field_key
            .bytes()
            .all(|byte| matches!(byte, b'!'..=b'~') && !matches!(byte, b':' | b';' | b',' | b'='))
}

fn is_configured_tracestate_field_value(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| is_tracestate_member_value_byte(byte) && byte != b';')
}

fn is_header_safe_tracestate_member_value(value: &str) -> bool {
    value.is_empty()
        || (value.bytes().all(is_tracestate_member_value_byte)
            && value.as_bytes().last().is_some_and(|byte| *byte != b' '))
}

fn is_tracestate_member_value_byte(byte: u8) -> bool {
    matches!(byte, b' '..=b'~') && !matches!(byte, b',' | b'=')
}

fn invalid_tracestate_config(message: String) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    ))
}
