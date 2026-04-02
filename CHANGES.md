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
