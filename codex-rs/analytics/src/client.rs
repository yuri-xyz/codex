use crate::events::AppServerRpcTransport;
use crate::events::GuardianReviewAnalyticsResult;
use crate::events::GuardianReviewTrackContext;
use crate::facts::AnalyticsFact;
use crate::facts::AnalyticsJsonRpcError;
use crate::facts::AppInvocation;
use crate::facts::HookRunFact;
use crate::facts::SkillInvocation;
use crate::facts::SubAgentThreadStartedInput;
use crate::facts::TrackEventsContext;
use crate::facts::TurnResolvedConfigFact;
use crate::facts::TurnTokenUsageFact;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ClientResponse;
use codex_app_server_protocol::InitializeParams;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_login::AuthManager;
use codex_plugin::PluginTelemetryMetadata;
#[cfg(test)]
use std::collections::HashSet;
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use tokio::sync::mpsc;

#[derive(Clone, Default)]
pub struct AnalyticsEventsClient;

#[cfg(test)]
pub(crate) struct AnalyticsEventsQueue {
    pub(crate) sender: mpsc::Sender<crate::events::TrackEventRequest>,
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
        let key = format!(
            "{}:{}:{}",
            tracking.thread_id,
            tracking.turn_id,
            plugin.plugin_id.as_key()
        );
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
        Self
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
        _connection_id: u64,
        _request_id: RequestId,
        _request: ClientRequest,
    ) {
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

    pub fn track_turn_resolved_config(&self, _fact: TurnResolvedConfigFact) {}

    pub fn track_turn_token_usage(&self, _fact: TurnTokenUsageFact) {}

    pub fn track_plugin_installed(&self, _plugin: PluginTelemetryMetadata) {}

    pub fn track_plugin_uninstalled(&self, _plugin: PluginTelemetryMetadata) {}

    pub fn track_plugin_enabled(&self, _plugin: PluginTelemetryMetadata) {}

    pub fn track_plugin_disabled(&self, _plugin: PluginTelemetryMetadata) {}

    pub(crate) fn record_fact(&self, _input: AnalyticsFact) {}

    pub fn track_response(&self, _connection_id: u64, _response: ClientResponse) {}

    pub fn track_error_response(
        &self,
        _connection_id: u64,
        _request_id: RequestId,
        _error: JSONRPCErrorError,
        _error_type: Option<AnalyticsJsonRpcError>,
    ) {
    }

    pub fn track_notification(&self, _notification: ServerNotification) {}
}
