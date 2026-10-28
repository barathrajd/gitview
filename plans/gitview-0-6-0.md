# Gitview 0.6.0 — complete the partial Basic workflows

> **Status: released October 29, 2026.** All six workflow milestones shipped.
> The maintainer released without waiting for the complete performance and safety
> gates; the open items are tracked under "Upcoming" in
> [RELEASE.md](../RELEASE.md). Linux x86_64 moved to the next release.

## Objective

Complete the six partially implemented Basic workflow groups in the current
Git CLI capability matrix and ship them as Gitview 0.6.0. The release serves
developers who use Gitview for everyday local Git work and makes no claim that
remotes, merge, or export workflows are complete.

## Success criteria

- Repository setup supports safe initialization and cloning with clear path,
  progress, and Git error handling.
- Worktree review supports the planned path operations and arbitrary-path
  ignore inspection without losing unreviewed content or bypassing Git safety
  checks silently.
- Commit composition supports the configured external editor while preserving
  the reviewed staged snapshot and the existing single-Enter commit flow.
- History supports path history, commit/path comparisons, and patch summaries.
- Local branches support rename, delete, and previous checkout; tags support
  listing, creation, verification, and deletion.
- Diff and inspection support branch and available remote-ref comparisons plus
  object description, all read-only.
- The capability matrix accurately marks every included workflow and leaves
  remotes, merge, and export clearly incomplete.
- The release passes the project's full tests, build, formatting, and lint
  gates on macOS ARM64. Linux x86_64 is deferred to the next release.
- Release automation publishes the crate plus a verified macOS ARM64 archive
  with its checksum; `install.sh` selects and verifies it; release notes match
  the shipped behavior and say Linux packaging is deferred.

## In scope / Out of scope

**In scope**

- Complete repository, worktree, commit, history, local branches/tags, and
  diff/inspection workflows currently marked Partial in the capability matrix.
- No implicit network activity. `clone` may access the source the user
  explicitly provides; ongoing remote setup, fetch, pull, and push stay out of
  scope. Comparisons may use remote refs already present locally.
- Native macOS ARM64 checks and binary archive.
- Verified checksum handling in `install.sh` for the macOS archive.
- Linux x86_64 checks, archives, and installer selection are deferred to the
  next release; 0.6.0 makes no Linux support claim beyond the 0.2.1 notes.
- Test-first implementation, safety review, accurate capability and release
  documentation, and the existing release-please process.

**Out of scope**

- Remote setup, fetch, pull, push, and ahead/behind synchronization.
- Merge preview/application, conflict resolution, and merge editor flows.
- Archive/bundle export and import workflows.
- Windows, desktop applications, AI chat, performance claims, or features not
  supported by the repository's Git CLI capability plan.

## Approach

Implement each workflow as a small end-to-end slice: establish the expected
Git behavior in a test, add the UI action and its safety checks, then update
the matrix. Run Git subprocesses outside rendering and pass paths as arguments
or NUL-delimited input. Keep review and mutation separate, recheck stale state
before writes, and retain Git's refusal conditions. Reuse the repository's
terminal handoff pattern for external editors, with an explicit compatibility
spike before implementation. Keep the existing macOS ARM64 and crates.io workflows; a native Linux x86_64
release job is deferred to the next release. Do not publish or tag until
the user approves the final release candidate.

## Milestones

1. **Repository setup (complete):** add `gitview init` and `gitview clone`
   commands with destination validation, Git clone progress/error display,
   and tests for paths with spaces and existing destinations.
2. **Worktree paths (complete):** arbitrary repository-relative ignore-rule
   lookup, confirmed tracked-path removal, confirmed moves of clean tracked
   files, and staged-rename discard with stale-state checks are implemented.
   Git's normal safety checks remain enabled for destructive path operations.
3. **Commit editor handoff (complete):** Ctrl+E resolves Git's configured
   editor, invokes quoted executable arguments without shell evaluation, pauses
   the input reader, and restores the terminal. Saving a changed message with a
   non-empty subject commits it without a second Enter; an unchanged save returns
   to the form, and amend keeps its confirmation. The original staged snapshot
   remains required. Editor failure
   keeps the original message. Verified with an interactive pseudo-terminal.
4. **Branches and tags (complete):** confirmed branch rename and deletion
   are implemented with stale-tip and checkout checks and Git safeguards.
   Previous checkout supports local branches and detached commits with a
   reviewed destination and stale-state checks. Local tags support listing,
   lightweight creation at reviewed HEAD, Git signature verification, and
   deletion with atomic object checks. Annotated/signed creation is not exposed.
5. **History and diff inspection (complete):** literal file/directory
   history is implemented with a persistent, clearable filter and the existing
   200-commit limit. Commit/path comparisons and patch summaries now resolve
   fixed commit IDs. A single read-only review combines per-file statistics
   and foldable diffs with shared highlighting and layout toggles. Branch names and
   locally stored remote refs are supported without fetching. Object inspection
   resolves fixed IDs for commits, tags, trees, and blobs; it shows type, size,
   bounded previews, binary notices, and commit descriptions.
6. **Coverage and release gate:** complete the approved
   [performance review](gitview-0-6-0-performance.md) and its measured fixes
   before release, then complete capability-matrix acceptance,
   focused safety review, full test/build/lint checks on macOS ARM64, macOS
   archive checksum verification, installer checks, crate dry run, and release
   notes. Linux checks and archives move to the next release. Release only after all included features pass.

## Risks & open questions

- **Editor command syntax:** Git editor settings can include arguments and
  quoting. Test a safe argv-based invocation; if a configured command cannot
  be parsed without a shell, report it and keep the in-app composer available.
- **Git safety differences:** removal, move, and ref operations can be refused
  by Git when state changes. Keep those refusals visible and never silently
  force an operation.
- **Linux archive installation (deferred):** verifying the archive layout,
  checksum, and installer path on a clean Linux x86_64 runner moves to the next
  release. Until then, no Linux asset is published or called supported.
- **Release gate baseline:** the current checkout has Clippy warnings under
  local Rust 1.99.1. Identify which warnings also occur in the release CI
  toolchain and clear the gate without unrelated refactoring.
- **Security checklist source:** this checkout has no `CONSTRAINTS.md` or
  `security-and-hardening` skill. Use the repository's documented safety rules
  and add a release-specific focused review; don't claim a separate audit that
  was not run.

## Boundaries

Git remains the source of truth. Do not invoke shell command strings with
user-controlled paths, silently discard staged or working-tree content, fetch
implicitly, weaken existing protections, or claim remote/merge/export support.
Keep all release and website claims tied to features that pass acceptance.
