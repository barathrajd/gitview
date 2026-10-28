# Git CLI capability matrix

This matrix records what the current checkout implements and what remains
planned. It uses Git 2.38.0 as the roadmap's inventory baseline; Gitview has
not verified or enforced that minimum yet. A status describes Gitview's current
UI and behavior, not whether Git itself supports the command.

| Tier | Workflow | Git commands | Current status | Safety | Acceptance evidence |
| --- | --- | --- | --- | --- | --- |
| Basic | Repository | `init`, `clone`, `status` | Partial — in progress: opens the current repository and reads status; `gitview init` and `gitview clone` are available, while bare/detached/unborn/linked-worktree explanations remain incomplete | Initialization refuses a destination inside an existing repository; clone rejects a non-empty destination and displays Git progress/errors | `tests/repository.rs`, `tests/cli.rs` |
| Basic | Worktree | `status`, `add`, `restore`, `rm`, `mv`, `check-ignore` | Partial — in progress: grouped status, file diffs, file/hunk stage and unstage, confirmed discard of selected unstaged or untracked paths and staged renames (restoring the original path), confirmed untracking that keeps the working copy, confirmed tracked-path removal, clean tracked-file moves, and ignore-rule inspection for selected or arbitrary repository-relative paths | Stage, discard, removal, and move recheck reviewed state; Git's normal refusal protects uncommitted edits and existing destinations; rename discard confirms both paths and restores them from HEAD; untracking rechecks status and the staged index entry; ignore inspection is read-only and rejects paths outside the repository | `tests/worktree.rs`, `tests/worktree_discard.rs`, `tests/worktree_ignore.rs`, `tests/worktree_remove.rs`, `tests/worktree_move.rs`, `tests/details.rs` |
| Basic | Commit | `commit`, `citool` | Partial — in progress: create from a reviewed staged set and amend `HEAD` with an explicit warning; Ctrl+E opens the configured Git editor and returns the edited draft for review | Commit rejects stale staged state; amend rechecks `HEAD` and the staged index after its warning; the input reader pauses during editor handoff; shell expressions are rejected; Git hooks and signing remain enabled | `tests/editor.rs`, `tests/commit.rs`, `tests/commit_amend.rs`, `tests/app.rs`, `tests/ui.rs` |
| Basic | History | `log`, `show`, `shortlog`, `describe`, `whatchanged` | Partial — in progress: graph, bounded history search, literal file/directory history, commit details, and changed-file diffs; commit comparisons with optional literal paths and patch summaries are available; commit descriptions and local object inspection are available | Read-only | `tests/history.rs`, `tests/path_history.rs`, `tests/details.rs`, `tests/comparison.rs`, `tests/object.rs` |
| Basic | Branches and tags | `branch`, `switch`, `checkout`, `tag` | Implemented for local workflows: list, create, and switch local branches; confirmed branch rename and non-force deletion; previous checkout to local branches or detached commits; local tag listing, lightweight creation, signature verification, and confirmed deletion; annotated/signed tag creation is not exposed | Branch creation confirms name and HEAD; switching preserves Git safeguards; previous checkout rechecks its resolved destination and current checkout; rename/delete reject stale branch tips and checkout state, preserve duplicate-name and unmerged/checked-out branch safeguards; tag creation atomically rejects duplicates and rechecks HEAD, deletion requires the reviewed object ID, verification checks the reviewed object and reports Git status | `tests/branches.rs`, `tests/tags.rs` |
| Basic | Remotes | `remote`, `fetch`, `pull`, `push` | Not implemented — in progress | No network or remote mutation is exposed | Acceptance not added |
| Basic | Merge workbench | `merge`, `mergetool` | Not implemented — in progress | No merge mutation is exposed | Acceptance not added |
| Basic | Diff and inspection | `diff`, `diff-tree`, object and ref comparisons | Partial — in progress: working-tree and commit-file diffs, fixed commit-snapshot comparisons with optional literal paths, and branch/locally stored remote-ref comparisons with patch summaries; local object descriptions with type, size, bounded previews, and binary notices are available | Read-only, except staging actions listed above | `tests/details.rs`, `tests/diff_layout.rs`, `tests/comparison.rs`, `tests/object.rs` |
| Basic | Export | `archive`, `bundle` | Not implemented — in progress | No export or import action is exposed | Acceptance not added |
| Basic | Optional handoffs | `gui` | Not implemented — deferred | No external GUI handoff is exposed | Acceptance not added |
| Medium | Stash | `stash` | Not implemented — deferred | No stash mutation is exposed | Acceptance not added |
| Medium | History editing | `rebase`, `cherry-pick`, `revert`, `reset` | Not implemented — deferred | No history rewrite or reset action is exposed | Acceptance not added |
| Medium | Conflict resolution | `rerere`, configured merge tools | Not implemented — deferred | Conflict states and resolution actions are not supported | Acceptance not added |
| Medium | Remote branches | Remote branch tracking, pruning, guarded force push | Not implemented — deferred | No remote ref mutation is exposed | Acceptance not added |
| Medium | Linked worktrees | `worktree` | Not implemented — deferred | No linked worktree mutation is exposed | Acceptance not added |
| Medium | Submodules | `submodule` | Not implemented — deferred | No submodule action is exposed | Acceptance not added |
| Medium | Sparse checkout | `sparse-checkout` | Not implemented — deferred | No sparse pattern or index mutation is exposed | Acceptance not added |
| Medium | Code archaeology | `blame`, `annotate`, `grep`, `bisect`, `range-diff` | Not implemented — deferred | No investigation page is exposed | Acceptance not added |
| Medium | Patch exchange | `format-patch`, `apply`, `am`, `send-email`, `request-pull` | Not implemented — deferred | No patch or email exchange action is exposed | Acceptance not added |
| Medium | Recovery | `reflog`, safe ref recovery | Not implemented — deferred | Recovery is unavailable in the UI | Acceptance not added |
| Medium | Repository conventions | `config`, `check-attr`, `check-mailmap`, `verify-commit`, `verify-tag` | Partial — in progress: folder-scoped Git config profiles are supported; tag signature verification is available; other listed inspection and convention editors are not | Profile selection is read-only in the UI; editing uses the configured editor | `tests/config_profiles.rs`, `tests/ui.rs` |
| Advanced | Large and partial repositories | Shallow/partial clone, promisor remotes, selective backfill | Not implemented — deferred | No partial repository controls are exposed | Acceptance not added |
| Advanced | Object and ref explorer | `cat-file`, `ls-files`, `ls-tree`, `show-ref`, `for-each-ref`, `rev-parse`, `rev-list`, `merge-base`, `hash-object` | Not implemented — deferred | No object or ref explorer is exposed | Acceptance not added |
| Advanced | Health and maintenance | `gc`, `maintenance`, `fsck`, `diagnose`, `count-objects`, `verify-pack`, `pack-refs`, `prune`, `repack`, `commit-graph`, `multi-pack-index` | Not implemented — deferred | No maintenance action is exposed | Acceptance not added |
| Advanced | Offline exchange | `archive`, `bundle`, `fast-export`, `fast-import` | Not implemented — deferred | No import or export action is exposed | Acceptance not added |
| Advanced | Notes and replacement refs | `notes`, `replace` | Not implemented — deferred | No ref identity mutation is exposed | Acceptance not added |
| Advanced | Interoperability | `git svn`, `git p4`, `git cvs`, Quilt, Arch | Not implemented — deferred | No external VCS integration is exposed | Acceptance not added |
| Advanced | Guarded command runner | Remaining plumbing and server/helper commands, including `update-index`, `update-ref`, `read-tree`, `write-tree`, `filter-branch`, `upload-pack`, `receive-pack`, `http-backend`, and `git-daemon` | Not implemented — deferred | No arbitrary command runner is exposed | Acceptance not added |
| Excluded | Platform and AI | Windows support and AI chat | Excluded from the 2026 roadmap | No platform or AI behavior is claimed | Not applicable |

## Status meanings

Use **shipped** only when the complete workflow and its safety checks pass
acceptance. **In progress** means some behavior exists or the workflow is in
the Basic implementation tier. **Deferred** means the workflow belongs to a
later tier. **Excluded** means the roadmap explicitly omits the work.
**Not implemented** means Gitview does not offer the workflow in its current
UI. Roadmap status is not a release claim.

## Compatibility and scope

The inventory follows the high-level and ancillary porcelain command groups in
the [Git 2.38.0 command reference](https://git-scm.com/docs/git/2.38.0), with
workflow groupings from the
[Git CLI capability roadmap](../plans/git-cli-capability-roadmap-2026.md).
Gitview currently requires Git to be installed. The project has not yet
validated a minimum Git version; `git switch` is used for branch operations.
Review this matrix against the selected Git minimum before each release.

The matrix covers end-user workflows. Plumbing and server/helper commands do
not imply dedicated pages; the roadmap places them behind a future guarded
command runner. Windows and AI chat remain outside the 2026 roadmap.
