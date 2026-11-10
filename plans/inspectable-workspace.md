# Inspectable workspace

## Objective

Give Gitview developers and agents a reliable way to understand the current
workspace and verify interactions without reading terminal characters. Adapt
the headless inspection ideas in [Rune](https://github.com/anomalyco/rune) to
Gitview's existing terminal UI and replay harness.

## Success criteria

The following outcomes make the work verifiable:

- A versioned structured snapshot reports the active view, terminal size,
  visible panel bounds, focus, selection, and loading or error state when
  available. Hidden and unavailable values are represented explicitly.
- Replay tests capture snapshots after scripted input and verify focus,
  selection, and layout at wide and narrow terminal sizes.
- A read-only `get_ui_state` MCP tool returns the current snapshot as valid
  JSON, with a deterministic size limit and explicit truncation metadata.
- Snapshots exclude agent terminal contents, prompts, file contents, diffs,
  and repository paths by default. Tests verify these exclusions.
- Representative terminal buffer captures cover worktree, history, and
  agent-pane layouts without depending on fonts or image rendering.
- Existing key bindings, Git operations, and control-channel checks continue
  to pass; inspection doesn't block the render loop on Git or filesystem work.

## In scope / Out of scope

The first version covers inspection and repeatable verification:

- A small, documented snapshot schema shared by replay and the control channel.
- Read-only inspection of Gitview-owned state and panel geometry.
- Scripted replay assertions and a small set of stable terminal captures.
- Consistent labels and states for the panels covered by the snapshot.

The following work is excluded:

- Replacing Ratatui with Rune, adding a native GUI, or adding a Rune dependency.
- Exposing agent terminal output, arbitrary screen text, patches, or file data.
- A general remote input API, agent-defined UI, or new Git write operations.
- Completing the structured history, sidebar, or agent-driven panel milestones
  already owned by the [workspace plan](workspace-ui-0-8-0.md).
- PNG-based golden tests, font-dependent rendering, and a new visual framework.

## Approach

The current replay harness drives the real application loop using Ratatui's
in-memory backend and can capture a terminal buffer. The workspace model owns
panel focus and computes layout. The existing MCP server forwards requests to a
private local control socket. Extend these paths rather than creating another
application runner or control server.

Define a version 1 snapshot with `schema_version`, terminal dimensions, active
view, focused panel, and a list of visible panels with cell coordinates and
dimensions. Give each panel a stable identifier and optional semantic selection
and state fields; use enum identifiers or opaque indices instead of names or
paths. Explicitly distinguish unavailable data from an empty selection. Build
the snapshot from application state and the layout actually used for the most
recent frame. Describe it as a point-in-time view, which can become stale.

Publish a bounded, immutable copy from the application loop for the socket
handler to read. Return an explicit unavailable error before the first frame or
after shutdown. Bound strings and list lengths before JSON serialization; keep
the final response within the existing 64 KiB channel limit and return valid
JSON with a `truncated` flag instead of cutting serialized JSON mid-value.
Reuse existing socket access controls. Never include terminal output or prompt
text, including when an agent pane or modal is active.

The implementation must verify the actual layout and state integration points
before choosing where to publish the snapshot. Rune's headless element-tree and
scripted-input concepts are described in its [project README](https://github.com/anomalyco/rune#readme);
Gitview's implementation remains terminal-specific.

## Milestones

Each milestone delivers an independently verifiable slice:

1. **Replay inspection:** Define the minimal schema and capture a snapshot from
   a real replay frame. Verify focus and geometry at 118 and 119 columns, and
   verify that the snapshot reflects the rendered layout.
2. **Selection and states:** Add stable selection identifiers and available
   loading, error, and modal indicators for worktree and history. Replay a
   selection change and a loading or error transition; check privacy exclusions.
3. **Agent inspection:** Add read-only `get_ui_state` through the existing MCP
   and socket paths. Test unavailable state, valid bounded JSON, truncation,
   access controls, and consistency with a replayed user action.
4. **Terminal captures:** Add a small deterministic set of buffer captures for
   worktree, history, and agent layouts, including narrow mode and ASCII mode.
   Document how to inspect and update expected output.
5. **Verification and documentation:** Document the schema, privacy boundary,
   and tool behavior. Run focused replay and control tests, then the repository's
   format, full test, and lint checks.

## Risks & open questions

The following risks have explicit fallbacks:

- **Layout may differ from the workspace model:** capture geometry from the
  actual frame layout; if a panel lacks a reliable rectangle, mark it
  unavailable until its renderer exposes one.
- **Snapshots may expose sensitive information:** use an allowlist of fields,
  omit free text and paths, and test both normal screens and prompts.
- **Snapshots can become stale:** publish after a completed frame and document
  point-in-time semantics; return unavailable instead of guessing on startup.
- **Large output can break the socket limit:** cap data before serialization;
  return a smaller valid response with `truncated: true` if necessary.
- **Snapshots may become brittle during 0.8.0:** keep the schema minimal and
  versioned; add panels as the workspace plan lands. This plan doesn't assume
  all planned sidebar and history changes already exist.
- **Full buffer captures may be noisy:** fix dimensions and fixture data, and
  limit captures to representative layouts; use semantic assertions for behavior.

## Boundaries

Keep Gitview keyboard-first and preserve existing controls, terminal restoration,
ASCII support, write serialization, and asynchronous Git reads. Preserve the
control channel's private socket, peer checks, and response limits. Keep agent
terminal input and output separate from inspection. Do not add an externally
accessible control endpoint or expose prompts and repository contents.
