use std::{
    io,
    sync::mpsc::{self, RecvTimeoutError},
    time::{Duration, Instant},
};

use crate::{
    app::{AppAction, AppState, BranchAction, View},
    ui::{self, ASCII_BORDER, Screen},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::Backend, widgets::ListState};

pub use super::helpers::Navigation;
use super::{
    helpers::{
        CommitReadResult, backend_error, combined_commit_message, commit_after_editor,
        edit_profile_registry, hunk_line_positions, navigation_key, normalize_commit_paste,
        reload_profile_context, selected_search_commit, split_commit_message,
        update_search_results,
    },
    model::App,
};

/// What `run_app` needs from the terminal it runs in. The real terminal and the
/// in-memory test backend each provide one.
pub trait Host {
    type Backend: Backend;

    /// Waits up to `timeout` for the next input event. `terminal` shows the
    /// last frame, so a test host can inspect the screen.
    fn next_event(
        &mut self,
        terminal: &Terminal<Self::Backend>,
        timeout: Duration,
    ) -> Result<Event, RecvTimeoutError>;

    /// Runs an interactive external program with the terminal handed back to it.
    fn run_external<T>(
        &mut self,
        terminal: &mut Terminal<Self::Backend>,
        program: impl FnOnce() -> T,
    ) -> io::Result<T>;
}

pub fn run_app<H: Host>(
    terminal: &mut Terminal<H::Backend>,
    host: &mut H,
    root: &std::path::Path,
    ascii: bool,
    mut profile_label: String,
    config_dir: &std::path::Path,
    profiles: &mut Vec<crate::config::FolderProfile>,
) -> io::Result<()> {
    let border = if ascii {
        ASCII_BORDER
    } else {
        ratatui::symbols::border::PLAIN
    };
    let help = false;
    let searching = false;
    let history_filter: Option<std::path::PathBuf> = None;
    let history_path_input: Option<String> = None;
    let query = String::new();
    let search_results = Vec::new();
    let search_selection = ListState::default();
    let search_debounce: Option<Instant> = None;
    let (history_sender, receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<String>, String>)>();
    let history_request = 0u64;
    let sender = history_sender.clone();
    let path = root.to_path_buf();
    let requested_history_path = history_filter.clone();
    std::thread::spawn(move || {
        let _ = sender.send((
            0,
            crate::history::load_history_for_path(&path, requested_history_path.as_deref()),
        ));
    });
    let history: Option<Result<Vec<String>, String>> = None;
    let selection = ListState::default();
    let (details_sender, details_receiver) =
        std::sync::mpsc::channel::<(u64, Result<crate::details::CommitDetails, String>)>();
    let details: Option<(u64, Option<Result<crate::details::CommitDetails, String>>)> = None;
    let request_id = 0u64;
    let file_selection = ListState::default();
    let (diff_sender, diff_receiver) = std::sync::mpsc::channel::<(u64, Result<String, String>)>();
    let diff: Option<(u64, Option<Result<String, String>>)> = None;
    let diff_root = root.to_path_buf();
    let ordinary_diff_worker = crate::read_worker::LatestRead::new(move |request| {
        crate::diff_loader::load(&diff_root, request)
    });
    let prepared_diff: Option<ui::PreparedDiff> = None;
    let diff_scroll = 0u16;
    let diff_raw_view = false;
    let diff_side_by_side = true;
    let diff_entry: Option<crate::worktree::Entry> = None;
    let selected_hunk = 0usize;
    let branch_view = false;
    let object_page: Option<crate::object_page::ObjectPage> = None;
    let comparison_page: Option<crate::comparison_page::ComparisonPage> = None;
    let tag_page: Option<crate::tag_page::TagPage> = None;
    let profiles_view = false;
    let profile_selection = ListState::default();
    let profile_feedback = String::new();
    let profile_registry_path = config_dir.join("profiles.json");
    let (branch_sender, branch_receiver) =
        std::sync::mpsc::channel::<(u64, Result<crate::branches::Branches, String>)>();
    let branch_request = 0u64;
    let (operation_sender, operation_receiver) =
        std::sync::mpsc::channel::<(String, Result<(), String>)>();
    let branch_confirm: Option<BranchAction> = None;
    let branch_input: Option<String> = None;
    let branch_rename: Option<crate::branches::BranchPreview> = None;
    let branch_feedback = String::new();
    let ignore_path_input: Option<String> = None;
    let remove_path_input: Option<String> = None;
    let remove_confirm: Option<crate::worktree::RemovePathPreview> = None;
    let move_path_input = false;
    let move_source = String::new();
    let move_destination = String::new();
    let move_destination_focused = false;
    let move_confirm: Option<crate::worktree::MovePathPreview> = None;
    let preview_root = root.to_path_buf();
    let preview_worker = crate::read_worker::LatestRead::new(move |request| {
        crate::preview::prepare(&preview_root, request)
    });
    let pending_preview: Option<crate::preview::Context> = None;
    let commit_preview_retry = false;
    let commit_preview_mode: Option<bool> = None;
    let preview_root = root.to_path_buf();
    let commit_preview_worker = crate::read_worker::LatestRead::new(move |amend: bool| {
        if amend {
            CommitReadResult::Amend(crate::commit::preview_amend(&preview_root))
        } else {
            CommitReadResult::Create(crate::commit::preview_commit(&preview_root))
        }
    });
    let commit_message: Option<String> = None;
    let commit_description = String::new();
    let commit_description_focused = false;
    let commit_amend = false;
    let amend_confirm = false;
    let commit_expected: Option<crate::commit::CommitPreview> = None;
    let amend_expected: Option<crate::commit::AmendPreview> = None;
    let commit_feedback = String::new();
    let (commit_sender, commit_receiver) =
        mpsc::channel::<(String, String, bool, Result<(), String>)>();
    let discard_confirm: Option<crate::worktree::DiscardPreview> = None;
    let (discard_sender, discard_receiver) = mpsc::channel::<Result<(), String>>();
    let untrack_confirm: Option<crate::worktree::UntrackPreview> = None;
    let (untrack_sender, untrack_receiver) = mpsc::channel::<Result<(), String>>();
    let (remove_sender, remove_receiver) = mpsc::channel::<Result<(), String>>();
    let (move_sender, move_receiver) = mpsc::channel::<Result<(), String>>();
    let mutation_pending = false;
    let status_mutation: Option<u64> = None;
    let diff_entry_refresh: Option<(crate::worktree::Section, crate::worktree::Section)> = None;
    let branches: Option<Result<crate::branches::Branches, String>> = None;
    let branch_selection = ListState::default();
    let app_state = AppState::default();
    let (status_sender, status_receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<crate::worktree::Entry>, String>)>();
    let (status_watch_sender, status_watch_receiver) =
        std::sync::mpsc::sync_channel::<Vec<crate::worktree::Entry>>(1);
    let status: Option<Result<Vec<crate::worktree::Entry>, String>> = None;
    let status_request = 0u64;
    let status_selection = ListState::default();
    {
        let sender = status_sender.clone();
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send((0, crate::worktree::load_status(&path)));
        });
    }
    {
        let sender = status_watch_sender;
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let mut previous = None;
            loop {
                std::thread::sleep(Duration::from_millis(750));
                let Ok(entries) = crate::worktree::load_status(&path) else {
                    continue;
                };
                if previous.as_ref() != Some(&entries) {
                    match sender.try_send(entries.clone()) {
                        Ok(()) => previous = Some(entries),
                        Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
                    }
                }
            }
        });
    }
    {
        let sender = branch_sender.clone();
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send((0, crate::branches::load_branches(&path)));
        });
    }
    let mut app = App {
        border,
        help,
        searching,
        history_filter,
        history_path_input,
        query,
        search_results,
        search_selection,
        search_debounce,
        history_sender,
        receiver,
        history_request,
        history,
        selection,
        details_sender,
        details_receiver,
        details,
        request_id,
        file_selection,
        diff_sender,
        diff_receiver,
        diff,
        ordinary_diff_worker,
        prepared_diff,
        diff_scroll,
        diff_raw_view,
        diff_side_by_side,
        diff_entry,
        selected_hunk,
        branch_view,
        object_page,
        comparison_page,
        tag_page,
        profiles_view,
        profile_selection,
        profile_feedback,
        profile_registry_path,
        branch_sender,
        branch_receiver,
        branch_request,
        operation_sender,
        operation_receiver,
        branch_confirm,
        branch_input,
        branch_rename,
        branch_feedback,
        ignore_path_input,
        remove_path_input,
        remove_confirm,
        move_path_input,
        move_source,
        move_destination,
        move_destination_focused,
        move_confirm,
        preview_worker,
        pending_preview,
        commit_preview_retry,
        commit_preview_mode,
        commit_preview_worker,
        commit_message,
        commit_description,
        commit_description_focused,
        commit_amend,
        amend_confirm,
        commit_expected,
        amend_expected,
        commit_feedback,
        commit_sender,
        commit_receiver,
        discard_confirm,
        discard_sender,
        discard_receiver,
        untrack_confirm,
        untrack_sender,
        untrack_receiver,
        remove_sender,
        remove_receiver,
        move_sender,
        move_receiver,
        mutation_pending,
        status_mutation,
        diff_entry_refresh,
        branches,
        branch_selection,
        app_state,
        status_sender,
        status_receiver,
        status_watch_receiver,
        status,
        status_request,
        status_selection,
        needs_redraw: true,
    };
    loop {
        if let Some(page) = &mut app.object_page {
            app.needs_redraw |= page.tick();
        }
        if let Some(page) = &mut app.comparison_page {
            app.needs_redraw |= page.tick();
        }
        if let Some(page) = &mut app.tag_page {
            app.needs_redraw |= page.tick();
        }
        while let Ok(entries) = app.status_watch_receiver.try_recv() {
            app.needs_redraw = true;
            if app.mutation_pending {
                continue;
            }
            let unchanged = app
                .status
                .as_ref()
                .and_then(|current| current.as_ref().ok())
                .is_some_and(|current| current == &entries);
            if unchanged {
                continue;
            }
            let selected_entry = app
                .status
                .as_ref()
                .and_then(|current| current.as_ref().ok())
                .and_then(|current| app.status_selection.selected().and_then(|i| current.get(i)))
                .cloned();
            let selected = selected_entry
                .and_then(|selected| entries.iter().position(|entry| entry == &selected))
                .or_else(|| {
                    (!entries.is_empty()).then(|| {
                        app.status_selection
                            .selected()
                            .unwrap_or(0)
                            .min(entries.len() - 1)
                    })
                });
            app.status_selection.select(selected);
            app.status = Some(Ok(entries));
        }
        while let Ok((id, result)) = app.receiver.try_recv() {
            app.needs_redraw = true;
            if id != app.history_request {
                continue;
            }
            if result.as_ref().is_ok_and(|rows| !rows.is_empty()) {
                app.selection.select(Some(0));
            }
            app.history = Some(result);
            if app.searching {
                app.search_debounce = Some(Instant::now());
            }
        }
        if app.searching
            && app
                .search_debounce
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            update_search_results(
                app.history.as_ref(),
                &app.query,
                &mut app.search_results,
                &mut app.search_selection,
            );
            app.search_debounce = None;
            app.needs_redraw = true;
        }
        while let Ok((id, result)) = app.details_receiver.try_recv() {
            app.needs_redraw = true;
            if let Some((active, value)) = &mut app.details
                && *active == id
            {
                if result.as_ref().is_ok_and(|commit| !commit.files.is_empty()) {
                    app.file_selection.select(Some(0));
                }
                *value = Some(result);
            }
        }
        while let Ok((id, result)) = app.diff_receiver.try_recv() {
            app.needs_redraw = true;
            if let Some((active, value)) = &mut app.diff
                && *active == id
            {
                app.prepared_diff = None;
                match result {
                    Ok(text) => {
                        *value = None;
                        app.ordinary_diff_worker
                            .request(crate::diff_loader::Request {
                                id,
                                source: crate::diff_loader::Source::Loaded {
                                    text,
                                    path: app
                                        .diff_entry
                                        .as_ref()
                                        .map(|entry| entry.path.clone().into()),
                                },
                            });
                    }
                    Err(error) => *value = Some(Err(error)),
                }
            }
        }
        if let Some((id, result)) = app.ordinary_diff_worker.take_result()
            && let Some((active, value)) = &mut app.diff
            && *active == id
        {
            app.needs_redraw = true;
            match result {
                Ok(prepared) => {
                    *value = Some(Ok(prepared.text().to_owned()));
                    app.prepared_diff = Some(prepared);
                }
                Err(error) => {
                    *value = Some(Err(error));
                    app.prepared_diff = None;
                }
            }
            app.selected_hunk = 0;
            app.diff_scroll = 0;
        }
        while let Ok((id, result)) = app.status_receiver.try_recv() {
            app.needs_redraw = true;
            let is_mutation = app.status_mutation == Some(id);
            if is_mutation {
                app.mutation_pending = false;
                app.status_mutation = None;
                if let (Some(entry), Some((preferred, fallback)), Ok(entries)) =
                    (&app.diff_entry, app.diff_entry_refresh, &result)
                {
                    app.diff_entry = entries
                        .iter()
                        .find(|current| {
                            current.section == preferred
                                && current.path == entry.path
                                && current.original == entry.original
                        })
                        .or_else(|| {
                            entries.iter().find(|current| {
                                current.section == fallback
                                    && current.path == entry.path
                                    && current.original == entry.original
                            })
                        })
                        .cloned();
                }
                app.diff_entry_refresh = None;
            }
            if id != app.status_request {
                continue;
            }
            // Keep the last known-good list visible if a Git mutation fails.
            if is_mutation && result.is_err() {
                continue;
            }
            if let Ok(entries) = &result {
                let previous_entry = app
                    .status
                    .as_ref()
                    .and_then(|current| current.as_ref().ok())
                    .and_then(|current| {
                        app.status_selection
                            .selected()
                            .and_then(|index| current.get(index))
                    });
                let prefer_staged = previous_entry
                    .is_some_and(|previous| previous.section == crate::worktree::Section::Staged);
                let preferred_index = entries
                    .iter()
                    .position(|entry| {
                        let in_preferred_section = if prefer_staged {
                            entry.section == crate::worktree::Section::Staged
                        } else {
                            entry.section != crate::worktree::Section::Staged
                        };
                        in_preferred_section
                            && previous_entry.is_some_and(|previous| {
                                entry.path == previous.path && entry.original == previous.original
                            })
                    })
                    .or_else(|| {
                        entries.iter().position(|entry| {
                            if prefer_staged {
                                entry.section == crate::worktree::Section::Staged
                            } else {
                                entry.section != crate::worktree::Section::Staged
                            }
                        })
                    });
                let fallback_index = entries.iter().position(|entry| {
                    if prefer_staged {
                        entry.section != crate::worktree::Section::Staged
                    } else {
                        entry.section == crate::worktree::Section::Staged
                    }
                });
                let same_entry = previous_entry.and_then(|previous| {
                    entries.iter().position(|entry| {
                        entry.section == previous.section
                            && entry.path == previous.path
                            && entry.original == previous.original
                    })
                });
                app.status_selection.select(if entries.is_empty() {
                    None
                } else if is_mutation {
                    preferred_index
                        .or(fallback_index)
                        .or(same_entry)
                        .or(Some(0))
                } else {
                    same_entry.or(Some(0))
                });
            } else {
                app.status_selection.select(None);
            }
            app.status = Some(result);
        }
        while let Ok((id, result)) = app.branch_receiver.try_recv() {
            app.needs_redraw = true;
            if id != app.branch_request {
                continue;
            }
            if result.as_ref().is_ok_and(|data| !data.branches.is_empty()) {
                app.branch_selection.select(Some(0));
            } else {
                app.branch_selection.select(None);
            }
            app.branches = Some(result);
        }
        while let Ok((success_message, result)) = app.operation_receiver.try_recv() {
            app.needs_redraw = true;
            app.mutation_pending = false;
            app.branch_feedback = match result {
                Ok(()) => success_message,
                Err(error) => {
                    format!("{error} (branch state refreshed; a hook may have changed HEAD)")
                }
            };
            app.branch_confirm = None;
            app.request_id += 1;
            app.details = None;
            app.diff = None;
            app.prepared_diff = None;
            app.ordinary_diff_worker.cancel();
            app.history_request += 1;
            app.history = None;
            let id = app.history_request;
            let sender = app.history_sender.clone();
            let path = root.to_path_buf();
            let requested_history_path = app.history_filter.clone();
            std::thread::spawn(move || {
                let _ = sender.send((
                    id,
                    crate::history::load_history_for_path(&path, requested_history_path.as_deref()),
                ));
            });
            app.status_request += 1;
            app.status = None;
            let id = app.status_request;
            let sender = app.status_sender.clone();
            let path = root.to_path_buf();
            std::thread::spawn(move || {
                let _ = sender.send((id, crate::worktree::load_status(&path)));
            });
            app.branch_request += 1;
            app.branches = None;
            let id = app.branch_request;
            let sender = app.branch_sender.clone();
            let path = root.to_path_buf();
            std::thread::spawn(move || {
                let _ = sender.send((id, crate::branches::load_branches(&path)));
            });
        }
        while let Ok((message, description, was_amend, result)) = app.commit_receiver.try_recv() {
            app.needs_redraw = true;
            app.mutation_pending = false;
            app.amend_confirm = false;
            match result {
                Ok(()) => {
                    app.commit_message = None;
                    app.commit_description.clear();
                    app.commit_description_focused = false;
                    app.commit_expected = None;
                    app.amend_expected = None;
                    app.commit_amend = false;
                    app.commit_feedback = if was_amend {
                        "Commit amended; HEAD now has a new ID.".into()
                    } else {
                        "Commit created.".into()
                    };
                    app.history_request += 1;
                    app.history = None;
                    let id = app.history_request;
                    let sender = app.history_sender.clone();
                    let path = root.to_path_buf();
                    let requested_history_path = app.history_filter.clone();
                    std::thread::spawn(move || {
                        let _ = sender.send((
                            id,
                            crate::history::load_history_for_path(
                                &path,
                                requested_history_path.as_deref(),
                            ),
                        ));
                    });
                    app.status_request += 1;
                    app.status = None;
                    app.status_mutation = Some(app.status_request);
                    let id = app.status_request;
                    let sender = app.status_sender.clone();
                    let path = root.to_path_buf();
                    std::thread::spawn(move || {
                        let result = crate::worktree::load_status(&path);
                        let _ = sender.send((id, result));
                    });
                }
                Err(error) => {
                    app.commit_feedback = error;
                    app.commit_message = Some(message);
                    app.commit_description = description;
                    app.commit_amend = was_amend;
                    app.amend_expected = None;
                    app.commit_expected = None;
                    app.commit_preview_retry = true;
                    app.commit_preview_mode = Some(was_amend);
                    app.commit_preview_worker.request(was_amend);
                }
            }
        }
        while let Ok(result) = app.discard_receiver.try_recv() {
            app.needs_redraw = true;
            app.mutation_pending = false;
            app.discard_confirm = None;
            app.commit_feedback = match result {
                Ok(()) => "Selected file discarded.".into(),
                Err(error) => error,
            };
            app.status_request += 1;
            let id = app.status_request;
            app.status_mutation = Some(id);
            let sender = app.status_sender.clone();
            let path = root.to_path_buf();
            app.status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, crate::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = app.untrack_receiver.try_recv() {
            app.needs_redraw = true;
            app.mutation_pending = false;
            app.untrack_confirm = None;
            app.commit_feedback = match result {
                Ok(()) => "Stopped tracking the selected file; kept its working copy.".into(),
                Err(error) => error,
            };
            app.status_request += 1;
            let id = app.status_request;
            app.status_mutation = Some(id);
            let sender = app.status_sender.clone();
            let path = root.to_path_buf();
            app.status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, crate::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = app.remove_receiver.try_recv() {
            app.needs_redraw = true;
            app.mutation_pending = false;
            app.remove_confirm = None;
            app.commit_feedback = match result {
                Ok(()) => "Removed the tracked path and staged its deletion.".into(),
                Err(error) => error,
            };
            app.status_request += 1;
            let id = app.status_request;
            app.status_mutation = Some(id);
            let sender = app.status_sender.clone();
            let path = root.to_path_buf();
            app.status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, crate::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = app.move_receiver.try_recv() {
            app.needs_redraw = true;
            app.mutation_pending = false;
            app.move_confirm = None;
            app.move_path_input = false;
            app.commit_feedback = match result {
                Ok(()) => "Moved the tracked path and staged its rename.".into(),
                Err(error) => error,
            };
            app.status_request += 1;
            let id = app.status_request;
            app.status_mutation = Some(id);
            let sender = app.status_sender.clone();
            let path = root.to_path_buf();
            app.status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, crate::worktree::load_status(&path)));
            });
        }
        if let Some(result) = app.preview_worker.take_result()
            && let Some(context) = app.pending_preview.take()
        {
            app.needs_redraw = true;
            use crate::preview::{Context, Value};
            match result {
                Ok(Value::Discard(preview)) => app.discard_confirm = Some(preview),
                Ok(Value::Untrack(preview)) => app.untrack_confirm = Some(preview),
                Ok(Value::Remove(preview)) => {
                    app.remove_path_input = None;
                    app.remove_confirm = Some(preview);
                }
                Ok(Value::Move(preview)) => {
                    app.move_path_input = false;
                    app.move_confirm = Some(preview);
                }
                Ok(Value::Previous(preview)) => {
                    app.branch_confirm = Some(BranchAction::Previous(preview))
                }
                Ok(Value::Branch(preview)) => {
                    if matches!(context, Context::BranchRename) {
                        app.branch_rename = Some(preview);
                        app.branch_input = Some(String::new());
                    } else {
                        app.branch_confirm = Some(BranchAction::Delete(preview));
                    }
                }
                Ok(Value::Ignore(rule)) => {
                    if matches!(context, Context::IgnorePath) {
                        app.ignore_path_input = None;
                    }
                    app.commit_feedback = match rule {
                        Some(rule) => format!(
                            "Ignored by {}:{} ({:?}).",
                            rule.source, rule.line, rule.pattern
                        ),
                        None => "No ignore rule matches the selected path.".into(),
                    };
                }
                Err(error) => {
                    if context.is_branch() {
                        app.branch_feedback = error;
                    } else {
                        app.commit_feedback = error;
                    }
                }
            }
        }
        if let Some(result) = app.commit_preview_worker.take_result()
            && app.commit_preview_mode.take().is_some()
        {
            app.needs_redraw = true;
            let retry = std::mem::take(&mut app.commit_preview_retry);
            match result {
                CommitReadResult::Create(result) => match result {
                    Ok(preview) => {
                        app.commit_expected = Some(preview);
                        app.amend_expected = None;
                        app.amend_confirm = false;
                        app.commit_amend = false;
                        if !retry {
                            app.commit_feedback.clear();
                            app.commit_description.clear();
                            app.commit_description_focused = false;
                            app.commit_message = Some(String::new());
                        }
                    }
                    Err(error) => {
                        app.commit_feedback = if retry {
                            format!("{}\nPreview refresh failed: {error}", app.commit_feedback)
                        } else {
                            error
                        };
                    }
                },
                CommitReadResult::Amend(result) => match result {
                    Ok(preview) => {
                        let (subject, description) = split_commit_message(&preview.message);
                        app.amend_expected = Some(preview);
                        app.commit_expected = None;
                        app.amend_confirm = false;
                        app.commit_amend = true;
                        if !retry {
                            app.commit_feedback.clear();
                            app.commit_description = description;
                            app.commit_description_focused = false;
                            app.commit_message = Some(subject);
                        }
                    }
                    Err(error) => {
                        app.commit_feedback = if retry {
                            format!("{}\nPreview refresh failed: {error}", app.commit_feedback)
                        } else {
                            error
                        }
                    }
                },
            }
        }
        if app.needs_redraw {
            terminal
                .draw(|frame| {
                    let workspace_feedback = if app.app_state.view() == View::History {
                        app.history_filter
                            .as_ref()
                            .map(|path| {
                                format!(
                                    "History path: {} · f change/clear. {}",
                                    path.display(),
                                    app.commit_feedback
                                )
                            })
                            .unwrap_or_else(|| app.commit_feedback.clone())
                    } else {
                        app.commit_feedback.clone()
                    };
                    let composed_message = combined_commit_message(
                        app.commit_message.as_deref().unwrap_or_default(),
                        &app.commit_description,
                    );
                    let help_page = if app.profiles_view {
                        ui::HelpPage::Profiles
                    } else if app.amend_confirm {
                        ui::HelpPage::AmendConfirmation
                    } else if app.commit_message.is_some() {
                        ui::HelpPage::CommitMessage
                    } else if app.move_path_input
                        || app.move_confirm.is_some()
                        || app.remove_path_input.is_some()
                        || app.remove_confirm.is_some()
                    {
                        ui::HelpPage::WorkingTree
                    } else if app.ignore_path_input.is_some() {
                        ui::HelpPage::IgnorePathInput
                    } else if app.branch_view && app.branch_input.is_some() {
                        ui::HelpPage::BranchInput
                    } else if app.branch_view && app.branch_confirm.is_some() {
                        ui::HelpPage::BranchConfirmation
                    } else if app.branch_view {
                        ui::HelpPage::Branches
                    } else if app.diff_entry.is_some() && app.diff.is_some() {
                        ui::HelpPage::WorktreeDiff
                    } else if app.diff.is_some() {
                        ui::HelpPage::CommitDiff
                    } else if app.details.is_some() {
                        ui::HelpPage::Commit
                    } else if app.searching {
                        ui::HelpPage::Search
                    } else if app.app_state.view() == View::WorkingTree {
                        ui::HelpPage::WorkingTree
                    } else {
                        ui::HelpPage::History
                    };
                    let screen = if let Some(context) = app.pending_preview {
                        Screen::PreviewLoading {
                            message: context.label(),
                        }
                    } else if let Some(amend) = app.commit_preview_mode {
                        Screen::CommitPreviewLoading { amend }
                    } else if let Some(page) = &app.object_page {
                        Screen::Object(page)
                    } else if let Some(page) = &app.comparison_page {
                        Screen::Comparison(page)
                    } else if let Some(path) = &app.history_path_input {
                        Screen::PathHistoryInput { path }
                    } else if let Some(page) = &app.tag_page {
                        Screen::Tags(page)
                    } else if app.profiles_view {
                        Screen::Profiles {
                            profiles,
                            selected: app.profile_selection.selected(),
                            active_profile: &profile_label,
                            registry_path: &app.profile_registry_path,
                            feedback: &app.profile_feedback,
                        }
                    } else if app.move_path_input {
                        Screen::MovePathInput {
                            source: &app.move_source,
                            destination: &app.move_destination,
                            destination_focused: app.move_destination_focused,
                            feedback: &app.commit_feedback,
                        }
                    } else if let Some(preview) = &app.move_confirm {
                        Screen::MovePathConfirmation {
                            source: preview.source(),
                            destination: preview.destination(),
                            pending: app.mutation_pending,
                        }
                    } else if let Some(path) = &app.remove_path_input {
                        Screen::RemovePathInput {
                            path,
                            feedback: &app.commit_feedback,
                        }
                    } else if let Some(preview) = &app.remove_confirm {
                        Screen::RemovePathConfirmation {
                            path: preview.path(),
                            pending: app.mutation_pending,
                        }
                    } else if let Some(path) = &app.ignore_path_input {
                        Screen::IgnorePathInput {
                            path,
                            feedback: &app.commit_feedback,
                        }
                    } else if app.amend_confirm {
                        let preview = app.amend_expected.as_ref();
                        Screen::AmendConfirmation {
                            repository: root,
                            profile: &profile_label,
                            branches: app.branches.as_ref(),
                            entries: app.status.as_ref(),
                            head: preview.map_or("unavailable", |preview| preview.head.as_str()),
                            original_message: preview
                                .map_or("Unavailable", |preview| preview.message.as_str()),
                            new_message: &composed_message,
                            pending: app.mutation_pending,
                        }
                    } else if let Some(preview) = &app.discard_confirm {
                        Screen::DiscardConfirmation {
                            entry: &preview.entry,
                            pending: app.mutation_pending,
                        }
                    } else if let Some(preview) = &app.untrack_confirm {
                        Screen::UntrackConfirmation {
                            entry: &preview.entry,
                            pending: app.mutation_pending,
                        }
                    } else if let Some(message) = &app.commit_message {
                        Screen::CommitInput {
                            repository: root,
                            profile: &profile_label,
                            branches: app.branches.as_ref(),
                            message,
                            description: &app.commit_description,
                            description_focused: app.commit_description_focused,
                            amend: app.commit_amend,
                            feedback: &app.commit_feedback,
                            entries: app.status.as_ref(),
                            pending: app.mutation_pending,
                        }
                    } else if app.branch_view && app.branch_input.is_some() {
                        if let Some(preview) = &app.branch_rename {
                            Screen::RenameBranchInput {
                                input: app.branch_input.as_deref().unwrap_or(""),
                                name: preview.name(),
                            }
                        } else {
                            Screen::BranchInput(app.branch_input.as_deref().unwrap_or(""))
                        }
                    } else if app.branch_view
                        && (app.mutation_pending || app.branch_confirm.is_some())
                    {
                        Screen::BranchConfirmation {
                            pending: app.mutation_pending,
                            action: app.branch_confirm.as_ref(),
                        }
                    } else if app.branch_view {
                        Screen::Branches {
                            branches: app.branches.as_ref(),
                            feedback: &app.branch_feedback,
                            selection: &mut app.branch_selection,
                        }
                    } else if let (Some(diff), Some(prepared)) = (&app.diff, &app.prepared_diff) {
                        Screen::PreparedDiff {
                            diff,
                            prepared,
                            scroll: app.diff_scroll,
                            selected_hunk: app.selected_hunk,
                            raw: app.diff_raw_view,
                            side_by_side: app.diff_side_by_side,
                            worktree_entry: app.diff_entry.as_ref(),
                        }
                    } else if let Some(diff) = &app.diff {
                        Screen::Diff {
                            diff,
                            scroll: app.diff_scroll,
                            selected_hunk: app.selected_hunk,
                            raw: app.diff_raw_view,
                            side_by_side: app.diff_side_by_side,
                            worktree_entry: app.diff_entry.as_ref(),
                        }
                    } else if let Some(details) = &app.details {
                        Screen::CommitDetails {
                            details,
                            selection: &mut app.file_selection,
                        }
                    } else if app.searching {
                        Screen::Search {
                            history: app.history.as_ref(),
                            matches: &app.search_results,
                            query: &app.query,
                            pending: app.search_debounce.is_some(),
                            selection: &mut app.search_selection,
                        }
                    } else {
                        Screen::Cockpit(ui::Cockpit {
                            repository: root,
                            profile: &profile_label,
                            branches: app.branches.as_ref(),
                            history: app.history.as_ref(),
                            status: app.status.as_ref(),
                            feedback: &workspace_feedback,
                            focused: app.app_state.view(),
                            history_selection: &mut app.selection,
                            working_tree_selection: &mut app.status_selection,
                        })
                    };
                    ui::render(frame, screen, app.border);
                    if app.help {
                        ui::render_help_overlay(frame, help_page, app.border);
                    }
                })
                .map_err(backend_error)?;
            app.needs_redraw = false;
        }
        let event = match host.next_event(terminal, Duration::from_millis(50)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };
        app.needs_redraw = true;
        if let Event::Paste(pasted) = &event {
            if app.pending_preview.is_some() || app.commit_preview_mode.is_some() {
                continue;
            }
            if let Some(page) = &mut app.object_page {
                page.paste(pasted);
                continue;
            }
            if let Some(page) = &mut app.comparison_page {
                page.paste(pasted);
                continue;
            }
            if let Some(path) = &mut app.history_path_input {
                if !app.help {
                    path.push_str(pasted);
                }
                continue;
            }
            if let Some(page) = &mut app.tag_page {
                page.paste(pasted);
                continue;
            }
            if app.move_path_input && !app.help {
                if app.move_destination_focused {
                    app.move_destination.push_str(pasted);
                } else {
                    app.move_source.push_str(pasted);
                }
            }
            if let Some(path) = &mut app.remove_path_input
                && !app.help
            {
                path.push_str(pasted);
            }
            if let Some(path) = &mut app.ignore_path_input
                && !app.help
            {
                path.push_str(pasted);
            }
            if app.commit_message.is_some()
                && !app.amend_confirm
                && !app.mutation_pending
                && !app.help
            {
                let pasted = normalize_commit_paste(pasted, app.commit_description_focused);
                if app.commit_description_focused {
                    app.commit_description.push_str(&pasted);
                } else if let Some(message) = &mut app.commit_message {
                    message.push_str(&pasted);
                }
                app.commit_feedback.clear();
            }
            continue;
        }
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            if app.help {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                    app.help = false;
                }
                continue;
            }
            if app.pending_preview.is_some() {
                if key.code == KeyCode::Esc {
                    app.preview_worker.cancel();
                    app.pending_preview = None;
                }
                continue;
            }
            if app.commit_preview_mode.is_some() {
                if key.code == KeyCode::Esc {
                    app.commit_preview_worker.cancel();
                    app.commit_preview_mode = None;
                    if std::mem::take(&mut app.commit_preview_retry) {
                        app.commit_feedback.push_str("\nPreview refresh cancelled; reopen the composer to review staged changes.");
                    } else {
                        app.commit_feedback.clear();
                    }
                }
                continue;
            }
            if let Some(page) = &mut app.object_page {
                if !page.key(root, key.code) {
                    app.object_page = None;
                }
                continue;
            }
            if let Some(page) = &mut app.comparison_page {
                if !page.key(root, key.code) {
                    app.comparison_page = None;
                }
                continue;
            }
            if let Some(input) = &mut app.history_path_input {
                match key.code {
                    KeyCode::Esc => app.history_path_input = None,
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                    KeyCode::Enter => {
                        app.history_filter =
                            (!input.is_empty()).then(|| std::path::PathBuf::from(&*input));
                        app.history_path_input = None;
                        app.history_request += 1;
                        let id = app.history_request;
                        let sender = app.history_sender.clone();
                        let path = root.to_path_buf();
                        let filter = app.history_filter.clone();
                        app.history = None;
                        app.selection.select(None);
                        std::thread::spawn(move || {
                            let _ = sender.send((
                                id,
                                crate::history::load_history_for_path(&path, filter.as_deref()),
                            ));
                        });
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(page) = &mut app.tag_page {
                if !page.key(root, key.code) {
                    app.tag_page = None;
                    app.history_request += 1;
                    let id = app.history_request;
                    let sender = app.history_sender.clone();
                    let path = root.to_path_buf();
                    app.history = None;
                    let requested_history_path = app.history_filter.clone();
                    std::thread::spawn(move || {
                        let _ = sender.send((
                            id,
                            crate::history::load_history_for_path(
                                &path,
                                requested_history_path.as_deref(),
                            ),
                        ));
                    });
                }
                continue;
            }
            if app.profiles_view {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('h') => app.profiles_view = false,
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('e') => edit_profile_registry(
                        terminal,
                        host,
                        root,
                        config_dir,
                        profiles,
                        &mut profile_label,
                        &mut app.profile_feedback,
                    )?,
                    KeyCode::Char('r') => {
                        match reload_profile_context(root, config_dir, profiles, &mut profile_label)
                        {
                            Ok(()) => app.profile_feedback = "Git config profiles reloaded.".into(),
                            Err(error) => app.profile_feedback = error,
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.profile_selection
                            .select(crate::app::move_up(app.profile_selection.selected()));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.profile_selection.select(crate::app::move_down(
                            app.profile_selection.selected(),
                            profiles.len(),
                        ));
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(path_text) = app.ignore_path_input.as_mut() {
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Esc => {
                        app.ignore_path_input = None;
                        app.commit_feedback.clear();
                    }
                    KeyCode::Backspace => {
                        path_text.pop();
                    }
                    KeyCode::Enter => {
                        let user_path = std::path::PathBuf::from(&*path_text);
                        app.pending_preview = Some(crate::preview::Context::IgnorePath);
                        app.commit_feedback.clear();
                        app.preview_worker
                            .request(crate::preview::Request::IgnorePath(user_path));
                    }
                    KeyCode::Char(ch) if !ch.is_control() => path_text.push(ch),
                    _ => {}
                }
                continue;
            }
            if app.move_path_input {
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Esc => {
                        app.move_path_input = false;
                        app.move_source.clear();
                        app.move_destination.clear();
                        app.commit_feedback.clear();
                    }
                    KeyCode::Tab => app.move_destination_focused = !app.move_destination_focused,
                    KeyCode::Backspace => {
                        if app.move_destination_focused {
                            app.move_destination.pop();
                        } else {
                            app.move_source.pop();
                        }
                    }
                    KeyCode::Enter if !app.move_destination_focused => {
                        if app.move_source.is_empty() {
                            app.commit_feedback = "Enter the tracked source path first.".into();
                        } else {
                            app.move_destination_focused = true;
                            app.commit_feedback.clear();
                        }
                    }
                    KeyCode::Enter => {
                        app.pending_preview = Some(crate::preview::Context::Move);
                        app.commit_feedback.clear();
                        app.preview_worker.request(crate::preview::Request::Move(
                            app.move_source.clone().into(),
                            app.move_destination.clone().into(),
                        ));
                    }
                    KeyCode::Char(ch) if !ch.is_control() => {
                        if app.move_destination_focused {
                            app.move_destination.push(ch);
                        } else {
                            app.move_source.push(ch);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(path_text) = app.remove_path_input.as_mut() {
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Esc => {
                        app.remove_path_input = None;
                        app.commit_feedback.clear();
                    }
                    KeyCode::Backspace => {
                        path_text.pop();
                    }
                    KeyCode::Enter => {
                        app.pending_preview = Some(crate::preview::Context::Remove);
                        app.commit_feedback.clear();
                        app.preview_worker.request(crate::preview::Request::Remove(
                            std::path::PathBuf::from(&*path_text),
                        ));
                    }
                    KeyCode::Char(ch) if !ch.is_control() => path_text.push(ch),
                    _ => {}
                }
                continue;
            }
            if app.amend_confirm {
                if app.mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('y') => {
                        let subject = app.commit_message.clone().unwrap_or_default();
                        let description = app.commit_description.clone();
                        let message = combined_commit_message(&subject, &description);
                        let expected = app.amend_expected.clone();
                        let path = root.to_path_buf();
                        let sender = app.commit_sender.clone();
                        app.mutation_pending = true;
                        std::thread::spawn(move || {
                            let result = expected
                                .as_ref()
                                .ok_or_else(|| "Amend preview is unavailable".to_owned())
                                .and_then(|preview| {
                                    crate::commit::amend_commit(&path, &message, preview)
                                });
                            let _ = sender.send((subject, description, true, result));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => app.amend_confirm = false,
                    _ => {}
                }
                continue;
            }
            if app.commit_message.is_some() {
                if app.mutation_pending {
                    continue;
                }
                let mut submit = false;
                match key.code {
                    KeyCode::Esc => {
                        app.commit_message = None;
                        app.commit_description.clear();
                        app.commit_description_focused = false;
                        app.commit_expected = None;
                        app.amend_expected = None;
                        app.commit_amend = false;
                        app.commit_feedback.clear();
                    }
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let message = combined_commit_message(
                            app.commit_message.as_deref().unwrap_or_default(),
                            &app.commit_description,
                        );
                        match crate::editor::configured_editor(root) {
                            Err(error) => app.commit_feedback = error,
                            Ok(editor) => {
                                let result = host.run_external(terminal, || {
                                    crate::editor::edit_with_command(root, &editor, &message)
                                })?;
                                match result {
                                    Ok(edited) => {
                                        submit = commit_after_editor(
                                            app.commit_message.as_deref().unwrap_or_default(),
                                            &app.commit_description,
                                            &edited,
                                        );
                                        let (subject, description) = split_commit_message(&edited);
                                        app.commit_message = Some(subject);
                                        app.commit_description = description;
                                        app.commit_feedback = if submit {
                                            String::new()
                                        } else {
                                            "Returned from editor; review the message before committing.".into()
                                        };
                                    }
                                    Err(error) => app.commit_feedback = error,
                                }
                            }
                        }
                    }
                    KeyCode::Tab => {
                        app.commit_description_focused = !app.commit_description_focused
                    }
                    KeyCode::Backspace => {
                        if app.commit_description_focused {
                            app.commit_description.pop();
                        } else if let Some(message) = &mut app.commit_message {
                            message.pop();
                        }
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if app.commit_description_focused {
                            app.commit_description.push('\n');
                        }
                    }
                    KeyCode::Enter | KeyCode::Char('\r' | '\n') => submit = true,
                    KeyCode::Char(character) if !character.is_control() => {
                        if app.commit_description_focused {
                            app.commit_description.push(character);
                        } else if let Some(message) = &mut app.commit_message {
                            message.push(character);
                        }
                        app.commit_feedback.clear();
                    }
                    _ => {}
                }
                if submit {
                    let message = app.commit_message.as_deref().unwrap_or_default();
                    if message.trim().is_empty() {
                        app.commit_feedback = "Enter a commit message.".into();
                    } else if (app.commit_amend && app.amend_expected.is_none())
                        || (!app.commit_amend && app.commit_expected.is_none())
                    {
                        app.commit_feedback = if app.commit_amend {
                            "Amend preview unavailable; reopen the composer to review changes."
                                .into()
                        } else {
                            "Commit preview unavailable; reopen the composer to review staged changes.".into()
                        };
                    } else {
                        let subject = message.to_owned();
                        let description = app.commit_description.clone();
                        if app.commit_amend {
                            app.amend_confirm = true;
                            app.commit_feedback.clear();
                        } else {
                            let message = combined_commit_message(&subject, &description);
                            let staged = app.commit_expected.clone();
                            let path = root.to_path_buf();
                            let sender = app.commit_sender.clone();
                            app.mutation_pending = true;
                            app.commit_feedback.clear();
                            std::thread::spawn(move || {
                                let result = staged
                                    .as_ref()
                                    .ok_or_else(|| "Staged changes are unavailable".to_owned())
                                    .and_then(|preview| {
                                        crate::commit::create_commit(&path, &message, preview)
                                    });
                                let _ = sender.send((subject, description, false, result));
                            });
                        }
                    }
                }
                continue;
            }
            if let Some(preview) = app.discard_confirm.as_ref() {
                if app.mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('y') => {
                        app.mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = app.discard_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(crate::worktree::discard(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => app.discard_confirm = None,
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = app.untrack_confirm.as_ref() {
                if app.mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('y') => {
                        app.mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = app.untrack_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(crate::worktree::untrack(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => app.untrack_confirm = None,
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = app.remove_confirm.as_ref() {
                if app.mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('y') => {
                        app.mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = app.remove_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(crate::worktree::remove_path(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => app.remove_confirm = None,
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = app.move_confirm.as_ref() {
                if app.mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Char('y') => {
                        app.mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = app.move_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(crate::worktree::move_path(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => app.move_confirm = None,
                    _ => {}
                }
                continue;
            }
            if app.searching {
                match key.code {
                    KeyCode::Esc => {
                        app.searching = false;
                        app.search_debounce = None;
                    }
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Backspace => {
                        if app.query.pop().is_some() {
                            app.search_debounce = Some(Instant::now() + Duration::from_millis(180));
                        }
                    }
                    KeyCode::Enter => {
                        if app.search_debounce.is_some() {
                            update_search_results(
                                app.history.as_ref(),
                                &app.query,
                                &mut app.search_results,
                                &mut app.search_selection,
                            );
                        }
                        app.search_debounce = None;
                        if let Some((index, id)) = selected_search_commit(
                            app.history.as_ref(),
                            &app.search_results,
                            app.search_selection.selected(),
                        ) {
                            app.selection.select(Some(index));
                            app.request_id += 1;
                            let path = root.to_path_buf();
                            let sender = app.details_sender.clone();
                            let current = app.request_id;
                            app.file_selection.select(None);
                            app.details = Some((current, None));
                            std::thread::spawn(move || {
                                let _ =
                                    sender.send((current, crate::details::load_commit(&path, &id)));
                            });
                            app.searching = false;
                        }
                    }
                    KeyCode::Up => {
                        app.search_selection
                            .select(crate::app::move_up(app.search_selection.selected()));
                    }
                    KeyCode::Down => {
                        app.search_selection.select(crate::app::move_down(
                            app.search_selection.selected(),
                            app.search_results.len(),
                        ));
                    }
                    KeyCode::Char(c) => {
                        app.query.push(c);
                        app.search_debounce = Some(Instant::now() + Duration::from_millis(180));
                    }
                    _ => {}
                }
                continue;
            }
            if key.code == KeyCode::Char('W')
                && !app.help
                && !app.mutation_pending
                && (!app.branch_view
                    || (app.branch_input.is_none() && app.branch_confirm.is_none()))
            {
                let _ = app.app_state.handle_key(KeyCode::Char('W'), false);
                app.branch_view = false;
                app.diff = None;
                app.prepared_diff = None;
                app.ordinary_diff_worker.cancel();
                app.diff_entry = None;
                app.details = None;
                continue;
            }
            if app.branch_view && !app.help {
                if app.mutation_pending {
                    continue;
                }
                if let Some(input) = &mut app.branch_input {
                    match key.code {
                        KeyCode::Char('?') => app.help = true,
                        KeyCode::Esc => {
                            app.branch_input = None;
                            app.branch_rename = None;
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                        KeyCode::Enter => {
                            let name = input.clone();
                            app.branch_input = None;
                            if let Some(preview) = app.branch_rename.take() {
                                app.branch_confirm = Some(BranchAction::Rename { preview, name });
                                continue;
                            }
                            if let Some(Ok(data)) = &app.branches {
                                if let Some(base) = &data.head_commit {
                                    if name.is_empty() {
                                        app.branch_feedback = "Enter a branch name.".into();
                                    } else {
                                        app.branch_confirm = Some(BranchAction::Create {
                                            name,
                                            base: base.clone(),
                                        });
                                    }
                                } else {
                                    app.branch_feedback =
                                        "Create the first commit before creating a branch.".into();
                                }
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                if let Some(action) = &app.branch_confirm {
                    match key.code {
                        KeyCode::Char('?') => app.help = true,
                        KeyCode::Char('y') => {
                            app.mutation_pending = true;
                            let action = action.clone();
                            let path = root.to_path_buf();
                            let sender = app.operation_sender.clone();
                            std::thread::spawn(move || {
                                let (success_message, result) = match action {
                                    BranchAction::Previous(preview) => (
                                        "Returned to previous checkout.".to_owned(),
                                        crate::branches::switch_previous_checkout(&path, &preview),
                                    ),
                                    BranchAction::Switch(name) => (
                                        "Branch switched successfully.".to_owned(),
                                        crate::branches::switch_branch(&path, &name),
                                    ),
                                    BranchAction::Rename { preview, name } => (
                                        "Branch renamed successfully.".to_owned(),
                                        crate::branches::rename_branch(&path, &preview, &name),
                                    ),
                                    BranchAction::Delete(preview) => (
                                        "Branch deleted successfully.".to_owned(),
                                        crate::branches::delete_branch(&path, &preview),
                                    ),
                                    BranchAction::Create { name, base } => (
                                        "Branch created and switched successfully.".to_owned(),
                                        crate::branches::create_branch(&path, &name, &base),
                                    ),
                                };
                                let _ = sender.send((success_message, result));
                            });
                        }
                        KeyCode::Char('n') | KeyCode::Esc => app.branch_confirm = None,
                        _ => {}
                    }
                    continue;
                }
                match navigation_key(key) {
                    KeyCode::Char('t') => {
                        app.tag_page = Some(crate::tag_page::TagPage::open(root));
                    }
                    KeyCode::Char('-') => {
                        app.pending_preview = Some(crate::preview::Context::Previous);
                        app.branch_feedback.clear();
                        app.preview_worker
                            .request(crate::preview::Request::Previous);
                    }
                    KeyCode::Char('r' | 'd') => {
                        if let (Some(Ok(data)), Some(index)) =
                            (&app.branches, app.branch_selection.selected())
                            && let Some(name) = data.branches.get(index)
                        {
                            app.pending_preview = Some(if key.code == KeyCode::Char('r') {
                                crate::preview::Context::BranchRename
                            } else {
                                crate::preview::Context::BranchDelete
                            });
                            app.branch_feedback.clear();
                            app.preview_worker
                                .request(crate::preview::Request::Branch(name.clone()));
                        }
                    }
                    KeyCode::Char('c') => {
                        if let Some(Ok(data)) = &app.branches {
                            if data.head_commit.is_some() {
                                app.branch_input = Some(String::new());
                                app.branch_feedback.clear();
                            } else {
                                app.branch_feedback =
                                    "Create the first commit before creating a branch.".into();
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let (Some(Ok(data)), Some(index)) =
                            (&app.branches, app.branch_selection.selected())
                            && let Some(name) = data.branches.get(index)
                        {
                            if matches!(&data.head, crate::branches::HeadState::Attached(current) if current == name)
                            {
                                app.branch_feedback = "Already on this branch.".into();
                            } else {
                                app.branch_confirm = Some(BranchAction::Switch(name.clone()));
                                app.branch_feedback.clear();
                            }
                        }
                    }
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => app.help = true,
                    KeyCode::Esc | KeyCode::Char('b') => app.branch_view = false,
                    KeyCode::Char('H') => {
                        app.branch_view = false;
                        let _ = app.app_state.handle_key(KeyCode::Char('H'), false);
                    }
                    KeyCode::Down => {
                        if let Some(Ok(data)) = &app.branches
                            && !data.branches.is_empty()
                        {
                            app.branch_selection.select(Some(
                                (app.branch_selection.selected().unwrap_or(0) + 1)
                                    .min(data.branches.len() - 1),
                            ));
                        }
                    }
                    KeyCode::Up => {
                        if let Some(index) = app.branch_selection.selected() {
                            app.branch_selection.select(Some(index.saturating_sub(1)));
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if key.code == KeyCode::Char('H') && !app.help && !app.mutation_pending {
                let _ = app.app_state.handle_key(KeyCode::Char('H'), false);
                app.searching = false;
                app.diff = None;
                app.prepared_diff = None;
                app.ordinary_diff_worker.cancel();
                app.diff_entry = None;
                app.details = None;
                continue;
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
                        .position(|profile| profile.name == profile_label),
                );
                app.profile_feedback.clear();
                app.profiles_view = true;
                continue;
            }
            if key.code != KeyCode::Char('c') {
                app.commit_feedback.clear();
            }
            let blocked =
                app.help || app.mutation_pending || app.details.is_some() || app.diff.is_some();
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
                            .and_then(|rows| {
                                app.selection.selected().and_then(|index| rows.get(index))
                            })
                            .and_then(|row| crate::history::commit_id(row))
                            .unwrap_or("HEAD");
                        app.object_page = Some(crate::object_page::ObjectPage::open(revision));
                    }
                    AppAction::OpenComparison => {
                        let target = app
                            .history
                            .as_ref()
                            .and_then(|rows| rows.as_ref().ok())
                            .and_then(|rows| {
                                app.selection.selected().and_then(|index| rows.get(index))
                            })
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
                                app.selection.select(crate::app::move_down(
                                    app.selection.selected(),
                                    rows.len(),
                                ));
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
                continue;
            }
            match navigation_key(key) {
                KeyCode::Char('q') if !app.mutation_pending => return Ok(()),
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
                    if !app.help
                        && app.app_state.view() == View::WorkingTree
                        && app.diff.is_none() =>
                {
                    if let (Some(Ok(entries)), Some(index)) =
                        (&app.status, app.status_selection.selected())
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
                    if !app.help
                        && app.details.is_none()
                        && app.app_state.view() == View::History =>
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
                        && app.diff_entry.as_ref().is_some_and(|entry| {
                            entry.section != crate::worktree::Section::Untracked
                        }) =>
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
                        && app.diff_entry.as_ref().is_some_and(|entry| {
                            entry.section != crate::worktree::Section::Untracked
                        }) =>
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
                                || (all_hunks
                                    && entry.section == crate::worktree::Section::Untracked)))
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
                            let status_result =
                                result.and_then(|()| crate::worktree::load_status(&path));
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
                            (app.file_selection.selected().unwrap_or(0) + 1)
                                .min(commit.files.len() - 1),
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
        }
    }
}
