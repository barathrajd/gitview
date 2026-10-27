# Refactor and release 0.2.1

This plan addresses the maintainability and safety findings from the repository
analysis and packages the result as Gitview 0.2.1. It targets macOS and Linux;
Windows support remains deferred.

## Objective

Give developers using Gitview a maintainable 0.2.1 release with the existing
history, diff, working-tree, and local-branch workflows preserved. Split UI
state, input handling, and rendering into testable units; tighten the safety
checks around Git mutations; and add UI-level regression coverage.

## Success criteria

The release is ready when each criterion can be verified from source, tests,
the built artifact, or the published release.

- UI state, input handling, rendering, and background-task coordination have
  clear module boundaries, and the existing user workflows still behave the
  same.
- Automated UI tests cover history, working-tree, branch, help, loading, error,
  confirmation, and narrow and wide diff layouts.
- Stale selections and repository changes detected before a Git mutation cannot
  redirect the operation to a different selected path or branch. Git's own
  protections remain enabled for changes made concurrently by other processes.
- A regression test demonstrates that the configured Ratatui panic hook restores
  the terminal after a panic; no duplicate restoration hook is added.
- macOS and Linux checks pass for path parsing, untracked-file diffs, build,
  formatting, tests, and linting. The release archive targets macOS Apple
  Silicon.
- Package metadata and the `--version` output report `0.2.1`; the `v0.2.1` tag
  and private GitHub release point to the verified source and include the macOS
  Apple Silicon archive.

## In scope / Out of scope

The scope focuses on the five review topics while honoring the current platform
and release boundaries.

**In scope**

- Separate UI state, input/actions, rendering, and background work from
  `src/main.rs` while preserving the current controls and behavior.
- Add UI-level tests using Ratatui's in-memory test backend, plus focused tests
  for state transitions and mutation guards.
- Revalidate mutation targets immediately before invoking Git and retain Git's
  non-forcing safety behavior for branch switches and file operations.
- Verify path parsing and untracked diffs on macOS and Linux; document
  Windows-specific path handling as deferred.
- Update the crate and lockfile versions, CLI version assertion, release
  documentation, and `v0.2.1` GitHub release with the macOS Apple Silicon
  archive and GitHub source archives.

**Out of scope**

- Windows support, Windows-specific path decoding, or Windows release assets.
- New Git workflows, changes to keybindings, changes to staging granularity, or
  changes to branch-switch semantics.
- Guaranteeing atomicity against another process changing files or refs between
  Gitview's final validation and Git's operation; Gitview cannot lock out
  independent Git clients.
- Linux release packaging, code signing, notarization, or changes to the
  repository's private visibility.

## Approach

Extract state and input transitions into an application module and move drawing
into a rendering module that accepts state and a Ratatui frame. Keep Git calls
in the existing adapter modules and background work outside rendering. Use
Ratatui's `TestBackend` for rendered-state checks; it is designed for integration
tests of terminal UIs ([Ratatui `TestBackend` documentation](https://docs.rs/ratatui/0.30.2/ratatui/backend/struct.TestBackend.html)).

The prior review's panic-restoration finding does not require a production
refactor: the current `ratatui::try_init()` installs the panic hook, which
restores terminal state ([Ratatui 0.30.2 initialization documentation](https://docs.rs/ratatui/0.30.2/ratatui/init/index.html)). Add a regression
check around the existing lifecycle instead of installing a competing hook.

For mutations, preserve the current Git argument-array calls, explicit selected
paths, stale-selection checks, and non-forcing branch operations. Revalidate
immediately before launching each operation, and test stale selections and
conflicting repository state. Concurrent external changes can still race after
that validation; rely on Git's own index and branch protections and surface any
failure without claiming success. Git documents the safety behavior of
[`git switch`](https://git-scm.com/docs/git-switch).

Keep byte-preserving Unix path handling for macOS and Linux. The existing
lossy non-Unix conversion is Windows-specific and remains deferred; do not
advertise Windows support. Build and verify the macOS Apple Silicon archive,
then create and publish the annotated tag and release using GitHub CLI's
[release creation workflow](https://cli.github.com/manual/gh_release_create).

## Milestones

Each milestone delivers a reviewable slice while keeping the existing Gitview
workflows available.

- [x] **1. Extract workspace state:** Model history/working-tree view changes
  and blocked Tab navigation in testable application state; entering the
  working-tree view requests status while returning to history preserves the
  existing state. Verify the transition with integration tests and a PTY smoke
  check.
- [x] **2. Extract history and working-tree actions:** Move list navigation,
  search, refresh, and stage/unstage input transitions into testable actions;
  preserve current selection rules and stale-result handling. Verify actions
  with integration tests and a PTY staging smoke check.
- [x] **3. Extract rendering and add UI tests:** Move frame drawing into a
  rendering module; test history, details, working-tree and branch states,
  loading and error messages, help and confirmations, and narrow and wide diff
  layouts with `TestBackend`. Verify the live help screen and normal terminal
  restoration in a PTY smoke check.
- [ ] **4. Harden mutation boundaries:** Revalidate selected paths and branch
  state at the operation boundary, preserve Git's non-forcing safeguards, and
  test stale selections, dirty-worktree failures, and changes detected before
  execution.
- [ ] **5. Verify lifecycle and supported platforms:** Add a panic-restoration
  regression check for Ratatui's existing hook; verify path parsing and
  untracked diffs on macOS and Linux; run format, test, lint, and build checks.
- [ ] **6. Package 0.2.1:** Update `Cargo.toml`, `Cargo.lock`, CLI version
  expectations, and release documentation; build and inspect the macOS Apple
  Silicon archive; create and push `v0.2.1`; publish the private GitHub release
  with the archive and verify its tag and assets.

## Risks & open questions

The implementation owner resolves these risks within the listed fallback.

- **External mutation race:** Another Git process can change state after
  Gitview validates it. Keep Git's own protections, revalidate immediately
  before execution, and report the operation result; do not promise a global
  lock or perfect atomicity.
- **UI extraction changes behavior:** Moving state and rendering can alter
  selection or timing behavior. Migrate one workflow at a time and retain
  behavior-focused tests before continuing.
- **Linux runner availability:** Linux is not currently verified in the README.
  Use an available Linux CI or development environment for the planned checks;
  if none is available, record Linux as unverified and do not imply support.
- **Windows path handling remains known:** `cfg(not(unix))` currently decodes
  Git path bytes lossily, and untracked diff handling uses `/dev/null`. Keep
  Windows deferred and disclose that limitation until a Windows plan includes
  compatible path conversion and validation.
- **Private release assets:** GitHub release visibility follows the repository
  visibility. Confirm the uploaded archive is downloadable to repository
  collaborators before marking the release milestone complete.

## Boundaries

Preserve the existing history, commit details, diffs, working-tree review,
stage/unstage, search, and local-branch flows. Do not discard, overwrite,
stash, or commit user changes, and do not add forceful Git operations. Keep
Windows and Linux release packaging deferred; the 0.2.1 binary asset is for
macOS Apple Silicon. Publish only after the source, tests, and archive pass the
release checks.
