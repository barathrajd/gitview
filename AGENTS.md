# Repository Guidelines

## Project Structure & Scope

This repository currently contains a product plan, not an initialized Rust project. Read `GIT_TUI_TWO_WEEK_PLAN.md` for the MVP scope, architecture, schedule, and acceptance criteria. `skills-lock.json` and `.agents/skills/` contain agent skill configuration. Source code, tests, assets, and a Cargo manifest have not yet been added.

The planned application is a keyboard-first Git terminal UI. Keep the first release focused on browsing history and diffs, working-tree review, and stage/unstage actions; the plan lists features deferred beyond the MVP.

## Build, Test, and Development

There are no project build or test commands yet. After initializing the Rust project, document the actual commands here. Standard Cargo commands to verify once the manifest exists include `cargo run` for local execution, `cargo test` for tests, `cargo fmt --check` for formatting, and `cargo clippy` for linting. Do not present these as currently working commands.

## Coding Style & Architecture

For Rust code, use `rustfmt` conventions and idiomatic names: `snake_case` for modules and functions, `PascalCase` for types. Follow the planned separation between application state, Git adapter, UI rendering, and input/actions. Invoke Git with argument arrays rather than shell command strings; prefer machine-readable, NUL-delimited output where available. Keep subprocess work off the render loop and expose loading and error states.

## Testing Guidelines

Add automated tests alongside implementation, including Git parsing and filenames with spaces or unusual characters. Exercise a sample repository with merges, tags, and staged, unstaged, and untracked changes. Check terminal restoration, narrow layouts, detached HEAD, empty repositories, and large diffs. No testing framework or coverage threshold is configured yet.

## Commits & Pull Requests

No Git history is available here, so no existing commit convention can be established. Write concise, descriptive commit subjects. Pull requests should explain the change and validation, link relevant issues, and include terminal screenshots or recordings for visible UI changes.
