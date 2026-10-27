# Git CLI capability roadmap through 2026

This plan replaces the two-week MVP horizon with a complete, tiered capability
map for Gitview. The delivery horizon is December 31, 2026; given the scope and
remaining calendar time, the milestones define release slices and an explicit
completion gate rather than promise that every advanced capability will ship by
year end.

## Objective

Build Gitview into a polished Rust TUI that covers Git's end-user porcelain
workflows through discoverable pages for everyday, collaborative, and advanced
repository work. Keep script-oriented plumbing and Git server/helper programs
available only through a guarded advanced command runner, not as ordinary UI
pages. The first audience includes new Git users, daily contributors, and
maintainers of complex repositories; the app must explain consequential Git
operations and preserve the user's ability to review before applying them.

## Success criteria

- A version-pinned capability matrix covers every documented user-facing
  porcelain workflow and maps each workflow to a page, action, safety gate, and
  acceptance check.
- Every capability is classified as Basic, Medium, or Advanced and tracked as
  shipped, in progress, deferred, or excluded, with no unclassified item.
- Basic workflows cover repository setup, working-tree review, staging,
  commits, history, local branches and tags, remote synchronization, and safe
  integration.
- Medium workflows cover recovery, conflict resolution, history editing,
  stashes, linked worktrees, submodules, sparse checkout, patch exchange, and
  repository investigation.
- Advanced workflows cover large/partial repositories, maintenance, object and
  ref inspection, offline transfer, notes and replacement refs, interoperability,
  and guarded access to plumbing commands.
- Every mutating workflow previews its target and consequences when practical,
  confirms destructive or remote actions, preserves Git's built-in safeguards,
  and displays progress and actionable errors.
- Git subprocesses use argument arrays and machine-readable output where
  available, run outside the render loop, and handle unusual filenames safely.
- The release checklist covers macOS Apple Silicon and Linux x86_64. Windows
  remains deferred; AI chat remains post-1.0 and outside this roadmap.
- By December 31, 2026, Gitview has shipped the Basic tier and published the
  state of every Medium and Advanced item, including a concrete follow-on plan
  for work that cannot safely fit the calendar.

## In scope / Out of scope

**In scope**

- Dedicated, contextual subpages for user-facing Git porcelain workflows.
- Basic, Medium, and Advanced feature tiers, with the complete inventory below.
- An advanced command runner for plumbing and server/helper commands, with
  argument preview, captured output, risk labels, and explicit confirmation
  before mutations.
- Git 2.38+ as the initial compatibility floor, subject to a release-time
  review against the commands selected for each release.
- macOS Apple Silicon and Linux x86_64; explicit fetches and other remote
  actions with visible progress.
- Conventional TUI navigation, consistent page help, keyboard discoverability,
  and narrow-terminal behavior.

**Out of scope**

- A bespoke UI page for every internal plumbing command, transport helper,
  server process, or external VCS bridge.
- Windows support in this roadmap; retain it as a later platform goal.
- AI chat, model integrations, API-key handling, or consumer-subscription
  integrations; revisit after the core Git workbench is stable.
- Replacing Git's object model or silently changing Git configuration.
- Automatic network access during local previews.

## Approach

Use Git's documented distinction between end-user porcelain and script-oriented
plumbing as the coverage boundary. Organize pages around user tasks rather than
one screen per command. Show the underlying Git command and relevant refs,
paths, strategy, and remote for operations that can change repository state.
For advanced plumbing, provide a guarded command runner with a clear read-only
or mutating classification; never claim a dedicated workflow for a helper that
has no meaningful interactive use.

The inventory is based on the official Git command reference and documentation
for repository layout, worktrees, sparse checkout, attributes, ignore rules,
and submodules. `git --list-cmds` is useful for auditing an installed version,
but Git documents that list as internal/experimental, so it is not the fixed
product contract. Refresh the inventory against the selected Git minimum before
each release. See [Git command reference][git], [CLI conventions][gitcli],
[repository layout][layout], [worktrees][worktree], [sparse checkout][sparse],
[submodules][submodules], [attributes][attributes], and [ignore rules][ignore].

### Basic — everyday Git

Basic pages should make the daily edit-review-commit-share loop complete.

- **Repository:** open the current repository, initialize, clone, inspect
  repository state, and explain bare, detached, unborn, and linked-worktree
  states.
- **Worktree:** grouped staged, modified, and untracked files; file navigation;
  diff review; stage/unstage files, hunks, and all; restore selected paths with
  confirmation; add, remove, and move tracked paths; inspect ignore status.
- **Commit:** compose and review a message, create a commit, amend with an
  explicit warning, and inspect signing or identity failures.
- **History:** graph, commit details, path history, message/path search, tags
  and refs, commit and branch comparisons, rename-aware diffs, and patch
  summaries.
- **Branches and tags:** list, create, switch, rename, and delete local
  branches; inspect upstream state; list, create, verify, and delete tags.
- **Remotes:** list and edit remote definitions, fetch explicitly, inspect
  ahead/behind state, pull, and push with destination and ref preview.
- **Merge workbench:** preview fast-forward, merge-commit, and squash outcomes;
  choose a strategy; confirm apply; inspect conflicts; open the configured
  editor; continue or abort.
- **Diff and inspection:** working-tree, staged, commit, branch, and remote-ref
  comparisons; whitespace and rename options; show and describe objects.

Command coverage includes `init`, `clone`, `status`, `add`, `restore`, `rm`,
`mv`, `diff`, `commit`, `log`, `show`, `shortlog`, `describe`, `branch`,
`switch`, `checkout` where needed for compatible workflows, `tag`, `remote`,
`fetch`, `pull`, `push`, `merge`, and `mergetool`.

The Basic checklist also maps `citool` to the commit page, `gui` to an optional
external-tool handoff, and `archive`/`bundle` to the read-only export preview
until their full offline-exchange workflow is added in Advanced. Aliases such as
`annotate` map to their canonical workflow rather than creating duplicate
pages.

### Medium — collaboration and recovery

Medium pages address multi-person work, mistakes, and non-linear history.

- **Stash:** save tracked and untracked work, inspect, apply, pop, create a
  branch from, and drop entries.
- **History editing:** interactive rebase, reorder, squash, fixup, reword, edit,
  drop, cherry-pick single commits or sequences, revert, and reset modes with
  recovery explanations.
- **Conflict resolution:** per-file conflict state, index stages, side-by-side
  and three-way views, configured merge tools, continue, skip, and abort flows;
  expose `rerere` status and reuse behavior.
- **Remote branches:** upstream tracking, remote branch management, pruning
  previews, range comparison, and guarded `--force-with-lease` workflows.
- **Linked worktrees:** add, list, lock, unlock, move, remove, and prune, while
  showing the branch and path relationship for each worktree.
- **Submodules:** status, init, update, sync, recursion settings, conflicts, and
  commit-pointer changes.
- **Sparse checkout:** choose paths, inspect patterns and cone mode, and explain
  sparse-index implications before changing it.
- **Code archaeology:** blame, grep, pickaxe search, bisect start/good/bad/skip,
  and commit-range inspection.
- **Patch exchange:** create and inspect patch series, apply patches, apply
  mailbox series, and show prerequisites for email-based exchange.
- **Recovery:** reflog browser, safe reference recovery, compare-before-restore,
  and visible `undo` limits when an operation cannot be reversed safely.
- **Repository conventions:** inspect and edit scoped configuration, ignore and
  attribute rules, mailmap, hooks, aliases, signing, and external editor/diff
  tool settings.

Command coverage includes `stash`, `rebase`, `cherry-pick`, `revert`, `reset`,
`reflog`, `rerere`, `worktree`, `submodule`, `sparse-checkout`, `blame`,
`grep`, `bisect`, `range-diff`, `format-patch`, `apply`, `am`, `send-email`,
`request-pull`, `config`, `check-ignore`, `check-attr`, `check-mailmap`,
`verify-commit`, `verify-tag`, `whatchanged`, `fast-export`, and `fast-import`.
Legacy or specialized commands such as `filter-branch` are documented as
expert-only and receive a guarded handoff rather than a routine workflow.

### Advanced — repository scale and internals

Advanced pages expose infrequent workflows with safeguards and enough context
for expert users.

- **Large and partial repositories:** shallow history, partial clone, promisor
  remotes, sparse indexes, selective object backfill, and repository-size
  diagnostics.
- **Object and ref explorer:** browse blobs, trees, commits, tags, refs,
  revision expressions, index entries, merge bases, and reachable/unreachable
  history in read-only mode.
- **Health and maintenance:** diagnose, verify connectivity, inspect object
  counts and packs, schedule maintenance, and preview garbage collection,
  repacking, pruning, commit-graph, and multi-pack-index work.
- **Offline exchange:** create and inspect bundles and archives, verify imported
  refs, and preview what an import adds.
- **Notes and replacement refs:** inspect and edit notes; list, create, and
  remove replacement refs with clear object-identity warnings.
- **Interoperability:** document or integrate Git-SVN, Perforce, CVS, Quilt,
  and Arch workflows only when their external dependencies are installed.
- **Guarded command runner:** expose remaining plumbing and server/helper
  commands as explicit commands with argument-by-argument preview, output
  capture, an operation risk label, and a confirmation gate for writes.

Command families include `cat-file`, `ls-files`, `ls-tree`, `show-ref`,
`for-each-ref`, `rev-parse`, `rev-list`, `merge-base`, `hash-object`,
`update-index`, `update-ref`, `read-tree`, `write-tree`, pack and maintenance
commands, `bundle`, `archive`, `notes`, `replace`, and relevant foreign-VCS
helpers. The remaining documented command families—including `gc`,
`maintenance`, `fsck`, `diagnose`, `count-objects`, `verify-pack`, `pack-refs`,
`prune`, `repack`, `commit-graph`, `multi-pack-index`, `fast-import`,
`fast-export`, `filter-branch`, and `replace`—must be mapped to a dedicated
page, an optional external handoff, or the guarded runner in the compatibility
matrix. Server operations such as `upload-pack`, `receive-pack`,
`http-backend`, and `git-daemon` are command-runner territory, not normal TUI
workflows. Experimental commands and version-gated commands such as `backfill`
must be labeled with the minimum Git version and are not silently assumed to
exist on the 2.38 floor.

### Safety rules for every tier

- Never discard, clean, reset, rewrite, delete, prune, or force-update silently.
- Before mutation, re-read repository state and reject stale selections.
- Show target branch, remote, refspec, paths, strategy, and affected commits
  where applicable.
- Require a clean worktree for merge-preview apply in the initial workflow;
  never fetch implicitly during preview.
- Preserve Git's own refusal conditions. Offer safer alternatives such as
  `--force-with-lease` and explain the tradeoff.
- Keep command output, exit status, and recovery guidance visible after errors.
- Never interpolate user-controlled values into shell command strings.

## Milestones

Milestones are ordered by user-visible slices. The dates describe release
checkpoints through December 31, 2026; move unfinished work explicitly instead
of weakening its safety or acceptance criteria.

1. **October 28–November 7 — establish the compatibility map:** inventory every
   documented porcelain command family and subpage; record current Gitview
   support, risk level, and acceptance coverage; update outdated README claims.
2. **November 8–21 — finish Basic worktree and commit flow:** complete file
   navigation, discard confirmation, commit/amend, path operations, and
   working-tree regression coverage.
3. **November 22–December 5 — finish Basic refs and sharing:** complete local
   branch/tag management, remote setup, explicit fetch, ahead/behind, pull, and
   push with preview and progress/error states.
4. **December 6–19 — finish Basic integration and recovery foundation:** ship
   merge preview/apply, conflicts, editor handoff, continue/abort, and safe
   recovery entry points; complete macOS/Linux release checks.
5. **December 20–31 — publish the tiered coverage report:** ship verified Basic
   scope, classify every Medium and Advanced workflow against the matrix, and
   produce scoped follow-on releases with their safety gates. Do not mark a
   workflow complete until its acceptance checks pass.

## Risks & open questions

- **Calendar compression:** the full Git surface is too broad to promise in two
  months. The fallback is to ship Basic, publish the full matrix, and schedule
  remaining Medium/Advanced capabilities as explicit follow-on releases.
- **Git version spread:** commands and options evolve. Pin the support baseline
  per release and report unsupported Git versions at startup.
- **Risk of destructive actions:** use preview, explicit confirmation, stale
  state checks, and Git-native protections; omit a UI action if it cannot be
  made safe and understandable.
- **Remote and credential differences:** delegate transport/authentication to
  Git and show its errors without storing credentials in Gitview.
- **Advanced command runner scope:** keep risky plumbing opt-in, classify
  commands, and default to read-only exploration. Unknown commands require
  explicit confirmation and remain an expert feature.
- **External integrations:** `gh`, email tooling, VCS bridges, editors, and diff
  tools may be absent; core Git pages remain usable and show dependency status.
- **REA limitation:** REA opened the local Git binary, but its binary-analysis
  provider was unavailable. This plan relies on official Git documentation and
  command behavior, not unverified binary-internal findings.

## Boundaries

Git remains the source of truth and Gitview remains a Rust terminal app. Preserve
existing history, diff, hunk staging, worktree, and branch workflows. Use Git
without shell interpolation, keep subprocess work outside rendering, and make
state-changing actions inspectable. The coverage promise is all documented
end-user porcelain workflows plus guarded access to relevant plumbing and
helpers, not a dedicated screen for every internal command. Keep AI and Windows
out of this 2026 delivery scope.

[git]: https://git-scm.com/docs/git
[gitcli]: https://git-scm.com/docs/gitcli
[layout]: https://git-scm.com/docs/gitrepository-layout
[worktree]: https://git-scm.com/docs/git-worktree
[sparse]: https://git-scm.com/docs/git-sparse-checkout
[submodules]: https://git-scm.com/docs/gitsubmodules
[attributes]: https://git-scm.com/docs/gitattributes
[ignore]: https://git-scm.com/docs/gitignore
