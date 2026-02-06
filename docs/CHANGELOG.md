# Changelog

All notable updates to this project are documented in this file.

## v0.1.3 - 2026-02-06

### Added
- Full-screen TUI flow for interactive mode across provider, MCP, prompts, skills, and config workflows.
- Persistent TUI runtime/session with page stack to avoid re-initializing the UI for each step.
- Bottom lightweight status spinner for long-running actions (no extra loading page).

### Changed
- Interactive mode is TUI-first by default; non-TUI fallback paths in interactive flows were removed.
- Main interactive title alignment and multiple menu text entries were polished for readability.
- Provider and speedtest command output labels were further localized (i18n coverage expanded).

### Fixed
- Flicker/flash issues when entering nested pages (provider, MCP, skills, config paths).
- Cases where TUI frame disappeared after external editor operations (for example, editing config snippets via `vim`).
- Skill search/info/repo flows that previously dropped out of TUI or appeared to hang.
- Multi-select interactions with filtering where space/left/right behavior could break.
- Cursor flash between page transitions while loading status is shown.

