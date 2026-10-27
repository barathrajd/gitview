# Rust Terminal Git UI: Two-Week MVP Plan

**Working title:** `gitview` (temporary; choose the final name before publishing)  
**Goal:** Build a modern, keyboard-first terminal application for daily Git work, with a commit graph and readable diffs. It should reduce the need to open VS Code solely to inspect Git history and changes.

## Product direction

Use **Git Graph** as a workflow reference and **Lazygit** as a terminal interaction reference. The product should have its own terminal-first design rather than reproduce either interface. Git Graph’s useful reference features include branch/ref visibility, commit details, file changes, and Git actions from the graph. The first release should prioritize a reliable graph-to-diff review flow and safe common operations.

### Two-week launch target

A usable **MVP preview release** for macOS, with Linux builds if the development environment permits. “Launch” means a tagged release, install/run instructions, a demo repository, and known limitations. Two weeks is realistic for a focused MVP by one developer; it is not enough to match every Lazygit or Git Graph operation or promise a production-hardened cross-platform app.

## MVP experience

```text
┌ Repository / branch / sync state ──────────────────────────────────────┐
│ Commits + graph       │ Commit details / changed files │ Diff preview  │
│                       │                                │               │
├───────────────────────┴────────────────────────────────┴───────────────┤
│ Key hints · search · operation feedback                                │
└───────────────────────────────────────────────────────────────────────┘
```

- Main view: commit graph with current/local/remote branch and tag labels.
- Selecting a commit shows message, author, date, hash, and changed files.
- Selecting a changed file opens a colored, **side-by-side line diff** when terminal width allows; fall back to unified diff on narrow terminals.
- A working-tree view shows staged, unstaged, and untracked files, with diff preview and stage/unstage actions.
- Search/filter commits by message, author, or hash.
- Keyboard-first navigation, a discoverable help overlay, resize handling, and terminal restoration on normal exit or panic.
- Read-only browsing works even before the user runs a mutation.

## Explicitly out of scope for the first release

Interactive rebase, cherry-pick, conflict resolution, submodule management, reflog/stash management, worktree management, GitHub/GitLab integration, mouse-first interaction, a plugin system, and a full porcelain implementation in Rust. Add these only after the core review flow is stable and user feedback supports them.

## Recommended implementation choices

- **Rust + Ratatui + Crossterm** for the terminal UI and input loop. Ratatui is a Rust TUI library; its current docs describe Crossterm support and terminal initialization/restoration helpers. Pin compatible crate versions in `Cargo.toml` and `Cargo.lock` at project start.
- **Use the installed `git` executable for Git operations in the MVP.** Invoke it with argument arrays (never construct shell command strings), capture stdout/stderr, and parse stable formats such as `--porcelain=v2 -z`, `--format`, and `--numstat`. This keeps Git’s mature behavior for status, graph/log, refs, and diff while limiting implementation risk.
- Keep a Git adapter boundary so a later version can evaluate `gix` (gitoxide) for in-process Git operations. Do not build the two-week schedule around implementing Git semantics with a new library.
- Use a small app-state/event architecture: `app` state, `git` adapter, `ui` rendering, and `input`/actions. Keep Git subprocess work off the render loop; show loading/error states and refresh after mutations.
- Prefer a restrained theme: clear selection, readable contrast, branch/ref badges, consistent spacing, and full-line diff backgrounds. Do not depend on Nerd Font glyphs for essential meaning; provide a plain-text fallback.

## Two-week schedule (10 working days)

| Day | Phase | Work | Exit check |
|---|---|---|---|
| 1 | Scope and UX | Confirm MVP workflow; sketch main screen and keyboard map; define supported OS/terminal; create sample repo with merge, tags, staged/unstaged/untracked changes. | Screen sketch and acceptance checklist are agreed; scope is frozen. |
| 2 | Project foundation | Create Cargo project; add Ratatui/Crossterm; implement CLI repo discovery, `--help`, error handling, terminal lifecycle and panic-safe restoration. | App starts in a Git repo, gives a useful message outside one, and restores terminal state on exit. |
| 3 | Git data layer | Implement repository root/branch detection, status parsing, commit log and ref parsing using safe subprocess calls and machine-readable Git output. | Sample repo data appears in typed models; Git errors are shown without crashing. |
| 4 | Graph view | Render commit rows, graph lanes, branch/tag labels, current commit and selection; support up/down navigation and resize. | User can browse a useful recent history in a terminal at least 100 columns wide. |
| 5 | Details and files | Add commit details and changed-file list; support file selection and refresh. | Selecting commits and files updates details without blocking rendering. |
| 6 | Diff review | Render unified and side-by-side diffs, line-level additions/deletions, context, scrolling, and narrow-terminal fallback. | User can inspect a selected commit’s changed file and understand changes without relying on `+`/`-` alone. |
| 7 | Working tree | Add staged/unstaged/untracked sections, file diff preview, stage/unstage and refresh. | Core daily review flow works on real working-tree changes. |
| 8 | Search and polish | Add commit search/filter, help overlay, key hints, empty/loading/error states, theme and accessibility pass. | First-time user can discover navigation and recover from normal errors. |
| 9 | Hardening and packaging | Exercise sample and real repositories; handle detached HEAD, no commits, renames/deletions, large diffs, missing Git, narrow terminal; build macOS binary and Linux binary if available; write README. | Release checklist passes on target environment; critical data parsing/UI bugs fixed. |
| 10 | Preview launch | Final smoke check; record demo; tag `v0.1.0`; publish binary/source and install/run guide; collect feedback/issues. | A new user can install or build, open a repo, inspect history/diffs, and find limitations. |

## Acceptance criteria for v0.1.0

1. Opens the current Git repository and reports a clear error outside one or when Git is unavailable.
2. Displays recent commit history with recognizable graph lanes and local/remote branches and tags.
3. Shows commit metadata and changed-file list for the selected commit.
4. Shows both commit diffs and working-tree diffs, including staged, unstaged, and untracked files.
5. Provides side-by-side colored diff when there is enough width and a readable fallback otherwise.
6. Supports stage and unstage for selected files; destructive actions are not in the MVP.
7. Never blocks the UI during Git subprocess work; errors are visible and actionable.
8. Restores terminal state on quit and on panic; supports terminal resize and a plain-text glyph fallback.
9. Includes installation/build instructions, keybindings, supported platforms, limitations, and a sample/demo workflow.

## Daily operating rhythm

- Start each day with a runnable build and a small acceptance goal.
- Keep a working vertical slice; avoid building a large Git abstraction before the UI can display real data.
- At the end of each day, run the app against the sample repository and record blockers.
- Freeze MVP scope after Day 1. Move new feature requests to a post-release list.
- Reserve Days 9–10 for reliability and release work; do not spend them adding major features.

## Risks and mitigations

- **Graph rendering edge cases:** start with bounded recent history and a clear graph model; test merges and branches in the sample repo.
- **Git output parsing:** use NUL-delimited or explicitly formatted output where available; test filenames with spaces, tabs, and unusual characters.
- **Terminal width and Unicode variation:** responsive layout plus ASCII/plain-text fallback.
- **Large repositories/diffs:** cap initial history, load details on selection, and avoid rendering entire blobs eagerly.
- **Schedule pressure:** if behind, ship read-only graph + commit details + diff first; keep stage/unstage as the first feature to cut.
- **Release expectations:** label the first tag as an MVP preview and list missing operations clearly.

## Post-MVP roadmap

1. Branch checkout/create/delete and fetch/pull/push with confirmation and progress.
2. Compare arbitrary commits/branches; improved graph filtering and navigation.
3. Stash, tags, worktrees, and safe commit creation.
4. Interactive rebase and conflict workflows only after a robust operation/undo design.
5. Evaluate `gix` incrementally behind the Git adapter; keep Git CLI fallback for incomplete operations.
6. Add Windows support and broader terminal compatibility based on actual demand.

## References

- [Git Graph for VS Code: feature overview](https://marketplace.visualstudio.com/items?itemName=mhutchie.git-graph)
- [Ratatui documentation](https://ratatui.rs/)
- [Ratatui installation and version requirements](https://ratatui.rs/installation/)
- [gitoxide / gix project](https://github.com/GitoxideLabs/gitoxide)
