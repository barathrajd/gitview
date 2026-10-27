# Gitview releases

This file is the single release plan and release-notes record for Gitview.

## Upcoming

No release is currently in progress. Release Please will prepare future releases
from Conventional Commits on `main`.

## Released

### Gitview 0.4.0 — October 27, 2026

Gitview 0.4.0 adds hunk-level staging, grouped worktree sections, and
page-specific keyboard help.

Highlights:

- Added selected-hunk stage and unstage actions, plus whole-file and
  whole-worktree operations.
- Grouped staged changes separately from unstaged and untracked changes, with
  section-aware selection and highlighting.
- Added background worktree status refresh and contextual help overlays.
- Opened Gitview in the worktree by default and kept selection within the
  originating section after stage or unstage actions.

Downloads:

- `gitview-v0.4.0-macos-arm64.tar.gz` contains the macOS Apple Silicon
  executable.
- `gitview-v0.4.0-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.4.0` tag.

- Release: [GitHub v0.4.0][release-040]
- Tag: `v0.4.0`
- Release PR: [#5](https://github.com/barathrajd/gitview/pull/5)

### Gitview 0.3.3 — October 27, 2026

Published by the Release Please GitHub Action. The action created the matching
tag and release, built the macOS Apple Silicon binary, and uploaded a
single-executable archive and checksum.

- Release: [GitHub v0.3.3][release-033]
- Tag: `v0.3.3`
- Release PR: [#4](https://github.com/barathrajd/gitview/pull/4)

### Gitview 0.3.2 — October 27, 2026

Gitview 0.3.2 publishes a macOS archive containing only the executable and
documents the GNU `find` prerequisite GAH needs on macOS. GAH installation was
verified by the user.

Highlights:

- Reduced the macOS release archive to the Gitview executable only.
- Split GAH installation, Gitview installation, and launch instructions into
  separate steps.
- Explained that macOS GAH installation requires GNU `find` from Homebrew
  `findutils`.
- Consolidated versioned release-note files into this `RELEASE.md`.

Downloads:

- `gitview-v0.3.2-macos-arm64.tar.gz` contains only the macOS Apple Silicon
  executable.
- `gitview-v0.3.2-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.3.2` tag.

- Release: [GitHub v0.3.2][release-032]
- Tag: `v0.3.2`
- Plan: [license and local installation](plans/release-installation-0-3-1.md)
- Definition of Done: [release-plan reference][definition-of-done]

### Gitview 0.3.1 — October 27, 2026

Gitview 0.3.1 clarifies software reuse terms and adds a no-sudo installation
path for macOS Apple Silicon.

Highlights:

- Added the MIT License.
- Documented installing GAH and using it to install Gitview into
  `~/.local/bin`.

Downloads:

- `gitview-v0.3.1-macos-arm64.tar.gz` contains only the macOS Apple Silicon
  executable.
- `gitview-v0.3.1-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.3.1` tag.

GAH installation was verified on macOS Apple Silicon using GNU `find` from
Homebrew `findutils`. Linux release packaging and Windows support remain
deferred.

- Release: [GitHub v0.3.1][release-031]
- Tag: `v0.3.1`
- Plan: [license and local installation](plans/release-installation-0-3-1.md)
- Definition of Done: [release-plan reference][definition-of-done]

### Gitview 0.3.0 — October 27, 2026

Gitview 0.3.0 improves history and diff review with a responsive cockpit,
clearer search, and consistent keyboard guidance.

Highlights:

- Added a responsive cockpit for repository status, history, and commit
  details.
- Added debounced history search with selectable results that open commit
  details directly.
- Added old and new line numbers to diff views.
- Replaced wide shortcut tables with a compact, consistent shortcut bar.
- Made **Esc** and **?** close the help panel and removed the ambiguous
  `n next match` shortcut.

Downloads:

- `gitview-v0.3.0-macos-arm64.tar.gz` contains the macOS Apple Silicon binary
  and README.
- `gitview-v0.3.0-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.3.0` tag.

Linux was verified for 0.2.1 but has not been rechecked for this release.
Windows support and Linux release packaging remain deferred.

- Release: [GitHub v0.3.0][release-030]
- Tag: `v0.3.0`
- Plan: [UI navigation polish](plans/ui-navigation-0-3-0.md)
- Definition of Done: [release-plan reference][definition-of-done]

### Gitview 0.2.1 — October 27, 2026

Gitview 0.2.1 refactors the terminal UI into testable application state,
actions, and rendering modules while preserving the history, working-tree,
diff, and local branch workflows.

Highlights:

- Added Ratatui `TestBackend` coverage for help, history, branches, commit
  details, working-tree states, errors, confirmations, and narrow and wide
  diff layouts.
- Added regression coverage for stale branch and unstage selections.
- Added a PTY regression test confirming Ratatui restores terminal modes and
  leaves the alternate screen when the application panics.
- Verified formatting, tests, Clippy, and builds on macOS Apple Silicon and
  Linux x86_64.

Downloads:

- `gitview-v0.2.1-macos-arm64.tar.gz` contains the macOS Apple Silicon binary
  and README.
- GitHub provides source archives for the `v0.2.1` tag.

Windows support and Linux release packaging remain deferred.

- Release: [GitHub v0.2.1][release-021]
- Tag: `v0.2.1`
- Plan: [0.2.1 refactor](plans/refactor-0-2-1.md)

[release-030]: https://github.com/barathrajd/gitview/releases/tag/v0.3.0
[release-031]: https://github.com/barathrajd/gitview/releases/tag/v0.3.1
[release-032]: https://github.com/barathrajd/gitview/releases/tag/v0.3.2
[release-033]: https://github.com/barathrajd/gitview/releases/tag/v0.3.3
[release-040]: https://github.com/barathrajd/gitview/releases/tag/v0.4.0
[release-021]: https://github.com/barathrajd/gitview/releases/tag/v0.2.1
[definition-of-done]:
  .agents/skills/release-plan/references/definition-of-done.md
