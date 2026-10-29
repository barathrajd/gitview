use std::io;

use super::{
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};
use crate::{
    app::View,
    ui::{self, Screen},
};

pub(super) fn draw_screen<H: Host>(app: &mut App, env: &mut Env<'_, H>) -> io::Result<Flow> {
    let Env {
        terminal,
        root,
        profiles,
        ..
    } = env;
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
                        active_profile: &app.profile_label,
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
                        profile: &app.profile_label,
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
                        profile: &app.profile_label,
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
                } else if app.branch_view && (app.mutation_pending || app.branch_confirm.is_some())
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
                        profile: &app.profile_label,
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
    Ok(Flow::Next)
}
