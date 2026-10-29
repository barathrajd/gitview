use crate::{app::BranchAction, config::FolderProfile};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Wrap},
};
use std::path::Path;

use super::branches::render_branches;
use super::cockpit::render_cockpit;
use super::commit::{
    AmendConfirmation, CommitWorkspace, render_amend_confirmation, render_commit_workspace,
};
use super::diff::render_diff;
use super::help::{render_help, render_message_page};
use super::hints::render_key_hints;
use super::panels::{render_commit_details, render_history, render_search, render_working_tree};
use super::screen::{HistoryScreen, Screen};
use super::theme::{ACCENT, BAR_BACKGROUND, MUTED, page_block};

pub fn render(
    frame: &mut Frame<'_>,
    screen: Screen<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    // Clear cells left by the previous full-screen view before rendering the
    // next one, including when switching from a wide pane layout to a dialog.
    frame.render_widget(Clear, frame.area());
    match screen {
        Screen::PreparedDiff {
            diff,
            prepared,
            scroll,
            selected_hunk,
            raw,
            side_by_side,
            worktree_entry,
        } => render_diff(
            frame,
            diff,
            scroll,
            selected_hunk,
            raw,
            side_by_side,
            worktree_entry,
            border,
            Some(prepared),
        ),
        Screen::PreviewLoading { message } => render_message_page(
            frame,
            "PREPARING PREVIEW",
            message,
            border,
            &[("Esc", "cancel")],
        ),
        Screen::CommitPreviewLoading { amend } => render_message_page(
            frame,
            if amend {
                "PREPARING AMEND"
            } else {
                "PREPARING COMMIT"
            },
            "Loading staged changes...",
            border,
            &[("Esc", "cancel")],
        ),
        Screen::Object(page) => page.render(frame, border),
        Screen::Comparison(page) => page.render(frame, border),
        Screen::PathHistoryInput { path } => render_message_page(
            frame,
            "HISTORY PATH FILTER",
            &format!(
                "Repository-relative file or directory: {path}_\n\nEmpty path clears the filter. Paths are literal; rename following is not enabled."
            ),
            border,
            &[("Enter", "apply/clear"), ("Esc", "cancel")],
        ),
        Screen::Tags(page) => page.render(frame, border),
        Screen::Cockpit(cockpit) => render_cockpit(frame, cockpit, border),
        Screen::Help => render_help(frame, border),
        Screen::BranchInput(input) => {
            let text = format!("New local branch name: {input}_");
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: true })
                    .block(page_block("CREATE BRANCH", border)),
                areas[0],
            );
            render_key_hints(frame, areas[1], &[("Enter", "continue"), ("Esc", "cancel")]);
        }
        Screen::RenameBranchInput { input, name } => render_message_page(
            frame,
            "RENAME LOCAL BRANCH",
            &format!("Rename {}\n\nNew name: {input}_", name.to_string_lossy()),
            border,
            &[("Enter", "review rename"), ("Esc", "cancel")],
        ),
        Screen::IgnorePathInput { path, feedback } => {
            let text = format!(
                "Repository-relative path: {path}_\n\n{}",
                if feedback.is_empty() { "" } else { feedback }
            );
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("INSPECT IGNORE RULE", border)),
                areas[0],
            );
            render_key_hints(
                frame,
                areas[1],
                &[("Enter", "inspect path"), ("Esc", "cancel")],
            );
        }
        Screen::RemovePathInput { path, feedback } => {
            let text = format!(
                "Tracked repository-relative path: {path}_\n\n{}",
                if feedback.is_empty() {
                    "Git will refuse removal if it would lose uncommitted work."
                } else {
                    feedback
                }
            );
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("REMOVE TRACKED PATH", border)),
                areas[0],
            );
            render_key_hints(
                frame,
                areas[1],
                &[("Enter", "review removal"), ("Esc", "cancel")],
            );
        }
        Screen::RemovePathConfirmation { path, pending } => {
            let text = if pending {
                "Removing the tracked path and staging its deletion...".to_owned()
            } else {
                format!(
                    "Remove this tracked path and stage its deletion?\n\n{}\n\nGit's normal safety checks remain enabled.",
                    path.to_string_lossy()
                )
            };
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("CONFIRM TRACKED PATH REMOVAL", border)),
                areas[0],
            );
            let hints = if pending {
                &[("Working", "removal in progress")][..]
            } else {
                &[("y", "remove and stage deletion"), ("n/Esc", "cancel")][..]
            };
            render_key_hints(frame, areas[1], hints);
        }
        Screen::MovePathInput {
            source,
            destination,
            destination_focused,
            feedback,
        } => {
            let source_cursor = if destination_focused { "" } else { "_" };
            let destination_cursor = if destination_focused { "_" } else { "" };
            let message = if feedback.is_empty() {
                "Only clean tracked sources can be moved. Git will stage the rename."
            } else {
                feedback
            };
            let text = format!(
                "Source: {source}{source_cursor}\nDestination: {destination}{destination_cursor}\n\n{message}"
            );
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("MOVE TRACKED PATH", border)),
                areas[0],
            );
            render_key_hints(
                frame,
                areas[1],
                &[
                    ("Tab", "switch fields"),
                    ("Enter", "continue/review"),
                    ("Esc", "cancel"),
                ],
            );
        }
        Screen::MovePathConfirmation {
            source,
            destination,
            pending,
        } => {
            let text = if pending {
                "Moving the tracked path and staging its rename...".to_owned()
            } else {
                format!(
                    "Move this tracked file and stage its rename?\n\n{}\n→ {}\n\nOnly the confirmed clean source will be moved.",
                    source.to_string_lossy(),
                    destination.to_string_lossy()
                )
            };
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("CONFIRM PATH MOVE", border)),
                areas[0],
            );
            let hints = if pending {
                &[("Working", "move in progress")][..]
            } else {
                &[("y", "move and stage rename"), ("n/Esc", "cancel")][..]
            };
            render_key_hints(frame, areas[1], hints);
        }
        Screen::BranchConfirmation { pending, action } => {
            let text = if pending {
                "Running branch operation...".to_owned()
            } else {
                match action {
                    Some(BranchAction::Switch(name)) => {
                        format!("Switch to local branch {name:?}?")
                    }
                    Some(BranchAction::Previous(preview)) => format!(
                        "Return to {}?\n\nGit's normal checkout safety checks remain enabled.",
                        preview.destination()
                    ),
                    Some(BranchAction::Create { name, base }) => {
                        format!("Create and switch to local branch {name:?} from commit {base}?")
                    }
                    Some(BranchAction::Rename { preview, name }) => {
                        format!("Rename local branch {:?} to {name:?}?", preview.name())
                    }
                    Some(BranchAction::Delete(preview)) => format!(
                        "Delete local branch {:?}?\n\nGit refuses unmerged branches and branches checked out in any worktree. No force deletion is used.",
                        preview.name()
                    ),
                    None => "Branch operation unavailable".to_owned(),
                }
            };
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("CONFIRM BRANCH OPERATION", border)),
                areas[0],
            );
            let hints = if pending {
                &[("Working", "operation in progress")][..]
            } else {
                &[("y", "confirm"), ("n/Esc", "cancel")][..]
            };
            render_key_hints(frame, areas[1], hints);
        }
        Screen::CommitInput {
            repository,
            profile,
            branches,
            message,
            description,
            description_focused,
            amend,
            feedback,
            entries,
            pending,
        } => render_commit_workspace(
            frame,
            CommitWorkspace {
                repository,
                profile,
                branches,
                message,
                description,
                description_focused,
                amend,
                feedback,
                entries,
                pending,
            },
            border,
        ),
        Screen::AmendConfirmation {
            repository,
            profile,
            branches,
            entries,
            head,
            original_message,
            new_message,
            pending,
        } => render_amend_confirmation(
            frame,
            AmendConfirmation {
                repository,
                profile,
                branches,
                entries,
                head,
                original_message,
                new_message,
                pending,
            },
            border,
        ),
        Screen::DiscardConfirmation { entry, pending } => {
            let action = match entry.section {
                crate::worktree::Section::Unstaged => "Restore this file to its staged version?",
                crate::worktree::Section::Untracked => "Permanently delete this untracked file?",
                crate::worktree::Section::Staged if entry.original.is_some() => {
                    "Permanently discard this staged rename and restore its original path?"
                }
                crate::worktree::Section::Staged => "Staged files cannot be discarded.",
            };
            let text = if pending {
                "Applying discard operation...".to_owned()
            } else if entry.section == crate::worktree::Section::Staged
                && let Some(original) = &entry.original
            {
                format!(
                    "{action}\n\n{} → {}\n\nStaged and unstaged content at these paths will be replaced by HEAD.",
                    original.to_string_lossy(),
                    entry.path.to_string_lossy()
                )
            } else {
                format!("{action}\n\n{}", entry.path.to_string_lossy())
            };
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("CONFIRM DISCARD", border)),
                areas[0],
            );
            let hints = if pending {
                &[("Working", "discard in progress")][..]
            } else {
                &[("y", "discard file"), ("n/Esc", "cancel")][..]
            };
            render_key_hints(frame, areas[1], hints);
        }
        Screen::UntrackConfirmation { entry, pending } => {
            let text = if pending {
                "Updating the index; keeping the working copy...".to_owned()
            } else {
                format!(
                    "Stop tracking this path and keep the working copy?\n\n{}\n\nThe staged deletion will be included in your next commit.",
                    entry.path.to_string_lossy()
                )
            };
            let areas =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(page_block("CONFIRM STOP TRACKING", border)),
                areas[0],
            );
            let hints = if pending {
                &[("Working", "index update in progress")][..]
            } else {
                &[("y", "stop tracking"), ("n/Esc", "cancel")][..]
            };
            render_key_hints(frame, areas[1], hints);
        }
        Screen::Branches {
            branches,
            feedback,
            selection,
        } => render_branches(frame, branches, feedback, selection, border),
        Screen::Diff {
            diff,
            scroll,
            selected_hunk,
            raw,
            side_by_side,
            worktree_entry,
        } => render_diff(
            frame,
            diff,
            scroll,
            selected_hunk,
            raw,
            side_by_side,
            worktree_entry,
            border,
            None,
        ),
        Screen::CommitDetails { details, selection } => {
            render_commit_details(frame, details, selection, border)
        }
        Screen::WorkingTree { status, selection } => {
            render_working_tree(frame, status, selection, border)
        }
        Screen::History {
            history,
            searching,
            query,
            search_message,
            root,
            selection,
        } => render_history(
            frame,
            HistoryScreen {
                history,
                searching,
                query,
                search_message,
                root,
                selection,
            },
            border,
        ),
        Screen::Search {
            history,
            matches,
            query,
            pending,
            selection,
        } => render_search(frame, history, matches, query, pending, selection, border),
        Screen::Profiles {
            profiles,
            selected,
            active_profile,
            registry_path,
            feedback,
        } => render_profiles(
            frame,
            profiles,
            selected,
            active_profile,
            registry_path,
            feedback,
            border,
        ),
    }
}

pub(super) fn render_profiles(
    frame: &mut Frame<'_>,
    profiles: &[FolderProfile],
    selected: Option<usize>,
    active_profile: &str,
    registry_path: &Path,
    feedback: &str,
    border: ratatui::symbols::border::Set<'static>,
) {
    let mut lines = vec![Line::from(Span::styled(
        format!("Active account: {active_profile}"),
        Style::default().fg(ACCENT).bold(),
    ))];
    if profiles.is_empty() {
        lines.push(Line::from("No folder profiles configured."));
    } else {
        for (index, profile) in profiles.iter().enumerate() {
            let marker = if Some(index) == selected { ">" } else { " " };
            let state = if !profile.enabled {
                "DISABLED"
            } else if profile.name == active_profile {
                "ACTIVE"
            } else {
                "ENABLED"
            };
            let style = if Some(index) == selected {
                Style::default()
                    .fg(Color::Rgb(226, 232, 240))
                    .bg(BAR_BACKGROUND)
            } else {
                Style::default().fg(MUTED)
            };
            lines.push(Line::from(Span::styled(
                format!("{marker} {}  [{state}]", profile.name),
                style,
            )));
            if Some(index) == selected {
                lines.push(Line::from(format!(
                    "  Folder: {}",
                    profile.worktree_root.display()
                )));
                lines.push(Line::from(format!(
                    "  Config: {}",
                    profile.config_path.display()
                )));
            }
        }
    }
    if !feedback.is_empty() {
        lines.push(Line::from(Span::styled(
            feedback,
            Style::default().fg(ACCENT),
        )));
    }
    lines.push(Line::from(format!("Registry: {}", registry_path.display())));
    let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true })
            .block(page_block("GIT CONFIG PROFILES", border)),
        areas[0],
    );
    render_key_hints(
        frame,
        areas[1],
        &[
            ("e", "edit registry"),
            ("r", "reload"),
            ("Esc/h", "back"),
            ("?", "help"),
            ("q", "quit"),
        ],
    );
}
