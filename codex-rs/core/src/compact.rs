use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
use crate::session::PreviousTurnSettings;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::web_search::web_search_action_detail;
use codex_analytics::CodexCompactionEvent;
use codex_analytics::CompactionImplementation;
use codex_analytics::CompactionPhase;
use codex_analytics::CompactionReason;
use codex_analytics::CompactionStatus;
use codex_analytics::CompactionStrategy;
use codex_analytics::CompactionTrigger;
use codex_analytics::now_unix_seconds;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::items::ContextCompactionItem;
use codex_protocol::items::TurnItem;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::LocalShellAction;
use codex_protocol::models::LocalShellExecAction;
use codex_protocol::models::LocalShellStatus;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::WebSearchAction;
use codex_protocol::protocol::CompactedItem;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::TurnStartedEvent;
use codex_protocol::protocol::WarningEvent;
use codex_protocol::user_input::UserInput;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::truncate_text;

use codex_model_provider_info::ModelProviderInfo;

pub const SUMMARIZATION_PROMPT: &str = include_str!("../templates/compact/prompt.md");
pub const SUMMARY_PREFIX: &str = include_str!("../templates/compact/summary_prefix.md");
const COMPACT_USER_MESSAGE_MAX_TOKENS: usize = 20_000;
const DETERMINISTIC_COMPACT_EVENT_LIMIT: usize = 40;
const DETERMINISTIC_COMPACT_LINE_LIMIT: usize = 200;
const DETERMINISTIC_COMPACT_CONTINUATION: &str = "You left here, continue.";

/// Controls whether compaction replacement history must include initial context.
///
/// Pre-turn/manual compaction variants use `DoNotInject`: they replace history with a summary and
/// clear `reference_context_item`, so the next regular turn will fully reinject initial context
/// after compaction.
///
/// Mid-turn compaction must use `BeforeLastUserMessage` because the model is trained to see the
/// compaction summary as the last item in history after mid-turn compaction; we therefore inject
/// initial context into the replacement history just above the last real user message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InitialContextInjection {
    BeforeLastUserMessage,
    DoNotInject,
}

pub(crate) fn should_use_remote_compact_task(_provider: &ModelProviderInfo) -> bool {
    false
}

pub(crate) async fn run_inline_auto_compact_task(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    initial_context_injection: InitialContextInjection,
    reason: CompactionReason,
    phase: CompactionPhase,
) -> CodexResult<()> {
    run_compact_task_inner(
        sess,
        turn_context,
        initial_context_injection,
        CompactionTrigger::Auto,
        reason,
        phase,
    )
    .await?;
    Ok(())
}

pub(crate) async fn run_compact_task(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    _input: Vec<UserInput>,
) -> CodexResult<()> {
    let start_event = EventMsg::TurnStarted(TurnStartedEvent {
        turn_id: turn_context.sub_id.clone(),
        started_at: turn_context.turn_timing_state.started_at_unix_secs().await,
        model_context_window: turn_context.model_context_window(),
        collaboration_mode_kind: turn_context.collaboration_mode.mode,
    });
    sess.send_event(&turn_context, start_event).await;
    run_compact_task_inner(
        sess.clone(),
        turn_context,
        InitialContextInjection::DoNotInject,
        CompactionTrigger::Manual,
        CompactionReason::UserRequested,
        CompactionPhase::StandaloneTurn,
    )
    .await
}

async fn run_compact_task_inner(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    initial_context_injection: InitialContextInjection,
    trigger: CompactionTrigger,
    reason: CompactionReason,
    phase: CompactionPhase,
) -> CodexResult<()> {
    let attempt = CompactionAnalyticsAttempt::begin(
        sess.as_ref(),
        turn_context.as_ref(),
        trigger,
        reason,
        CompactionImplementation::Responses,
        phase,
    )
    .await;
    let result = run_compact_task_inner_impl(
        Arc::clone(&sess),
        Arc::clone(&turn_context),
        initial_context_injection,
    )
    .await;
    attempt
        .track(
            sess.as_ref(),
            compaction_status_from_result(&result),
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
    result
}

async fn run_compact_task_inner_impl(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
    initial_context_injection: InitialContextInjection,
) -> CodexResult<()> {
    let compaction_item = TurnItem::ContextCompaction(ContextCompactionItem::new());
    sess.emit_turn_item_started(&turn_context, &compaction_item)
        .await;
    let history_snapshot = sess.clone_history().await;
    let history_items = history_snapshot.raw_items();
    let summary_text = build_deterministic_summary_text(history_items);

    let mut new_history = build_compacted_history(Vec::new(), &[], &summary_text);

    if matches!(
        initial_context_injection,
        InitialContextInjection::BeforeLastUserMessage
    ) {
        let initial_context = sess.build_initial_context(turn_context.as_ref()).await;
        new_history =
            insert_initial_context_before_last_real_user_or_summary(new_history, initial_context);
    }
    let reference_context_item = match initial_context_injection {
        InitialContextInjection::DoNotInject => None,
        InitialContextInjection::BeforeLastUserMessage => Some(turn_context.to_turn_context_item()),
    };
    let compacted_item = CompactedItem {
        message: summary_text.clone(),
        replacement_history: Some(new_history.clone()),
    };
    sess.replace_compacted_history(new_history, reference_context_item, compacted_item)
        .await;
    sess.recompute_token_usage(&turn_context).await;

    sess.emit_turn_item_completed(&turn_context, compaction_item)
        .await;
    let warning = EventMsg::Warning(WarningEvent {
        message: "Heads up: Long threads and multiple compactions can cause the model to be less accurate. Start a new thread when possible to keep threads small and targeted.".to_string(),
    });
    sess.send_event(&turn_context, warning).await;
    Ok(())
}

pub(crate) struct CompactionAnalyticsAttempt {
    thread_id: String,
    turn_id: String,
    trigger: CompactionTrigger,
    reason: CompactionReason,
    implementation: CompactionImplementation,
    phase: CompactionPhase,
    active_context_tokens_before: i64,
    started_at: u64,
    start_instant: Instant,
}

impl CompactionAnalyticsAttempt {
    pub(crate) async fn begin(
        sess: &Session,
        turn_context: &TurnContext,
        trigger: CompactionTrigger,
        reason: CompactionReason,
        implementation: CompactionImplementation,
        phase: CompactionPhase,
    ) -> Self {
        let active_context_tokens_before = sess.get_total_token_usage().await;
        Self {
            thread_id: sess.conversation_id.to_string(),
            turn_id: turn_context.sub_id.clone(),
            trigger,
            reason,
            implementation,
            phase,
            active_context_tokens_before,
            started_at: now_unix_seconds(),
            start_instant: Instant::now(),
        }
    }

    pub(crate) async fn track(
        self,
        sess: &Session,
        status: CompactionStatus,
        error: Option<String>,
    ) {
        let active_context_tokens_after = sess.get_total_token_usage().await;
        sess.services
            .analytics_events_client
            .track_compaction(CodexCompactionEvent {
                thread_id: self.thread_id,
                turn_id: self.turn_id,
                trigger: self.trigger,
                reason: self.reason,
                implementation: self.implementation,
                phase: self.phase,
                strategy: CompactionStrategy::Memento,
                status,
                error,
                active_context_tokens_before: self.active_context_tokens_before,
                active_context_tokens_after,
                started_at: self.started_at,
                completed_at: now_unix_seconds(),
                duration_ms: Some(
                    u64::try_from(self.start_instant.elapsed().as_millis()).unwrap_or(u64::MAX),
                ),
            });
    }
}

pub(crate) fn compaction_status_from_result<T>(result: &CodexResult<T>) -> CompactionStatus {
    match result {
        Ok(_) => CompactionStatus::Completed,
        Err(CodexErr::Interrupted | CodexErr::TurnAborted) => CompactionStatus::Interrupted,
        Err(_) => CompactionStatus::Failed,
    }
}

fn build_deterministic_summary_text(items: &[ResponseItem]) -> String {
    let rendered_events = render_compaction_events(items);
    let start = rendered_events
        .len()
        .saturating_sub(DETERMINISTIC_COMPACT_EVENT_LIMIT);
    let mut sections = rendered_events[start..].to_vec();
    sections.push(DETERMINISTIC_COMPACT_CONTINUATION.to_string());

    format!("{SUMMARY_PREFIX}\n{}", sections.join("\n\n"))
}

fn render_compaction_events(items: &[ResponseItem]) -> Vec<String> {
    let mut tool_names_by_call_id = HashMap::new();

    items
        .iter()
        .filter_map(|item| render_compaction_event(item, &mut tool_names_by_call_id))
        .collect()
}

fn render_compaction_event(
    item: &ResponseItem,
    tool_names_by_call_id: &mut HashMap<String, String>,
) -> Option<String> {
    let rendered = match item {
        ResponseItem::Message { role, content, .. } if role == "user" => {
            if crate::event_mapping::is_contextual_user_message_content(content) {
                None
            } else {
                content_items_to_text(content).map(|text| format!("User\n{text}"))
            }
        }
        ResponseItem::Message { role, content, .. } if role == "assistant" => {
            content_items_to_text(content).map(|text| format!("Assistant\n{text}"))
        }
        ResponseItem::Reasoning {
            summary, content, ..
        } => render_reasoning_event(summary, content.as_deref()),
        ResponseItem::LocalShellCall { status, action, .. } => {
            render_local_shell_event(status, action)
        }
        ResponseItem::FunctionCall {
            call_id,
            name,
            namespace,
            arguments,
            ..
        } => {
            let tool_name = qualified_tool_name(namespace.as_deref(), name);
            tool_names_by_call_id.insert(call_id.clone(), tool_name.clone());
            Some(render_tool_call_event(&tool_name, arguments))
        }
        ResponseItem::CustomToolCall {
            call_id,
            name,
            input,
            ..
        } => {
            tool_names_by_call_id.insert(call_id.clone(), name.clone());
            Some(render_tool_call_event(name, input))
        }
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        }
        | ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => render_tool_output_event(tool_names_by_call_id.get(call_id), output),
        ResponseItem::ToolSearchCall {
            call_id,
            execution,
            arguments,
            ..
        } => {
            if let Some(call_id) = call_id {
                tool_names_by_call_id.insert(call_id.clone(), "tool_search".to_string());
            }
            let body = pretty_json_value(arguments).unwrap_or_else(|| arguments.to_string());
            Some(format!("Tool search call ({execution})\n{body}"))
        }
        ResponseItem::ToolSearchOutput {
            call_id,
            execution,
            status,
            tools,
        } => {
            let label = call_id
                .as_ref()
                .and_then(|id| tool_names_by_call_id.get(id))
                .cloned()
                .unwrap_or_else(|| "tool_search".to_string());
            let body = if tools.is_empty() {
                "[]".to_string()
            } else {
                pretty_json_value(&serde_json::Value::Array(tools.clone()))
                    .unwrap_or_else(|| serde_json::Value::Array(tools.clone()).to_string())
            };
            Some(format!(
                "Tool output: {label} ({execution}, {status})\n{body}"
            ))
        }
        ResponseItem::WebSearchCall { status, action, .. } => {
            render_web_search_event(status.as_deref(), action.as_ref())
        }
        ResponseItem::ImageGenerationCall {
            status,
            revised_prompt,
            result,
            ..
        } => {
            let mut sections = vec![format!("Image generation ({status})")];
            if let Some(prompt) = revised_prompt
                .as_ref()
                .filter(|prompt| !prompt.trim().is_empty())
            {
                sections.push(format!("Prompt\n{prompt}"));
            }
            if !result.trim().is_empty() {
                sections.push(format!("Result\n{result}"));
            }
            Some(sections.join("\n"))
        }
        ResponseItem::Compaction { .. } | ResponseItem::Other | ResponseItem::Message { .. } => {
            None
        }
    }?;

    Some(truncate_rendered_event_lines(
        &rendered,
        DETERMINISTIC_COMPACT_LINE_LIMIT,
    ))
}

fn render_reasoning_event(
    summary: &[codex_protocol::models::ReasoningItemReasoningSummary],
    content: Option<&[codex_protocol::models::ReasoningItemContent]>,
) -> Option<String> {
    let mut sections = Vec::new();
    let summary_text = summary
        .iter()
        .map(|entry| match entry {
            codex_protocol::models::ReasoningItemReasoningSummary::SummaryText { text } => {
                text.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !summary_text.trim().is_empty() {
        sections.push(format!("Reasoning summary\n{summary_text}"));
    }

    let raw_text = content
        .unwrap_or_default()
        .iter()
        .map(|entry| match entry {
            codex_protocol::models::ReasoningItemContent::ReasoningText { text }
            | codex_protocol::models::ReasoningItemContent::Text { text } => text.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !raw_text.trim().is_empty() {
        sections.push(format!("Reasoning\n{raw_text}"));
    }

    (!sections.is_empty()).then(|| sections.join("\n"))
}

fn render_local_shell_event(
    status: &LocalShellStatus,
    action: &LocalShellAction,
) -> Option<String> {
    match action {
        LocalShellAction::Exec(LocalShellExecAction {
            command,
            working_directory,
            ..
        }) => {
            let mut sections = vec![format!(
                "Shell command ({})\n{}",
                local_shell_status_label(status),
                command.join(" ")
            )];
            if let Some(working_directory) = working_directory
                .as_ref()
                .filter(|working_directory| !working_directory.trim().is_empty())
            {
                sections.push(format!("cwd: {working_directory}"));
            }
            Some(sections.join("\n"))
        }
    }
}

fn local_shell_status_label(status: &LocalShellStatus) -> &'static str {
    match status {
        LocalShellStatus::Completed => "completed",
        LocalShellStatus::InProgress => "in_progress",
        LocalShellStatus::Incomplete => "incomplete",
    }
}

fn qualified_tool_name(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(namespace) if !namespace.trim().is_empty() => format!("{namespace}.{name}"),
        _ => name.to_string(),
    }
}

fn render_tool_call_event(tool_name: &str, input: &str) -> String {
    let body = pretty_json_str(input).unwrap_or_else(|| input.to_string());
    format!("Tool call: {tool_name}\n{body}")
}

fn render_tool_output_event(
    tool_name: Option<&String>,
    output: &FunctionCallOutputPayload,
) -> Option<String> {
    output.body.to_text().map(|text| {
        let label = tool_name.map_or("tool", String::as_str);
        format!("Tool output: {label}\n{text}")
    })
}

fn render_web_search_event(
    status: Option<&str>,
    action: Option<&WebSearchAction>,
) -> Option<String> {
    let action_label = action.map_or_else(
        || "web search".to_string(),
        |action| match action {
            WebSearchAction::Search { .. } => "web search".to_string(),
            WebSearchAction::OpenPage { .. } => "open page".to_string(),
            WebSearchAction::FindInPage { .. } => "find in page".to_string(),
            WebSearchAction::Other => "web search".to_string(),
        },
    );
    let detail = action.map(web_search_action_detail).unwrap_or_default();
    let heading = match status {
        Some(status) if !status.trim().is_empty() => format!("{action_label} ({status})"),
        _ => action_label,
    };

    if detail.trim().is_empty() {
        Some(heading)
    } else {
        Some(format!("{heading}\n{detail}"))
    }
}

fn pretty_json_str(value: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .and_then(|value| pretty_json_value(&value))
}

fn pretty_json_value(value: &serde_json::Value) -> Option<String> {
    serde_json::to_string_pretty(value).ok()
}

fn truncate_rendered_event_lines(text: &str, max_lines: usize) -> String {
    if max_lines == 0 {
        return format!("… event truncated after {max_lines} lines");
    }

    let mut kept: Vec<String> = Vec::new();
    let mut truncated = false;
    for (line_count, line) in text.lines().enumerate() {
        if line_count == max_lines {
            truncated = true;
            break;
        }
        kept.push(line.to_string());
    }

    if !truncated {
        return text.to_string();
    }

    kept.push(format!("… event truncated after {max_lines} lines"));
    kept.join("\n")
}

pub fn content_items_to_text(content: &[ContentItem]) -> Option<String> {
    let mut pieces = Vec::new();
    for item in content {
        match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                if !text.is_empty() {
                    pieces.push(text.as_str());
                }
            }
            ContentItem::InputImage { .. } => {}
        }
    }
    if pieces.is_empty() {
        None
    } else {
        Some(pieces.join("\n"))
    }
}

pub(crate) fn collect_user_messages(items: &[ResponseItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match crate::event_mapping::parse_turn_item(item) {
            Some(TurnItem::UserMessage(user)) => {
                if is_summary_message(&user.message()) {
                    None
                } else {
                    Some(user.message())
                }
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn is_summary_message(message: &str) -> bool {
    message.starts_with(format!("{SUMMARY_PREFIX}\n").as_str())
}

/// Inserts canonical initial context into compacted replacement history at the
/// model-expected boundary.
///
/// Placement rules:
/// - Prefer immediately before the last real user message.
/// - If no real user messages remain, insert before the compaction summary so
///   the summary stays last.
/// - If there are no user messages, insert before the last compaction item so
///   that item remains last (remote compaction may return only compaction items).
/// - If there are no user messages or compaction items, append the context.
pub(crate) fn insert_initial_context_before_last_real_user_or_summary(
    mut compacted_history: Vec<ResponseItem>,
    initial_context: Vec<ResponseItem>,
) -> Vec<ResponseItem> {
    let mut last_user_or_summary_index = None;
    let mut last_real_user_index = None;
    for (i, item) in compacted_history.iter().enumerate().rev() {
        let Some(TurnItem::UserMessage(user)) = crate::event_mapping::parse_turn_item(item) else {
            continue;
        };
        // Compaction summaries are encoded as user messages, so track both:
        // the last real user message (preferred insertion point) and the last
        // user-message-like item (fallback summary insertion point).
        last_user_or_summary_index.get_or_insert(i);
        if !is_summary_message(&user.message()) {
            last_real_user_index = Some(i);
            break;
        }
    }
    let last_compaction_index = compacted_history
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, item)| matches!(item, ResponseItem::Compaction { .. }).then_some(i));
    let insertion_index = last_real_user_index
        .or(last_user_or_summary_index)
        .or(last_compaction_index);

    // Re-inject canonical context from the current session since we stripped it
    // from the pre-compaction history. Prefer placing it before the last real
    // user message; if there is no real user message left, place it before the
    // summary or compaction item so the compaction item remains last.
    if let Some(insertion_index) = insertion_index {
        compacted_history.splice(insertion_index..insertion_index, initial_context);
    } else {
        compacted_history.extend(initial_context);
    }

    compacted_history
}

pub(crate) fn build_compacted_history(
    initial_context: Vec<ResponseItem>,
    user_messages: &[String],
    summary_text: &str,
) -> Vec<ResponseItem> {
    build_compacted_history_with_limit(
        initial_context,
        user_messages,
        summary_text,
        COMPACT_USER_MESSAGE_MAX_TOKENS,
    )
}

fn build_compacted_history_with_limit(
    mut history: Vec<ResponseItem>,
    user_messages: &[String],
    summary_text: &str,
    max_tokens: usize,
) -> Vec<ResponseItem> {
    let mut selected_messages: Vec<String> = Vec::new();
    if max_tokens > 0 {
        let mut remaining = max_tokens;
        for message in user_messages.iter().rev() {
            if remaining == 0 {
                break;
            }
            let tokens = approx_token_count(message);
            if tokens <= remaining {
                selected_messages.push(message.clone());
                remaining = remaining.saturating_sub(tokens);
            } else {
                let truncated = truncate_text(message, TruncationPolicy::Tokens(remaining));
                selected_messages.push(truncated);
                break;
            }
        }
        selected_messages.reverse();
    }

    for message in &selected_messages {
        history.push(ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: message.clone(),
            }],
            phase: None,
        });
    }

    let summary_text = if summary_text.is_empty() {
        "(no summary available)".to_string()
    } else {
        summary_text.to_string()
    };

    history.push(ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text: summary_text }],
        phase: None,
    });

    history
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod tests;
