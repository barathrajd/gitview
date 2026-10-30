//! The agent pane's part of the event loop.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{input::Focus, model::App};
use crate::{
    app::View,
    pane::{PaneState, encode_key, encode_paste},
    workspace::Panel,
};

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
        app.workspace_prefix = false;
    }
}

/// Moves focus to the other side. If the program has already ended, `poll`
/// hands focus straight back, so keys can't be typed into a dead program.
pub(super) fn toggle_focus(app: &mut App) {
    app.workspace_prefix = false;
    app.focus = match app.focus {
        Focus::Agent => Focus::Gitview,
        Focus::Gitview => {
            app.workspace.show(Panel::Agent);
            app.workspace.focus(Panel::Agent);
            Focus::Agent
        }
    };
}

/// Apply workspace chords to the agent; replay unrelated sequences unchanged.
pub(super) fn workspace_key(app: &mut App, event: &Event) -> bool {
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Release {
            return false;
        }
        if !app.workspace_prefix
            && key.code == KeyCode::Char('w')
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            app.workspace_prefix = true;
            return true;
        }
        if app.workspace_prefix
            && matches!(key.code, KeyCode::Char('o' | '+' | '-' | 'x' | '='))
            && !key
                .modifiers
                .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            app.workspace_prefix = false;
            app.workspace.focus(Panel::Agent);
            match key.code {
                KeyCode::Char('+') => app.workspace.resize(Panel::Agent, 4),
                KeyCode::Char('-') => app.workspace.resize(Panel::Agent, -4),
                KeyCode::Char('o') => app.workspace.toggle_fullscreen(),
                KeyCode::Char('=') => app.workspace.restore_panels(),
                KeyCode::Char('x') => {
                    if app.workspace.hide(Panel::Agent) {
                        app.focus = Focus::Gitview;
                        app.app_state.show(match app.workspace.focused() {
                            Panel::Sidebar => View::WorkingTree,
                            _ => View::History,
                        });
                    }
                }
                _ => unreachable!(),
            }
            return true;
        }
    }
    // Mouse and resize events don't finish a keyboard chord.
    if matches!(event, Event::Key(_) | Event::Paste(_)) && std::mem::take(&mut app.workspace_prefix)
    {
        forward(
            app,
            &Event::Key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)),
        );
    }
    false
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
