# Release plan

This file tracks planned releases and records published Gitview releases.

## Upcoming

No release is currently in progress. Release Please will prepare future releases
from Conventional Commits on `main`.

## Released

### Gitview 0.3.1 — October 27, 2026

Added the MIT license and published a macOS Apple Silicon archive, checksum,
and source archives. Verified installation through GAH into `~/.local/bin`.

- Release: [GitHub v0.3.1][release-031]
- Tag: `v0.3.1`
- Plan: [license and local installation](plans/release-installation-0-3-1.md)
- Definition of Done: [release-plan reference][definition-of-done]

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
[release-031]: https://github.com/barathrajd/gitview/releases/tag/v0.3.1
[release-021]: https://github.com/barathrajd/gitview/releases/tag/v0.2.1
[definition-of-done]:
  .agents/skills/release-plan/references/definition-of-done.md
