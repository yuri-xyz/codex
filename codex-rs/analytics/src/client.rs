use crate::events::AppServerRpcTransport;
use crate::events::GuardianReviewAnalyticsResult;
use crate::events::GuardianReviewTrackContext;
#[cfg(test)]
use crate::events::TrackEventRequest;
#[cfg(test)]
use crate::events::TrackEventsRequest;
use crate::facts::AnalyticsFact;
use crate::facts::AnalyticsJsonRpcError;
use crate::facts::AppInvocation;
use crate::facts::CodexGoalEvent;
use crate::facts::ExternalAgentConfigImportCompletedInput;
use crate::facts::ExternalAgentConfigImportFailureInput;
use crate::facts::HookRunFact;
use crate::facts::PluginInstallRequested;
use crate::facts::PluginInstallSource;
use crate::facts::SkillInvocation;
use crate::facts::SubAgentThreadStartedInput;
use crate::facts::TrackEventsContext;
use crate::facts::TurnCodexErrorFact;
use crate::facts::TurnProfileFact;
use crate::facts::TurnResolvedConfigFact;
use crate::facts::TurnTokenUsageFact;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::InitializeParams;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ServerResponse;
use codex_login::AuthManager;
use codex_plugin::PluginTelemetryMetadata;
use codex_protocol::request_permissions::RequestPermissionsResponse;
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use tokio::sync::mpsc;

#[derive(Clone, Default)]
pub struct AnalyticsEventsClient {
    #[cfg(test)]
    pub(crate) queue: Option<AnalyticsEventsQueue>,
}

#[cfg(test)]
#[derive(Clone)]
pub(crate) struct AnalyticsEventsQueue {
    pub(crate) sender: mpsc::Sender<AnalyticsFact>,
    pub(crate) app_used_emitted_keys: Arc<Mutex<HashSet<String>>>,
    pub(crate) plugin_used_emitted_keys: Arc<Mutex<HashSet<String>>>,
}

#[cfg(test)]
impl AnalyticsEventsQueue {
    pub(crate) fn should_enqueue_app_used(
        &self,
        tracking: &TrackEventsContext,
        app: &AppInvocation,
    ) -> bool {
        let _sender = &self.sender;
        let app_key = app
            .connector_id
            .as_ref()
            .or(app.app_name.as_ref())
            .cloned()
            .unwrap_or_default();
        let key = format!("{}:{}:{app_key}", tracking.thread_id, tracking.turn_id);
        self.app_used_emitted_keys
            .lock()
            .expect("app-used dedupe lock")
            .insert(key)
    }

    pub(crate) fn should_enqueue_plugin_used(
        &self,
        tracking: &TrackEventsContext,
        plugin: &PluginTelemetryMetadata,
    ) -> bool {
        let _sender = &self.sender;
        let plugin_key = plugin
            .plugin_id
            .as_ref()
            .map(codex_plugin::PluginId::as_key)
            .or_else(|| plugin.remote_plugin_id.clone())
            .or_else(|| {
                plugin
                    .capability_summary
                    .as_ref()
                    .map(|summary| summary.config_name.clone())
            })
            .unwrap_or_default();
        let key = format!("{}:{}:{}", tracking.thread_id, tracking.turn_id, plugin_key);
        self.plugin_used_emitted_keys
            .lock()
            .expect("plugin-used dedupe lock")
            .insert(key)
    }
}

impl AnalyticsEventsClient {
    pub fn new(
        _auth_manager: Arc<AuthManager>,
        _base_url: String,
        _analytics_enabled: Option<bool>,
    ) -> Self {
        Self::disabled()
    }

    pub fn disabled() -> Self {
        Self {
            #[cfg(test)]
            queue: None,
        }
    }

    pub fn track_skill_invocations(
        &self,
        _tracking: TrackEventsContext,
        _invocations: Vec<SkillInvocation>,
    ) {
    }

    pub fn track_initialize(
        &self,
        _connection_id: u64,
        _params: InitializeParams,
        _product_client_id: String,
        _rpc_transport: AppServerRpcTransport,
    ) {
    }

    pub fn track_subagent_thread_started(&self, _input: SubAgentThreadStartedInput) {}

    pub fn track_guardian_review(
        &self,
        _tracking: &GuardianReviewTrackContext,
        _result: GuardianReviewAnalyticsResult,
        _completed_at_ms: u64,
    ) {
    }

    pub fn track_app_mentioned(
        &self,
        _tracking: TrackEventsContext,
        _mentions: Vec<AppInvocation>,
    ) {
    }

    pub fn track_request(
        &self,
        connection_id: u64,
        request_id: RequestId,
        request: &ClientRequest,
    ) {
        if !matches!(
            request,
            ClientRequest::TurnStart { .. } | ClientRequest::TurnSteer { .. }
        ) {
            return;
        }
        self.record_fact(AnalyticsFact::ClientRequest {
            connection_id,
            request_id,
            request: Box::new(request.clone()),
        });
    }

    pub fn track_app_used(&self, _tracking: TrackEventsContext, _app: AppInvocation) {}

    pub fn track_hook_run(&self, _tracking: TrackEventsContext, _hook: HookRunFact) {}

    pub fn track_plugin_used(
        &self,
        _tracking: TrackEventsContext,
        _plugin: PluginTelemetryMetadata,
    ) {
    }

    pub fn track_compaction(&self, _event: crate::facts::CodexCompactionEvent) {}

    pub fn track_goal_event(&self, _event: CodexGoalEvent) {}

    pub fn track_plugin_install_requested(
        &self,
        _tracking: TrackEventsContext,
        _request: PluginInstallRequested,
    ) {
    }

    pub fn track_turn_resolved_config(&self, _fact: TurnResolvedConfigFact) {}

    pub fn track_turn_token_usage(&self, _fact: TurnTokenUsageFact) {}

    pub fn track_turn_profile(&self, _fact: TurnProfileFact) {}

    pub fn track_turn_codex_error(&self, _fact: TurnCodexErrorFact) {}

    pub fn track_plugin_installed(&self, _plugin: PluginTelemetryMetadata) {}

    pub fn track_plugin_install_failed(
        &self,
        _plugin: PluginTelemetryMetadata,
        _source: PluginInstallSource,
        _error_type: String,
    ) {
    }

    pub fn track_external_agent_config_import_completed(
        &self,
        _input: ExternalAgentConfigImportCompletedInput,
    ) {
    }

    pub fn track_external_agent_config_import_failure(
        &self,
        _input: ExternalAgentConfigImportFailureInput,
    ) {
    }

    pub fn track_plugin_uninstalled(&self, _plugin: PluginTelemetryMetadata) {}

    pub fn track_plugin_enabled(&self, _plugin: PluginTelemetryMetadata) {}

    pub fn track_plugin_disabled(&self, _plugin: PluginTelemetryMetadata) {}

    #[cfg(not(test))]
    pub(crate) fn record_fact(&self, _input: AnalyticsFact) {}

    #[cfg(test)]
    pub(crate) fn record_fact(&self, input: AnalyticsFact) {
        if let Some(queue) = self.queue.as_ref() {
            let _ = queue.sender.try_send(input);
        }
    }

    pub fn track_response(
        &self,
        connection_id: u64,
        request_id: RequestId,
        response: ClientResponsePayload,
    ) {
        self.track_response_inner(
            connection_id,
            request_id,
            response,
            /*thread_originator*/ None,
        );
    }

    pub fn track_response_with_thread_originator(
        &self,
        connection_id: u64,
        request_id: RequestId,
        response: ClientResponsePayload,
        thread_originator: String,
    ) {
        self.track_response_inner(connection_id, request_id, response, Some(thread_originator));
    }

    fn track_response_inner(
        &self,
        connection_id: u64,
        request_id: RequestId,
        response: ClientResponsePayload,
        thread_originator: Option<String>,
    ) {
        if !matches!(
            response,
            ClientResponsePayload::ThreadStart(_)
                | ClientResponsePayload::ThreadResume(_)
                | ClientResponsePayload::ThreadFork(_)
                | ClientResponsePayload::TurnStart(_)
                | ClientResponsePayload::TurnSteer(_)
        ) {
            return;
        }
        self.record_fact(AnalyticsFact::ClientResponse {
            connection_id,
            request_id,
            response: Box::new(response),
            thread_originator,
        });
    }

    pub fn track_error_response(
        &self,
        _connection_id: u64,
        _request_id: RequestId,
        _error: JSONRPCErrorError,
        _error_type: Option<AnalyticsJsonRpcError>,
    ) {
    }

    pub fn track_notification(&self, _notification: ServerNotification) {}

    pub fn track_server_request(&self, _connection_id: u64, _request: ServerRequest) {}

    pub fn track_server_response(&self, _completed_at_ms: u64, _response: ServerResponse) {}

    pub fn track_effective_permissions_approval_response(
        &self,
        _completed_at_ms: u64,
        _request_id: RequestId,
        _response: RequestPermissionsResponse,
    ) {
    }

    pub fn track_server_request_aborted(&self, _completed_at_ms: u64, _request_id: RequestId) {}
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AnalyticsEventsDestination {
    Http { url: String },
    CaptureFile { path: PathBuf },
}

#[cfg(test)]
impl AnalyticsEventsDestination {
    pub(crate) fn from_base_url_and_capture_file(
        base_url: String,
        capture_file: Option<PathBuf>,
    ) -> Self {
        #[cfg(debug_assertions)]
        if let Some(path) = capture_file {
            let _ = crate::analytics_capture::initialize(&path);
            return Self::CaptureFile { path };
        }

        let url = format!(
            "{}/codex/analytics-events/events",
            base_url.trim_end_matches('/')
        );
        Self::Http { url }
    }
}

#[cfg(all(test, debug_assertions))]
pub(crate) fn capture_track_events_request(
    destination: &AnalyticsEventsDestination,
    payload: &TrackEventsRequest,
) -> bool {
    match destination {
        AnalyticsEventsDestination::CaptureFile { path } => {
            let _ = crate::analytics_capture::append_payload(path, payload);
            true
        }
        AnalyticsEventsDestination::Http { .. } => false,
    }
}

#[cfg(all(test, debug_assertions))]
pub(crate) async fn send_track_events_request<Auth>(
    _auth: &Auth,
    destination: &AnalyticsEventsDestination,
    events: Vec<TrackEventRequest>,
) {
    let payload = TrackEventsRequest { events };
    let _ = capture_track_events_request(destination, &payload);
}

#[cfg(test)]
fn track_event_request_batches(events: Vec<TrackEventRequest>) -> Vec<Vec<TrackEventRequest>> {
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();

    for event in events {
        if event.should_send_in_isolated_request() {
            if !current_batch.is_empty() {
                batches.push(current_batch);
                current_batch = Vec::new();
            }
            batches.push(vec![event]);
        } else {
            current_batch.push(event);
        }
    }

    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
