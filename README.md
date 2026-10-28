# Gitview

Gitview is a keyboard-first terminal interface for reviewing Git history and
working-tree changes. See [RELEASE.md](RELEASE.md) for release notes and
history.

## Install Gitview

Run the installer locally to choose between the prebuilt macOS Apple Silicon
binary and a source build through Cargo. The prebuilt option verifies the
release SHA-256 checksum and installs `gitview` into `~/.local/bin`; it doesn't
require GAH or Homebrew `findutils`.

```sh
bash ./install.sh
```

Choose **1** for the prebuilt release or **2** to build and install the latest
crate from crates.io with Cargo. The Cargo option requires Rust and builds the
executable from source, so it can take longer than downloading the prebuilt
binary.

To install the latest release, omit the tag:

```sh
bash ./install.sh
```

After the installer is published on `main`, you can run it directly with
`curl`:

```sh
curl -fsSL https://raw.githubusercontent.com/barathrajd/gitview/main/install.sh \
  | bash
```

To install directly with Cargo:

```sh
cargo install gitview --locked
```

Set `GITVIEW_INSTALL_DIR` to install somewhere other than `~/.local/bin`. The
prebuilt release currently supports macOS Apple Silicon. Linux release
packaging remains deferred. You can also download the archive from the
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

GitHub Actions builds the macOS Apple Silicon release archive. Earlier
versions were tested on macOS Apple Silicon with Rust 1.99.1.
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
  **S** stages all changes, **U** unstages all staged changes, and **r**
  refreshes status. Open a file diff with **Enter** to select hunks with **[**
  and **]**; **s/u** stage or unstage the selected hunk, and **S/U** act on the
  whole file.
- Press **c** in the working-tree view to enter a commit message, review the
  staged file list, and confirm the commit. Gitview commits only the staged
  changes; it preserves unstaged edits and Git's configured hooks and signing.
- Press **b** to browse local branches. Select one and press **Enter**, or press
  **c** to create a branch from the current commit. Confirm with **y** or cancel
  with **n** or **Esc**.
- **?** opens help and **Esc** closes it; **q** quits. Each page shows its
  available actions in the footer and help panel.

## Git config profiles

Gitview uses your normal global Git configuration by default. You can map
folder trees to separate Git config files so repositories under a client or
office folder use that account. Gitview applies the matching config to its own
Git commands; without a match, it leaves Git's normal global configuration in
place.

The profile registry is `profiles.json` under `$XDG_CONFIG_HOME/gitview`, or
`~/.config/gitview` when `XDG_CONFIG_HOME` is unset. A registry entry looks
like this:

```json
{
  "profiles": [
    {
      "name": "client",
      "worktree_root": "/Users/you/Developer/Client",
      "config_path": "profiles/client.gitconfig",
      "enabled": true
    }
  ]
}
```

Use an absolute folder path for `worktree_root`. `config_path` can be absolute
or relative to Gitview's config directory. The deepest matching folder wins;
`enabled` defaults to `true`. Keep credentials in Git credential helpers or
SSH, not in profile files.

Press **p** in Gitview to view profiles. Press **e** to open the registry in
`$VISUAL` or `$EDITOR`, then **r** to reload it. The header shows the active
profile. To make terminal `git` commands use the same folder mapping, run
`gitview --shell-integration`, review the zsh function it prints, and add it to
`~/.zshrc`. This shell integration is optional; remove the function from
`~/.zshrc` to disable it.

## Try a demo

From the source directory, run `scripts/create-demo.sh`. It prints the path to a
new temporary repository containing branches, a merge, a tag, and staged,
unstaged, and untracked changes. Change to that directory and run the Gitview
binary. The script leaves the demo directory for you to inspect and remove when
finished.

## Current limitations

Gitview currently focuses on history and working-tree review, file and hunk
staging, commit creation from a reviewed staged file set, local branch browsing
and creation, and folder-scoped Git config profiles. Commit amendment and editor
handoff, tag and remote management, restoring or moving working-tree paths,
conflict resolution, stash, rebase, and submodule workflows are not
implemented. See the [Git CLI capability matrix](docs/git-cli-capability-matrix.md)
for current coverage and planned work.

History is limited to 200 commits. Diffs are limited to 1 MB or 5,000 lines and
indicate truncation; binary changes show Git's binary notice. Diff lines wider
than the available pane are clipped. Terminals at least 120 columns wide show
side-by-side diffs; narrower terminals show a unified diff. Commit rename
changes are displayed as separate deletion and addition entries. Unmerged
working-tree entries are not supported; resolve conflicts using Git. Branch
creation requires a first commit. Git checkout hooks run normally and can
report failure after a switch has already occurred; Gitview refreshes the
branch state and displays the error. Linux builds and tests were verified for
0.2.1; Linux release packaging is not provided.

## License

Gitview is available under the MIT License. See [LICENSE](LICENSE).

## Release automation

Release Please opens or updates a release pull request from Conventional
Commits on `main`. Merging that pull request updates the Rust package version
and changelog, creates the matching `v` tag and GitHub release, then builds and
uploads the macOS Apple Silicon archive and SHA-256 checksum. The same workflow
publishes the package to crates.io and requires a repository Actions secret
named `CARGO_REGISTRY_TOKEN` containing a crates.io API token.
