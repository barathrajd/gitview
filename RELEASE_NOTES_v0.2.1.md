# Gitview 0.2.1

Gitview 0.2.1 refactors the terminal UI into testable application state,
actions, and rendering modules while preserving the existing history,
working-tree, diff, and local branch workflows.

## Highlights

- Added Ratatui `TestBackend` coverage for help, history, branches, commit
  details, working-tree states, errors, confirmations, and narrow and wide
  diff layouts.
- Added regression coverage for stale branch and unstage selections.
- Added a PTY regression test confirming Ratatui restores terminal modes and
  leaves the alternate screen when the application panics.
- Verified formatting, tests, clippy, and builds on macOS Apple Silicon and
  Linux x86_64.

## Downloads

- `gitview-v0.2.1-macos-arm64.tar.gz` contains the macOS Apple Silicon binary
  and README.
- GitHub also provides source archives for the `v0.2.1` tag.

Windows support and Linux release packaging remain deferred.
