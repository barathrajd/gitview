use std::{io, time::Instant};

use crate::app::{
    AppAction, View,
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};
use crossterm::event::{KeyCode, KeyEvent};

pub(in crate::app) fn handle<H: Host>(
    app: &mut App,
    env: &mut Env<'_, H>,
    key: KeyEvent,
) -> io::Result<Flow> {
    let Env {
        terminal,
        root,
        profiles,
        ..
    } = env;
    if key.code == KeyCode::Char('H') && !app.help && !app.mutation_pending {
        let _ = app.app_state.handle_key(KeyCode::Char('H'), false);
        app.searching = false;
        app.diff = None;
        app.prepared_diff = None;
        app.ordinary_diff_worker.cancel();
        app.diff_entry = None;
        app.details = None;
        return Ok(Flow::Handled);
    }
    if key.code == KeyCode::Char('p')
        && !app.mutation_pending
        && app.details.is_none()
        && app.diff.is_none()
        && !app.branch_view
    {
        app.profile_selection.select(
            profiles
                .iter()
                .position(|profile| profile.name == app.profile_label),
        );
        app.profile_feedback.clear();
        app.profiles_view = true;
        return Ok(Flow::Handled);
    }
    if key.code != KeyCode::Char('c') {
        app.commit_feedback.clear();
    }
    let blocked = app.help || app.mutation_pending || app.details.is_some() || app.diff.is_some();
    if let Some(action) = app.app_state.handle_key(navigation_key(key), blocked) {
        match action {
            AppAction::LoadWorkingTree => {
                // Keep the cached working-tree status while switching panes.
                // The initial load, mutations, and explicit `r` refresh it.
            }
            AppAction::ReturnToHistory => {}
            AppAction::InspectObject => {
                let revision = app
                    .history
                    .as_ref()
                    .and_then(|rows| rows.as_ref().ok())
                    .and_then(|rows| app.selection.selected().and_then(|index| rows.get(index)))
                    .and_then(|row| crate::history::commit_id(row))
                    .unwrap_or("HEAD");
                app.object_page = Some(crate::object_page::ObjectPage::open(revision));
            }
            AppAction::OpenComparison => {
                let target = app
                    .history
                    .as_ref()
                    .and_then(|rows| rows.as_ref().ok())
                    .and_then(|rows| app.selection.selected().and_then(|index| rows.get(index)))
                    .and_then(|row| crate::history::commit_id(row))
                    .unwrap_or("HEAD");
                app.comparison_page = Some(crate::comparison_page::ComparisonPage::open(
                    target,
                    app.history_filter.as_deref(),
                ));
            }
            AppAction::StartPathHistory => {
                app.history_path_input = Some(
                    app.history_filter
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );
            }
            AppAction::StartSearch => {
                app.searching = true;
                app.query.clear();
                app.search_results = app
                    .history
                    .as_ref()
                    .and_then(|result| result.as_ref().ok())
                    .map(|rows| crate::history::search_matches(rows, ""))
                    .unwrap_or_default();
                let selected = app.selection.selected().unwrap_or(0);
                let result_selection = app
                    .search_results
                    .iter()
                    .position(|index| *index == selected)
                    .unwrap_or(0);
                app.search_selection
                    .select((!app.search_results.is_empty()).then_some(result_selection));
                app.search_debounce = app.history.is_none().then_some(Instant::now());
            }
            AppAction::OpenBranches => {
                app.branch_view = true;
                app.branches = None;
                app.branch_feedback.clear();
                app.branch_request += 1;
                let id = app.branch_request;
                let sender = app.branch_sender.clone();
                let path = root.to_path_buf();
                std::thread::spawn(move || {
                    let _ = sender.send((id, crate::branches::load_branches(&path)));
                });
            }
            AppAction::StartCommit | AppAction::StartAmend => {
                let amend = action == AppAction::StartAmend;
                app.commit_preview_retry = false;
                app.commit_preview_mode = Some(amend);
                app.commit_feedback.clear();
                app.commit_preview_worker.request(amend);
            }
            AppAction::PreviewDiscardSelected => {
                if let (Some(Ok(entries)), Some(index)) =
                    (&app.status, app.status_selection.selected())
                    && let Some(entry) = entries.get(index)
                {
                    app.pending_preview = Some(crate::preview::Context::Discard);
                    app.commit_feedback.clear();
                    app.preview_worker
                        .request(crate::preview::Request::Discard(entry.clone()));
                }
            }
            AppAction::PreviewUntrackSelected => {
                if let (Some(Ok(entries)), Some(index)) =
                    (&app.status, app.status_selection.selected())
                    && let Some(entry) = entries.get(index)
                {
                    app.pending_preview = Some(crate::preview::Context::Untrack);
                    app.commit_feedback.clear();
                    app.preview_worker
                        .request(crate::preview::Request::Untrack(entry.clone()));
                }
            }
            AppAction::InspectIgnoreRule => {
                if let (Some(Ok(entries)), Some(index)) =
                    (&app.status, app.status_selection.selected())
                    && let Some(entry) = entries.get(index)
                {
                    app.pending_preview = Some(crate::preview::Context::IgnoreSelected);
                    app.commit_feedback.clear();
                    app.preview_worker
                        .request(crate::preview::Request::IgnoreSelected(entry.clone()));
                }
            }
            AppAction::StartIgnorePathInput => {
                app.ignore_path_input = Some(String::new());
                app.commit_feedback.clear();
            }
            AppAction::StartRemovePathInput => {
                app.remove_path_input = Some(String::new());
                app.commit_feedback.clear();
            }
            AppAction::StartMovePathInput => {
                app.move_source.clear();
                app.move_destination.clear();
                app.move_destination_focused = false;
                app.move_path_input = true;
                app.commit_feedback.clear();
            }
            AppAction::StageAll => {
                app.status_request += 1;
                let id = app.status_request;
                let sender = app.status_sender.clone();
                let path = root.to_path_buf();
                app.mutation_pending = true;
                app.status_mutation = Some(id);
                std::thread::spawn(move || {
                    let result = crate::worktree::stage_all(&path);
                    let _ = sender.send((
                        id,
                        result.and_then(|()| crate::worktree::load_status(&path)),
                    ));
                });
            }
            AppAction::UnstageAll => {
                app.status_request += 1;
                let id = app.status_request;
                let sender = app.status_sender.clone();
                let path = root.to_path_buf();
                app.mutation_pending = true;
                app.status_mutation = Some(id);
                std::thread::spawn(move || {
                    let result = crate::worktree::unstage_all(&path);
                    let _ = sender.send((
                        id,
                        result.and_then(|()| crate::worktree::load_status(&path)),
                    ));
                });
            }
            AppAction::StageSelected | AppAction::UnstageSelected => {
                if let (Some(Ok(entries)), Some(index)) =
                    (&app.status, app.status_selection.selected())
                    && let Some(entry) = entries.get(index)
                {
                    let entry = entry.clone();
                    let stage = action == AppAction::StageSelected;
                    let eligible = if stage {
                        entry.section != crate::worktree::Section::Staged
                    } else {
                        entry.section == crate::worktree::Section::Staged
                    };
                    if eligible {
                        app.status_request += 1;
                        let id = app.status_request;
                        let sender = app.status_sender.clone();
                        let path = root.to_path_buf();
                        app.mutation_pending = true;
                        app.status_mutation = Some(id);
                        std::thread::spawn(move || {
                            let result = if stage {
                                crate::worktree::stage(&path, &entry)
                            } else {
                                crate::worktree::unstage(&path, &entry)
                            };
                            let _ = sender.send((
                                id,
                                result.and_then(|()| crate::worktree::load_status(&path)),
                            ));
                        });
                    }
                }
            }
            AppAction::RefreshWorkingTree => {
                app.status = None;
                app.status_request += 1;
                let id = app.status_request;
                let sender = app.status_sender.clone();
                let path = root.to_path_buf();
                std::thread::spawn(move || {
                    let _ = sender.send((id, crate::worktree::load_status(&path)));
                });
            }
            AppAction::MoveUp => match app.app_state.view() {
                View::History => app
                    .selection
                    .select(crate::app::move_up(app.selection.selected())),
                View::WorkingTree => app
                    .status_selection
                    .select(crate::app::move_up(app.status_selection.selected())),
            },
            AppAction::MoveDown => match app.app_state.view() {
                View::History => {
                    if let Some(Ok(rows)) = &app.history {
                        app.selection
                            .select(crate::app::move_down(app.selection.selected(), rows.len()));
                    }
                }
                View::WorkingTree => {
                    if let Some(Ok(entries)) = &app.status {
                        app.status_selection.select(crate::app::move_down(
                            app.status_selection.selected(),
                            entries.len(),
                        ));
                    }
                }
            },
        }
        return Ok(Flow::Handled);
    }
    match navigation_key(key) {
        KeyCode::Char('q') if !app.mutation_pending => return Ok(Flow::Quit),
        KeyCode::Esc if app.help => app.help = false,
        KeyCode::Char('?') => app.help = !app.help,
        KeyCode::Tab if !app.help && app.diff.is_some() => {
            app.diff_side_by_side = !app.diff_side_by_side;
            app.diff_scroll = 0;
        }
        KeyCode::Tab
        | KeyCode::Char('b')
        | KeyCode::Char('/')
        | KeyCode::Char('n')
        | KeyCode::Char('r') => {}
        KeyCode::Esc if !app.help => {
            if app.diff.is_some() {
                app.diff = None;
                app.prepared_diff = None;
                app.ordinary_diff_worker.cancel();
                app.diff_entry = None;
            } else {
                app.details = None;
            }
        }
        KeyCode::Char('v') if !app.help && app.diff.is_some() => {
            app.diff_raw_view = !app.diff_raw_view;
            app.diff_scroll = 0;
        }
        KeyCode::Enter
            if !app.help && app.app_state.view() == View::WorkingTree && app.diff.is_none() =>
        {
            if let (Some(Ok(entries)), Some(index)) = (&app.status, app.status_selection.selected())
                && let Some(entry) = entries.get(index)
            {
                let entry = entry.clone();
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
        }
        KeyCode::Enter
            if !app.help && app.details.is_none() && app.app_state.view() == View::History =>
        {
            if let (Some(Ok(rows)), Some(index)) = (&app.history, app.selection.selected())
                && let Some(id) = rows
                    .get(index)
                    .and_then(|row| crate::history::commit_id(row))
            {
                app.request_id += 1;
                let id = id.to_owned();
                let path = root.to_path_buf();
                let sender = app.details_sender.clone();
                let current = app.request_id;
                app.details = Some((current, None));
                std::thread::spawn(move || {
                    let _ = sender.send((current, crate::details::load_commit(&path, &id)));
                });
            }
        }
        KeyCode::Enter if !app.help && app.diff.is_none() => {
            if let (Some((_, Some(Ok(commit)))), Some(index)) =
                (&app.details, app.file_selection.selected())
                && let Some(file) = commit.files.get(index)
            {
                app.request_id += 1;
                let current = app.request_id;
                let hash = commit.hash.clone();
                let file = file.clone();
                app.diff = Some((current, None));
                app.prepared_diff = None;
                app.ordinary_diff_worker.cancel();
                app.diff_scroll = 0;
                app.diff_entry = None;
                app.selected_hunk = 0;
                app.ordinary_diff_worker
                    .request(crate::diff_loader::Request {
                        id: current,
                        source: crate::diff_loader::Source::Commit {
                            revision: hash,
                            file,
                        },
                    });
            }
        }
        KeyCode::Down if !app.help && app.diff.is_some() => {
            app.diff_scroll = app.diff_scroll.saturating_add(1);
        }
        KeyCode::Up if !app.help && app.diff.is_some() => {
            app.diff_scroll = app.diff_scroll.saturating_sub(1);
        }
        KeyCode::Char(']')
            if !app.help
                && app
                    .diff_entry
                    .as_ref()
                    .is_some_and(|entry| entry.section != crate::worktree::Section::Untracked) =>
        {
            if let Some((_, Some(Ok(text)))) = &app.diff {
                let positions = hunk_line_positions(
                    app.prepared_diff.as_ref(),
                    text,
                    app.diff_raw_view,
                    app.diff_side_by_side,
                    terminal.size().map_or(80, |size| size.width),
                );
                if !positions.is_empty() {
                    app.selected_hunk = (app.selected_hunk + 1).min(positions.len() - 1);
                    app.diff_scroll = positions[app.selected_hunk] as u16;
                }
            }
        }
        KeyCode::Char('[')
            if !app.help
                && app
                    .diff_entry
                    .as_ref()
                    .is_some_and(|entry| entry.section != crate::worktree::Section::Untracked) =>
        {
            if let Some((_, Some(Ok(text)))) = &app.diff {
                let positions = hunk_line_positions(
                    app.prepared_diff.as_ref(),
                    text,
                    app.diff_raw_view,
                    app.diff_side_by_side,
                    terminal.size().map_or(80, |size| size.width),
                );
                if !positions.is_empty() {
                    app.selected_hunk = app.selected_hunk.saturating_sub(1);
                    app.diff_scroll = positions[app.selected_hunk] as u16;
                }
            }
        }
        KeyCode::Char('s' | 'u' | 'S' | 'U') if !app.help && !app.mutation_pending => {
            let stage = matches!(key.code, KeyCode::Char('s' | 'S'));
            let all_hunks = matches!(key.code, KeyCode::Char('S' | 'U'));
            if let (Some(entry), Some((_, Some(Ok(text))))) = (&app.diff_entry, &app.diff)
                && ((stage
                    && (entry.section == crate::worktree::Section::Unstaged
                        || (all_hunks && entry.section == crate::worktree::Section::Untracked)))
                    || (!stage && entry.section == crate::worktree::Section::Staged))
                && (all_hunks
                    || app.selected_hunk
                        < hunk_line_positions(
                            app.prepared_diff.as_ref(),
                            text,
                            app.diff_raw_view,
                            app.diff_side_by_side,
                            terminal.size().map_or(80, |size| size.width),
                        )
                        .len())
            {
                let entry = entry.clone();
                let hunk = app.selected_hunk;
                let preferred_section = if stage {
                    if all_hunks {
                        crate::worktree::Section::Staged
                    } else {
                        crate::worktree::Section::Unstaged
                    }
                } else if all_hunks {
                    crate::worktree::Section::Unstaged
                } else {
                    crate::worktree::Section::Staged
                };
                let fallback_section = if stage {
                    crate::worktree::Section::Staged
                } else {
                    crate::worktree::Section::Unstaged
                };
                app.diff_entry_refresh = Some((preferred_section, fallback_section));
                app.status_request += 1;
                let status_id = app.status_request;
                app.status_mutation = Some(status_id);
                app.mutation_pending = true;
                let diff_id = app.diff.as_ref().map(|(id, _)| *id).unwrap_or_default();
                let status_sender = app.status_sender.clone();
                let diff_sender = app.diff_sender.clone();
                let path = root.to_path_buf();
                std::thread::spawn(move || {
                    let result = if all_hunks && stage {
                        crate::worktree::stage(&path, &entry)
                    } else if all_hunks {
                        crate::worktree::unstage(&path, &entry)
                    } else if stage {
                        crate::worktree::stage_hunk(&path, &entry, hunk)
                    } else {
                        crate::worktree::unstage_hunk(&path, &entry, hunk)
                    };
                    let status_result = result.and_then(|()| crate::worktree::load_status(&path));
                    let diff_result = match &status_result {
                        Ok(entries) => entries
                            .iter()
                            .find(|current| {
                                current.section == preferred_section
                                    && current.path == entry.path
                                    && current.original == entry.original
                            })
                            .or_else(|| {
                                entries.iter().find(|current| {
                                    current.section == fallback_section
                                        && current.path == entry.path
                                        && current.original == entry.original
                                })
                            })
                            .map_or_else(
                                || Ok(String::new()),
                                |current| crate::worktree::load_diff(&path, current),
                            ),
                        Err(error) => Err(error.clone()),
                    };
                    let _ = status_sender.send((status_id, status_result));
                    let _ = diff_sender.send((diff_id, diff_result));
                });
            }
        }
        KeyCode::Down if !app.help && app.details.is_some() => {
            if let Some((_, Some(Ok(commit)))) = &app.details
                && !commit.files.is_empty()
            {
                app.file_selection.select(Some(
                    (app.file_selection.selected().unwrap_or(0) + 1).min(commit.files.len() - 1),
                ));
            }
        }
        KeyCode::Up if !app.help && app.details.is_some() => {
            if let Some(i) = app.file_selection.selected() {
                app.file_selection.select(Some(i.saturating_sub(1)));
            }
        }
        _ => {}
    }
    Ok(Flow::Next)
}
