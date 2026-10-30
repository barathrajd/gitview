# Gitview releases

This file is the single release plan and release-notes record for Gitview.

## Upcoming

### Gitview 0.7.1 — patch release

This patch fixes latest-release discovery in the installer and the commit count
shown for a merged agent session. The website adds a searchable feature sidebar,
changelog links to feature demonstrations, shared branding, and automatic
version updates and deployment after a release. Known 0.7.0 limitations remain.


### Next release — follow-up tasks carried over from 0.6.0

Gitview 0.6.0 shipped with the known issues below. They are open work for a
future release and aren't release promises.

- Linux x86_64: native checks, performance evidence, a release archive with
  checksum, and installer selection on a Linux runner.
- Memory: explain why native RSS on a large (about 940 KB) comparison varies
  between runs (256 to 13,552 KiB growth over 100 open/close cycles against a
  10,240 KiB budget); live heap stays flat. See the
  [performance report](docs/performance-0-6-0.md).
- Memory: the renderer-only diff RSS diagnostic
  (`measures_diff_cache_lifecycle`) exceeded its budget about half the time
  while live heap stayed flat. Decided: reshape it. RSS shows one bounded
  allocator step of ten-odd MiB at an unpredictable point, and macOS compression
  hides idle leaked memory from RSS, so the check now fails on a sustained rise
  (ignoring the two largest single-block jumps). Leak detection is the
  live-allocation diagnostic, which catches 20 KB per cycle. The
  large-comparison variance above is still open.
- Evidence: rerun the full pre-release pass on a clean revision, and confirm
  Clippy on the CI toolchain from the release pull request.
- Process: complete the focused security review and the shared Definition of
  Done review that 0.6.0 did not wait for.
- Coverage gaps: syntax highlighting for TypeScript, TSX, and TOML, and
  cancelling running Git commands when a comparison or object page closes.

## Released

### Gitview 0.6.0 — October 29, 2026

Gitview 0.6.0 completes six everyday local Git workflows: repository setup,
worktree path operations, commit composition, history, local branches and tags,
and diff and object inspection. Remotes, merge and conflict workflows, and
export remain out of scope.

Highlights:

- Added `gitview init` and `gitview clone`; initialization refuses an existing
  repository and clone refuses a non-empty destination.
- Added ignore-rule lookup for any path, stop tracking, confirmed removal and
  moves of tracked paths, and discard of a staged rename.
- Added Ctrl+E to edit the commit message in the configured Git editor. Saving a
  changed message commits it without a second Enter; an unchanged or empty
  subject returns to the form, and amend keeps its confirmation.
- Added history filtering by file or directory, one-page comparison of two
  commits with folding, and object description for commits, tags, trees, and
  blobs, including comparison against remote refs already stored locally.
- Added local branch rename, delete, and previous-branch return, and local tag
  listing, lightweight creation, verification, and deletion.
- Reviewed responsiveness across the interface: diffs, comparison, history
  search, tag and object pages, and previews prepare off the render loop,
  cancel on Escape, and bound retained memory. Results and limits are in the
  [performance report](docs/performance-0-6-0.md).

Known issues:

- Native memory on a large comparison varies between runs; one run exceeded the
  10 MiB growth budget over 100 open/close cycles. See the follow-up tasks above.
- A renderer-only memory diagnostic still exceeds its budget; live heap stays
  flat in the same test.
- TypeScript, TSX, and TOML diffs aren't syntax highlighted.
- Linux packaging is not provided; Linux was last verified for 0.2.1.
- Annotated and signed tag creation, remotes, merge, and export are unavailable.

Validation:

- A full pre-release pass ran on a clean worktree at revision `3a0df05` on
  macOS Apple Silicon (Rust 1.99.1, Git 2.56.0). Formatting, the full test suite,
  Clippy with the CI command, the release build, every performance diagnostic,
  and every pseudo-terminal script passed except the known renderer-only memory
  diagnostic. Native memory over 100 cycles grew 8,752 KiB for the ordinary
  diff (budget 10,240 KiB) and 160 KiB for the tag page.
- The release shipped without the security review or the shared Definition of
  Done review, by the maintainer's decision.

Downloads:

- `gitview-v0.6.0-macos-arm64.tar.gz` contains the macOS Apple Silicon binary.
- `gitview-v0.6.0-macos-arm64.tar.gz.sha256` contains the archive checksum.
- GitHub provides source archives for the `v0.6.0` tag.

- Crate: [gitview 0.6.0 on crates.io](https://crates.io/crates/gitview/0.6.0)
- Release: [GitHub v0.6.0][release-060]
- Tag: `v0.6.0`
- Release PR: [#8](https://github.com/barathrajd/gitview/pull/8)
- Plan: [Gitview 0.6.0](plans/gitview-0-6-0.md)

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
[release-060]: https://github.com/barathrajd/gitview/releases/tag/v0.6.0
[release-021]: https://github.com/barathrajd/gitview/releases/tag/v0.2.1
[definition-of-done]:
  .agents/skills/release-plan/references/definition-of-done.md
