#![allow(dead_code)]

use codex_protocol::protocol::W3cTraceContext;
use opentelemetry::Context;
use tracing::Span;

pub fn current_span_w3c_trace_context() -> Option<W3cTraceContext> {
    let _ = Span::current();
    None
}

pub fn span_w3c_trace_context(_span: &Span) -> Option<W3cTraceContext> {
    None
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

pub(crate) fn context_from_trace_headers(
    _traceparent: Option<&str>,
    _tracestate: Option<&str>,
) -> Option<Context> {
    None
}
