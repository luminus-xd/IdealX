# Repository Guidelines

## Project Structure & Module Organization
The bot entry point lives in `src/main.rs`, where Serenity event handling, Shuttle integration, and tracing setup converge. Message generation is split into service modules: `src/claude.rs` handles Anthropic calls, `src/chatgpt.rs` encapsulates OpenAI access, and user slash commands reside in `src/commands/` (e.g. `age.rs`). Shared request types are defined alongside each provider; add new integrations near the relevant file. Keep integration tests adjacent to the module or stage broader suites under `tests/`. Deployment configuration lives in the root `Dockerfile`, and runtime secrets are sourced from `Secrets.toml`.

## Build, Test, and Development Commands
- `cargo build` compiles the bot and surfaces type errors quickly.
- `cargo run --bin ideal-x` boots the Discord client locally; export the Discord and Claude tokens beforehand.
- `cargo test` runs unit and integration tests; append `-- --nocapture` to observe log output.
- `cargo fmt` and `cargo clippy --all-targets --all-features` enforce formatting and linting before review.
- `cargo shuttle deploy --allow-dirty` publishes to Shuttle, ensuring `Secrets.toml` uploads with the first release.

## Coding Style & Naming Conventions
Use Rust 2021 defaults with four-space indentation. Modules and functions stay `snake_case`; structs, enums, and traits use `PascalCase`. Prefer `tracing` macros for diagnostics over `println!`, and keep user-facing strings localized where practical. Run `cargo fmt` before committing; resolve `clippy` warnings unless a comment explains the exception.

## Testing Guidelines
Add focused unit tests with `#[cfg(test)]` blocks near the logic they validate, especially around regex filtering and message chunking. For integration flows, place tests under `tests/` and mock Discord events or API calls. Failing tests must block deployment; aim for coverage that exercises new branches. Document manual Discord verification steps in the PR when automated checks are not feasible.

## Commit & Pull Request Guidelines
Commits should use short, present-tense summaries (the log mixes English and Japanese, both are acceptable) and keep related changes together. Reference issues with `Refs #NN` when relevant. Pull requests need a concise behavior summary, screenshots of Discord output for visible changes, and notes on secret or environment adjustments. Request at least one review before merging.

## Secrets & Deployment Notes
Store `DISCORD_TOKEN` and `CLAUDE_TOKEN` in `Secrets.toml`; never commit real values. Run `cargo shuttle login` before deploying, and rotate tokens promptly after testing ephemeral environments.
