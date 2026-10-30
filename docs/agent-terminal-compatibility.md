# Agent terminal compatibility

These notes describe the 0.8.0 development branch. Gitview embeds a terminal
emulator inside its agent pane; the terminal app still draws the final screen.
Compatibility and rendering will continue to improve in future releases.

## Normal terminal and embedded pane

The embedded pane supports everyday agent interaction, but it implements fewer
terminal features than a full terminal app.

| Behavior | Normal terminal app | Gitview agent pane |
| --- | --- | --- |
| Rendering | Uses the terminal app's emulator and theme. | Parses child output with `vt100`, then paints cells through Ratatui. It uses a dark default background. |
| Theme detection | Can report its configured colors to programs. | Answers background-color queries with the pane's actual dark background. Foreground-color queries remain unanswered. |
| Mouse-enabled programs | Receive the terminal's mouse reports. | Receive supported clicks, wheel, and motion reports translated into pane-relative coordinates while focused. |
| Programs without mouse reporting | Scrolling depends on the terminal and program. | Wheel events browse ordinary pane scrollback. Alternate-screen programs receive Page Up/Down for viewport scrolling. Programs must handle those keys to scroll their own views. |
| Scrollback | Capacity and behavior depend on the terminal app. | Keeps up to 2,000 lines. Typing returns to live output. An alternate-screen program manages its own conversation history. |
| Input | Goes directly to the foreground program. | The focus key and workspace chords stay with Gitview; other input reaches the focused agent. |
| Clipboard and links | May support native selection, clickable hyperlinks, and clipboard control sequences. | Native terminal behavior is not fully reproduced. OSC hyperlinks and clipboard integration are not forwarded. |
| Modern terminal extensions | Support depends on the terminal app. | Keyboard-mode negotiation and several terminal queries remain unsupported. |
| Streaming updates | May honor synchronized-update sequences. | Frames can briefly show an intermediate update because synchronized painting is not implemented. |
| Text layout | Uses the terminal app's width and font rules. | Unicode cell widths and wrapping can differ from the outer terminal, particularly after resizing. |

## Changes in the 0.8.0 development work

The current fixes address the issues found while running Antigravity CLI
(`agy`) inside a shell and directly as an agent.

- The pane clears old symbols and styles before painting new cells.
- Background-color queries now receive an answer matching the painted pane,
  so agents can detect a dark background instead of choosing a fallback theme.
- Mouse capture follows agent focus. Wheel input no longer depends on the
  outer terminal converting wheel movement into arrow keys.
- Mouse capture is disabled when focus returns to Gitview, when an external
  editor takes over, and during normal terminal cleanup.
- Full-width mode, hide/restore, and resize retain the agent process.
- With an agent visible, Tab switches the single neighboring Gitview pane
  between Worktree and History.

Restart both Gitview and the agent after upgrading: programs can cache their
terminal capabilities and theme at startup. The embedded pane does not promise
pixel-for-pixel rendering or every interaction available in a standalone
terminal app.

## Verification and remaining work

Automated tests cover protocol encoding, real-terminal mouse delivery and
cleanup, wheel fallbacks, scrolling, background-color replies, and pane
rendering. The full serial suite passes with 500 tests; the optional live-agent
and performance probes remain separate checks. An installed AGY 1.4.11 probe
on macOS stays running and receives the background-color reply without a
prompt being sent.

The latest user screenshot confirms the improved AGY colors. Conversation
scrolling still depends on the agent's rendering mode and its own viewport
behavior. Broader terminal-extension support and rendering fidelity remain
compatibility work for future releases; no release date is promised for the
unsupported features listed above.
