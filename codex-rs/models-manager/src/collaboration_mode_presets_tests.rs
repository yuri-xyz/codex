use super::*;
use pretty_assertions::assert_eq;

#[test]
fn preset_names_use_mode_display_names() {
    assert_eq!(plan_preset().name, ModeKind::Plan.display_name());
    assert_eq!(build_preset().name, ModeKind::Build.display_name());
    assert_eq!(
        unrestricted_preset().name,
        ModeKind::Unrestricted.display_name()
    );
    assert_eq!(default_preset().name, ModeKind::Default.display_name());
    assert_eq!(plan_preset().model, None);
    assert_eq!(
        plan_preset().reasoning_effort,
        Some(Some(ReasoningEffort::Medium))
    );
    assert_eq!(default_preset().model, None);
    assert_eq!(default_preset().reasoning_effort, None);
}

#[test]
fn build_preset_includes_build_mode_instructions() {
    let build_instructions = build_preset()
        .developer_instructions
        .expect("build preset should include instructions")
        .expect("build instructions should be set");

    assert!(build_instructions.contains("You are now in Build mode."));
    assert!(build_instructions.contains("runtime/UI will request it automatically"));
}

#[test]
fn unrestricted_preset_includes_unrestricted_mode_instructions() {
    let unrestricted_instructions = unrestricted_preset()
        .developer_instructions
        .expect("unrestricted preset should include instructions")
        .expect("unrestricted instructions should be set");

    assert!(unrestricted_instructions.contains("You are now in Unrestricted mode."));
    assert!(unrestricted_instructions.contains("avoid approval prompts"));
}

#[test]
fn default_mode_instructions_replace_mode_names_placeholder() {
    let default_instructions = default_preset()
        .developer_instructions
        .expect("default preset should include instructions")
        .expect("default instructions should be set");

    assert!(!default_instructions.contains("{{KNOWN_MODE_NAMES}}"));

    let known_mode_names = format_mode_names(&TUI_VISIBLE_COLLABORATION_MODES);
    let expected_snippet = format!("Known mode names are {known_mode_names}.");
    assert!(default_instructions.contains(&expected_snippet));

    let expected_availability_message = request_user_input_availability_message(
        ModeKind::Default,
        /*default_mode_request_user_input*/ true,
    );
    assert!(default_instructions.contains(&expected_availability_message));
    assert!(default_instructions.contains("prefer using the `request_user_input` tool"));
}

#[test]
fn default_mode_instructions_prefer_request_user_input_by_default() {
    let default_instructions = default_preset()
        .developer_instructions
        .expect("default preset should include instructions")
        .expect("default instructions should be set");

    assert!(default_instructions.contains("prefer using the `request_user_input` tool"));
}

#[test]
fn default_mode_instructions_use_plain_text_questions_when_feature_disabled() {
    let default_instructions = default_preset_with_config(CollaborationModesConfig {
        default_mode_request_user_input: false,
    })
    .developer_instructions
    .expect("default preset should include instructions")
    .expect("default instructions should be set");

    assert!(!default_instructions.contains("prefer using the `request_user_input` tool"));
    assert!(
        default_instructions.contains("ask the user directly with a concise plain-text question")
    );
}
