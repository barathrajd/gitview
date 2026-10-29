//! The agent pane's part of the event loop.

use crossterm::event::Event;

use super::{input::Focus, model::App};
use crate::pane::{PaneState, encode_key, encode_paste};

/// Applies output the program produced, asks for a redraw when anything
/// changed, and hands focus back to Gitview once the program has ended.
pub(super) fn poll(app: &mut App) {
    let Some(pane) = app.pane.as_mut() else {
        return;
    };
    let before = (pane.generation(), pane.state());
    let state = pane.poll();
    if before != (pane.generation(), state) {
        app.needs_redraw = true;
    }
    if state != PaneState::Running && app.focus == Focus::Agent {
        app.focus = Focus::Gitview;
    }
}

/// Moves focus to the other side. If the program has already ended, `poll`
/// hands focus straight back, so keys can't be typed into a dead program.
pub(super) fn toggle_focus(app: &mut App) {
    app.focus = match app.focus {
        Focus::Agent => Focus::Gitview,
        Focus::Gitview => Focus::Agent,
    };
}

/// Sends a key or paste to the program, encoded for the modes it asked for.
pub(super) fn forward(app: &mut App, event: &Event) {
    let Some(pane) = app.pane.as_mut() else {
        return;
    };
    if pane.state() != PaneState::Running {
        return;
    }
    let bytes = match event {
        Event::Key(key) => encode_key(*key, pane.screen().application_cursor()),
        Event::Paste(text) => Some(encode_paste(text, pane.screen().bracketed_paste())),
        _ => None,
    };
    if let Some(bytes) = bytes {
        // A write can fail only if the program just ended; poll reports that.
        let _ = pane.send(&bytes);
    }
}
