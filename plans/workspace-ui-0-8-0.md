# Workspace UI for 0.8.0

## Objective

Gitview shows one full-screen page at a time (history, working tree,
branches, sessions) with the agent pane beside it. History is the raw text of
`git log --graph` capped at 200 commits, search is a substring match over
that text, and navigation stops at `j`/`k`. Developers reviewing an agent's
work want an IDE-like workspace instead: everything visible at once, a
readable commit graph, search that finds things, and vim motions everywhere.
This plan rebuilds the interface around that for 0.8.0.

## Success criteria

- **Workspace.** One screen with a sidebar (sessions, branches, changed
  files), a centre panel (history, a commit, or a diff), the agent pane on the
  right when an agent runs, and a bottom bar (status, mode, and the
  command/search line). Each panel can be focused, hidden, and resized from
  the keyboard. Below 119 columns, panels stack and the focused one fills the
  screen, as the agent layout does today.
- **Every current action still works** with its current key in the panel where
  it applies. The replay and live tests for staging, committing, branches,
  tags, profiles, sessions, and the agent pass after porting.
- **Structured history.** Gitview draws the graph from parsed commits:
  coloured lanes, and aligned columns for the graph, short hash, ref badges,
  subject, author, and relative date. Commits load in pages as the selection
  nears the end, with no 200-commit cap; it scrolls smoothly in a 100,000-commit
  repository. `v` toggles the raw `git log --graph` view, and the selection
  follows the same commit.
- **Search.** `/` opens a fuzzy, ranked search with matches highlighted. The
  filters `author:`, `path:`, `msg:`, `since:`, `until:`, and `ref:` go to
  `git log` and search all history, not only loaded commits. `n` and `N` move
  between matches, and Esc restores the previous selection.
- **Vim motions** in every list and diff: `gg`, `G`, `Ctrl-d`, `Ctrl-u`,
  `Ctrl-f`, `Ctrl-b`, counts (`5j`), `zz`, and `Tab` between panels. Help lists them.
- **Agent-driven panels.** The MCP select tools open their target in the right
  panel, a new `search_history` tool runs a search and shows the results, and
  the sidebar marks files the agent changed in this session.
- **No regression** in idle CPU, the diff-cache memory budgets, or the
  performance checks from 0.6.0.

## In scope / Out of scope

**In scope**

- The workspace layout, panel focus, hide, and keyboard resize.
- Structured, paged history with a raw toggle.
- Fuzzy search with Git-side filters, and `n`/`N`.
- Core vim motions and counts.
- MCP: select tools targeting panels, `search_history`, and changed-file marks.
- Help, README, and website updates, and screenshots for the PR.

**Out of scope**

- Agent-defined UI (A2UI-style cards or layouts).
- Vim marks, jump lists, and configurable keymap modes.
- Mouse resizing and drag, and searching diff contents (`-S`/`-G`).
- New Git features (rebase, stash, remotes, conflicts).

## Approach

Introduce a workspace model that owns panels and focus, and move today's
pages into it one at a time behind the key-replay harness, which already pins
their behavior. History becomes a list of parsed commits from
`git log --format` with parents, loaded in pages with `--skip` and
`--max-count`. A lane-assignment pass over parents draws the graph; the raw
view keeps today's `--graph` output. Search parses filters into `git log`
options (`--author`, `--grep`, `--since`, `--until`, a ref, `-- <path>`) run off
the render loop, and ranks the results with a fuzzy matcher. A motion layer
turns key sequences (`gg`, counts, panel chords) into the same actions
every panel already handles.

**Decided (2026-10-30, with the user)**

- Workspace panels, not a command palette.
- A structured graph drawn by Gitview, with the raw view kept behind `v`.
- Fuzzy search with Git-side field filters.
- Core vim motions only.
- The agent drives panels through MCP; no agent-defined UI.
- This ships as 0.8.0, after 0.7.0.

**Researched**

- **Fuzzy matcher:** `frizbee` (MIT, 0.13.0, updated Aug 2026, requires Rust
  1.89). `nucleo-matcher` is MPL-2.0 and last released Feb 2024;
  `fuzzy-matcher` is MIT but unreleased since Oct 2020 (all from crates.io,
  2026-10-30). `frizbee`'s API and speed are **UNVERIFIED** until milestone 3.
  *Fallback:* `fuzzy-matcher`.
- **Git filters:** `git log --all --author --grep -i --since --skip -n -- <path>`
  combine in one call (checked on this repository).

## Milestones

1. **Workspace shell (in progress).** The panel model, focus, hide, and resize. Move the
   working tree, history, and agent pane into it with their current
   rendering; all replay and live tests pass.
2. **Vim motions.** `gg`, `G`, the page keys, counts, `zz`, with `Tab` for panel focus
   across all panels, with replay tests for each.
3. **Search.** Fuzzy ranking over loaded commits, then the Git-side filters
   and `n`/`N`, with cancellation and tests for each filter.
4. **Structured history.** Parse commits with parents, page them, draw the
   lanes and columns, keep `v` for raw; graph tests with merges, octopus
   merges, and long-lived branches; measure scrolling on 100,000 commits.
5. **Sidebar and remaining pages.** Sessions, branches, and changed files in
   the sidebar; move branches, tags, profiles, objects, and comparison in.
6. **Agent-driven panels.** Select tools target panels, `search_history`,
   changed-file marks; rerun the live agent suite.
7. **Docs and release.** Help, README, website, screenshots, and the full
   format, test, lint, and release-build pass.

## Risks & open questions

- **The rewrite touches everything.** *Mitigation:* the replay harness pins
  each page's behavior before it moves, and every milestone keeps the full
  suite green. Each milestone ships on its own branch.
- **Graph layout speed** on very large repositories is unmeasured. *Fallback:*
  lay out only the visible window plus a margin, and cache lanes per page.
- **Key conflicts:** `G`, `n`, `N`, and `Ctrl-w` may clash with existing keys
  or with an agent's own keys. Today's single-letter actions keep their keys,
  and keys go to the agent only while it has focus. Conflicts are listed in
  milestone 2 and resolved there.
- **Terminal width:** many panels on a narrow screen. *Fallback:* the stacked
  layout below 119 columns.

## Boundaries

- Every current action keeps its key and behavior; nothing is removed.
- Git writes keep going through `writes::serialized`, and reads stay off the
  render loop.
- The control channel's security properties hold: read-only tools, size caps,
  and the private socket.
- Idle CPU, memory budgets, and the 0.6.0 performance checks must not get
  worse.
- `--ascii` keeps working in every new panel.

## Implementation progress

Milestone 1 starts with the workspace state and geometry model in
`src/workspace.rs`. Tests cover focus, hiding and restoring panels, keeping the
last panel visible, resizing, the 119-column breakpoint, and tiny terminals.
The existing renderer and input router still need to adopt this model; no new
workspace shortcuts are exposed yet.

Verification: full `cargo test --locked --features test-support` passed
(472 passed, 0 failed, 15 ignored). Clippy passed with warnings
denied. An existing agent-exit timing test failed once; the full rerun passed.

The cockpit renderer now uses the workspace model for its 119-column boundary
and compact split. Real application replay tests cover 118 and 119 columns.
Clippy passed. The full suite reached the known watcher shutdown timing
failure; rerunning watcher tests and every remaining test target passed.
Persistent state, panel controls, and combined agent geometry remain open.

### Persistent cockpit controls

The app now owns its cockpit workspace model. `Tab` focuses the
worktree/history, `Ctrl-w +/-` grows or shrinks the focused panel, `Ctrl-w x` hides the
focused panel, and `Ctrl-w =` restores both. Help lists the controls. The
default sidebar is 40 columns to keep ordinary filenames readable.

Five runtime workspace tests pass, including hide/restore, persistent resize,
and filename readability. Clippy passes. The full no-fail-fast suite exposed
filename clipping (fixed) and watcher timing/event failures under parallel
load. All affected targets and cockpit replays pass on a serial rerun
(59 tests); the other full-suite targets passed.

Milestone 1 remains in progress: the agent still uses its original geometry,
and combined agent focus, visibility, and resize need integration.

User correction: keep Tab for panel focus; remove redundant Ctrl-w h/l.
Resize targets the focused panel, matching the hide command.

### Agent geometry and full-width mode

The app initializes the workspace with the agent's presence. The outer agent
split now uses its retained width and visibility from the workspace, clamps
widths to the terminal, and resizes the PTY to the resulting inner rectangle.
The cockpit lays out only the Gitview area so it does not reserve agent space
twice. Existing Ctrl-backslash focus switching and agent input forwarding are
preserved.

`Ctrl-w o` toggles the focused Gitview panel to full width and back, retaining
requested sizes. `Ctrl-w x` hides it and exits full-width mode; `Ctrl-w =`
restores the Gitview panels. In full-width mode, Ctrl-backslash switches to the
full-width agent and back. Return focus to Gitview before using Ctrl-w o; the
agent continues receiving its own Ctrl-w keys. Help lists the new toggle.

Verification: the full serial suite passed (481 passed, 15 ignored), Clippy
passed, and an additional real-app replay confirmed full-width agent focus and
layout restoration. Milestone 1 remains in progress: direct keyboard agent
hide/restore and resize controls still need a route that preserves agent input.
