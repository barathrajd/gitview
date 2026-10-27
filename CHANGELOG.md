# Changelog

## [0.3.2](https://github.com/barathrajd/gitview/compare/v0.3.1...v0.3.2) (2026-10-27)


### Bug Fixes

* **release-please:** match existing version tags ([40dee60](https://github.com/barathrajd/gitview/commit/40dee601c49351be7442cfc7a3e05ad43244ea18))

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
