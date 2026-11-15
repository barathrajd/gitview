<p align="center">
  <img src="website/public/logo.svg" width="96" height="96"
    alt="Gitview logo: a Git branch with a terminal chevron" />
</p>

# Gitview

[Website](https://gitview.barathrajdevarajan.dev/) ·
[Features](https://gitview.barathrajdevarajan.dev/features/) ·
[Changelog](https://gitview.barathrajdevarajan.dev/changelog/) ·
[Install](https://gitview.barathrajdevarajan.dev/#install)

Gitview is a keyboard-first terminal interface for reviewing Git history and
working-tree changes. See [RELEASE.md](RELEASE.md) for release notes and
history.

## Install Gitview

Gitview runs on macOS with Apple Silicon and on Linux x86_64. Each release
ships a prebuilt binary for both, and the Linux one is statically linked, so it
runs on any distribution, Alpine included.

### Homebrew

```sh
brew install barathrajd/tap/gitview
```

This uses the [barathrajd/tap](https://github.com/barathrajd/homebrew-tap) tap,
which each release updates. Upgrade with `brew upgrade gitview`. If an older
`gitview` from the installer below (`~/.local/bin`) or from Cargo
(`~/.cargo/bin`) comes first on your `PATH`, remove it so the Homebrew one
runs.

### Installer script

```sh
curl -fsSL https://raw.githubusercontent.com/barathrajd/gitview/main/install.sh \
  | bash
```

The installer picks the archive for your platform, verifies its SHA-256
checksum, and installs `gitview` into `~/.local/bin`. Set
`GITVIEW_INSTALL_DIR` to install somewhere else. It needs Bash and `curl`;
Alpine doesn't include Bash by default (`apk add bash curl`). Run it from a
clone with `bash ./install.sh` to choose between the prebuilt binary and
Cargo, or pass a tag such as `bash ./install.sh v0.7.0 --prebuilt`.

### Cargo

```sh
cargo install gitview --locked
```

This builds Gitview from crates.io and works wherever Rust does. You can also
download the archives yourself from the
[GitHub releases](https://github.com/barathrajd/gitview/releases); each comes
with a `.sha256` checksum. Windows isn't supported.

## What's new in 0.8.0

Gitview 0.8.0 turns the separate pages into one keyboard-first workspace. You
see your sessions, branches, and changed files beside history, and pages open
in the centre without hiding them. [Controls](#controls) lists every key, and
the [release notes](RELEASE.md) cover the details.

- **Workspace:** the sidebar stacks **Sessions**, **Branches**, and the
  **Worktree**. **Tab** moves down the sidebar and then to history, and
  **Shift+Tab** moves up. Commit details, diffs, tags, profiles, object
  inspection, comparison, and branch and session actions open in the centre
  panel.
- **Structured history:** Gitview draws the commit graph with colored lanes and
  aligned columns for the hash, branch and tag labels, subject, author, and
  age. History loads more commits as you scroll, with no 200-commit limit.
  Press **v** for Git's own `git log --graph` text.
- **Search:** **/** searches the focused list in place: history, changed files,
  a commit's files, branches, tags, and sessions. Matches are highlighted, and
  **n**/**N** move between them. History search also accepts `author:`,
  `path:`, `msg:`, `since:`, `until:`, and `ref:` filters, which search all of
  history.
- **Vim motions:** **gg**, **G**, counts such as **5j**, **Ctrl+D**/**Ctrl+U**,
  **Ctrl+F**/**Ctrl+B**, and **zz** work in every list and diff.
- **Agent pane:** mouse and wheel input reach the agent, and **Ctrl+W O**
  toggles any pane full width.

One bottom bar lists the keys for the focused panel. When they don't all fit,
it ends with **? more**, and **?** shows the full list.

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
shows the key. Press **Ctrl+w**, then **o**, to toggle the focused pane
between full width and the workspace split. This works directly in the agent
pane too. **Ctrl+w +** grows the focused pane, **Ctrl+w -** shrinks it, and
**Ctrl+w x** hides it. Hiding the agent keeps its program running; press
`Ctrl+\` to reopen it, or **Ctrl+w =** to restore all panes. These workspace
chords stay with Gitview; other keys and paste reach the focused agent, so
**q** still reaches it. Other Ctrl+w sequences are forwarded after the next
key. While Gitview has focus, nothing reaches the agent. Focus returns to
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
the repository. Git itself can undo a change to the staging area when another
command, such as `git reset <file>`, runs at the same moment. Gitview can't stop
that, but it notices, asks you to try again, and shows the real state.

### Give the agent its own worktree

Add `--session` to run the agent in a new worktree on its own branch, so its
changes stay apart from yours until you choose to merge them:

```sh
gitview --session --claude
```

Gitview creates the branch `gitview/session-<date>-<time>` from the branch
you're on, checks it out under `.git/gitview/sessions/`, and runs both the
agent and Gitview there. The session stays after you quit. Gitview needs a
checked-out branch to start one, since that's where the work merges back.

Press **A** to focus the **Sessions** section at the top of the sidebar. Each
session shows a short state: how many commits it has on top of its base branch
(`2 ahead`), `clean`, or `+ edits` when it has uncommitted work. Reviews and
confirmations open in the centre panel.

- **Enter** reviews the session's commits and diff.
- **m** then **y** merges it into the base branch with a merge commit, in your
  main worktree. Gitview refuses if either branch moved since the review, if
  the main worktree or the session has uncommitted changes, or if the base
  branch isn't checked out. A conflict stops the merge and changes nothing.
- **d** shows the commits and files that would be lost, then **y** deletes the
  worktree and its branch. You can't discard the session Gitview is running
  in.

Sessions are recorded in `.git/gitview/sessions/manifest.json`. If a crash
leaves a session's folder or branch missing, or a worktree the manifest never
recorded, the page says so and **d** cleans it up.

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

Gitview is one workspace. The sidebar on the left stacks **Sessions**,
**Branches**, and the **Worktree**, and history fills the centre. Below 119
columns, or beside an agent, only the focused panel shows.

- **Tab** moves focus down the sidebar (Sessions, Branches, Worktree) and then
  to history; **Shift+Tab** moves up. **A**, **b**, **W**, and **H** jump to
  Sessions, Branches, the Worktree, and history. **Esc** in a sidebar section
  returns to history. Pages such as commit details, diffs, tags, and
  comparison open in the centre panel, and **Esc** closes them.
- One bottom bar lists the keys for the focused panel or open page. When they
  don't all fit, it ends with **? more**; press **?** for the full list.
- **gg** and **G** jump to the first and last row, a count repeats a motion
  (**5j**), **Ctrl+D**/**Ctrl+U** move half a page, **Ctrl+F**/**Ctrl+B** a
  full page, and **zz** centers the selection. These work in every list and
  diff.
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
- History draws its own commit graph: colored lanes, then the short hash,
  branch and tag labels, subject, author, and age (`2h`, `3d`, `5w`). Narrow
  panels drop the age, then the author. More commits load as you scroll. Press
  **v** to switch to Git's `git log --graph` text, which shows at most 200
  commits, and **v** again to switch back; the selected commit stays selected.
- **/** searches the focused list in place: history, changed files, a commit's
  files, branches, tags, or sessions. The selection jumps to the best match as
  you type, and matched letters are highlighted. **Enter** keeps the match,
  **Esc** returns to where you started, and **n**/**N** step through matches.
  In history, **Tab** opens the full ranked results page, and the filters
  `author:`, `path:`, `msg:`, `since:`, `until:`, and `ref:` search all of
  history with Git (up to 1,000 commits). Quote values with spaces, such as
  `author:"Alice Smith"`.
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
- Press **b** to focus **Branches** in the sidebar. Select one and press
  **Enter**, or press **c** to create a branch from the current commit.
  Prompts and confirmations open in the centre panel; confirm with **y** or
  cancel with **n** or **Esc**. Press **r** to rename the selected branch or
  **d** to review deletion. Git refuses unmerged or checked-out branch
  deletion; changed branch tips or checkout state require a new preview. Press
  **-** in Branches to review and return to the previous branch or detached
  commit. The destination and checkout state are rechecked before switching.
- Press **t** from branches to open local tags. Use **c** to create a
  lightweight tag at the reviewed `HEAD`, **d** to confirm local deletion, and
  **v** to verify the selected tag signature with Git. Lightweight and unsigned
  tags report verification failure. Use **j/k** to scroll verification output,
  **r** to refresh the list, and **Esc** to return. Tag creation never replaces
  an existing tag; deletion rejects a changed object. Annotated and signed tag
  creation are not available in the TUI.
- **?** opens help for the focused panel and **Esc** closes it; **q** quits.

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

Run these from the Gitview source directory:

```sh
make playground          # build the optimized binary and open a new playground
make playground-large    # the same, plus 40 untracked files for Worktree scrolling
make playground ARGS="--run sh"   # pass Gitview arguments
make playground-clean    # delete every playground
```

The targets invoke `scripts/run-feature-playground.sh`, which creates a
repository under `target/playground/`, prints its path, and runs the optimized
`gitview` binary there with an isolated config directory. You can also run that
launcher directly; it then creates the playground in your temporary directory
and uses `cargo run`. The sample repository includes:

- A main branch, a topic branch, a merge commit, and a tag for history and graph
  browsing, search, commit details, and branch switching.
- Forty small commits, one commit that changes thirty files, and thirty
  `topic-NN` branches, for list motions such as counts, page keys, `gg`/`G`,
  and `zz`.
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

See [Agent terminal compatibility](docs/agent-terminal-compatibility.md) for
normal-terminal differences, the 0.8.0 development fixes, and remaining limits.

The agent pane is a first version, checked on macOS with `claude` and `codex`.
It keeps the last 2,000 lines of the agent's scrollback. It answers a program's
questions about the cursor position, terminal type, and the embedded pane's
dark background color. Other queries, such as foreground colors and keyboard
modes, remain unanswered.

With the agent focused, wheel events browse ordinary terminal scrollback.
Typing returns to the live screen. For alternate-screen programs without
mouse reporting, the wheel sends Page Up/Down instead of moving the input
cursor with arrow keys. Programs that enable mouse reporting receive clicks,
wheel events, and requested motion reports with pane-relative coordinates.
Mouse capture turns off when focus returns to Gitview. Terminal focus changes,
hyperlinks, and notifications aren't forwarded. Drawing doesn't wait for a
program's synchronized updates, so a half-drawn frame can show briefly.
Shift+Enter is sent as Alt+Enter, which agents read as
a new line. The control channel was checked with Claude Code and with Codex
0.165.0 on macOS; on Linux only its automated tests have run. See the
[security review](docs/agent-workspace-security.md) for what the control
channel allows and what it guards against.

The raw `git log --graph` view (**v**) shows at most 200 commits; the
structured history has no limit. Diffs are limited to 1 MB or 5,000 lines and
indicate truncation; binary changes show Git's binary notice. IDE diffs show
syntax-colored Before and After panes by default, with long lines wrapped and
aligned. Press **v** to switch between IDE and raw diff styling, and **Tab** to
switch between side-by-side and unified layouts. Both layouts are available in
either style. Commit rename changes are displayed as separate deletion and
addition entries. Unmerged working-tree entries are not supported; resolve
conflicts using Git. Branch creation requires a first commit. Git checkout
hooks run normally and can report failure after a switch has already occurred;
Gitview refreshes the branch state and displays the error. Releases ship
prebuilt binaries for macOS Apple Silicon and Linux x86_64 only; other systems
can build with Cargo, and Windows isn't supported.

## License

Gitview is available under the MIT License. See [LICENSE](LICENSE).

## Release automation

Release Please opens or updates a release pull request from Conventional
Commits on `main`. Merging that pull request updates the Rust package version
and changelog, creates the matching `v` tag and GitHub release, then builds and
uploads the macOS Apple Silicon archive and SHA-256 checksum. The same workflow
publishes the package to crates.io and requires a repository Actions secret
named `CARGO_REGISTRY_TOKEN` containing a crates.io API token.
