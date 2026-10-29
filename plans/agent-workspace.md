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

- [ ] **1. Spike the pane:** Run an arbitrary command in a PTY pane with focus
  switching, resize, scrollback cap, and terminal restoration. Test with a
  shell, `claude`, and `codex`; record which escape sequences or keys fail and
  decide whether `vt100` is enough. Verify in a real pseudo-terminal.
  *Progress:* PTY core (`src/pane.rs`: spawn, input, resize, scrollback cap,
  exit state, spawn errors) done. Remaining: ratatui pane widget, focus key,
  terminal-restoration check, and the `claude`/`codex` fidelity findings.
- [ ] **2. Wire the CLI and layout:** Add `--run`, `--claude`, and `--codex`,
  missing-executable errors, a narrow-terminal layout (agent full screen with a
  toggle), and help text. Test exit, crash, and kill paths.
- [ ] **3. Refresh on change:** Add the debounced watcher and stale-result
  handling. Test rapid edits, large trees, and ignored paths; check CPU use
  while idle.
- [ ] **4. Control channel and MCP:** Add the socket, permissions, and the
  read-only tools with size limits. Test with a real Claude session and with a
  scripted client, including unauthorized and malformed requests.
- [ ] **5. Serialize writes:** Route user and control-channel Git mutations
  through one queue. Test concurrent stage and unstage from both sides.
- [ ] **6. Worktree sessions:** Add create, list, review, merge, and discard for
  per-session worktrees. Test dirty trees, changed refs, and cleanup after a
  crash.
- [ ] **7. Harden and document:** Memory and scrollback budgets, ASCII mode,
  security review of the socket, controls and limitations in the docs, and the
  full format, test, lint, and release-build pass.

## Risks & open questions

- **Terminal fidelity:** Agent TUIs use the alternate screen, truecolor, mouse,
  bracketed paste, and possibly the kitty keyboard protocol. Fallback: switch
  emulator crates, or document unsupported sequences. Milestone 1 decides this.
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
