# Gitview 1.0 cockpit design

This design explains how the 1.0 terminal interface keeps repository context
visible while users move between history and working-tree tasks. It is tied to
the approved [Gitview 1.0 plan](plans/gitview-1-0-0.md).

## Context

Gitview 0.2.1 replaces its full-screen view as users switch between history,
working-tree, branch, commit, and diff screens. That model is easy to test, but
it hides surrounding repository state and makes the application feel less like
a coherent workbench. Gitview 1.0 needs a persistent view that can grow to cover
Lazygit v0.66.2 workflows without copying Lazygit's design.

## Goals / Non-goals

The cockpit must keep core repository context visible, make the focused panel
and available actions easy to discover, and remain usable at narrow terminal
sizes. Git subprocesses must stay outside rendering, and existing keyboard
workflows must remain available as the panes are introduced.

This design does not create a native desktop app, an AI panel, Windows-specific
behavior, or a new Git implementation. It does not force users to adopt one
terminal font or copy Lazygit's visual system.

## Proposed design

Keep the terminal-only Ratatui application and render a shared frame with four
regions: a repository header, a primary work area, an operation/status strip,
and a contextual key-hint bar. The header shows the repository, current branch
or detached state, and a compact working-tree summary. The primary work area
contains a working-tree pane, a history pane, and an inspector pane when width
permits. The inspector shows the selected commit summary or a focused action
preview; full commit details and diff screens remain accessible from existing
keyboard actions.

Use responsive layout tiers. At 128 columns and wider, show all three panes. At
88 to 127 columns, show working tree and history side by side and use the
inspector on demand. Below 88 columns, show the focused pane and retain the
other panes through the same keyboard navigation. Keep the terminal's default
background, use a restrained teal/cyan accent for focus, muted borders and
secondary text, and distinct success, warning, and error colors. Avoid relying
on Nerd Font glyphs; the existing ASCII border option remains available.

Represent the home view as data rather than reading application state while
drawing:

```rust
struct Cockpit<'a> {
    repository: &'a Path,
    branch: Option<&'a Branches>,
    history: Option<&'a Result<Vec<String>, String>>,
    working_tree: Option<&'a Result<Vec<Entry>, String>>,
    active_pane: Pane,
    history_selection: &'a mut ListState,
    working_tree_selection: &'a mut ListState,
}

enum Pane {
    History,
    WorkingTree,
    Inspector,
}
```

The app state owns the active pane and existing selection/loading state. Key
events become actions for that pane; overlays and confirmations continue to
block unrelated navigation. The renderer receives a cockpit view model and
does not run Git. Existing worker threads and request IDs remain the first
implementation of background work; consolidate them behind a coordinator only
when a behavior slice needs that boundary. This keeps the first visual change
small and avoids introducing an async runtime before it is needed.

## Alternatives considered

- **Keep full-screen replacement views:** This preserves the current structure
  but hides repository context and does not satisfy the pane-based workbench
  goal.
- **Copy Lazygit's pane structure and keybindings:** This might shorten
  discovery work, but it would give Gitview a derivative identity and increase
  keyboard conflicts. Keep a distinct layout and add contextual hints.
- **Add Tokio and a general event bus before the UI change:** This provides a
  larger concurrency foundation, but the current worker-thread pattern already
  keeps Git calls off the render loop. Add that complexity only when the next
  operation slice requires it.

## Risks & mitigations

- **Narrow terminals lose context:** Collapse to the focused pane below 88
  columns and keep a visible pane switch hint. Cover each layout tier with
  TestBackend snapshots.
- **Pane state and current selection become inconsistent:** Keep one app-owned
  active pane and independent selections for history and working tree. Test
  focus changes and blocked navigation before connecting more Git actions.
- **Startup status and branch requests compete with rendering:** Show loading
  states and poll worker results between draws. Never wait for Git inside the
  render callback.
- **Truecolor and glyph support vary:** Keep semantic color roles, support the
  existing ASCII mode, and never encode status with color alone.

## Rollout & rollback

Introduce the cockpit as the default home view after its TestBackend coverage
and PTY smoke check pass. Keep branch, details, help, and diff screens working
while the cockpit is introduced. The change adds no persistent data or config
migration; revert the cockpit commit to restore the 0.2.1 view model if a
terminal compatibility regression appears.
