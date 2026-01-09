# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust TUI markdown reader. Current layout:

- `src/main.rs` is the CLI entry point.
- Future modules should live in `src/` (e.g., `src/ui.rs`, `src/markdown.rs`).
- Integration tests go in `tests/` and mirror `src/` module names when possible.
- Optional supporting assets should live in `assets/` and docs in `docs/`.

Keep paths stable once introduced and document any new top-level directories here.

## Build, Test, and Development Commands
Use standard Cargo commands:

- `cargo build` - compile a debug build.
- `cargo run -- <path>` - run the reader against a markdown file.
- `cargo test` - run unit and integration tests.
- `cargo fmt` - format Rust code with `rustfmt`.
- `cargo clippy --all-targets --all-features -- -D warnings` - lint and fail on warnings.

## Coding Style & Naming Conventions
Follow idiomatic Rust style:

- 4-space indentation; rely on `rustfmt` for formatting.
- `snake_case` for files, modules, functions, and variables.
- `CamelCase` for types and traits.
- Keep modules focused; prefer small, testable functions.

## Testing Guidelines
Testing should use Rust’s built-in test framework:

- Unit tests live alongside code in `mod tests` blocks.
- Integration tests live in `tests/` and use descriptive file names (e.g., `tests/markdown_render.rs`).
- When adding coverage requirements, document the threshold here and the exact command to verify it.

## Commit & Pull Request Guidelines
There is minimal Git history. Use clear, imperative commit subjects (e.g., "Add TUI layout skeleton"). Keep commits focused and reversible.

For pull requests:

- Include a short summary, testing notes, and any linked issues.
- Provide screenshots or terminal captures for UI changes.
- Call out any breaking behavior changes.

## Security & Configuration Tips
Do not commit secrets. If configuration is needed, use `.env` or `config/*.toml` and add safe defaults or examples.

## Agent-Specific Instructions
Keep this file up to date as commands, tooling, or structure change. Use it as the source of truth for contributors and automated agents.
