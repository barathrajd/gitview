# Gitview releases

This file is the single release plan and release-notes record for Gitview.

## Upcoming

### Gitview 0.6.0 — target December 31, 2026 (planned)

Complete the six partially implemented Basic workflows: repository, worktree,
commit, history, local branches/tags, and diff/inspection. Remotes, merge, and
export remain outside this release.

Features:

- Repository and worktree implementation milestones are complete: init/clone,
  ignore inspection, path removal/moves, and staged-rename discard. Release
  acceptance review remains pending; see the [0.6.0 plan](plans/gitview-0-6-0.md).
- Commit and history implementation milestones are complete: editor handoff,
  path filtering, fixed-revision comparisons, and patch summaries. Release
  acceptance review remains pending.
- Local branch/tag and diff/inspection implementation milestones are complete:
  branch rename/delete/previous checkout, lightweight tag creation and local
  tag verification/deletion, comparisons, and object inspection. Annotated and
  signed tag creation remain unavailable. Release acceptance review is pending.
- Publish crates.io, macOS ARM64, and Linux x86_64 artifacts with checksums
  and native installer selection — planned; see the
  [0.6.0 plan](plans/gitview-0-6-0.md).

Release gate:

- All included features pass their acceptance checks; excluded workflows stay
  labeled incomplete.
- Full tests, formatting, build, and Clippy pass on the supported CI toolchain.
- macOS ARM64 and Linux x86_64 builds and archive checksums are verified.
- The installer selects the matching archive and verifies its checksum on both
  platforms.
- Crate packaging dry run passes; `CHANGELOG.md`, release notes, and
  installation instructions match the actual artifacts.
- Confirm there is no persistent-data migration and document how to reinstall
  0.5.x if users need to roll back.
- Complete a focused security review of Git argument handling, path handling,
  destructive actions, terminal restoration, and release workflows.
- Review the repository instructions and satisfy the shared
  [DoD](.agents/skills/release-plan/references/definition-of-done.md).
- The user reviews and approves the release candidate before publishing.

## Released

### Gitview 0.5.0 — October 28, 2026

Gitview 0.5.0 adds folder-scoped Git configuration profiles so repositories
can use separate author identities and Git settings while the primary account
remains the default.

Highlights:

- Applied the most specific matching profile to Gitview's Git subprocesses;
  unmatched repositories continue to use the normal global config.
- Added a profile view, registry editor and reload actions, and active-account
  display in the repository header.
- Added an opt-in zsh integration snippet for terminal Git commands.
- Updated worktree selection colors and documented current staging shortcuts.

Validation:

- GitHub Actions passed formatting, tests, Clippy, CLI help, installer help,
  and crate packaging checks.
- Installed `gitview 0.5.0` from crates.io with Cargo and from the macOS
  Apple Silicon release archive; the installer verified the archive checksum.

Downloads:

- `gitview-v0.5.0-macos-arm64.tar.gz` contains the macOS Apple Silicon binary.
- `gitview-v0.5.0-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.5.0` tag.

- Crate: [gitview 0.5.0 on crates.io](https://crates.io/crates/gitview/0.5.0)
- Release: [GitHub v0.5.0][release-050]
- Tag: `v0.5.0`
- Release PR: [#7](https://github.com/barathrajd/gitview/pull/7)
- Plan: [Git config profiles by repository folder](plans/git-config-profiles.md)

### Gitview 0.4.1 — October 28, 2026

Gitview 0.4.1 adds crates.io installation and a verified installer for the
macOS Apple Silicon binary.

Highlights:

- Published the `gitview` crate to crates.io with package metadata.
- Added `cargo install gitview --version 0.4.1 --locked` as an installation
  path and updated `install.sh` with the Cargo option.
- Added release preflight checks for formatting, tests, Clippy, CLI help,
  installer help, and crate packaging.
- Added manual GitHub Actions recovery paths for publishing the crate and
  building the macOS release archive.

Validation:

- Release workflow preflight passed.
- Installed from crates.io with Cargo and verified `gitview --version` reports
  `gitview 0.4.1`.
- Installed the GitHub release archive with `install.sh`, verified its SHA-256
  checksum, and confirmed the installed binary reports `gitview 0.4.1`.

Downloads:

- `gitview-v0.4.1-macos-arm64.tar.gz` contains the macOS Apple Silicon binary.
- `gitview-v0.4.1-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.4.1` tag.

- Crate: [gitview 0.4.1 on crates.io](https://crates.io/crates/gitview/0.4.1)
- Release: [GitHub v0.4.1][release-041]
- Tag: `v0.4.1`
- Release PR: [#6](https://github.com/barathrajd/gitview/pull/6)

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
[release-041]: https://github.com/barathrajd/gitview/releases/tag/v0.4.1
[release-050]: https://github.com/barathrajd/gitview/releases/tag/v0.5.0
[release-021]: https://github.com/barathrajd/gitview/releases/tag/v0.2.1
[definition-of-done]:
  .agents/skills/release-plan/references/definition-of-done.md
