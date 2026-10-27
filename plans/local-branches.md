# Local Branch Operations

## Objective

Let developers using Gitview inspect, switch, and create local branches without leaving the terminal, while preserving Git's protection for uncommitted work and making branch changes visible in the review workflow.

## Success criteria

- A discoverable keyboard action opens a navigable list of local branches, identifies the current branch, and clearly reports detached HEAD or an unborn branch.
- Selecting a different local branch and confirming switches to it, then refreshes branch, history, commit selection, and working-tree state. Git errors remain visible and the app remains usable.
- Creating a valid, non-existing branch from the current committed HEAD and confirming both creates and switches to it; invalid or duplicate names produce an actionable error without changing the repository.
- When local changes would be overwritten, switching fails and leaves the current branch, index, and working tree intact. Non-conflicting local changes remain subject to Git's normal behavior.
- Branch operations run off the input/render loop; in-progress actions cannot be duplicated, and stale read results cannot overwrite the refreshed state. Existing review, diff, and stage/unstage workflows still pass their checks.

## In scope / Out of scope

**In scope**

- List and select local branches; indicate current, detached, and unborn states.
- Confirm and switch to an existing local branch; confirm creation from current committed HEAD and switch to it.
- Validate names, show operation progress and errors, and refresh affected views after a successful operation.
- macOS verification with isolated temporary Git repositories, including dirty trees and branches used by another worktree.

**Out of scope**

- Fetch, pull, push, remote tracking creation or network authentication; branch deletion, rename, force switching, automatic stashing, and conflict resolution.
- Creating from an arbitrary revision, creating an orphan branch, or switching to a remote-only branch.
- Managing worktrees or submodules; Windows support and Linux release packaging.

## Approach

Add a local-branches view reachable with `b` from the history or working-tree view; Up/Down selects, Enter opens a confirmation, `c` opens a branch-name prompt, and Esc cancels. The prompt and confirmation show the exact branch and action. Creating a branch starts at the current committed HEAD and switches to it; if HEAD is unborn, report that a first commit is required and leave state unchanged. On detached HEAD, allow creation from the current commit and label the detached state. Empty lists, the current branch, and errors get explicit messages.

List `refs/heads` using `git for-each-ref` with explicit fields and NUL delimiters; obtain current/unborn/detached state separately and treat branch names as data, not shell syntax. Git documents the format fields and `HEAD` marker in [for-each-ref](https://git-scm.com/docs/git-for-each-ref). Validate entered names using [git check-ref-format](https://git-scm.com/docs/git-check-ref-format), reject option-like or revision-expansion input, and pass arguments directly. Use `git switch --no-guess --no-recurse-submodules -- <local-name>` for switching and `git switch --no-recurse-submodules -c <name>` for creation, without force, merge, or orphan flags. Git's [switch documentation](https://git-scm.com/docs/git-switch) describes protection of conflicting changes and transactional creation. Check the precise command forms against the installed Git in the first implementation slice; if unsupported, return an actionable version error rather than silently weaken safety.

Execute operations on a worker, serialize mutations, and invalidate outstanding history/status/detail requests on success before reloading. Re-read branch state after each operation; display Git's error on failure and do not claim success. Git may run repository-configured hooks; surface hook failures and document this behavior rather than bypassing them.

## Milestones

- [x] **1. Browse local branches:** Add the branch view, current/detached/unborn indicators, navigation and loading/error states. Test a repository with multiple branches, detached HEAD and no commits; verify the actual terminal view.
- [x] **2. Switch safely:** Confirm a selected local branch, execute the switch on a worker, and refresh review state. Test clean, dirty-conflicting, dirty-non-conflicting and other-worktree cases; verify the repository state and terminal feedback.
- [x] **3. Create and switch:** Add the name prompt and confirmation, validate the name, and create from committed HEAD. Test valid, invalid, duplicate, detached and unborn cases; verify no unintended branch or working-tree changes on failure.
- [x] **4. Harden and document:** Test fast navigation and stale results during switching, mutation serialization, help/ASCII mode and existing review workflows. Document controls, Git version requirements and limitations; run the full test suite, format, lint and build checks.

## Risks & open questions

- **Git versions and option parsing:** The selected command forms were tested with Git 2.54; `git switch` exists in Git 2.23, but the minimum supported Git has not been verified. Fallback: fail with a clear requirement, and document the tested version rather than using a forceful or ambiguous substitute.
- **Concurrent repository changes:** another process may change HEAD or branch refs between selection and execution. Revalidate the selected local ref immediately before switching, let Git arbitrate, and refresh afterward; surface failures.
- **Hooks and worktrees:** Git hooks may have side effects, and a branch may be checked out elsewhere. Preserve Git's default protections, display errors, and do not bypass hooks or worktree checks.
- **Unborn HEAD:** branch creation from an unborn branch is deferred; show a first-commit-required message. This is a deliberate fallback until an orphan-branch workflow is planned.

## Boundaries

Do not discard, overwrite, stash, or commit user changes; never pass force, merge, or orphan options. Switch only to an explicitly selected local branch and create only the confirmed name from the current committed HEAD. No remote communication. Preserve terminal restoration, responsiveness, and existing review/staging behavior. Release and publishing remain outside this plan.
