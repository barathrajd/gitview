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
  screen, as the agent layout does today. When the agent is visible, show only
  the focused Gitview pane beside it, with Tab switching Worktree and History
  (user correction, October 31, 2026).
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
- Mouse resizing and dragging workspace borders, and searching diff contents
  (`-S`/`-G`). Agent mouse forwarding was added to scope at the user’s request.
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

1. **Workspace shell (complete).** The panel model, focus, hide, and resize. Move the
   working tree, history, and agent pane into it with their current
   rendering; all replay and live tests pass.
2. **Vim motions (in progress).** `gg`, `G`, the page keys, counts, `zz`, with `Tab` for panel focus
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

### Direct agent full-width toggle

The user requested the same full-width shortcut in the agent. Ctrl-w o now
toggles directly while the agent has focus, retaining focus and restoring the
workspace on a second invocation. Other Ctrl-w sequences are forwarded once
the next key is known; Ctrl-backslash still switches focus. Agent hide and
resize chords remain separate unfinished work.

Both Ctrl-w o and Ctrl-w Ctrl-o are supported. Verification: 28 agent and
workspace replay tests passed, the optimized build passed, and the user
confirmed shell full-width toggling works. Earlier control-channel tests and
Clippy also passed for this slice.

### Agent resize and hide/restore

Ctrl-w +/- now resizes the focused agent. Ctrl-w x hides it without ending
the child process and returns focus to Gitview, also leaving full-width mode.
Ctrl-w = restores every available panel; Ctrl-backslash reopens a hidden agent.
Unrecognized Ctrl-w sequences still reach the agent. README and help list the
controls. The user requested manual test steps before committing this slice.

Verification: full serial suite passed (487 passed, 0 failed,
15 ignored), Clippy passed, and the optimized binary was rebuilt.
The user confirmed resize, hide/restore, and full-width behavior works.

### Combined layout and agent terminal fidelity (awaiting user testing)

User correction: when a shell or agent is visible, show only the focused
Gitview pane beside it, even on a wide terminal. Tab switches Worktree and
History. Hiding the agent lets Gitview show its ordinary multi-pane layout.
This replaces the earlier proposal to show both beside the agent. Hiding both
Gitview panels leaves the agent full width; the focus key reopens Gitview.

At the user's request, agent mouse forwarding is included in this slice.
Capture follows agent focus and is disabled on focus return, external editors,
and terminal cleanup. Mouse reports use
pane-relative coordinates and the requested legacy, UTF-8, or SGR encoding.
The agent widget resets cells before painting to avoid inheriting stale
symbols or inverse/background styles. This is covered by a regression test;
the exact AGY screenshot still needs user validation.

No changes are committed before the user tests them.

Verification: the full serial suite passed before the layout-policy correction
(496 passed, 15 ignored). After that correction, all 95 affected tests passed,
including real-terminal mouse capture and cleanup. Clippy and the optimized
build passed. The user still needs to validate AGY rendering and mouse behavior.

### AGY scrolling and background detection (awaiting user testing)

The installed AGY probe remains running with mouse reporting disabled, uses
normal screen mode at startup, and asks OSC 11 for the terminal background.
Previously the outer terminal could turn wheel movement into arrows reaching
AGY's input, and the background query went unanswered.

The focused agent now captures wheel input even without a child mouse mode.
Ordinary output browses local scrollback; typing restores the live screen.
Alternate-screen programs get Page Up/Down for viewport scrolling. Native mouse
protocol forwarding remains available when the child enables it. The embedded
pane paints a known dark default background and reports the same RGB through
OSC 11, so theme detection has a consistent answer. AGY must restart to pick
up its theme again. Its post-fix probe no longer lists OSC 11 as unanswered.

Real-terminal tests verify wheel fallback and cleanup, while a pane regression
checks scrolling and returning to live output. Exact AGY output colors and
conversation scrolling remain subject to the user's manual test.

Verification after the AGY fixes: full serial suite passed (500 passed,
0 failed, 16 ignored); Clippy and the optimized build passed. The
installed AGY probe stayed running and no longer reported an unanswered
background-color query. User confirmation is still required before committing.

The user reviewed the latest AGY rendering screenshot and authorized committing
the terminal fixes. Compatibility differences are recorded in
`docs/agent-terminal-compatibility.md`. Milestone 1 is complete with the user's
single-Gitview-pane layout correction. Milestone 2 starts with cockpit list
motions; agent input and workspace chords keep their current behavior.

## Handoff on October 31, 2026

The user stopped implementation for a credit limit and requested a merge to
main. Milestone 1 and the agent terminal compatibility fixes are complete.
Milestone 2 has not started: no vim-motion implementation or failing test was
written. Start that milestone next, on a fresh feature branch from main.

### Milestone 2: cockpit gg/G and counts (awaiting user testing)

History and Worktree lists accept `gg` (first row), `G` (last row), and a
count before `j`/`k` or the arrows (`5j`). A pending count or `g` is dropped
by any other key, which keeps its usual action, and by blocked screens.
Agent input, prompts, and Ctrl-w chords are handled before the parser.

Verification: full serial suite passed (505 passed, 0 failed, 16 ignored),
Clippy passed, optimized binary rebuilt. Page and half-page keys, `zz`, and
motions in other lists and diffs remain open.

Fix (committed): Ctrl- and Alt-modified letters no longer run plain view
shortcuts; Ctrl-u had unstaged and Ctrl-d opened the discard preview.

### Milestone 2: page keys and zz (committed)

Ctrl-d/Ctrl-u move half the focused list's visible rows; Ctrl-f/Ctrl-b and
Page Down/Up move a full page. Rows come from the workspace layout of the last
drawn frame. `zz` scrolls History so the selection is centered; the Worktree
list keeps no scroll offset between frames, so `zz` there is still open.

Verification: full serial suite passed (511 passed, 0 failed, 16 ignored),
Clippy passed, optimized binary rebuilt.

### Milestone 2: Worktree scroll and zz (committed)

The Worktree list now keeps its scroll offset (in display rows, counting
section headings) between frames, so `zz` centers the selected file there
too. User testing found `r` reset it to the top: refresh cleared the cached
status, so the reload lost the selected file (an older reset that scroll
memory exposed). `r` now keeps the list until the reload arrives, and a
loading list no longer overwrites the saved offset.
Verification: full serial suite passed (512 passed, 0 failed,
16 ignored), Clippy passed, optimized binary rebuilt. Motions in the other
lists and in diffs remain open.
