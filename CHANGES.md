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
- Added a `Build` collaboration mode to the TUI mode cycle so file edits always require explicit approval while keeping build-oriented execution available without switching into Plan mode.
