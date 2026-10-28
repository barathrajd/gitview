# Gitview 0.6.0 — complete the partial Basic workflows

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
  gates on macOS ARM64 and Linux x86_64.
- Release automation publishes the crate plus verified macOS ARM64 and Linux
  x86_64 archives with checksums; `install.sh` selects and verifies the native
  archive; release notes match the shipped behavior.

## In scope / Out of scope

**In scope**

- Complete repository, worktree, commit, history, local branches/tags, and
  diff/inspection workflows currently marked Partial in the capability matrix.
- No implicit network activity. `clone` may access the source the user
  explicitly provides; ongoing remote setup, fetch, pull, and push stay out of
  scope. Comparisons may use remote refs already present locally.
- Native macOS ARM64 and Linux x86_64 checks and binary archives.
- Verified platform selection and checksum handling in `install.sh`.
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
spike before implementation. Add a native Linux x86_64 release job alongside
the existing macOS ARM64 and crates.io workflows. Do not publish or tag until
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
   the input reader, and restores the terminal and reviewed commit form. The
   original staged snapshot remains required by Enter-to-commit. Editor failure
   keeps the original message. Verified with an interactive pseudo-terminal.
4. **Branches and tags (in progress):** confirmed branch rename and deletion
   are implemented with stale-tip and checkout checks and Git safeguards.
   Finish previous checkout and local tag list/create/verify/delete.
5. **History and diff inspection:** add path history, path/commit comparisons,
   patch summaries, branch and locally available remote-ref comparisons, and
   object descriptions.
6. **Coverage and release gate:** complete capability-matrix acceptance,
   focused safety review, full test/build/lint checks on both target platforms,
   Linux and macOS archive checksum verification, installer checks, crate dry
   run, and release notes. Release only after all included features pass.

## Risks & open questions

- **Editor command syntax:** Git editor settings can include arguments and
  quoting. Test a safe argv-based invocation; if a configured command cannot
  be parsed without a shell, report it and keep the in-app composer available.
- **Git safety differences:** removal, move, and ref operations can be refused
  by Git when state changes. Keep those refusals visible and never silently
  force an operation.
- **Linux archive installation:** verify the archive layout, checksum, and
  installer path on a clean Linux x86_64 runner before calling the asset
  supported.
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
