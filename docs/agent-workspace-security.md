# Agent workspace security review

This review covers the parts of Gitview that a hosted coding agent can reach:
the control socket, the `gitview mcp` server, the tools behind them, and
worktree sessions. It was done on 2026-10-30 for milestone 7 of the
[agent workspace plan](../plans/agent-workspace.md).

## Threat model

The agent runs as the same user as Gitview, so it can already do anything
that user can do, including running `git` itself. The control channel must
not give it more than that, and must not let anyone else in. The concerns are:

- Another local user reaching the socket.
- A confused or hostile agent using the tools to read outside the
  repository, run Git with options it chose, or change the repository.
- A client exhausting Gitview's threads or memory.
- Crashes leaving sockets, worktrees, or branches behind.

Agents with the user's own rights, such as malware already running as that
user, are out of scope. A local socket can't defend against them.

## Findings and status

| Area | Finding | Status |
| --- | --- | --- |
| Socket access | The socket lives in a folder created with mode `0700`. An existing folder must be a directory (not a symlink) owned by the user, with no group or other access, or Gitview refuses to start the socket. The socket itself is `0600`. Each connection's peer user ID is checked (`getpeereid`, or `SO_PEERCRED` on Linux), and other users are dropped. | Done before this review |
| Request size | A request line is capped at 64 KiB; a longer one gets an error and the connection closes. | Done before this review |
| Tool arguments | Paths must be repository-relative, with no `..` or root components. A revision can't start with `-` and is resolved with `rev-parse --end-of-options` to a commit before use. Arguments of the wrong type are errors. | Done before this review |
| Tool output | Each reply is capped at 64 KiB and marks truncation. `get_log` returns at most 200 commits. | Done before this review |
| Writes | No tool stages, commits, or changes files. Selections only move what Gitview shows, and wait while the user is in a prompt. | Done before this review |
| Connection count | Each connection had its own thread with no limit. | Fixed: at most 8 at once; more get an error reply. |
| Idle connections | A connection that never sent anything held its thread forever. | Fixed: closed after 10 minutes without a request. |
| Stale sockets | A killed Gitview left its socket file behind. | Fixed: at start, sockets whose process no longer exists are removed. |
| Silent failure | When the socket couldn't start, the agent ran without it and nothing said so. | Fixed: the agent pane's title ends with "tools off". |
| Session manifest | Two Gitviews creating sessions at the same moment could pick the same name, or save over each other's manifest. | Fixed: `manifest.lock` covers naming, the worktree, and the save. |
| Session discard | Discarding the session Gitview runs in would delete its own folder and the agent's. | Fixed in milestone 6: refused. |
| Folder removal | Discard falls back to deleting a session folder only when Git no longer knows the worktree. | Done: only folders directly inside the sessions folder are deleted. |

## Residual risks

- **Same-user processes.** Any process running as the user can connect to the
  socket, as the threat model accepts. A per-launch token was considered and
  declined, since such a process could read it from the agent's environment.
- **`GITVIEW_SOCKET` is inherited.** Everything the agent starts sees it. The
  tools are read-only apart from selections, so this exposes repository
  contents the user's processes can already read.
- **Not tested:** a connection from a different user ID needs a second
  account or root; the check runs in `accept_loop` on every connection.
  Linux was checked only in a container through the automated tests.
- **Git can lose index updates.** An agent's `git reset <path>` running at the
  same moment as a Gitview write can undo that write. This is Git behavior.
  Gitview detects it and asks the user to try again; see milestone 5.
