# Changelog

## [0.4.1](https://github.com/barathrajd/gitview/compare/v0.4.0...v0.4.1) (2026-10-27)


### Bug Fixes

* **release:** add Cargo publishing and verified installer ([c3cfdc8](https://github.com/barathrajd/gitview/commit/c3cfdc81fd3543921f732140ed393705411a0534))
* **release:** bound installer download retries ([3608cd0](https://github.com/barathrajd/gitview/commit/3608cd0da358f413957d4eb6153e8565d5e99185))

## [0.4.0](https://github.com/barathrajd/gitview/compare/v0.3.3...v0.4.0) (2026-10-27)


### Features

* **ui:** add grouped worktree staging and page help ([73abbb6](https://github.com/barathrajd/gitview/commit/73abbb648d4fccb53a95e1db3b70285e9179f425))
* **worktree:** add hunk and bulk staging ([0bb168f](https://github.com/barathrajd/gitview/commit/0bb168f2b94f96a2d4a7189ad5d8f63a9e7e4bbd))

## [0.3.3](https://github.com/barathrajd/gitview/compare/v0.3.2...v0.3.3) (2026-10-27)


### Miscellaneous Chores

* **release:** request v0.3.3 ([b71a589](https://github.com/barathrajd/gitview/commit/b71a589c7c42289b5323cf1aa8248083fdc35cdd))

## 0.3.2 — October 27, 2026

### Fixed

- Package only the Gitview executable in the macOS archive so GAH can install
  the matching binary without encountering extra executable files.
- Document the macOS GNU `find` requirement for GAH and verify installation.

## 0.3.1 — October 27, 2026

### Added

- Added the MIT License.
- Added GAH instructions for installing Gitview into the current user's local
  binary directory.

## 0.3.0 — October 27, 2026

See the [GitHub release][release-030].

### Added

- Responsive cockpit with history, working tree, and repository details.
- Debounced history search with selectable results that open commit details.
- Old and new source line numbers in diff views.
- A compact shortcut bar and clearer help panel navigation.

### Changed

- Search result navigation uses Up/Down and Enter; the ambiguous next-match
  shortcut has been removed.
- Press Esc or `?` to close the help panel.

### Fixed

- Keep shortcut labels and colors consistent across TUI pages.
- Preserve the quit hint as lowercase `q`.

## 0.2.1 — October 27, 2026

Refactored UI state and rendering, hardened mutation boundaries, and added
regression coverage. See the [v0.2.1 release][release-021].

[release-030]: https://github.com/barathrajd/gitview/releases/tag/v0.3.0
[release-021]: https://github.com/barathrajd/gitview/releases/tag/v0.2.1
