use std::io;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{View, model::App, run::Flow},
    workspace::Panel,
};

/// Workspace chords apply to the cockpit; modal handlers run before this one.
pub(in crate::app) fn handle(app: &mut App, key: KeyEvent) -> io::Result<Flow> {
    if app.details.is_some() || app.diff.is_some() || app.mutation_pending {
        app.workspace_prefix = false;
        return Ok(Flow::Next);
    }
    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.workspace_prefix = true;
        return Ok(Flow::Handled);
    }
    if !std::mem::take(&mut app.workspace_prefix) {
        return Ok(Flow::Next);
    }
    let current = match app.app_state.view() {
        View::WorkingTree => Panel::Sidebar,
        View::History => Panel::Center,
    };
    app.workspace.focus(current);
    match key.code {
        KeyCode::Char('+') => app.workspace.resize(current, 4),
        KeyCode::Char('-') => app.workspace.resize(current, -4),
        KeyCode::Char('x') => {
            app.workspace.hide(current);
        }
        KeyCode::Char('=') => {
            app.workspace.show(Panel::Sidebar);
            app.workspace.show(Panel::Center);
        }
        _ => {}
    }
    app.app_state.show(match app.workspace.focused() {
        Panel::Sidebar => View::WorkingTree,
        _ => View::History,
    });
    Ok(Flow::Handled)
}
