use std::io;

use ratatui::layout::Rect;

use super::{
    helpers::*,
    input::Focus,
    model::{App, SearchList},
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
        // Where Gitview and the agent go, and the size the agent should believe.
        let size = terminal.size().map_err(backend_error)?;
        let layout = ui::plan_workspace_agent_layout(
            Rect::new(0, 0, size.width, size.height),
            &app.workspace,
            app.focus == Focus::Agent,
        );
        let pane_area = match layout {
            ui::AgentLayout::SideBySide { agent, .. } => Some(agent),
            ui::AgentLayout::AgentFull(area) => Some(area),
            _ => None,
        };
        app.agent_mouse_area = pane_area.map(ui::agent_pane_inner);
        let mouse_enabled = app.focus == Focus::Agent && app.agent_mouse_area.is_some();
        env.host.set_mouse_capture(mouse_enabled)?;
        if let (Some(area), Some(pane)) = (pane_area, app.pane.as_mut()) {
            let inner = ui::agent_pane_inner(area);
            let (rows, columns) = (inner.height.max(1), inner.width.max(1));
            if pane.size() != (rows, columns) {
                pane.resize(rows, columns);
            }
        }
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
                } else if app.app_state.view() == View::Branches && !app.branch_feedback.is_empty()
                {
                    app.branch_feedback.clone()
                } else if let Some(page) = app.sessions_page.as_ref().filter(|page| {
                    app.app_state.view() == View::Sessions && !page.feedback().is_empty()
                }) {
                    page.feedback().to_owned()
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
                } else if app.app_state.view() == View::Branches && app.branch_input.is_some() {
                    ui::HelpPage::BranchInput
                } else if app.app_state.view() == View::Branches && app.branch_confirm.is_some() {
                    ui::HelpPage::BranchConfirmation
                } else if app.app_state.view() == View::Branches {
                    ui::HelpPage::Branches
                } else if app.app_state.view() == View::Sessions {
                    ui::HelpPage::Sessions
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
                // The Sessions page runs its own search; its prompt shares
                // the status line with the other lists'.
                let prompt_line = search_line(app).or_else(|| {
                    app.sessions_page
                        .as_ref()
                        .filter(|_| app.app_state.view() == View::Sessions)
                        .and_then(|page| page.section().search)
                        .map(|(_, line)| line)
                });
                // Branch prompts and confirmations open over the centre panel,
                // with the sidebar still visible.
                let branch_focus = app.app_state.view() == View::Branches;
                let centre = if branch_focus && app.branch_input.is_some() {
                    Some(if let Some(preview) = &app.branch_rename {
                        Screen::RenameBranchInput {
                            input: app.branch_input.as_deref().unwrap_or(""),
                            name: preview.name(),
                        }
                    } else {
                        Screen::BranchInput(app.branch_input.as_deref().unwrap_or(""))
                    })
                } else if branch_focus && (app.mutation_pending || app.branch_confirm.is_some()) {
                    Some(Screen::BranchConfirmation {
                        pending: app.mutation_pending,
                        action: app.branch_confirm.as_ref(),
                    })
                } else if app.app_state.view() == View::Sessions {
                    // A session review or confirmation.
                    app.sessions_page
                        .as_ref()
                        .filter(|page| page.shows_centre())
                        .map(Screen::Sessions)
                } else {
                    None
                };
                // The workspace; `page_open` while a page covers the centre.
                macro_rules! cockpit {
                    ($page_open:expr) => {
                        ui::Cockpit {
                            repository: root,
                            profile: &app.profile_label,
                            branches: app.branches.as_ref(),
                            history: app.history.as_ref(),
                            history_entries: &app.history_entries,
                            status: app.status.as_ref(),
                            feedback: &workspace_feedback,
                            search_prompt: prompt_line.clone().map(|line| ui::SearchPrompt {
                                line,
                                query: &app.query,
                            }),
                            focused: app.app_state.view(),
                            history_selection: &mut app.selection,
                            working_tree_selection: &mut app.status_selection,
                            branch_selection: &mut app.branch_selection,
                            sessions: app.sessions_page.as_ref().map(|page| page.section()),
                            page_open: $page_open,
                        }
                    };
                }
                let mut workspace_only = false;
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
                        search_prompt: prompt_line.clone().map(|line| ui::SearchPrompt {
                            line,
                            query: &app.query,
                        }),
                        selection: &mut app.file_selection,
                    }
                } else if app.searching {
                    Screen::Search {
                        history: search_rows(app.history.as_ref(), app.filtered.as_ref()),
                        matches: &app.search_results,
                        query: &app.query,
                        pending: app.search_debounce.is_some()
                            || app.filtered.as_ref().is_some_and(|f| f.rows.is_none()),
                        truncated_at: app
                            .filtered
                            .as_ref()
                            .filter(|f| f.truncated)
                            .map(|_| FILTER_LIMIT),
                        selection: &mut app.search_selection,
                    }
                } else {
                    // The workspace is built below, once nothing else
                    // borrows its selections.
                    workspace_only = true;
                    Screen::PreviewLoading { message: "" }
                };
                // Pages open in the centre panel over the workspace, with the
                // sidebar still visible; prompts and confirmations above stay
                // full screen.
                let page = matches!(
                    screen,
                    Screen::Object(_)
                        | Screen::Comparison(_)
                        | Screen::Tags(_)
                        | Screen::Profiles { .. }
                        | Screen::PathHistoryInput { .. }
                        | Screen::PreparedDiff { .. }
                        | Screen::Diff { .. }
                        | Screen::CommitDetails { .. }
                );
                let (screen, centre) = if workspace_only {
                    // A branch prompt or session review in the centre brings
                    // its own hints.
                    let open = centre.is_some();
                    (Screen::Cockpit(cockpit!(open)), centre)
                } else if page {
                    (Screen::Cockpit(cockpit!(true)), Some(screen))
                } else {
                    (screen, centre)
                };
                let gitview_area = match layout {
                    ui::AgentLayout::GitviewOnly(area) => Some(area),
                    ui::AgentLayout::SideBySide { gitview, .. }
                    | ui::AgentLayout::GitviewWithBar { gitview, .. } => Some(gitview),
                    ui::AgentLayout::AgentFull(_) => None,
                };
                app.gitview_area = gitview_area;
                if let Some(area) = gitview_area {
                    let mut viewport = ui::Viewport::new(frame, area);
                    if let Screen::Cockpit(cockpit) = screen {
                        let panel = cockpit.focused.panel();
                        app.workspace.show(panel);
                        app.workspace.focus(panel);
                        viewport.render_widget(ratatui::widgets::Clear, area);
                        ui::render_workspace_cockpit(
                            &mut viewport,
                            cockpit,
                            &app.workspace,
                            app.border,
                        );
                        if let Some(centre) = centre {
                            // The page draws without its own hint row; its
                            // keys go to the workspace's single bottom bar.
                            let rect = ui::cockpit_centre_area(area, &app.workspace);
                            let hints = std::cell::RefCell::new(Vec::new());
                            ui::render_in(
                                &mut viewport.sub(rect).capture_hints(&hints),
                                centre,
                                app.border,
                            );
                            let bar = ratatui::layout::Rect {
                                y: area.bottom().saturating_sub(1),
                                height: 1,
                                ..area
                            };
                            ui::render_bar(&mut viewport, bar, &hints.into_inner());
                        }
                    } else {
                        ui::render_in(&mut viewport, screen, app.border);
                    }
                    if app.help {
                        ui::render_help_overlay_in(&mut viewport, help_page, app.border);
                    }
                }
                if let (Some(area), Some(pane)) = (pane_area, app.pane.as_ref()) {
                    let focused = app.focus == Focus::Agent;
                    ui::render_agent_pane(
                        &mut ui::Viewport::new(frame, area),
                        area,
                        pane,
                        focused,
                        app.border,
                    );
                }
                if let (ui::AgentLayout::GitviewWithBar { bar, .. }, Some(pane)) =
                    (layout, app.pane.as_ref())
                {
                    ui::render_agent_bar(&mut ui::Viewport::new(frame, bar), bar, pane);
                }
            })
            .map_err(backend_error)?;
        app.needs_redraw = false;
    }
    Ok(Flow::Next)
}

/// The status line while `/` searches inline: the query and which match is
/// selected.
fn search_line(app: &App) -> Option<String> {
    let search = app.inline_search.as_ref()?;
    let selected = match search.list {
        SearchList::History => app.selection.selected(),
        SearchList::Worktree => app.status_selection.selected(),
        SearchList::CommitFiles => app.file_selection.selected(),
        SearchList::Branches => app.branch_selection.selected(),
    };
    let status = if app.query.trim().is_empty() {
        String::new()
    } else if let Some(position) =
        selected.and_then(|row| search.matches.iter().position(|&m| m == row))
    {
        format!("match {} of {}", position + 1, search.matches.len())
    } else {
        "no match".to_owned()
    };
    Some(format!("/{}_   {status}", app.query))
}
