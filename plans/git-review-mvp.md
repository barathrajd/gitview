# Git Review MVP

## Objective

Give developers doing daily Git work a keyboard-first terminal application for reviewing repository history, commit details, diffs, and working-tree changes without switching to a graphical editor. Ship an explicitly labeled macOS preview; offer Linux builds if the environment permits.

## Success criteria

- From a Git repository, the app opens and shows recent commits with graph lanes, local/remote refs and tags; outside a repository or without Git, it reports an actionable error.
- Selecting a commit shows its message, author, date, hash and changed files; selecting a file shows its diff. Merge histories and renames are covered by the demo repository.
- The working-tree view distinguishes staged, unstaged and untracked files, previews their changes, and stages or unstages a selected file; a refresh reflects the resulting Git state.
- Diffs are side-by-side when space permits and unified on narrow terminals; additions and deletions remain identifiable without relying solely on color.
- Keyboard navigation, search by message/author/hash, help, resize, loading, empty and error states work without blocking input during Git operations. Terminal state is restored on normal exit and panic.
- A macOS preview includes a binary or reproducible build, installation instructions, keybindings, supported platforms, known limitations, and a demo workflow. Linux is included only if built and verified.

## In scope / Out of scope

**In scope**

- Recent history and graph, refs, commit/file selection, commit and working-tree diffs, search, stage/unstage and refresh.
- Responsive terminal layout, plain-text glyph fallback, errors, terminal lifecycle, sample repository and preview documentation.
- macOS validation; best-effort Linux builds and validation.

**Out of scope**

- Interactive rebase, cherry-pick, conflict resolution, submodules, stash/reflog/worktree management, Git hosting integrations, mouse-first controls and plugins.
- Destructive Git operations, Windows support, and replacing the Git CLI with a full Rust Git implementation.

## Approach

Use Rust with Ratatui and Crossterm, with separate app state, input/actions, rendering and a Git adapter. Select compatible versions and commit the lockfile at project initialization; the exact versions and minimum Rust version must be verified then. Crossterm supplies keyboard and resize events; use one event-reading thread as required by its [event documentation](https://docs.rs/crossterm/latest/crossterm/event/index.html). Restore the terminal on exit and panic using the selected Ratatui version's supported lifecycle ([Ratatui panic example](https://ratatui.rs/examples/apps/panic/)).

Run the installed `git` as a subprocess through literal argument arrays ([Rust `Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html)); use explicit formats and `git status --porcelain=v2 -z` for machine parsing ([Git status](https://git-scm.com/docs/git-status)). Use `--` and literal path handling for selected files; test unusual filenames. Run Git work on a worker and deliver results to the UI with request identities so stale selections cannot overwrite newer results. Refresh status after mutations and surface failures without claiming success.

Initial UX decisions: arrow keys navigate, Tab changes pane, Enter selects, `/` searches, `?` opens help, `r` refreshes, `q` quits; show available stage/unstage keys in the working-tree view and help. Use a unified diff when side-by-side panes cannot display readable content. Limit initial history and load details on selection; indicate any truncation. Treat binary files as binary and display a clear notice instead of pretending to render a text diff.

## Milestones

- [x] **1. Runnable shell:** Initialize Cargo, pin compatible dependencies, discover the repository, show a basic screen and help, and restore the terminal on quit and panic. Verify missing Git and non-repository errors.
- [x] **2. Browse history:** Load a bounded recent history asynchronously and navigate graph rows with refs. Verify branches, tags, merge lanes, empty repository and detached HEAD against a reproducible demo repository.
- [x] **3. Review commits:** Select a commit and file to view metadata and unified diff; add side-by-side display and narrow-width fallback. Verify renames, binary files and selection changes during loading.
- [x] **4. Review and stage changes:** Show staged, unstaged and untracked files and their diffs; stage/unstage selected files and refresh. Test filenames with whitespace, leading dashes and unusual characters, including an unborn branch and Git failures.
- [x] **5. Discover and harden:** Add search, help, feedback and resize handling. Test large histories/diffs, responsiveness, stale results, terminal recovery and plain-text fallback; document any limits.
- [x] **6. Local preview:** Verified the macOS build and demo, documented usage and limitations, prepared the Apple Silicon archive, and created the local `v0.1.0` tag. Linux was not verified; the user chose to skip the GitHub prerelease.

## Risks & open questions

- **Graph complexity:** merge topology may exceed the time budget. Fallback: bound history and document limitations; do not misrepresent parent relationships.
- **Git parsing and path safety:** filenames and rename records can be tricky. Fallback: use Git's machine-readable formats, fixture tests, and report unsupported cases rather than perform an ambiguous mutation.
- **Diff size and terminal width:** large or binary diffs can harm responsiveness. Fallback: bound loaded content, indicate truncation, and show unified or binary notices.
- **Platform and dependency compatibility:** exact crate versions, minimum Rust version, and Linux release availability are unverified until initialization and platform checks. Fallback: pin tested compatible versions and ship the verified macOS preview with explicit limitations.
- **Schedule:** the two-week target is an estimate. If behind, propose a read-only preview and defer staging only with the owner's agreement; do not silently change the confirmed scope.

## Boundaries

Preserve the user's repository and existing index except for explicitly selected stage/unstage actions. Never construct shell commands from repository content; do not run destructive commands. Keep the UI responsive and restore terminal state on normal exit and panic. The existing two-week plan remains historical context; this plan and the roadmap describe the active MVP scope. Do not expand the MVP without an agreed plan change.
