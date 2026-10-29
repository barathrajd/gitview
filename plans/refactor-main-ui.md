# Refactor main.rs and ui.rs

## Objective

`src/main.rs` is 2,572 lines, and 2,100 of them are one function, `app()`: about
190 lines of local state, a worker-draining loop, and a single key handler.
`src/ui.rs` mixes help text, screen rendering, and the diff layout and
highlighting engine in 3,100 lines. This makes the planned
[agent workspace](agent-workspace.md) hard to attach (a pane needs a layout slot
and a key-routing layer) and risky to change. Restructure both files without
changing behavior, protected by a key-replay test harness built first. The
reader is the maintainer who builds the agent pane next.

## Success criteria

- A test can replay a list of key events against `app` on ratatui's in-memory
  backend and assert on the rendered screen text, without a real terminal.
- Characterization tests cover key flows in history, search, diff, help,
  worktree, branches, tags, object and comparison pages, commit, and profiles.
  They pass on the pre-refactor code before any code moves.
- No source file under `src/app/` or `src/ui/` exceeds 800 lines, and `src/main.rs`
  stays under 400 and holds only CLI parsing, terminal setup, and teardown.
- All existing tests, including the performance and live-allocation tests,
  pass unchanged. `cargo fmt --check` is clean, and `cargo clippy --all-targets`
  adds no new warnings.
- Public paths used by tests (`gitview::ui::render`, `PreparedDiff`, `Cockpit`,
  `Screen`, `HELP`, and the like) still resolve; tests need no edits other than
  new ones.
- Input is routed through one layer with a single focus target (Gitview), and
  the renderer has a named place to carve out a side pane; neither changes what
  the user sees.

## In scope / Out of scope

**In scope**

- Making `app` generic over the ratatui backend with an injectable event source.
- The key-replay harness and characterization tests.
- Grouping `app()` locals into an `App` state struct, then splitting key
  handling and result draining into per-screen modules under `src/app/`.
- Splitting `ui.rs` into a `src/ui/` tree: help, cockpit and panels, branches,
  commit workspace, and the diff engine (rows, wrapping, caches).
- A focus-aware input router and a layout slot that stay inert.

**Out of scope**

- New features, keybinding or help-text changes, and visual changes.
- An `Action` enum or a redesign of the state model, which is for the control
  channel and write-queue milestones of the agent workspace plan.
- Any PTY or pane code, new dependencies, or a version bump.
- Refactoring the other `src/*.rs` modules.

## Approach

Build the safety net before moving code. Events already reach `app()` through an
`mpsc` receiver, and drawing goes through one `terminal.draw`. Making `app`
generic over `Backend` and taking the receiver as a parameter lets tests drive
it with `TestBackend`. Worker results arrive on threads, so tests use the same
wait-until-the-screen-contains pattern as `tests/pane.rs` rather than fixed
sleeps. Assertions use screen text, not colors or timing.

With the net green, move the locals into an `App` struct as a pure field move.
This is the prerequisite for splitting the handler, since free functions need
grouped state to borrow. Then peel off one screen at a time into `src/app/`,
running the suite after each. Split `ui.rs` last, re-exporting public items
from `ui/mod.rs` so paths stay stable. Each move is its own commit. Do not mix a
move with an edit.

## Milestones

- [x] **1. Drivable `app`:** Make `app` generic over the backend and take the
  event receiver; move it into the library. Add one replay test (open and close
  help). `main` behavior is unchanged; run the existing PTY test
  (`tests/cockpit_live.rs`) to confirm.
  *Done:* `app::run::run_app` is generic over a `Host` trait (events plus
  running an external program); `TerminalHost` is the real one and
  `tests/app_replay.rs` has the scripted one. One difference: the profile
  editor now goes through `run_external`, so it also pauses the input reader and
  clears the screen on return, as the commit editor already did.
- [x] **2. Characterize history, search, diff, help:** Key-sequence tests for
  selection, search, filter by path, diff view modes, hunk navigation, and the
  help pages.
  *Done:* 15 tests in `tests/app_replay.rs` on a shared harness
  (`tests/support/replay.rs`). Hunk keys exist only on working-tree diffs, so
  they are tested there. The harness loads default keybindings, never the
  user's file. Mutation checks (breaking `v`, `Esc` in help, and `]`) each fail
  at least one test.
- [x] **3. Characterize the rest:** Worktree stage and unstage, branches,
  tags, object and comparison pages, commit, and profiles, plus a narrow
  terminal case.
  *Done:* `tests/app_replay_worktree.rs` (stage, unstage, stage all, commit
  composer, narrow layout) and `tests/app_replay_pages.rs` (branches, tags,
  object, comparison, profiles). Mutating flows are checked against Git; 15
  repeated parallel runs showed no flakes. **Not characterized; smoke-test by
  hand before milestone 6 ships:** discard, ignore rules, move, remove, and
  untrack; amend; tag create, delete, and verify; branch rename and
  previous-checkout; the external editors (`Ctrl+E` in the composer, `e` on the
  profile page), which the scripted host only runs inline; and the profile
  `r` reload, which needs the global Git config context.
- [x] **4. Extract `App` state:** Field move only, no logic change.
  *Done:* `app::model::App` holds the loop's 92 fields (flags, selections,
  pending operations, channels, workers); setup code is unchanged and moves its
  locals into the struct just before the loop. A token-level comparison of old
  and new `run_app`, ignoring the `app.` prefix, differs only by dropped `mut`s,
  the struct literal, two `format!` captures, shorthand fixes, and rustfmt
  braces. Locals shadowing a field name (`diff`, `details`, cloned senders)
  stay local. Drop order of the workers and channels is now field order.
- [x] **5. Split handlers:** Per-screen modules under `src/app/`; `main.rs`
  shrinks to the size limit.
  *Done:* `run.rs` is the setup and a 25-line loop (354 lines). Each event
  step is a function returning `Flow` (`Next`, `Handled`, `Quit`) instead of
  using `continue`: `drain`, `draw`, `paste`, and `keys/{overlays, prompts,
  commit, confirm, search, branches, views}`, tried in the old priority order.
  Largest file is 518 lines; `main.rs` is 146. A token comparison against the
  old loop shows only `continue`/`return` rewrites, function wrappers,
  `Env` destructuring, two reborrows, and rustfmt braces. The key groups were
  extracted together in one commit, not one per screen.
- [ ] **6. Split `ui.rs`:** The `src/ui/` tree, one area per commit, with the
  diff engine last because it holds the performance-sensitive caches.
- [ ] **7. Input router and layout slot:** Add the focus-aware router and the
  side-pane slot, both inert, with tests that Gitview is the only target. Then
  resume milestone 1 of the agent workspace plan.

## Risks & open questions

- **Borrow-checker friction** when grouping the locals. Fallback: group them in
  several small structs (per screen) instead of one.
- **Flaky async tests** from worker threads. Fallback: a deterministic drain
  helper in the harness; no fixed sleeps.
- **Performance regression** from struct indirection or moved caches. The
  `*_performance` and `*_live_allocations` tests are the gate; compare numbers
  before and after milestone 6.
- **Snapshot brittleness.** Assert on stable text fragments, not full frames.
- **Characterization gaps.** Some flows (editor launch, external `git` hooks)
  may be too hard to replay; list them in the plan's notes and smoke-test them
  by hand at the end.

## Boundaries

- Keybindings, help text, rendered output, and CLI flags do not change.
- Terminal restoration after a panic keeps working (`tests/terminal_panic.rs`).
- Memory budgets and the performance tests keep their current thresholds.
- Git is still invoked with argument arrays, and subprocess work stays off the
  render loop.
- The agent workspace plan is paused until this one ships.
