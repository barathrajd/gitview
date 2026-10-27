# Gitview

Gitview is a keyboard-first terminal interface for reviewing Git history and working-tree changes. This is an **MVP in development**, not a published release.

## Build and run

Install Rust and Git, then from the Gitview source directory run:

```sh
cargo build --release --locked
```

From any repository, run the resulting binary by its path, for example `/path/to/gitview/target/release/gitview`. Use `gitview --help` for controls. Development checks are `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --all-targets -- -D warnings`.

The current build and terminal checks have been performed on macOS with Rust 1.99.1 and Git 2.54. Linux and other Rust versions have not yet been verified.

## Controls

- **Up/Down** navigate lists or scroll a diff; **Enter** opens a commit or file; **Esc** goes back.
- **Tab** switches between history and the working tree. **/** searches history by message, author, or hash; **n** finds the next match.
- In the working-tree view, **s** stages the selected file, **u** unstages it, and **r** refreshes status.
- **?** opens help; **q** quits. Staging and unstaging apply to the whole selected file, not individual hunks.

## Try a demo

From the source directory, run `scripts/create-demo.sh`. It prints the path to a new temporary repository containing branches, a merge, a tag, and staged, unstaged, and untracked changes. Change to that directory and run the Gitview binary. The script leaves the demo directory for you to inspect and remove when finished.

## Current limitations

History is limited to 200 commits. Diffs are limited to 1 MB or 5,000 lines and indicate truncation; binary changes show Git's binary notice. Diff lines wider than the available pane are clipped. Terminals at least 120 columns wide show side-by-side diffs; narrower terminals show a unified diff. Commit rename changes are displayed as separate deletion and addition entries. Unmerged working-tree entries are not supported; resolve conflicts using Git. Submodules, remote operations, commit creation, interactive rebase, and conflict resolution are outside the MVP. Linux packaging, terminal compatibility, and release artifacts remain to be verified.
