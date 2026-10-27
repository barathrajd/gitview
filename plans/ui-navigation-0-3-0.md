# UI navigation polish for 0.3.0

This plan groups the responsive cockpit and navigation changes since 0.2.1 into
the Gitview 0.3.0 release.

## Objective

Make history, search, commit details, and diff review easier to navigate with a
consistent terminal layout and clear shortcuts.

## Scope

- Show repository status, history, and selected commit details in a responsive
  cockpit.
- Use one compact shortcut bar across pages and keep the full key guide in the
  help panel.
- Debounce history search, show selectable matches, and open the selected
  commit details directly.
- Show old and new line numbers in diffs.
- Close help with **Esc** or **?** and remove the ambiguous `n next match`
  shortcut.

## Acceptance criteria

- [x] The cockpit and subpages use consistent colors and shortcut styling.
- [x] Search results update while typing; Up/Down selects and Enter opens a
  commit.
- [x] Diff views show source line numbers.
- [x] Help closes with Esc and `?`.
- [x] The full test suite, Clippy, formatting, and macOS Apple Silicon release
  build pass.
- [x] The archive includes the executable and README.

## Release boundary

Ship the macOS Apple Silicon archive and GitHub source archives. Linux release
packaging, Windows support, merge workflows, and AI chat remain out of scope.
