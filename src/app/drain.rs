use std::{io, time::Instant};

use super::{
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};
use crate::app::BranchAction;

pub(super) fn drain_results<H: Host>(app: &mut App, env: &mut Env<'_, H>) -> io::Result<Flow> {
    let Env { root, .. } = env;
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
        app.refresh.history.finish(id, Instant::now());
        if id != app.history_request {
            continue;
        }
        // A reload keeps the same commit selected, wherever it moved to.
        let selected_commit = app
            .history
            .as_ref()
            .and_then(|rows| rows.as_ref().ok())
            .zip(app.selection.selected())
            .and_then(|(rows, index)| rows.get(index))
            .and_then(|row| crate::history::commit_id(row))
            .map(str::to_owned);
        if let Ok(rows) = &result
            && !rows.is_empty()
        {
            let kept = selected_commit.as_deref().and_then(|wanted| {
                rows.iter()
                    .position(|row| crate::history::commit_id(row) == Some(wanted))
            });
            app.selection.select(Some(kept.unwrap_or(0)));
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
    let diff_result = app.ordinary_diff_worker.take_result();
    if let Some((id, _)) = &diff_result {
        app.refresh.diff.finish(*id, Instant::now());
    }
    if let Some((id, result)) = diff_result
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
        // A reload for new text puts the reader back where they were; opening a
        // diff starts at the top.
        let place = app
            .diff_refresh
            .filter(|(refresh_id, ..)| *refresh_id == id);
        if let Some((_, scroll, hunk)) = place {
            app.diff_refresh = None;
            let (lines, hunks) = app
                .diff
                .as_ref()
                .and_then(|(_, shown)| shown.as_ref())
                .and_then(|shown| shown.as_ref().ok())
                .map_or((0, 0), |text| {
                    (
                        text.lines().count(),
                        text.lines().filter(|line| line.starts_with("@@ ")).count(),
                    )
                });
            app.diff_scroll =
                scroll.min(u16::try_from(lines.saturating_sub(1)).unwrap_or(u16::MAX));
            app.selected_hunk = hunk.min(hunks.saturating_sub(1));
        } else {
            app.selected_hunk = 0;
            app.diff_scroll = 0;
        }
    }
    while let Ok((id, result)) = app.status_receiver.try_recv() {
        app.needs_redraw = true;
        app.refresh.status.finish(id, Instant::now());
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
        super::refresh::retarget_open_diff(app);
    }
    while let Ok((id, result)) = app.branch_receiver.try_recv() {
        app.needs_redraw = true;
        app.refresh.branches.finish(id, Instant::now());
        if id != app.branch_request {
            continue;
        }
        let selected_branch = app
            .branches
            .as_ref()
            .and_then(|data| data.as_ref().ok())
            .zip(app.branch_selection.selected())
            .and_then(|(data, index)| data.branches.get(index))
            .cloned();
        if let Ok(data) = &result
            && !data.branches.is_empty()
        {
            let kept = selected_branch
                .and_then(|wanted| data.branches.iter().position(|name| *name == wanted));
            app.branch_selection.select(Some(kept.unwrap_or(0)));
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
    Ok(Flow::Next)
}
