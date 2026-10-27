# Gitview 1.0.0: a full-featured Rust Git TUI

> **Scope update:** This document records the original Lazygit v0.66.2 parity
> baseline. The broader Git porcelain coverage, Basic/Medium/Advanced tiers,
> and December 31, 2026 checkpoints are now tracked in the
> [Git CLI capability roadmap](git-cli-capability-roadmap-2026.md).

This plan grows Gitview from a Git review MVP into a full keyboard-driven Git
workbench, with a modern terminal interface and a merge preview that protects
the current worktree until the user confirms an operation.

## Objective

Build Gitview into a polished, keyboard-first Rust terminal application for
solo developers and teams, from Git newcomers to experienced users who manage
complex history. Version 1.0.0 targets the documented workflows in Lazygit
v0.66.2, with an original visual design and a safer merge preview as its
defining improvement; AI chat is a later phase.

## Success criteria

- A version-pinned compatibility checklist covers every end-user workflow in
  Lazygit's v0.66.2 README, configuration guide, and English keybinding guide,
  with each workflow implemented or explicitly recorded as deferred.
- Existing Gitview history, commit details, diffs, working-tree review, staging,
  and local branch actions continue to pass their regression tests.
- Users can review files and individual hunks or lines, commit and amend,
  manage stashes, branches, tags, remotes, and worktrees, and use the documented
  history-editing and diagnostic workflows from the TUI.
- Users can preview fast-forward-only, merge-commit, and squash outcomes using
  local refs before choosing and confirming an apply action. Preview leaves
  refs, the index, and the working tree unchanged; apply is blocked unless the
  worktree is clean and the reviewed refs still match.
- An applied merge exposes conflict state, opens the configured editor, and
  provides explicit continue and abort actions.
- The interface presents clear focus, selection, operation state, and key help;
  it remains readable across narrow and wide terminal sizes and without special
  font glyphs. UI snapshots cover each major workflow and layout.
- Git subprocesses run outside the render loop, use argument arrays and
  machine-readable output where available, and return visible progress and
  actionable errors.
- Formatting, tests, linting, and release builds pass on macOS Apple Silicon and
  Linux x86_64. The documented minimum Git version is 2.38.
- The 1.0.0 release notes and compatibility checklist identify AI chat and
  Windows support as deferred.

## In scope / Out of scope

This section defines the Gitview 1.0 compatibility target and its boundaries.

**In scope**

- A polished, terminal-only, responsive pane layout with a distinct Gitview
  visual identity.
- Lazygit v0.66.2 documented Git workflows, including file and hunk staging,
  commits, stash actions, branches, remotes, tags, worktrees, merge and rebase
  flows, cherry-pick, revert, bisect, history comparison, undo and redo, and
  submodule workflows.
- A merge preview for fast-forward-only, merge-commit, and squash modes, with
  conflict paths and resulting changes visible before an explicit apply.
- Merge conflict state, configured-editor handoff, and continue or abort
  controls.
- GitHub pull-request workflows through the installed and authenticated `gh`
  CLI, plus the documented configuration, custom commands, keybindings, theme,
  external editor, and external diff-tool workflows.
- macOS and Linux support, with Git 2.38 or later.

**Out of scope**

- AI chat, model-provider integrations, API keys, or using consumer AI
  subscriptions. Plan these after 1.0.0.
- Windows support and Windows release assets; Windows remains deferred from the
  prior platform decision.
- A separate native desktop application. Gitview remains a terminal program.
- Copying Lazygit's source code, assets, branding, or exact visual design.
- Automatic remote fetches during preview. Users fetch explicitly, then preview
  the refs available in their local repository.

## Approach

Keep the current Rust, Ratatui, Crossterm, and Git CLI foundation. Expand the
existing app, UI, process, repository, history, branch, diff, and worktree
modules around typed operations and a task coordinator so Git commands stay off
the render loop. Keep Git as the source of truth for merge, rebase, staging,
history editing, and remote behavior; use argument arrays and NUL-delimited
formats where Git provides them. Use the installed `gh` CLI for GitHub features,
and show an actionable unavailable state when it is missing or unauthenticated.

The compatibility baseline is Lazygit v0.66.2, pinned to its [release][lazygit]
and [keybinding guide][keybindings]. Its documented workflows include granular
staging, interactive rebase, cherry-pick, bisect, worktrees, custom patches,
history comparison, reflog-based undo, and GitHub pull-request awareness.

For merge previews, use `git merge-tree --write-tree`, which calculates a merge
without changing the index or working tree and supports normal content merges,
rename detection, and directory/file conflicts. It writes merge-result objects
to the Git object database, so the preview is non-mutating for refs, index, and
working-tree files but can leave unreachable objects for Git garbage collection
to reclaim. This interface is documented in Git 2.38, so require Git 2.38 or
later. Fast-forward-only preview also checks ancestry; merge-commit and squash
preview show the calculated tree and conflict list. On apply, require a clean
worktree, confirm the selected mode, and verify the current branch and target
object IDs still match the preview. Git remains responsible for the operation
and its normal conflict state. Git documents the behavior of
[`git merge-tree`][merge-tree] and [`git merge`][merge].

REA opened the official v0.66.2 macOS arm64 and Linux x86_64 release artifacts,
with observed SHA-256 values `f5fec331953589ef11aa9a65851a62867398ebdb5bc03c4b3616c60ffa9163f5`
and `3ea229a849a77cae673282433cc9ef821a9164e284bf8ec55b9c2d8d25a5510b`,
respectively. Its binary-layout provider rejected both artifacts, and string
analysis reported that no Ghidra, Hopper, IDA, or Kuna provider is configured.
No binary-internal claims are made from those calls. The feature inventory
above comes from Lazygit's version-pinned public documentation; the official
[release page][lazygit-release] identifies v0.66.2 as the latest release found
during this research.

[lazygit]: https://github.com/jesseduffield/lazygit/blob/v0.66.2/README.md
[lazygit-release]: https://github.com/jesseduffield/lazygit/releases/tag/v0.66.2
[keybindings]: https://github.com/jesseduffield/lazygit/blob/v0.66.2/docs/keybindings/Keybindings_en.md
[merge-tree]: https://git-scm.com/docs/git-merge-tree/2.38.0
[merge]: https://git-scm.com/docs/git-merge/2.52.0

## Milestones

Each milestone delivers an end-to-end workflow and adds it to the parity
checklist.

1. **[x] Design the Gitview cockpit:** Define the visual system, pane layout,
   keyboard discovery, terminal-size behavior, and Lazygit v0.66.2 parity
   checklist. Refine app state and background-task coordination enough to add
   new workflows without blocking rendering. Delivered the responsive cockpit,
   pinned parity checklist, and startup refresh regression coverage; formatting,
   full tests, Clippy, and release build pass.
2. **Review and shape working-tree changes:** Add file-tree navigation,
   line/hunk selection, stage and unstage, discard confirmations, commit and
   amend flows, stash variants, and external editor/diff-tool handoff. Keep the
   existing full-file staging behavior available. The first slice adds
   test-covered hunk staging and unstaging for tracked text changes: use `[` and
   `]` to select a hunk in a working-tree diff, then `s` or `u` to stage or
   unstage it; use `S` or `U` to stage or unstage the whole file. Wide diffs
   wrap long lines side by side, and `H` returns to history from subpages.
   Existing full-file actions remain available from the worktree list.
   File-tree navigation, discard, commit, stash, and editor workflows remain
   to be implemented.
3. **Manage refs and remote work:** Add local and remote branch workflows,
   fetch, pull, push, tags, worktrees, submodules, comparisons, and visible
   progress/error states. Keep remote access explicit and preserve Git's
   non-forcing protections.
4. **Preview and apply merges safely:** Add the non-mutating merge preview for
   the three selected integration modes, show graph and conflict details, block
   apply on dirty worktrees, revalidate preview refs, and provide merge
   conflict editor handoff, continue, and abort actions.
5. **Edit and diagnose history:** Add interactive rebase, commit reorder and
   squash/fixup/drop/reword, cherry-pick, revert, bisect, commit comparison,
   custom patch operations, and reflog-based undo/redo with recovery-focused
   tests.
6. **Add collaboration and customization:** Add GitHub pull-request discovery
   and actions through `gh`, configurable services, custom commands and
   keybindings, color themes, and discoverable contextual help.
7. **Complete the 1.0 parity and release gate:** Close the pinned feature
   checklist, exercise clean and conflicted repositories, terminal sizes,
   detached and empty repositories, filenames with unusual bytes, and large
   histories/diffs; verify macOS and Linux builds; publish version 1.0.0 with
   source and platform assets.

## Risks & open questions

The implementation owner tracks these risks through the parity checklist and
uses the stated fallback when a dependency or workflow is unavailable.

- **Large parity surface:** Lazygit has many workflows and configuration
  options. Keep the checklist pinned to v0.66.2; do not let upstream releases
  silently expand the 1.0 target.
- **Merge preview side effects:** `merge-tree --write-tree` writes objects even
  though it leaves refs, index, and worktree untouched. Document this behavior
  and let normal Git garbage collection reclaim unreachable results.
- **Stale refs or concurrent Git commands:** Revalidate branch, target object,
  and worktree state immediately before apply. If state changed, discard the
  preview and ask the user to refresh it. A separate process can still race
  after validation; surface Git's result and recovery instructions.
- **Unavailable `gh` or credentials:** Keep all core Git workflows usable and
  report which GitHub action needs `gh` or authentication.
- **Terminal differences:** Test macOS and Linux terminals and keep essential
  meaning available without Nerd Font glyphs. Windows-specific handling remains
  deferred.
- **REA coverage:** The current session has no configured native-analysis
  provider. Use the version-pinned source documentation as the bounded feature
  baseline; rerun REA only if a suitable provider becomes available and binary
  evidence is needed.

## Boundaries

Gitview remains a Rust terminal application that invokes Git as a subprocess.
Keep the existing history and review workflows working while adding broader
operations. Do not run Git commands through a shell string, force destructive
operations by default, auto-fetch during merge preview, or apply a merge without
explicit confirmation. Do not include AI or Windows support in the 1.0.0 scope.
