# Agent Workspace

## Objective

Let developers run a coding agent such as Claude Code or Codex inside Gitview,
next to the history, diff, and staging views, so they can watch what the agent
changes and review it without leaving the terminal. The agent can read and
steer the same Gitview state through a local control channel, and the user can
still use every Gitview key by hand. Later phases give each agent session its
own worktree, in the style of Conductor-like agent apps.

This is not the AI chat deferred in the [1.0.0 plan](gitview-1-0-0.md). Gitview
hosts agents the user already has installed; it does not call model providers,
store API keys, or manage subscriptions.

## Success criteria

- `gitview --run <command>` starts the command in a pseudo-terminal pane beside
  the Git views. `--claude` and `--codex` are shortcuts for those commands and
  report a clear message when the executable is not installed.
- One key moves focus between the agent pane and Gitview. While the agent pane
  has focus, every key except the focus key reaches the agent unchanged; while
  Gitview has focus, no key reaches the agent.
- Claude Code and Codex are usable in the pane: colors, the alternate screen,
  bracketed paste, resize, and clean exit all work. Exiting Gitview restores the
  terminal even if the agent crashes or is killed.
- Gitview history, status, and diff views refresh within one second of a file
  change made by the agent, without being asked, and without blocking input.
- A local control channel lets the agent read status, log, and diffs and ask
  Gitview to select a commit or file. Only the owning user can connect to it.
- User and agent Git writes go through one serialized queue, so a stage action
  and an agent action cannot race on the index.
- In the worktree phase, each session runs in its own worktree and branch, and
  the user can review, merge, or discard it from Gitview with confirmation.
- Existing review, staging, branch, and config-profile workflows keep passing
  their checks. Layouts remain usable on narrow terminals.

## In scope / Out of scope

**In scope**

- A PTY-backed terminal pane with emulation, scrollback with a fixed cap, focus
  switching, resize, and an actionable error when the command cannot start.
- File-system watching that refreshes Git views.
- A per-user Unix socket control channel and a `gitview mcp` stdio server that
  exposes read-mostly tools: `get_status`, `get_log`, `get_diff`, `get_show`,
  `select_commit`, `select_file`, `focus_pane`.
- Per-session worktrees with a session list, diff review, merge, and discard.
- macOS verification first; Linux follows the Linux release work.

**Out of scope**

- Model-provider calls, API keys, subscriptions, or Gitview's own AI chat.
- Letting the agent stage, commit, or push through the control channel in the
  first release. Mutating tools need a separate design with user confirmation.
- Remote or networked control, multi-user sessions, and Windows support.
- Agent-specific features beyond running the command, such as parsing agent
  output or injecting prompts.

## Approach

Spawn the agent with `portable-pty` on a worker thread and feed its output to a
terminal emulator (`vt100` first; `alacritty_terminal` if fidelity tests fail).
Render the emulator's screen as a ratatui widget and forward keys, paste, and
resize events to the PTY while it has focus. The focus key must be one the
agents do not use, and it is configurable. Use the existing rule that
subprocess work stays off the render loop, and expose starting, running, and
exited states in the interface.

Watch the working tree with `notify`, debounce events, and ignore `.git`
internals except `HEAD`, refs, and the index. Reuse the existing refresh and
stale-result protection so a burst of agent edits cannot overwrite newer state.

Serve the control channel on a Unix socket in a user-private directory with
`0600` permissions, and pass its path to the agent as `GITVIEW_SOCKET`. The
`gitview mcp` subcommand is a small stdio MCP server that forwards tool calls
to that socket and reuses the Git adapter's parsers and limits. Tool output is
size-capped and read-only. Register it for Claude with `--mcp-config`; document
the Codex equivalent after checking it against the installed version.

For sessions, create `git worktree add` checkouts under a Gitview-managed
directory with a generated branch name, run each agent there, and show the
session's diff against its base. Merge and discard use the existing
confirmation patterns and refuse when the worktree is dirty or refs changed
since review.

## Milestones

- [x] **1. Spike the pane:** Run an arbitrary command in a PTY pane with focus
  switching, resize, scrollback cap, and terminal restoration. Test with a
  shell, `claude`, and `codex`; record which escape sequences or keys fail and
  decide whether `vt100` is enough. Verify in a real pseudo-terminal.
  *Done:* `gitview --run <command> [args...]` starts the command in a pane
  beside the Git views (`src/pane/`, `ui::render_agent_pane`, `app::agent`).
  `Ctrl+\` moves focus (both its kitty form and the legacy `Ctrl+4` that
  plain terminals send); while the pane has focus every key and paste goes to
  the program unchanged except that key, otherwise nothing does. The router,
  the loop, and the real binary in a pseudo-terminal are tested: quit, a failing
  or externally killed agent, closing the terminal (the agent goes too), and the
  focus key and arrows crossing a real terminal byte for byte. Dropping a pane
  kills its program, and pane setup is serialized because macOS `openpty`
  fails intermittently under concurrent calls (5 of 8 stress runs). Narrow
  terminals hide the pane and keep input in Gitview; the full-screen toggle is
  milestone 2.

  *Fidelity findings* (`tests/pane_fidelity.rs`; run the probes with
  `--ignored --nocapture`; `claude` and `codex` as installed on 2026-10-30):

  | Check | `claude` | `codex` |
  | --- | --- | --- |
  | First text on screen | 188 ms | 318 ms |
  | Screen drawn correctly | trust dialog, box lines, `❯` | update dialog, alternate screen |
  | Styled cells | 254 (187 truecolor) | 144 (bold and reverse) |
  | Bracketed paste | on, forwarded | on, forwarded |
  | Down key moves the highlight | yes | yes |
  | Resize makes it redraw | yes | yes |
  | Ctrl+C twice exits | yes (code 1) | yes (code 0) |

  **Decision: `vt100` is enough for rendering; keep it.** Every sequence it
  didn't understand is non-visual. No fallback to `alacritty_terminal` is
  needed. The gaps are about input and replies, not drawing, and neither agent
  stalled on them. Follow-ups, in the milestones that touch them:

  - *Queries nothing answers.* `claude` asks for device attributes (`CSI c`),
    kitty keyboard flags (`CSI ? u`), and the terminal version (`CSI > q`);
    `codex` asks for the cursor position (`CSI 6 n`), device attributes,
    kitty flags, and the foreground and background colors (OSC 10 and 11).
    Both start fine without replies. Answering cursor position and device
    attributes is cheap; colors need the host terminal's real answer.
    Not answering kitty flags keeps agents on legacy keys, which `encode_key`
    produces. Revisit in milestone 7.
  - *Mouse.* `codex` turns on any-motion SGR mouse tracking, but Gitview
    doesn't capture the mouse, so the wheel does nothing in the pane.
  - *Focus reporting* (`CSI ? 1004 h`) is requested but not sent.
  - *Synchronized output* (`CSI ? 2026 h/l`; `codex` used it 42 times in one
    probe run) is ignored, so a half-drawn frame can show briefly.
  - *Hyperlinks and notifications* (OSC 8 and OSC 9) are dropped.
  - *Shift+Enter* has no code of its own without the kitty protocol, so it is
    sent as Alt+Enter, which agents read as a newline. Not yet checked inside
    a live prompt, since the probes send no prompt.
  - A plain `sh` also asks for 8-bit meta mode (`CSI ? 1034 h`); harmless.
- [x] **2. Wire the CLI and layout:** Add `--run`, `--claude`, and `--codex`,
  missing-executable errors, a narrow-terminal layout (agent full screen with a
  toggle), and help text. Test exit, crash, and kill paths.
  *Done:* `--claude` and `--codex` are shortcuts for those commands, and like
  `--run` they pass everything after the flag to the program. The program is
  looked up on `PATH` before Gitview touches the terminal; a missing shortcut
  names the product and suggests `gitview --run <command>`, and an empty `PATH`
  entry is skipped so a repository can't make Gitview run a program it ships.
  Side by side starts at 119 columns (Gitview keeps 60). On a narrower terminal
  the focus key swaps Gitview and the agent, each full screen, and a one-line
  bar under Gitview shows the agent's status and the key; the bar drops the
  hint once the agent has ended, and a terminal under 3 rows shows Gitview
  alone. The agent is told its size whenever it is brought forward, never fewer
  than one row. `--help` and the README describe all of it. The exit, crash,
  and kill paths run against the real binary in a pseudo-terminal, including a
  live resize between the layouts.
- [x] **3. Refresh on change:** Add the debounced watcher and stale-result
  handling. Test rapid edits, large trees, and ignored paths; check CPU use
  while idle.
  *Done:* `watch::RepoWatcher` (the `notify` crate) reports working-file,
  index, and ref changes. Only `HEAD`, `index`, `packed-refs`, and `refs/**`
  count under `.git`, lock files and reads never do, and a linked worktree
  watches its own files plus the shared refs. A burst settles after 120 ms of
  quiet, and no change is held back more than 400 ms. The watcher starts in the
  background (a fixed 0.3 s or more, whatever the tree size) and a full refresh
  follows when it is ready; if it can't start, or `GITVIEW_NO_WATCH` is set, the
  old polling runs. History, branches, the working tree, and an open diff
  reload on their own. Each view has a reload slot: a burst becomes one more
  reload, a slot waits as long as its last reload took (at least 150 ms), and
  nothing starts during the user's own Git operation. Requests reuse the loop's
  ids, so a late result can't overwrite a newer one. Selections follow the same
  commit or branch, an open diff keeps its scroll and hunk and follows its file
  between the staged and unstaged lists. One related fix: every Git command now
  sets `GIT_OPTIONAL_LOCKS=0`, because a background `git status` rewrote the
  index (taking `index.lock`, which can make an agent's `git add` fail) and a
  watcher would have seen that as a change.

  *Measured* (release build, 200-file repository, `git` children included):
  idle CPU fell from **2.08%** of a core to **0.08%**, and idle `git status`
  runs from 7 in 5 s to 0. An edit appears in under 1.5 s end to end and costs 1
  to 3 `git status` runs. With the polling fallback the idle cost is 2.73%.
  Not measured: very large repositories, and Linux (inotify watch limits are the
  likely failure there, which is what the fallback is for).
- [x] **4. Control channel and MCP:** Add the socket, permissions, and the
  read-only tools with size limits. Test with a real Claude session and with a
  scripted client, including unauthorized and malformed requests.
  *Decided (2026-10-30):* a private folder plus a peer user ID check, with no
  token; 64 KiB per reply and at most 200 commits; `--claude` registers the
  server itself; a hand-written JSON-RPC server instead of the `rmcp` crate.
  *Done:* `control::server::ControlServer` listens in `$XDG_RUNTIME_DIR/gitview`
  or `<temp>/gitview-<uid>`. The folder must be ours and mode 0700 (a shared
  one is refused), the socket is 0600, a peer with another user ID is dropped
  (`getpeereid`, `SO_PEERCRED` on Linux), and each server's socket name is
  unique. Requests are JSON lines of at most 64 KiB; a longer one closes the
  connection, and a malformed one gets an error reply on a connection that
  stays open. `control::tools` implements `get_status`, `get_log`, `get_diff`,
  and `get_show`: paths must be repository-relative, a revision can't start
  with `-` and is resolved with `rev-parse --end-of-options`, and an empty
  repository has an empty log. `gitview mcp` (`control::mcp`) answers
  `initialize`, `ping`, `tools/list`, and `tools/call`, and marks tool errors
  with `isError`. In the app, `select_commit` opens the commit's details,
  `select_file` opens the diff, and `focus_pane` moves focus; a request waits
  while the user is in a prompt or dialog, and is dropped if its target hasn't
  loaded within 3 s. `--claude` adds `--mcp-config=<json>` (the `=` form, so the
  variadic option can't swallow a prompt).
  *Checked live (macOS):* `claude -p` in a Gitview pane called `get_log` and
  got the branch's commits, and `get_diff` with `../../etc/passwd` was refused.
  Codex 0.165.0 strips the environment of MCP servers: without
  `env_vars = ["GITVIEW_SOCKET"]` the tool says Gitview isn't reachable, and
  with it `get_log` works. The README documents that.
  *Not tested:* a connection from another user ID (needs a second account or
  root; the check is in `accept_loop`), Linux, and a long interactive session.
  If the socket can't be created, the agent runs without `GITVIEW_SOCKET` and
  Gitview says nothing about it; revisit in milestone 7.
- [x] **5. Serialize writes:** Route user and control-channel Git mutations
  through one queue. Test concurrent stage and unstage from both sides.
  *Decided (2026-10-30):* the control channel has no write tools (see Out of
  scope), so "both sides" means Gitview and the agent's own Git commands in its
  terminal. Gitview can't queue those; it can run its writes one at a time and
  wait out the agent's locks.
  *Done:* `writes::serialized` runs one write at a time under a process-wide
  lock and retries while Git reports a held lock file (`….lock': File exists`),
  for up to 2 s with backoff, then says another Git command is using the
  repository. Git takes its lock before changing anything, so a refused write
  changed nothing. A multi-step write rechecks its preview, so a retry after a
  partial change fails safely. All 21 write calls in the app go through it:
  stage, unstage, hunks, stage and unstage all, discard, untrack, remove, move,
  commit, amend, the five branch actions, and tag create and delete. Gitview's
  reads already set `GIT_OPTIONAL_LOCKS=0`, so they never block the agent.
  *Tested:* overlapping writes never run together; a write waits out an
  `index.lock` released after 300 ms; a lock that stays is reported after the
  wait; other errors aren't retried. While a second process loops `git add`, 20
  rounds of stage and unstage all succeed and the index ends as Gitview left it.
  While it loops `git add` and `git reset <path>`, no Gitview write fails on the
  lock. With the retry turned off, 3 or 4 of these tests fail on every run. In
  the app, `S` succeeds while a lock is held for 300 ms.
  *Found:* Git itself can lose an index change when another process runs
  `git reset <path>` at the same moment. Reproduced with plain Git, no Gitview
  code: 2 of 113 adds were undone by a looping `git reset -q`. Gitview can't
  prevent that from outside. Its writes recheck status first, so it reports
  "File status changed; refresh and try again" rather than acting on stale
  state, and the watcher then shows the real index. Agent write tools, and a
  queue they would share, wait for their own design.
- [ ] **6. Worktree sessions:** Add create, list, review, merge, and discard for
  per-session worktrees. Test dirty trees, changed refs, and cleanup after a
  crash.
- [ ] **7. Harden and document:** Memory and scrollback budgets, ASCII mode,
  security review of the socket, controls and limitations in the docs, and the
  full format, test, lint, and release-build pass.

## Risks & open questions

- **Terminal fidelity:** Agent TUIs use the alternate screen, truecolor, mouse,
  bracketed paste, and possibly the kitty keyboard protocol. Fallback: switch
  emulator crates, or document unsupported sequences. Milestone 1 decided: keep
  `vt100` (see its findings for the remaining gaps).
- **Key conflicts:** The focus key may clash with an agent or a terminal
  multiplexer. Make it configurable and show it in the status line.
- **Memory:** Earlier performance work found native diff memory close to its
  budget. Cap agent scrollback and measure the combined footprint.
- **Control-channel safety:** The agent is untrusted input. Keep tools read-only
  at first, cap output, and avoid exposing paths outside the repository.
- **Codex integration:** The way Codex accepts MCP servers must be verified
  against the installed version before documenting it.
- **Scope and sequencing:** The 0.6.0 follow-ups and the Linux release come
  first. Decide later whether this ships as a 0.x feature or part of 1.0.
- **Worktree cleanup:** Crashes can leave worktrees and branches behind. Track
  sessions in a manifest and offer a repair action.
