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

## What's new in 0.6.0

Gitview 0.6.0, released October 29, 2026, completes six everyday local Git
workflows. Each is described in [Controls](#controls), and the
[website](https://gitview.barathrajdevarajan.dev/features/) shows a screenshot
of each one.

- **Repository setup:** `gitview init` and `gitview clone` from the command line.
- **Path operations:** ignore-rule lookup for any path, stop tracking, confirmed
  removal and moves of tracked paths, and discard of a staged rename.
- **Commit editor:** Ctrl+E edits the message in your Git editor, and saving a
  changed message commits it without a second Enter. Amend keeps its warning.
- **History:** filter by file or directory, compare two commits on one page, and
  search commits as you type.
- **Branches and tags:** rename, delete, and return to the previous branch;
  list, create (lightweight), verify, and delete local tags.
- **Inspection:** describe any commit, tag, tree, or blob and compare against
  remote refs already stored locally. Nothing is fetched.

Known issues: native memory on a very large comparison varies between runs, a
renderer-only memory diagnostic still exceeds its budget, and TypeScript, TSX,
and TOML diffs aren't syntax highlighted. The [release notes](RELEASE.md) list
the follow-up work. Linux packaging isn't provided yet, so 0.6.0 ships the crate
and the macOS Apple Silicon archive. Remotes, merge and conflict workflows, and
export remain out of scope.

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

## Initialize or clone a repository

Use the CLI commands to create a repository or clone an explicitly selected
source. Initialization defaults to the current directory; clone progress is
shown by Git. Gitview refuses to initialize inside an existing repository and
refuses to clone into a non-empty destination.

```sh
gitview init [directory]
gitview clone <repository> [directory]
```

## Run a coding agent beside Gitview

Run Claude Code, Codex, or any other program in a pane next to the Git views, so
you can watch what it changes and review it without leaving the terminal.
Gitview hosts a program you already have installed. It doesn't call model
providers or store keys.

```sh
gitview --claude
gitview --codex
gitview --run aider --model sonnet
```

Everything after `--claude`, `--codex`, or `--run` goes to that program, and the
program ends when you quit Gitview. If it isn't on your `PATH`, Gitview says so
before it changes the terminal.

Press `Ctrl+\` to move focus between Gitview and the agent; the pane's border
shows the key. While the agent has focus, every key and paste goes to it
unchanged except `Ctrl+\`, so Gitview's own keys, including **q**, reach the
agent. While Gitview has focus, nothing reaches the agent. Focus returns to
Gitview when the program ends.

On a terminal 119 columns or wider, the agent sits to the right of Gitview. On a
narrower one, Gitview and the agent take turns filling the screen, and a bar at
the bottom shows the agent's status and the focus key while Gitview is in front.

Gitview refreshes by itself when the repository changes, so you can watch an
agent work. Edits to working files, staging, commits, and branch switches appear
in the working tree, history, and branch name within about a second, without a
key press. An open working-tree diff reloads in place and keeps your scroll
position, and a history or branch selection stays on the same commit or branch.
Gitview watches the repository with the operating system's file notifications,
so an idle Gitview runs no Git commands. If your file system doesn't support
them, for example a network drive, Gitview falls back to checking the working
tree every second or so; set `GITVIEW_NO_WATCH=1` to choose that.

Gitview's own Git writes, such as staging, committing, and branch switches, run
one at a time. If the agent's Git command is holding the repository's lock when
you press a key, Gitview waits up to two seconds for it instead of failing. If
the lock is still held after that, Gitview says another Git command is using
the repository.

### Let the agent read and steer Gitview

An agent started from Gitview can read the repository through Gitview and ask
it to show things, using the Model Context Protocol (MCP). Gitview passes the
agent `GITVIEW_SOCKET`, the path of a socket in a folder only you can open
(`$XDG_RUNTIME_DIR/gitview`, or `gitview-<uid>` in your temporary folder).
Connections from other users are refused. `gitview mcp` is a stdio MCP server
that talks to that socket. Its tools are:

| Tool | What it does |
| --- | --- |
| `get_status` | Lists staged, unstaged, and untracked files. |
| `get_log` | Lists recent commits, 50 by default and at most 200. |
| `get_diff` | Shows one file's unstaged diff, or its staged diff. |
| `get_show` | Shows a commit's message, file summary, and patch. |
| `select_commit` | Opens a commit in Gitview, or one of its file diffs with `path`. |
| `select_file` | Opens a changed file's diff in Gitview's working tree. |
| `focus_pane` | Moves focus to Gitview or to the agent. |

The `get_` tools only return text to the agent; the `select_` tools and
`focus_pane` change what you see. So ask the agent to "open" or "select"
something in Gitview when you want it on screen. A selection's reply says what
Gitview will show, such as `Gitview will open commit e2d1429 (…)`, and a file
that hasn't changed is refused rather than silently ignored.

None of them stage, commit, or change files. Each reply stops at 64 KiB and
says when it was cut. Paths must be inside the repository, and a revision
can't be a Git option. If you're typing in a prompt or answering a dialog, a
selection waits until you finish, and its reply says so.

`--claude` registers the server for you. For Codex, add it to
`~/.codex/config.toml`. Codex doesn't pass environment variables to MCP
servers unless they're listed, so name the socket variable:

```toml
[mcp_servers.gitview]
command = "gitview"
args = ["mcp"]
env_vars = ["GITVIEW_SOCKET"]
```

With `--run`, register `gitview mcp` the way your agent expects. Outside
Gitview the tools reply that Gitview isn't reachable.

## Controls

- **Up/Down** or **j/k** navigate lists and scroll diffs. **Enter** or **l**
  opens a commit or file; **Esc** or **h** goes back. Search and branch-name
  prompts accept these letters as normal text.
- Press **f** while history is focused to filter commits by a literal
  repository-relative file or directory. Submit an empty path to clear the
  filter. This view does not follow renames. Search and commit details use the
  filtered history, and the filter remains active across refreshes.
- Press **D** while history is focused to compare two local commit revisions.
  The form starts with the selected commit and its first parent; edit either
  revision with **Tab** to switch fields. Enter an optional literal file or
  directory path, then press **Enter** to review all file diffs on one page.
  Base/target IDs stay visible, and each file header shows additions/deletions.
  Use **[**/**]** to select a file, **Enter/Space** to fold it, **C** to collapse
  all files, and **E** to expand all. Use **j/k** to scroll, **v** to toggle
  IDE/raw styling, and **Tab** to toggle split/unified layout. **Esc** edits the
  comparison and **H** returns to history. Branches and remote refs already
  stored locally work as revisions; no fetch is performed. A root commit has
  no parent, so choose an explicit base for that comparison.
- Press **o** while history is focused to inspect a local Git object. Enter a
  commit, tag, tree, or blob revision, such as `HEAD`, `HEAD^{tree}`, or
  `HEAD:src/main.rs`. The result shows the fixed object ID, type, size, and
  bounded preview; commits also show Git's nearest-tag description. Binary
  blobs show a notice. Use **j/k** to scroll, **e/Esc** to edit the revision,
  and **H** to return to history.
- **Tab** switches between history and the working tree. **/** opens debounced
  history search. **Up/Down** selects a result, **Enter** opens its details,
  **Backspace** edits the query, and **Esc** returns to history.
- In the working-tree view, **s** stages the selected file, **u** unstages it,
  **S** stages all changes, **U** unstages all staged changes, and **r**
  refreshes status. Press **d** to review and confirm discarding one unstaged or
  untracked file; tracked files return to their staged version, and untracked
  files are removed. On a staged rename, **d** restores the original path and
  removes the renamed path, including staged and unstaged edits to both paths.
  Open a file diff with **Enter** to select hunks with **[**
  and **]**; **s/u** stage or unstage the selected hunk, and **S/U** act on the
  whole file. Press **i** to inspect the selected path's ignore rule or **I**
  to enter any repository-relative path, including an ignored file absent from
  the worktree list. Press **x** to stop tracking the selected file while
  keeping its working copy; this stages its deletion for the next commit.
  Press **R** to enter any tracked repository-relative path, review its exact
  removal, and confirm staging its deletion; Git's normal refusal checks remain
  enabled for files with uncommitted edits.
  Press **M** to move a clean tracked file: enter source and destination paths,
  review the rename, then confirm. Git stages the rename and refuses an
  existing destination.
- Press **c** in the working-tree view to enter a commit message. The editor
  keeps the read-only worktree beside a short subject and optional description.
  Press **Tab** to switch fields and **Ctrl+J** to add a description line. Press
  **Ctrl+E** to edit the subject and description in your configured Git editor.
  Save and exit to commit the edited message without pressing **Enter** again.
  If you exit without changing it, or save an empty subject, you return to the
  form. Amending still asks for confirmation.
  Gitview uses Git's editor selection (`GIT_EDITOR`, `core.editor`, `VISUAL`,
  `EDITOR`, or Git's fallback). Executable paths and quoted arguments work;
  shell expressions are unsupported. Editor failures keep the original message.
  You can paste into either field; pasted subject line breaks become spaces, while
  description line breaks are preserved. Press **Enter** once to commit the
  staged changes; Gitview preserves unstaged edits and Git's configured hooks
  and signing.
- Press **a** in the working-tree view to amend `HEAD`. Gitview preloads the
  existing subject and description; **Enter** opens a warning that shows the
  old and replacement messages, and **y** confirms the rewrite. The reviewed
  staged changes are included, while unstaged edits remain in the worktree.
- Press **b** to browse local branches. Select one and press **Enter**, or press
  **c** to create a branch from the current commit. Confirm with **y** or cancel
  with **n** or **Esc**. Press **r** to rename the selected branch or **d**
  to review deletion. Git refuses unmerged or checked-out branch deletion;
  changed branch tips or checkout state require a new preview. Press **-** in
  the branch screen to review and return to the previous branch or detached
  commit. The destination and checkout state are rechecked before switching.
- Press **t** from branches to open local tags. Use **c** to create a
  lightweight tag at the reviewed `HEAD`, **d** to confirm local deletion, and
  **v** to verify the selected tag signature with Git. Lightweight and unsigned
  tags report verification failure. Use **j/k** to scroll verification output,
  **r** to refresh the list, and **Esc** to return. Tag creation never replaces
  an existing tag; deletion rejects a changed object. Annotated and signed tag
  creation are not available in the TUI.
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

## Try the feature playground

The feature playground creates a disposable repository and a separate Gitview
config directory. Use it to try the current workflows without changing your
own repository or Gitview profiles. Create a fresh playground whenever you want
to reset its state.

Install `cargo-make` once, then run the task from the Gitview source directory:

```sh
cargo install cargo-make
cargo make playground
```

The task invokes `scripts/run-feature-playground.sh`, which creates a temporary
repository, prints its path, and runs the `gitview` binary there with an
isolated config directory. You can also run that launcher directly. The sample
repository includes:

- A main branch, a topic branch, a merge commit, and a tag for history and graph
  browsing, search, commit details, and branch switching.
- A clean working-tree file, a staged file, an unstaged file, and an untracked
  file for status navigation, staging, unstaging, and discard confirmation.
- A file with one staged hunk and a separate unstaged hunk for reviewing diffs
  and staging or unstaging selected hunks.
- An isolated `Playground` Git config profile for the profiles view and profile
  matching. The configuration lives under the temporary directory.

In the TUI, use **Tab** to switch views, **/** to search history, **b** to
browse, switch, or create branches, **p** to inspect profiles, and **?** to open
the in-app help.
In the working tree, use **s/u** on files or selected hunks, **S/U** for all
files or the selected file, **d** to review a discard, **c** to create a commit,
and **a** to amend the latest commit. The amend flow asks you to review the
replacement before confirming. Use **Esc** to cancel a prompt or action.

Actions such as discard, commit, amend, and branch switching change only the
temporary repository. The script leaves it in place after Gitview exits; remove
the printed temporary directory when you no longer need it. Create a fresh
playground to try commit and amend independently because a commit consumes the
staged changes, and amend rewrites the temporary `HEAD`. The simpler
`scripts/create-demo.sh` remains available for a small repository without the
isolated profile and split-hunk fixture.

## Performance checks

The 0.6.0 performance review is complete with open findings. See the
[measured results and remaining checks](docs/performance-0-6-0.md) for native
results, fixture details, and reproducible commands. Read-only preparation
screens accept **Esc** to cancel their result while Git finishes in the
background. Confirmed Git operations retain their normal completion handling.

## Current limitations

Gitview currently focuses on history and working-tree review, file and hunk
staging, confirmed discard of selected unstaged and untracked files and staged
renames, tracked path removal and moves, commit creation from a reviewed staged
file set, local branch browsing and creation, and folder-scoped Git config
profiles. Remote management, annotated and signed tag creation, conflict resolution,
stash, rebase, and submodule workflows are not implemented. See the
[Git CLI capability matrix](docs/git-cli-capability-matrix.md) for current
coverage and planned work.

The agent pane is a first version, checked on macOS with `claude` and `codex`.
It doesn't forward mouse input, and it doesn't answer the questions programs ask
their terminal, such as the cursor position or the theme colors; the programs
start without the answers. Shift+Enter is sent as Alt+Enter, which agents read
as a new line. The control channel was checked with Claude Code and with Codex
0.165.0 on macOS; on Linux only its automated tests have run.

History is limited to 200 commits. Diffs are limited to 1 MB or 5,000 lines and
indicate truncation; binary changes show Git's binary notice. IDE diffs show
syntax-colored Before and After panes by default, with long lines wrapped and
aligned. Press **v** to switch between IDE and raw diff styling, and **Tab** to
switch between side-by-side and unified layouts. Both layouts are available in
either style. Commit rename changes are displayed as separate deletion and
addition entries. Unmerged working-tree entries are not supported; resolve
conflicts using Git. Branch creation requires a first commit. Git checkout
hooks run normally and can report failure after a switch has already occurred;
Gitview refreshes the branch state and displays the error. Linux builds and
tests were verified for 0.2.1; Linux release packaging is not provided.

## License

Gitview is available under the MIT License. See [LICENSE](LICENSE).

## Release automation

Release Please opens or updates a release pull request from Conventional
Commits on `main`. Merging that pull request updates the Rust package version
and changelog, creates the matching `v` tag and GitHub release, then builds and
uploads the macOS Apple Silicon archive and SHA-256 checksum. The same workflow
publishes the package to crates.io and requires a repository Actions secret
named `CARGO_REGISTRY_TOKEN` containing a crates.io API token.
