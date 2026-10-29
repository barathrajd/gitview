# Roadmap

## Now

- [0.6.0 follow-ups](RELEASE.md#upcoming) — the known issues 0.6.0 shipped
  with: large-comparison memory variance, the renderer-only memory diagnostic,
  the security review, and a clean-revision rerun.
- [Main and UI refactor](plans/refactor-main-ui.md) — all seven milestones are
  done on `docs/agent-workspace-plan`: a key-replay test harness, then
  `main.rs` and `ui.rs` split into `src/app/` and `src/ui/`, plus an inert input
  router and layout slot. Merge it, then resume the agent workspace.
- [Git CLI capability roadmap](plans/git-cli-capability-roadmap-2026.md) —
  track remaining Basic, Medium, and Advanced workflows beyond this release.

## Next

- Linux x86_64 release — deferred from 0.6.0: native checks, performance
  evidence, archive with checksum, and installer selection on a Linux runner.
- [Agent workspace](plans/agent-workspace.md) — milestones 1 and 2 are done:
  `gitview --claude`, `--codex`, or `--run <command>` shows a program in a pane
  with a focus key, full screen on narrow terminals. Next is refreshing the Git
  views when the agent changes files. Run Claude Code or Codex in a pane inside
  Gitview, let the agent read and steer Gitview state through a
  local control channel, then add per-session worktrees.
- macOS desktop app — design the first desktop client after TUI completion.

## Later

- Linux desktop app — follow the macOS desktop app.
- Windows desktop app — follow the Linux desktop app.
- AI chat — plan provider and subscription support after the core Git workbench.

## Shipped

- 2026-10-29: [Gitview 0.6.0](plans/gitview-0-6-0.md) — six partial Basic
  workflows completed, with the [performance review](plans/gitview-0-6-0-performance.md)
  carried out and its open findings recorded; released as [v0.6.0][release-060].
- 2026-10-28: [Git config profiles](plans/git-config-profiles.md) —
  folder-based account settings, released in [Gitview v0.5.0][release-050].
- 2026-10-27: Gitview 0.4.0 — hunk staging, grouped worktree status, and
  contextual page help; published as [v0.4.0][release-040].
- 2026-10-27: [License, local installation, and release automation](plans/release-installation-0-3-1.md)
  — published as [Gitview 0.3.1][release-031].
- 2026-10-27: [UI navigation polish](plans/ui-navigation-0-3-0.md) — published
  as [Gitview 0.3.0][release-030].
- 2026-10-27: [Local branches](plans/local-branches.md) — completed local
  branch browsing, switching, and creation; included in
  [Gitview 0.3.0][release-030].

- 2026-10-27: [Git review MVP](plans/git-review-mvp.md) — tagged a locally
  tested macOS preview as `v0.1.0`.

[release-030]: https://github.com/barathrajd/gitview/releases/tag/v0.3.0
[release-031]: https://github.com/barathrajd/gitview/releases/tag/v0.3.1
[release-040]: https://github.com/barathrajd/gitview/releases/tag/v0.4.0
[release-050]: https://github.com/barathrajd/gitview/releases/tag/v0.5.0
[release-060]: https://github.com/barathrajd/gitview/releases/tag/v0.6.0
