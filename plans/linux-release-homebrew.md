# Linux release and Homebrew tap

## Objective

Gitview releases ship one prebuilt binary, for macOS on Apple Silicon. Linux
users can only build it with `cargo install`, and Mac users who expect
Homebrew have no formula. Starting with 0.7.0, every release also ships a
static Linux x86_64 binary, and Gitview installs with
`brew install barathrajd/tap/gitview` on macOS Apple Silicon and Linux
x86_64. It's for Linux developers and for anyone who installs tools with
Homebrew.

## Success criteria

- A release run uploads `gitview-<tag>-linux-x86_64.tar.gz` and its `.sha256`
  next to the macOS archive. The binary inside is a statically linked
  `x86_64-unknown-linux-musl` executable.
- The release job checks that binary before uploading it:
  - `file` reports it as statically linked;
  - `--version` prints the tag's version;
  - `--help` succeeds;
  - in an `alpine` container, with Git installed, it opens a repository with
    an agent pane in a pseudo-terminal, the agent reaches it through
    `gitview mcp`, and it exits with code 0.
- `install.sh` installs the Linux archive on Linux x86_64 and the macOS
  archive on macOS Apple Silicon. It verifies the checksum with `sha256sum`,
  or with `shasum -a 256` where that is missing. It names the supported
  platforms when run anywhere else.
- After each release, the public `barathrajd/homebrew-tap` tap has a
  `Formula/gitview.rb` with that release's version, URLs and SHA-256 values,
  pushed by the release workflow without a manual step.
- On macOS Apple Silicon and on Linux x86_64,
  `brew install barathrajd/tap/gitview` installs that release and
  `brew test gitview` passes. On an unsupported platform, Homebrew refuses
  with a clear message.
- The README, the website's install guide, `RELEASE.md` and `ROADMAP.md`
  describe the Homebrew and Linux options. This is published only after the
  release that ships them exists.
- One Linux performance run of the release binary is recorded in the release
  notes. There are no new budgets.

## In scope / Out of scope

**In scope**

- A `build-linux-x86_64` job in `.github/workflows/release-please.yml` on
  `ubuntu-latest`. It builds with the `x86_64-unknown-linux-musl` target,
  smoke-tests the binary, and uploads the archive and checksum.
- Linux x86_64 support and the checksum fallback in `install.sh`.
- The tap repository `barathrajd/homebrew-tap`, a formula template in this
  repository, a script that renders it, and an `update-homebrew-tap` job that
  pushes the rendered formula after both archives are uploaded.
- A `workflow_dispatch` dry run that builds, tests and renders everything
  for a given tag without uploading or pushing, so this can be checked before
  0.7.0.
- Documentation of both install paths, including the `PATH` caveat below.

**Out of scope**

- Linux on ARM, macOS on Intel, and Windows.
- `homebrew-core`, Homebrew bottles built from source, and casks.
- Distribution packages (`.deb`, `.rpm`, AUR, Nix) and container images.
- Rerunning the 0.6.0 performance suite against budgets on Linux.
- Changes to the macOS archive, its name, or the crates.io publish.
- Signing or notarizing binaries.

## Approach

Add a Linux build job beside `build-macos-arm64`. It installs the
`x86_64-unknown-linux-musl` target, runs
`cargo build --release --locked --target x86_64-unknown-linux-musl`, and
packages the binary the same way the macOS job does. A smoke step runs the
checks above, including a container run on `alpine`. Only when they pass does
the job upload the assets.

`install.sh` maps `uname -s` and `uname -m` to an archive name: `Darwin` with
`arm64` or `aarch64`, or `Linux` with `x86_64` or `amd64`. It verifies with
`sha256sum -c`, falling back to `shasum -a 256 -c`.

The formula lives as a template in this repository
(`packaging/homebrew/gitview.rb.in`). `scripts/render-homebrew-formula.sh`
fills it in from the tag and the two `.sha256` files. The
`update-homebrew-tap` job needs both build jobs. It renders the formula, runs
`brew style` on it, and commits it to the tap with a fine-grained token that
can write only to the tap repository (secret `HOMEBREW_TAP_TOKEN`).

The formula uses `on_macos` and `on_linux` blocks with `on_arm` and
`on_intel`, a `url` and `sha256` for each archive, `depends_on "git"`, and
`bin.install "gitview"`. Its test checks `gitview --version`.

**Decided (2026-10-30, with the user)**

- **Linux build:** x86_64 only, statically linked with musl, so one binary
  runs on any distribution, Alpine included.
- **Tap:** a separate repository, `barathrajd/homebrew-tap`, shared by
  future tools, so the install command is
  `brew install barathrajd/tap/gitview` (changed from a gitview-only tap on
  2026-10-30, so other tools can use the same tap).
- **Formula updates:** the release workflow pushes the new formula itself.
- **Linux gate:** CI on Linux plus the smoke test of the release binary, and
  one recorded performance run. No full performance parity.

**Settled by spikes**

- **The musl build works:** the binary builds and runs with no C toolchain
  beyond the Rust image's. The spike built `aarch64-unknown-linux-musl` on an
  arm64 Linux VM as a proxy; `file` reported it statically linked. On
  `alpine:latest`, which has no glibc, it opened a repository with an agent
  pane in a pseudo-terminal. The agent's socket was `srw-------`, `gitview
  mcp` returned `get_status`, and Gitview exited with code 0. The x86_64
  target itself is first built in milestone 1, on CI's x86_64 runner.
- **The formula shape:** a formula like the one above, pointing at the real
  v0.6.0 macOS archive, installed and passed `brew test` on Homebrew 7.0.11.
  `brew audit --strict` required `depends_on` before `on_macos` and no
  `version` line (Homebrew reads it from the URL). The tap
  `sinelaw/homebrew-fresh` uses the same per-platform layout.
- **Homebrew trust:** Homebrew 7 refuses to load formulas from an untrusted
  tap for `brew audit`, until `brew trust` is run. `brew install
  user/tap/formula` worked without it.
- **Checksums on Linux:** neither Alpine nor Debian has `shasum`. Both
  verify a `shasum`-format `.sha256` file with `sha256sum -c`.

## Milestones

1. **Linux archive in the release.** *Done. The x86_64 dry run passed on
   GitHub (run 41035029170): the musl build, the smoke test with its Alpine
   stage, and a 2.8 MB `gitview-dry-run-linux-x86_64` artifact. The first dry
   run had stopped on session tests that needed a Git identity, fixed in #19.
   Don't start a dry run while a `main` run is in progress: both runs'
   release-please steps update release PR #14's branch, and one fails.* Add the `build-linux-x86_64` job with
   its smoke test, plus a dry-run dispatch that skips the upload. Verify with
   a dry run on this branch on GitHub's x86_64 runner: the archive builds,
   `file` says static, and the Alpine run passes. Locally (arm64 Linux), the
   smoke script rejects a dynamically linked build and passes the static one,
   and its Alpine stage fails on a binary that can't run there.
2. **Installer selects Linux.** *Done.* Teach `install.sh` the Linux archive and the
   `sha256sum` fallback. Test it in Alpine (with `bash` and `curl` added),
   Debian and macOS against archives served from a local folder. Add an
   environment override for the download URL, used only by tests.
   *Result:* `tests/installer.rs` runs the real `install.sh` against a local
   release folder. Linux x86_64 (and `amd64`) gets the Linux archive, macOS
   Apple Silicon still gets the macOS one, and other platforms are refused
   with the supported list. A bad checksum installs nothing, either checksum
   tool works, and the installer stops without one. The tests passed on
   macOS and in Debian (`rust:latest`). On Alpine (with `bash` and `curl`),
   the installer verified with `sha256sum` and installed the real static
   binary. On this Mac it still installs the published v0.6.0.
3. **Formula template and renderer.** *Done. Verified by dry run
   41039081153.* On Linux, `brew install` and `brew test` passed with the
   dry-run archive. On `macos-15-intel`, Homebrew refused with "The arm64
   architecture is required for this software." On this Mac, the published
   v0.6.0 archive installed from a local tap and passed `brew test`. Found on
   the way: `url` must sit at the top level or inside `on_arm`/`on_intel`, and
   every machine needs a URL before Homebrew checks the architecture. So the
   macOS archive is the default URL, overridden for x86_64 Linux. Homebrew
   reads a `file://` archive's version as "86.64" (the CI copy states it);
   the real release URLs parse as the release version. Add the template and the render script,
   with tests that render a formula from sample checksums, run `brew style`
   on it, and install it from a local tap on macOS. On a Linux runner with
   Homebrew, run `brew install` and `brew test` against the dry-run archive.
4. **Tap repository and push job.** *Done. The shared tap
   `barathrajd/homebrew-tap` was created on 2026-10-30, and the user added
   `HOMEBREW_TAP_TOKEN`. In the dry run, `update-homebrew-tap` rendered the
   formula and GitHub accepted a dry-run push; the tap still has only its
   README commit.* With the user's confirmation, create
   `barathrajd/homebrew-tap` (public, with a README) and have the user add
   `HOMEBREW_TAP_TOKEN`. Add the `update-homebrew-tap` job. A dry run renders
   and checks the formula but doesn't push.
5. **Docs, then the 0.7.0 release.** Write the README and website install
   steps, `RELEASE.md` and `ROADMAP.md`, held back until the release exists.
   After the user merges the release PR, check that the release has both
   archives, the tap has 0.7.0, and `brew install` and `install.sh` work on
   macOS and in a Linux container. Record one Linux performance run.

## Risks & open questions

- **The x86_64 build is unproven until milestone 1.** The spike used
  aarch64. *Fallback:* if a dependency needs C on x86_64 musl, install
  `musl-tools` in the job. If that fails, ship a glibc build on the oldest
  available Ubuntu runner and state the glibc floor.
- **musl's allocator is slower than glibc's.** *Fallback:* the recorded
  performance run shows whether it matters. If diffs or history are visibly
  slower, switch the Linux build to a faster allocator, which needs a
  separate decision.
- **The token is set up by hand.** The user creates the fine-grained token
  and secret. *Fallback:* without it the job fails loudly and the release
  is otherwise complete; the formula can be pushed with a rerun.
- **Older copies shadow Homebrew's.** A `gitview` in `~/.local/bin` (from
  `install.sh`) or `~/.cargo/bin` comes earlier on many `PATH`s. Homebrew
  warned about this in the spike. *Mitigation:* the docs say to remove the old
  copy.
- **`install.sh` needs Bash.** Alpine doesn't install it by default.
  *Mitigation:* the docs say so, and Alpine users can use the archive
  directly or `cargo install`.
- **The unsupported-platform message is unchecked.** It's **UNVERIFIED**
  how Homebrew reports a formula with no URL for the platform, such as macOS
  on Intel or Linux on ARM. Milestone 3 checks the message and adds a
  `depends_on arch:` line if it's unclear.
- **Timing against 0.7.0.** Release PR #14 is open. Merging it before
  milestones 1–4 land would ship 0.7.0 without these. *Owner:* the user holds
  the merge until milestone 4 is done.

## Boundaries

- The macOS archive's name, contents, build and checksum stay as they are;
  `install.sh` keeps installing it on macOS exactly as before.
- The crates.io publish, release-please configuration, and the
  `pre-release-checks` job keep their behavior.
- A failure in the Linux build or the tap push must not undo or block the
  macOS archive or the crates.io publish of the same release.
- No secret other than `HOMEBREW_TAP_TOKEN` is added, and that token can
  write only to the tap repository.
- Nothing is pushed to the tap, and no tap repository is created, without
  the user's explicit go-ahead.
