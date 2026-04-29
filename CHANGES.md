# Changes

This file tracks high-level, fork-specific changes made in this personal `yuri-xyz` fork of Codex.

Keep entries brief and outcome-focused. Use this file for notable behavior, product, privacy, or workflow changes, not for low-level implementation detail.

You can also inspect the exact fork-specific commit history directly by filtering Git history to commits authored by `yuri-xyz`.

Example:

```bash
git log --author="yuri-xyz" --oneline
```

## Current Fork Changes

- Improved privacy by disabling telemetry-related reporting paths, including analytics, OTEL tracing/export, feedback upload, and trace propagation.
- Removed unused TUI slash commands: `/feedback`, `/diff`, and `/plugins`.
- Removed the debug-only `/rollout` slash command from the TUI because this fork does not need a user-facing rollout-path shortcut.
- Removed the debug-only `/testapproval` slash command from the TUI because this fork does not need synthetic approval-request tooling in the user command surface.
- Made the `request_user_input` question tool available in both Default and Plan collaboration modes by default.
- Increased visible exec-command output in the TUI so shell results keep more head/tail lines before truncation.
- Added a local installer script for Apple Silicon macOS that builds this fork and links it to the `code` command.
- Extended the local `code` installer to support Linux as well, and prefer rootless user-space link locations such as `~/.local/bin` on Fedora/Linux before system directories.
- Added Linux installer preflight checks so missing Fedora build dependencies such as `gcc`, `openssl-devel`, and `libcap-devel` fail fast with explicit package hints instead of surfacing later as Cargo build-script errors.
- Added a `Build` collaboration mode to the TUI mode cycle so file edits always require explicit approval while keeping build-oriented execution available without switching into Plan mode.
- Fixed `Build` mode patch approvals in the TUI so file edits open the approval UI with the real diff before the edit is shown in transcript history, and stopped the mode instructions from telling the model to ask for permission in plain text first.
- Fixed patch-approval diff rendering so the approval modal preserves file-extension-based syntax highlighting instead of dropping it when rendering per-file changes.
- Capped visible patch/diff previews in the TUI at 200 lines and added a truncation marker so large file edits do not flood transcript history or approval dialogs.
- Stopped logging passive background-terminal wait events into the chat transcript so polling-only waits stay in status UI instead of adding message noise.
- Renamed the status-line context-used label from `used` to `context` and highlight it in yellow once usage reaches 85% or higher.
- Renamed the visible TUI startup/header product label from `OpenAI Codex` to `Jailbroken Codex`.
- Updated the local `code` installer to link directly to the repo build artifact because copied home-directory installs were getting killed on startup on this machine.
- Disabled the TUI startup update check so this fork no longer fetches or shows upstream release-update prompts on launch.
- Replaced LLM-based context compaction with deterministic local compaction that folds the last 40 visible transcript events into a summary-style context message and ends it with `You left here, continue.` instead of calling remote compact APIs.
- Removed `/mention` from the slash-command list because this fork uses direct `@` mentions instead of a separate user-facing shortcut for opening mention insertion.
- Added a default Plan-mode handoff option to implement with fresh context, which starts a new session before sending the normal implementation handoff message.
- Changed Plan-mode implementation handoff so it submits the actual proposed plan text followed by `Implement the plan.` instead of sending only the bare implementation sentence.
- Adjusted the fresh-context Plan handoff to use the full clear-UI path so it clears visible chat history as well as model context before starting implementation in a new session.
- Disabled all GitHub Actions in this fork by moving workflow YAML files out of `.github/workflows`, so GitHub no longer discovers or runs the upstream CI/release/automation workflows here.
- Added embedded Funktion syntax highlighting for `.fun` files in the TUI so Funktion diffs, approval previews, and fenced code blocks render with language-aware coloring.
- Hid `/plan` from the visible slash-command list and prefix suggestions while keeping direct typed `/plan` usage available for users who already know the shortcut.
- Added an `Unrestricted` collaboration mode to the Shift+Tab mode cycle that temporarily applies Full Access behavior without approval prompts, then restores the prior permission settings when you leave the mode.
- Made `Shift+Tab` inside approval confirmations auto-approve the current request and immediately switch the session into `Unrestricted` mode so the rest of the request, and later requests, keep running without prompts until you cycle modes again.
- Added a `Meta+S` composer stash flow: with a non-empty draft it stashes the current prompt into a new stash group above the input, and with a blank composer it pops the most recently stashed draft back into the composer.
- Added a direnv-backed Nix development environment so repo tools such as `just`, Rust, OpenSSL, and libclang are loaded automatically when entering the checkout.
