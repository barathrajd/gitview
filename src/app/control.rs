//! Applies what the agent asked for through the control channel. A request
//! waits while the user is in a prompt or dialog, and one whose commit or file
//! hasn't loaded yet is retried briefly, then dropped.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use super::{View, input::Focus, model::App};
use crate::{
    control::server::ControlCommand,
    worktree::{Entry, Section},
};

/// How long a request waits for its commit or file to appear.
const PATIENCE: Duration = Duration::from_secs(3);

pub(super) fn step(app: &mut App, root: &Path) {
    if let Some((server, _)) = &app.control {
        server.set_busy(in_prompt(app));
    }
    if app.pending_control.is_none()
        && let Some(command) = app
            .control
            .as_ref()
            .and_then(|(_, commands)| commands.try_recv().ok())
    {
        app.pending_control = Some((command, Instant::now() + PATIENCE));
    }
    let Some((command, deadline)) = app.pending_control.take() else {
        return;
    };
    if in_prompt(app) {
        app.pending_control = Some((command, deadline));
        return;
    }
    if !apply(app, root, &command) && Instant::now() < deadline {
        app.pending_control = Some((command, deadline));
    }
    app.needs_redraw = true;
}

/// Returns false when the target isn't loaded yet.
fn apply(app: &mut App, root: &Path, command: &ControlCommand) -> bool {
    match command {
        ControlCommand::FocusAgent if app.pane.is_some() => app.focus = Focus::Agent,
        ControlCommand::FocusAgent => {}
        ControlCommand::FocusGitview => app.focus = Focus::Gitview,
        ControlCommand::SelectCommit { id, path } => {
            let Some(Ok(rows)) = &app.history else {
                return false;
            };
            let Some(index) = rows.iter().position(|row| {
                row.rsplit_once('\u{1f}')
                    .is_some_and(|(_, full)| full == id)
            }) else {
                return false;
            };
            close_views(app);
            app.app_state.show(View::History);
            app.selection.select(Some(index));
            open_commit_details(app, root, id.clone());
            app.details_file = path.clone();
            if let Some(path) = path {
                app.request_id += 1;
                let current = app.request_id;
                app.diff = Some((current, None));
                app.diff_scroll = 0;
                app.selected_hunk = 0;
                app.ordinary_diff_worker
                    .request(crate::diff_loader::Request {
                        id: current,
                        source: crate::diff_loader::Source::Commit {
                            revision: id.clone(),
                            file: path.into(),
                        },
                    });
            }
        }
        ControlCommand::SelectFile { path, staged } => {
            let Some(Ok(entries)) = &app.status else {
                return false;
            };
            let Some(index) = entries.iter().position(|entry| {
                entry.path == path.as_str() && (entry.section == Section::Staged) == *staged
            }) else {
                return false;
            };
            let entry = entries[index].clone();
            close_views(app);
            app.app_state.show(View::WorkingTree);
            app.status_selection.select(Some(index));
            open_worktree_diff(app, entry);
        }
    }
    true
}

/// True while the user is typing into a prompt or answering a dialog, which an
/// agent request must not interrupt.
fn in_prompt(app: &App) -> bool {
    app.searching
        || app.inline_search.is_some()
        || app.history_path_input.is_some()
        || app.branch_confirm.is_some()
        || app.branch_input.is_some()
        || app.branch_rename.is_some()
        || app.ignore_path_input.is_some()
        || app.remove_path_input.is_some()
        || app.remove_confirm.is_some()
        || app.move_path_input
        || app.move_confirm.is_some()
        || app.commit_message.is_some()
        || app.amend_confirm
        || app.discard_confirm.is_some()
        || app.untrack_confirm.is_some()
}

/// Back to the main views, as Esc would take the user.
fn close_views(app: &mut App) {
    app.help = false;
    app.branch_view = false;
    app.profiles_view = false;
    app.object_page = None;
    app.comparison_page = None;
    app.tag_page = None;
    app.sessions_page = None;
    app.details = None;
    app.diff = None;
    app.prepared_diff = None;
    app.ordinary_diff_worker.cancel();
    app.diff_entry = None;
}

/// Open the diff of a working-tree file, as Enter does.
pub(super) fn open_worktree_diff(app: &mut App, entry: Entry) {
    app.diff_entry = Some(entry.clone());
    app.request_id += 1;
    let current = app.request_id;
    app.diff = Some((current, None));
    app.prepared_diff = None;
    app.ordinary_diff_worker.cancel();
    app.diff_scroll = 0;
    app.selected_hunk = 0;
    app.ordinary_diff_worker
        .request(crate::diff_loader::Request {
            id: current,
            source: crate::diff_loader::Source::Worktree(entry),
        });
}

/// Load a commit's details off the loop, as Enter does in the history.
pub(super) fn open_commit_details(app: &mut App, root: &Path, id: String) {
    app.request_id += 1;
    let path = root.to_path_buf();
    let sender = app.details_sender.clone();
    let current = app.request_id;
    app.details = Some((current, None));
    std::thread::spawn(move || {
        let _ = sender.send((current, crate::details::load_commit(&path, &id)));
    });
}
