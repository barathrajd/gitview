# Release plan

This file tracks planned releases and records published Gitview releases.

## Upcoming

### Gitview 0.3.0 — October 27, 2026

Release a polished, responsive history and diff review UI with consistent
navigation.

- [done] UI navigation polish — [roadmap item](ROADMAP.md#next),
  [plan](plans/ui-navigation-0-3-0.md).

Release gate:

- [x] Feature acceptance criteria pass.
- [x] Full tests, formatting, Clippy, and release build pass.
- [x] macOS Apple Silicon artifact is built and inspected.
- [x] No known credential patterns were found in tracked files or reachable
  commit diffs. GitHub Secret Scanning is disabled on the private repository.
- [x] README and changelog match the release.
- [ ] Publish the tag and GitHub release with `gh`.
- [ ] Make the GitHub repository public and verify its visibility.
- [ ] Verify the public release page and archive download.
- Definition of Done: [release-plan reference](.agents/skills/release-plan/references/definition-of-done.md).

## Released

### Gitview 0.2.1 — October 27, 2026

Refactored UI state and rendering, hardened mutation boundaries, and added
regression coverage. Published as [v0.2.1](https://github.com/barathrajd/gitview/releases/tag/v0.2.1).
