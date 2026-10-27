# Changelog

## [0.4.0](https://github.com/barathrajd/gitview/compare/gitview-v0.3.1...gitview-v0.4.0) (2026-10-27)


### Features

* add responsive Git cockpit ([4ed749a](https://github.com/barathrajd/gitview/commit/4ed749ad06ea36da81e81750f3d1b4d5ff2c4e0e))
* add vim navigation and bump version to 0.2.0 ([4b1a6bc](https://github.com/barathrajd/gitview/commit/4b1a6bc77554f96a377223c4e728e52af6858ce5))
* browse local branches ([21d9e6d](https://github.com/barathrajd/gitview/commit/21d9e6d7c155807441ebaa06916b85d164587b31))
* create and switch local branches ([12c765a](https://github.com/barathrajd/gitview/commit/12c765a7a1bcf142ed92462578f6f1eb96af895f))
* gitview initial setup ([d1368bd](https://github.com/barathrajd/gitview/commit/d1368bd15a9f9990c17d47996cbec8346b67ef33))
* improve shortcut guide, search, and diff navigation ([eb58cfd](https://github.com/barathrajd/gitview/commit/eb58cfdb06b23caa4b0d79a2b79aa51157243043))
* switch local branches safely ([12d826b](https://github.com/barathrajd/gitview/commit/12d826b87119182b01aeb723e689e631556e1d80))


### Bug Fixes

* clear stale TUI screens and correct quit hint ([105d64b](https://github.com/barathrajd/gitview/commit/105d64b4c5e09d8e3143dc1727a92ec443fb3930))
* close help panel with escape ([b0eefb8](https://github.com/barathrajd/gitview/commit/b0eefb8d4a8bc043242fe8bba55f508ed9e924fd))
* compact TUI shortcut status bar ([a68dda9](https://github.com/barathrajd/gitview/commit/a68dda97932d34c8be7e2bdfa3cd439acd822e38))
* remove ambiguous next match shortcut ([7956aae](https://github.com/barathrajd/gitview/commit/7956aae754124a3171a4b60d755b39028b3bf301))
* show l navigation in shortcut bar ([5217b77](https://github.com/barathrajd/gitview/commit/5217b77f5e7a2d31111d860db20e5993338f841e))
* unify TUI shortcut hints and page styling ([fec947d](https://github.com/barathrajd/gitview/commit/fec947d4111b2ee48d4e85b62c04a4375d0c9d0e))

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
