use codex_collaboration_mode_templates::BUILD as COLLABORATION_MODE_BUILD;
use codex_collaboration_mode_templates::DEFAULT as COLLABORATION_MODE_DEFAULT;
use codex_collaboration_mode_templates::PLAN as COLLABORATION_MODE_PLAN;
use codex_collaboration_mode_templates::UNRESTRICTED as COLLABORATION_MODE_UNRESTRICTED;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::config_types::ModeKind;
use codex_protocol::config_types::TUI_VISIBLE_COLLABORATION_MODES;
use codex_protocol::openai_models::ReasoningEffort;
use codex_utils_template::Template;
use std::sync::LazyLock;

const KNOWN_MODE_NAMES_TEMPLATE_KEY: &str = "KNOWN_MODE_NAMES";
static COLLABORATION_MODE_DEFAULT_TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
    Template::parse(COLLABORATION_MODE_DEFAULT)
        .unwrap_or_else(|err| panic!("collaboration mode default template must parse: {err}"))
});

/// Stores feature flags that control collaboration-mode behavior.
///
/// Keep mode-related flags here so new collaboration-mode capabilities can be
/// added without large cross-cutting diffs to constructor and call-site
/// signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollaborationModesConfig {
    /// Enables `request_user_input` availability in Default mode.
    pub default_mode_request_user_input: bool,
}

impl Default for CollaborationModesConfig {
    fn default() -> Self {
        Self {
            default_mode_request_user_input: true,
        }
    }
}

pub fn builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    builtin_collaboration_mode_presets_with_config(CollaborationModesConfig::default())
}

fn builtin_collaboration_mode_presets_with_config(
    collaboration_modes_config: CollaborationModesConfig,
) -> Vec<CollaborationModeMask> {
    vec![
        default_preset(),
        build_preset(),
        unrestricted_preset(),
        plan_preset(),
    ]
    .into_iter()
    .map(|mut preset| {
        if preset.mode == Some(ModeKind::Default) {
            preset.developer_instructions =
                Some(Some(default_mode_instructions(collaboration_modes_config)));
        }
        preset
    })
    .collect()
}

fn plan_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Plan.display_name().to_string(),
        mode: Some(ModeKind::Plan),
        model: None,
        reasoning_effort: Some(Some(ReasoningEffort::Medium)),
        developer_instructions: Some(Some(COLLABORATION_MODE_PLAN.to_string())),
    }
}

fn build_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Build.display_name().to_string(),
        mode: Some(ModeKind::Build),
        model: None,
        reasoning_effort: None,
        developer_instructions: Some(Some(COLLABORATION_MODE_BUILD.to_string())),
    }
}

fn unrestricted_preset() -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Unrestricted.display_name().to_string(),
        mode: Some(ModeKind::Unrestricted),
        model: None,
        reasoning_effort: None,
        developer_instructions: Some(Some(COLLABORATION_MODE_UNRESTRICTED.to_string())),
    }
}

fn default_preset() -> CollaborationModeMask {
    default_preset_with_config(CollaborationModesConfig::default())
}

fn default_preset_with_config(
    collaboration_modes_config: CollaborationModesConfig,
) -> CollaborationModeMask {
    CollaborationModeMask {
        name: ModeKind::Default.display_name().to_string(),
        mode: Some(ModeKind::Default),
        model: None,
        reasoning_effort: None,
        developer_instructions: Some(Some(default_mode_instructions(collaboration_modes_config))),
    }
}

fn default_mode_instructions(collaboration_modes_config: CollaborationModesConfig) -> String {
    let known_mode_names = format_mode_names(&TUI_VISIBLE_COLLABORATION_MODES);
    let rendered = COLLABORATION_MODE_DEFAULT_TEMPLATE
        .render([(KNOWN_MODE_NAMES_TEMPLATE_KEY, known_mode_names.as_str())])
        .unwrap_or_else(|err| panic!("collaboration mode default template must render: {err}"));
    let availability_message = request_user_input_availability_message(
        ModeKind::Default,
        collaboration_modes_config.default_mode_request_user_input,
    );
    rendered.replace(
        "ask the user directly with a concise plain-text question",
        &availability_message,
    )
}

fn request_user_input_availability_message(
    _mode: ModeKind,
    default_mode_request_user_input: bool,
) -> String {
    if default_mode_request_user_input {
        "prefer using the `request_user_input` tool".to_string()
    } else {
        "ask the user directly with a concise plain-text question".to_string()
    }
}

fn format_mode_names(modes: &[ModeKind]) -> String {
    let mode_names: Vec<&str> = modes.iter().map(|mode| mode.display_name()).collect();
    match mode_names.as_slice() {
        [] => "none".to_string(),
        [mode_name] => (*mode_name).to_string(),
        [first, second] => format!("{first} and {second}"),
        [..] => mode_names.join(", "),
    }
}

#[cfg(test)]
#[path = "collaboration_mode_presets_tests.rs"]
mod tests;
