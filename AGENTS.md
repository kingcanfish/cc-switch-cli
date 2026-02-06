
# Repository Guidelines

## Project Structure

- `src/`: Rust crate for the `cc-switch-cli` CLI.
  - `src/cli/`: command parsing + interactive TUI flows.
  - `src/services/`: core business logic (providers, MCP, prompts, config, env).
- `tests/`: Rust integration tests (`*.rs`).
- `docs/`: design/refactor notes, release notes, and internal plans.
- `scripts/`: release/versioning helpers (shell + Node scripts).

## Build, Test, and Development Commands

Run commands from the repo root unless noted.

- Build debug binary: `cargo build`
- Build release binary: `cargo build --release`
- Run locally: `cargo run --bin cc-switch-cli -- --help`
- Run tests: `cargo test`
- Format: `cargo fmt`
- Lint (recommended): `cargo clippy --all-targets -- -D warnings`

## Coding Style & Naming Conventions

- Rust formatting is standard `rustfmt`; don’t hand-format—run `cargo fmt`.
- Use Rust conventions: `snake_case` for modules/functions, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Keep CLI output stable and user-facing strings i18n-aware (see `src/cli/i18n.rs`).

## Testing Guidelines

- Tests live in `tests/*.rs` and should avoid touching real user data; follow the existing pattern that isolates `HOME`.
- Prefer focused integration tests that exercise commands/services end-to-end.

## Commit & Pull Request Guidelines

- Follow existing commit patterns (mostly Conventional Commits): `feat: …`, `fix(scope): …`, `refactor: …`, `docs: …`, `chore: …`.
- PRs should include: what changed, why, how to test (`cargo test`), and any user-facing updates (README/CHANGELOG).

## Security & Configuration Tips

- Never commit real API keys or personal config files; the app manages data under `~/.cc-switch-cli/` and writes “live” configs for Claude/Codex/Gemini under their respective home directories.
