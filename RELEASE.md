# Release plan

This file tracks planned releases and records published Gitview releases.

## Upcoming

### Gitview 0.3.1 — October 27, 2026

Add an explicit MIT license and a no-sudo GAH installation path for macOS
Apple Silicon users.

- [done] License and user-local installation — [roadmap item](ROADMAP.md#next),
  [plan](plans/release-installation-0-3-1.md).

Release gate:

- [x] License text and installation instructions are in the repository.
- [x] Full tests, formatting, Clippy, and release build pass.
- [x] macOS Apple Silicon archive includes the executable, README, and license.
- [ ] Publish `v0.3.1` with `gh` and verify the public download.
- Definition of Done: [release-plan reference][definition-of-done].

## Released

### Gitview 0.3.0 — October 27, 2026

Published the responsive cockpit and improved search and diff navigation.
Published the macOS Apple Silicon archive and checksum, plus GitHub source
archives. Made the repository public after checking tracked files and reachable
commit diffs for common credential patterns.

- Release: [GitHub v0.3.0][release-030]
- Tag: `v0.3.0`
- Plan: [UI navigation polish](plans/ui-navigation-0-3-0.md)
- Definition of Done: [release-plan reference][definition-of-done]

### Gitview 0.2.1 — October 27, 2026

Refactored UI state and rendering, hardened mutation boundaries, and added
regression coverage. Published as [v0.2.1][release-021].

[release-030]: https://github.com/barathrajd/gitview/releases/tag/v0.3.0
[release-021]: https://github.com/barathrajd/gitview/releases/tag/v0.2.1
[definition-of-done]:
  .agents/skills/release-plan/references/definition-of-done.md
