# Changelog

## [0.6.0](https://github.com/barathrajd/gitview/compare/v0.5.0...v0.6.0) (2026-10-28)


### Features

* added the claude skills ([149323a](https://github.com/barathrajd/gitview/commit/149323ab3903e4ac0ace1634e42c678ddb6aa8a4))
* **branches:** add confirmed previous checkout ([2897eb7](https://github.com/barathrajd/gitview/commit/2897eb7e3aebaabbe3aaa6e8fc24bc440b8a6df3))
* **branches:** add confirmed rename and safe deletion ([dee8e61](https://github.com/barathrajd/gitview/commit/dee8e61f32e4defdd737e6aaf3a6523707a291e1))
* **cli:** add repository setup and ignore-rule inspection ([b7ea5c8](https://github.com/barathrajd/gitview/commit/b7ea5c8c72ecba41bbbd43ae709f865f6510f009))
* **commit:** add configured external editor handoff ([5373a75](https://github.com/barathrajd/gitview/commit/5373a75a8ed986b15105dc74cc6c9f57e2e99800))
* **commit:** add guarded HEAD amendment ([57459d8](https://github.com/barathrajd/gitview/commit/57459d82af37cb31e31e9b687abf06cdb32bc11e))
* **commit:** add optional commit descriptions ([296ae8a](https://github.com/barathrajd/gitview/commit/296ae8a30aa4c3cad4164b1b207c62367149f096))
* **commit:** add reviewed staged commit workflow ([acaad18](https://github.com/barathrajd/gitview/commit/acaad189ab88d8d1fac93142b2aedd3f0cc1c5a4))
* **commit:** commit directly after saving in the external editor ([86035f2](https://github.com/barathrajd/gitview/commit/86035f286c2696fe67b91921596d473bb72ab21e))
* **diff:** add commit comparisons and patch summaries ([bc699ff](https://github.com/barathrajd/gitview/commit/bc699ff16cb5dd5102a2636b9dfdedc1a88a2aa1))
* **diff:** add IDE styling and independent view toggles ([a8dafe4](https://github.com/barathrajd/gitview/commit/a8dafe4be23ebcda6745c2ddbbc3ca99d510acf9))
* **diff:** combine comparison summaries with collapsible file diffs ([df10575](https://github.com/barathrajd/gitview/commit/df105755213c251e5c3be788e56115d53b655c45))
* **diff:** default to wrapped side-by-side view ([daa41b9](https://github.com/barathrajd/gitview/commit/daa41b9cf9e74c0c0260fb3dc963811c1433d2b3))
* **history:** add persistent literal path filtering ([fbb40ce](https://github.com/barathrajd/gitview/commit/fbb40ce646983b693e23639f837a0d9fa617c69f))
* **inspect:** add local Git object descriptions ([9737cea](https://github.com/barathrajd/gitview/commit/9737cead19018415a76b485cb4c572936d224dab))
* **tags:** add local tag management and verification ([ad757d9](https://github.com/barathrajd/gitview/commit/ad757d92378bdefddaaa6ffe62d68f0423fa368f))
* **website:** add a mobile navigation menu ([3a0df05](https://github.com/barathrajd/gitview/commit/3a0df0519ba586b79010f8f424ec15bfb6f831b4))
* **website:** add feature and installation sections ([e6551dc](https://github.com/barathrajd/gitview/commit/e6551dcc099c4a93aab02f0f4620c04c1b2ec6dd))
* **website:** add footer and Pages setup guide ([a3c4fc3](https://github.com/barathrajd/gitview/commit/a3c4fc3332c76f72f9541309bdbf75752406ddba))
* **website:** add landing hero and terminal preview ([af70ede](https://github.com/barathrajd/gitview/commit/af70ede33416c154f3a85768e5e1b6911fc1cd23))
* **website:** make pages responsive and hide scrollbars ([3cf902d](https://github.com/barathrajd/gitview/commit/3cf902d0e31f60d756a05e463d0a6c19ead59c63))
* **website:** split features and changelog into their own pages ([fdaf2ee](https://github.com/barathrajd/gitview/commit/fdaf2ee0f9237bc50a85518e88d0f174e87106ca))
* **worktree:** add confirmed selected-file discard ([a242a64](https://github.com/barathrajd/gitview/commit/a242a64796116a42d901ecac1d87a3fbd1565e4c))
* **worktree:** add confirmed stop-tracking action ([0276416](https://github.com/barathrajd/gitview/commit/02764163811ca3ed1221698eaa4890bd37af0af0))
* **worktree:** add safe path removal and move workflows ([dbe634c](https://github.com/barathrajd/gitview/commit/dbe634c29e31c426f510a206ef32f39e68c19671))
* **worktree:** inspect ignore rules for selected paths ([b8cb4ea](https://github.com/barathrajd/gitview/commit/b8cb4ea6fa3ae7260733969c894d105ad04585de))


### Bug Fixes

* **diff:** align wrapped side-by-side lines ([7f2beb7](https://github.com/barathrajd/gitview/commit/7f2beb74b845b993b3912f597c6670c50a4c8179))
* **ui:** align new pages with the shared terminal theme ([17236d8](https://github.com/barathrajd/gitview/commit/17236d80c27b1ae3471d3f12264196c2dd993073))
* **website:** align terminal preview with current TUI ([18356d8](https://github.com/barathrajd/gitview/commit/18356d81535e54039b5d6d13c965cd40239fc709))
* **website:** serve custom domain from root ([9d0975a](https://github.com/barathrajd/gitview/commit/9d0975aad1ab9f62699397d83c8143d05f9d2dc0))
* **worktree:** discard edits to staged renames ([dba13da](https://github.com/barathrajd/gitview/commit/dba13da81d130189f61ebce21b6639c9a8185bd5))


### Performance Improvements

* **comparison:** bound the render cache by estimated bytes ([f4443b2](https://github.com/barathrajd/gitview/commit/f4443b205d8a3b63806e7cfb65a81a6a8bd33d73))
* **comparison:** cancel preparation when the page closes ([ff58978](https://github.com/barathrajd/gitview/commit/ff58978a3cac137e2e3203813945e10b7f06586c))
* **diff:** bound retained layouts and reduce padding allocations ([a2bac04](https://github.com/barathrajd/gitview/commit/a2bac04eb6b202e2b88148638786bbe46d073840))
* **diff:** cache render variants and prepare highlighting in background ([a320218](https://github.com/barathrajd/gitview/commit/a3202185bf83cefcc2fcd3ca122fd623a3411abc))
* **diff:** prepare ordinary diffs outside the render loop ([090250d](https://github.com/barathrajd/gitview/commit/090250def7b0fa397355cd4a43b4e6973601eef6))
* **tui:** keep commit previews responsive and avoid idle redraws ([bae752d](https://github.com/barathrajd/gitview/commit/bae752d9ca3684473c06bb39f6cc2671ab2c6e5f))
* **tui:** make path ref and tag previews cancellable ([e1a3609](https://github.com/barathrajd/gitview/commit/e1a3609b945c1161f26b5cee75097701e89cb205))

## [0.5.0](https://github.com/barathrajd/gitview/compare/v0.4.1...v0.5.0) (2026-10-27)


### Features

* add folder-scoped Git config profiles ([ce2a5d2](https://github.com/barathrajd/gitview/commit/ce2a5d2094bc7055354470bc55099b0e6cda1c46))

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
