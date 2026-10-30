# Changelog

## [0.7.1](https://github.com/barathrajd/gitview/compare/v0.7.0...v0.7.1) (2026-10-30)


### Features

* **release:** redeploy the website after each release ([c27deb7](https://github.com/barathrajd/gitview/commit/c27deb76afd13363f28ffa01879700790349dbe8))
* **website:** features by release with live terminal demos ([81e08ca](https://github.com/barathrajd/gitview/commit/81e08ca766e057066f26752a1854c10d5a656fb8))
* **website:** replace release panels with an anchored feature catalogue ([8a74d9b](https://github.com/barathrajd/gitview/commit/8a74d9bbc2f8c0f43b7a8a0e467621bc0d8ec784))
* **website:** version from Cargo.toml, changelog by version, new logo ([7ffa2cd](https://github.com/barathrajd/gitview/commit/7ffa2cdd8289f53f9db05afbd17770c8ba41e60e))


### Bug Fixes

* **install:** find the latest release without cutting curl off ([e2fec55](https://github.com/barathrajd/gitview/commit/e2fec550306ac75f0311979b3907f2b6d46cd5cb))
* **install:** find the latest release without cutting curl off ([7d4c059](https://github.com/barathrajd/gitview/commit/7d4c059304fb32e31a97cba7d030e342af932b1c))
* **sessions:** count a session's commits against its base branch ([d7ba471](https://github.com/barathrajd/gitview/commit/d7ba47156d838a24a61f6d0fa2e55fe20a6f2cbc))
* **website:** add searchable feature navigation and shared branding ([5aaf7fb](https://github.com/barathrajd/gitview/commit/5aaf7fb54c32555ffe8b4b5b84d9d99137167107))

## [0.7.0](https://github.com/barathrajd/gitview/compare/v0.6.0...v0.7.0) (2026-10-30)


### Features

* **agent:** answer cursor position and device attribute queries ([14aec1d](https://github.com/barathrajd/gitview/commit/14aec1d4e5067f5b26886fe03bb2acd8af6c71ad))
* **control:** add read-only, size-capped repository tools ([0a502e8](https://github.com/barathrajd/gitview/commit/0a502e8c65f05d10ac9744e34b641ba10d13959f))
* **control:** add the gitview mcp stdio server ([e2d1429](https://github.com/barathrajd/gitview/commit/e2d1429a5e21e212c9a4f024670aa1692c04b1e9))
* **control:** connect hosted agents to Gitview ([81f6842](https://github.com/barathrajd/gitview/commit/81f684298275cb627518b69720c29a8160762984))
* **control:** let hosted agents read and steer Gitview ([76ead2a](https://github.com/barathrajd/gitview/commit/76ead2add020a86038a4ec626bf505a4cba22632))
* **control:** say what selections show and open commit file diffs ([22aec7d](https://github.com/barathrajd/gitview/commit/22aec7ddcb2c1e14e9b0e299b5b35c34491776ae))
* **control:** serve the tools on a private Unix socket ([2f5be3a](https://github.com/barathrajd/gitview/commit/2f5be3a250614ea469ff5a89bbd9d50f97e80674))
* **install:** install the Linux x86_64 archive with install.sh ([6327234](https://github.com/barathrajd/gitview/commit/63272342d1c669dcae7f096da25cf315ee2e34ca))
* **pane:** add --claude and --codex and explain a missing program ([2eb220c](https://github.com/barathrajd/gitview/commit/2eb220cf9f99b6412ac2e1ee5ec3d75bfc1dd87b))
* **pane:** add --claude and --codex, and a full-screen agent on narrow terminals ([55ad9ed](https://github.com/barathrajd/gitview/commit/55ad9edaa736f7ce7d4854b730ae7ea4a826e061))
* **pane:** run a program in a pane beside Gitview with --run ([79fad7b](https://github.com/barathrajd/gitview/commit/79fad7badff8fd75fcd0868a18eb6bd63c6566d7))
* **pane:** run a program in a pane beside Gitview with --run ([b84292d](https://github.com/barathrajd/gitview/commit/b84292d1881dd32c122081f608c72f5a23e2d801))
* **pane:** show the agent full screen on narrow terminals ([a222b6b](https://github.com/barathrajd/gitview/commit/a222b6b6a86b7bdf178fd7173214a423f804caf7))
* **release:** build and smoke-test a static Linux x86_64 archive ([173e231](https://github.com/barathrajd/gitview/commit/173e231c2d9072f2eff78a04cd955f72a5a225cf))
* **release:** push the Homebrew formula to barathrajd/homebrew-tap ([3aea9dd](https://github.com/barathrajd/gitview/commit/3aea9ddf7b9d7c1defbeeb9b7374a756416abb04))
* **release:** render the Homebrew formula for each release ([a3b4f43](https://github.com/barathrajd/gitview/commit/a3b4f43cda0996983109360bbfb5ea5fcd38654c))
* **release:** ship a static Linux archive and a Homebrew formula ([2f10f5c](https://github.com/barathrajd/gitview/commit/2f10f5cb0712f4ed977aad76f9a282863c83d25f))
* **sessions:** add a Sessions page to review, merge, and discard ([77de62b](https://github.com/barathrajd/gitview/commit/77de62b298a37f3b957394c9f6fdfa91b0968e85))
* **sessions:** create and list agent worktree sessions ([2f83d1e](https://github.com/barathrajd/gitview/commit/2f83d1eb34477f3b187f5f5b8f3e8f05533d38fe))
* **sessions:** discard sessions and clean up after a crash ([fcf4b41](https://github.com/barathrajd/gitview/commit/fcf4b414d26c9e1581526320a62277f751126ab0))
* **sessions:** review a session and merge it into its base branch ([928370c](https://github.com/barathrajd/gitview/commit/928370c7f8ba6891e2894627978e8c4ba30cdc8d))
* **sessions:** run agents in their own worktree sessions ([bf17ae8](https://github.com/barathrajd/gitview/commit/bf17ae802ce321efaf9bd2e42d6e1b7ff1a03b72))
* **sessions:** start an agent in a new session with --session ([5df4bfe](https://github.com/barathrajd/gitview/commit/5df4bfea771aead029078f203ce2abe6063ce401))
* **watch:** refresh the views when the repository changes ([562645b](https://github.com/barathrajd/gitview/commit/562645b2b3f0c77341583a71535e5d23fbf0934e))
* **watch:** refresh the views when the repository changes ([c9dd238](https://github.com/barathrajd/gitview/commit/c9dd23866d220a6fc5880e9733059b151276717a))
* **writes:** route every Gitview Git write through one path ([be2c2de](https://github.com/barathrajd/gitview/commit/be2c2de02f4797ea6affca834d14b98a0dd6101b))
* **writes:** run Git writes one at a time and wait out an agent's locks ([908b842](https://github.com/barathrajd/gitview/commit/908b842d443330862fd0b675ffbe3821757b8245))
* **writes:** run Git writes one at a time and wait out held locks ([2d36351](https://github.com/barathrajd/gitview/commit/2d36351629295f0c503be4eca20fb58c2a4f28ac))


### Bug Fixes

* **agent:** draw the agent pane's border in ASCII with --ascii ([ffea3a2](https://github.com/barathrajd/gitview/commit/ffea3a200cd5f345cf9fbcd98497c440dd10e85f))
* **agent:** harden the agent workspace and document its security ([2396244](https://github.com/barathrajd/gitview/commit/2396244ce78498740430e7a0b0fe9cdc894a5d79))
* **ci:** allow the Linux-only lint in the live test helper ([c6ab582](https://github.com/barathrajd/gitview/commit/c6ab582a32e36fe0960578b901bda424083a2d29))
* **ci:** give the session test repositories a commit identity ([ec75669](https://github.com/barathrajd/gitview/commit/ec75669da9611cbc3e6c73ffddaef01ce98fb5b7))
* **ci:** give the session test repositories a commit identity ([44cf14b](https://github.com/barathrajd/gitview/commit/44cf14bb2bad142a843fae2cc57719ce3014b653))
* **ci:** make the Linux checks pass ([0a205a5](https://github.com/barathrajd/gitview/commit/0a205a55a77b2f1438450cf25408fe28cdfc2780))
* **ci:** move GitHub Actions to their Node 24 majors ([0431d1b](https://github.com/barathrajd/gitview/commit/0431d1b6a48e3aa5ba5530718360f3dd6a57ae42))
* **control:** cap socket connections and close idle ones ([9ab6915](https://github.com/barathrajd/gitview/commit/9ab6915651ab2c4019f04c003e9d9f2694a4dc2b))
* **control:** remove sockets left by Gitviews that were killed ([209584e](https://github.com/barathrajd/gitview/commit/209584e2edda4ed9d2c57507a5d1e0b16c31ddab))
* **control:** say in the agent pane when Gitview's tools are off ([b0e532f](https://github.com/barathrajd/gitview/commit/b0e532f132762befa8368bac56373d20ef6941fb))
* **process:** stop background Git reads from rewriting the index ([e85f1ed](https://github.com/barathrajd/gitview/commit/e85f1ed766b9f44fa97d283fd6e24bc5f931197f))
* **release:** let Homebrew load the formula on every machine ([feed8f2](https://github.com/barathrajd/gitview/commit/feed8f283f0ae773e5197ab90710556bf72b7085))
* **release:** state the version for the dry-run Homebrew install ([7c039bb](https://github.com/barathrajd/gitview/commit/7c039bb3cc966b4eb68477f424f7b7369ed3d903))
* **sessions:** lock the manifest while sessions are created or discarded ([ec88ed5](https://github.com/barathrajd/gitview/commit/ec88ed50daf213a2931092f4c66fba8be8b48679))
* **sessions:** merge in the main worktree and keep the current session ([7439241](https://github.com/barathrajd/gitview/commit/74392418ec266d44738382801dc2f052cd4522ff))
* **watch:** time a reload by its work, not by when its result was read ([7a3f26b](https://github.com/barathrajd/gitview/commit/7a3f26bb96a2efb09f6690aa62f875064618efbb))

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
