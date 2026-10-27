# Gitview

Gitview is a keyboard-first terminal interface for reviewing Git history and
working-tree changes. Version 0.3.3 is available as a source release and an
Apple Silicon macOS archive. See [RELEASE.md](RELEASE.md) for release notes and
history.

## Install the macOS Apple Silicon release

On macOS, GAH needs GNU `find` because the system BSD `find` does not support
the `-executable` option GAH uses. Install it with Homebrew first:

```sh
brew install findutils
```

Then follow these three steps:

1. Install GAH:

   ```sh
   curl -fsSL \
     https://raw.githubusercontent.com/get-gah/gah/v1.13.0/tools/install.sh | bash
   ```

2. Install Gitview through GAH:

   ```sh
   PATH="$(brew --prefix findutils)/libexec/gnubin:$PATH" gah install barathrajd/gitview --tag=v0.3.3
   ```

3. Launch Gitview:

   ```sh
   gitview
   ```

Ensure `~/.local/bin` is on your `PATH`. GAH installs there by default for the
current user. Its current macOS script can print “Done!” after BSD `find`
rejects `-executable`, without installing Gitview, so verify that
`gitview --version` works afterward. Gitview currently publishes a binary for
macOS Apple Silicon; Linux release packaging remains deferred. You can also
download the archive from the
[GitHub releases](https://github.com/barathrajd/gitview/releases).

## Build and run

Install Rust and Git, then from the Gitview source directory run:

```sh
cargo build --release --locked
```

From any repository, run the resulting binary by its path, for example
`/path/to/gitview/target/release/gitview`. Use `gitview --help` for controls
and `gitview --ascii` for plain terminal borders. Development checks are
`cargo fmt --check`, `cargo test --locked --features test-support`, and
`cargo clippy --locked --all-targets --features test-support -- -D warnings`.
The `test-support` feature enables a test-only PTY probe for terminal
restoration and is excluded from normal builds.

Version 0.3.3 was built by the GitHub Actions release workflow on macOS Apple
Silicon. Earlier versions were tested on macOS Apple Silicon with Rust 1.99.1.
Linux x86_64 was verified for version 0.2.1 but has not been rechecked for this
release. Branch operations require `git switch`, introduced in Git 2.23; the
minimum supported Git version has not yet been verified. Windows support
remains deferred.

## Controls

- **Up/Down** or **j/k** navigate lists and scroll diffs. **Enter** or **l**
  opens a commit or file; **Esc** or **h** goes back. Search and branch-name
  prompts accept these letters as normal text.
- **Tab** switches between history and the working tree. **/** opens debounced
  history search. **Up/Down** selects a result, **Enter** opens its details,
  **Backspace** edits the query, and **Esc** returns to history.
- In the working-tree view, **s** stages the selected file, **u** unstages it,
  and **r** refreshes status.
- Press **b** to browse local branches. Select one and press **Enter**, or press
  **c** to create a branch from the current commit. Confirm with **y** or cancel
  with **n** or **Esc**.
- **?** opens help and **Esc** closes it; **q** quits. Staging and unstaging
  apply to the whole selected file, not individual hunks.

## Try a demo

From the source directory, run `scripts/create-demo.sh`. It prints the path to a
new temporary repository containing branches, a merge, a tag, and staged,
unstaged, and untracked changes. Change to that directory and run the Gitview
binary. The script leaves the demo directory for you to inspect and remove when
finished.

## Current limitations

History is limited to 200 commits. Diffs are limited to 1 MB or 5,000 lines and
indicate truncation; binary changes show Git's binary notice. Diff lines wider
than the available pane are clipped. Terminals at least 120 columns wide show
side-by-side diffs; narrower terminals show a unified diff. Commit rename
changes are displayed as separate deletion and addition entries. Unmerged
working-tree entries are not supported; resolve conflicts using Git. Branch
creation requires a first commit. Git checkout hooks run normally and can
report failure after a switch has already occurred; Gitview refreshes the
branch state and displays the error. Submodules, remote operations, commit
creation, interactive rebase, and conflict resolution are outside the MVP.
Linux builds and tests were verified for 0.2.1; Linux release packaging is not
provided.

## License

Gitview is available under the MIT License. See [LICENSE](LICENSE).

## Release automation

Release Please opens or updates a release pull request from Conventional
Commits on `main`. Merging that pull request updates the Rust package version
and changelog, creates the matching `v` tag and GitHub release, then builds and
uploads the macOS Apple Silicon archive and SHA-256 checksum.
