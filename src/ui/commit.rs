use crate::{
    branches::{Branches, HeadState},
    worktree::Entry,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, ListState, Paragraph, Wrap},
};
use std::path::Path;

use super::cockpit::render_worktree_panel;
use super::hints::{hint_row_count, render_key_hints};
use super::theme::{ACCENT, BORDER, MUTED, page_block};

pub(super) struct CommitWorkspace<'a> {
    pub(super) repository: &'a Path,
    pub(super) profile: &'a str,
    pub(super) branches: Option<&'a Result<Branches, String>>,
    pub(super) message: &'a str,
    pub(super) description: &'a str,
    pub(super) description_focused: bool,
    pub(super) amend: bool,
    pub(super) feedback: &'a str,
    pub(super) entries: Option<&'a Result<Vec<Entry>, String>>,
    pub(super) pending: bool,
}

pub(super) struct AmendConfirmation<'a> {
    pub(super) repository: &'a Path,
    pub(super) profile: &'a str,
    pub(super) branches: Option<&'a Result<Branches, String>>,
    pub(super) entries: Option<&'a Result<Vec<Entry>, String>>,
    pub(super) head: &'a str,
    pub(super) original_message: &'a str,
    pub(super) new_message: &'a str,
    pub(super) pending: bool,
}

pub(super) fn render_repository_header(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    repository: &Path,
    profile: &str,
    branches: Option<&Result<Branches, String>>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let branch = match branches {
        Some(Ok(data)) => match &data.head {
            HeadState::Attached(name) => name.to_string_lossy().into_owned(),
            HeadState::Detached => "detached HEAD".to_owned(),
            HeadState::Unborn(name) => format!("{} (unborn)", name.to_string_lossy()),
        },
        Some(Err(_)) => "branch unavailable".to_owned(),
        None => "loading branch".to_owned(),
    };
    let header = Line::from(vec![
        Span::styled(
            " GITVIEW ",
            Style::default()
                .fg(Color::Rgb(12, 24, 30))
                .bg(ACCENT)
                .bold(),
        ),
        Span::styled(
            format!("  {}  ", repository.display()),
            Style::default().fg(Color::Rgb(226, 232, 240)).bold(),
        ),
        Span::styled(format!(" branch: {branch}"), Style::default().fg(MUTED)),
        Span::styled(format!("  account: {profile}"), Style::default().fg(MUTED)),
    ]);
    frame.render_widget(
        Paragraph::new(header).block(
            Block::bordered()
                .border_set(border)
                .border_style(Style::default().fg(BORDER)),
        ),
        area,
    );
}

pub(super) fn render_commit_workspace(
    frame: &mut Frame<'_>,
    workspace: CommitWorkspace<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let hints = if workspace.pending {
        &[("Working", "commit in progress")][..]
    } else {
        &[
            ("Tab", "switch subject/description"),
            ("Ctrl+J", "new description line"),
            ("Ctrl+E", "external editor"),
            (
                "Enter",
                if workspace.amend {
                    "review warning"
                } else {
                    "commit"
                },
            ),
            ("Esc", "cancel"),
        ][..]
    };
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(hint_row_count(hints, frame.area().width)),
    ])
    .split(frame.area());
    render_repository_header(
        frame,
        sections[0],
        workspace.repository,
        workspace.profile,
        workspace.branches,
        border,
    );
    let columns = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(sections[1]);
    let mut read_only_selection = ListState::default();
    render_worktree_panel(
        frame,
        columns[0],
        "WORKTREE · READ ONLY",
        workspace.entries,
        &mut read_only_selection,
        false,
        border,
    );
    let mut message_lines = Vec::new();
    if !workspace.feedback.is_empty() {
        message_lines.push(Line::styled(
            workspace.feedback,
            Style::default().fg(Color::Rgb(251, 191, 36)),
        ));
    }
    if workspace.pending {
        message_lines.push(Line::raw("Creating commit..."));
    } else {
        message_lines.push(Line::raw(format!("{}_", workspace.message)));
    }
    let editor = Layout::vertical([Constraint::Length(5), Constraint::Min(3)]).split(columns[1]);
    frame.render_widget(
        Paragraph::new(Text::from(message_lines))
            .wrap(Wrap { trim: false })
            .block(page_block(
                if workspace.description_focused {
                    if workspace.amend {
                        "AMEND MESSAGE"
                    } else {
                        "COMMIT MESSAGE"
                    }
                } else {
                    if workspace.amend {
                        "AMEND MESSAGE · ACTIVE"
                    } else {
                        "COMMIT MESSAGE · ACTIVE"
                    }
                },
                border,
            )),
        editor[0],
    );
    let description = if workspace.pending {
        "Commit in progress...".to_owned()
    } else if workspace.description.is_empty() {
        if workspace.description_focused {
            "_".to_owned()
        } else {
            "Optional commit description. Press Tab to edit.".to_owned()
        }
    } else {
        format!(
            "{}{}",
            workspace.description,
            if workspace.description_focused {
                "_"
            } else {
                ""
            }
        )
    };
    frame.render_widget(
        Paragraph::new(description)
            .wrap(Wrap { trim: true })
            .block(page_block(
                if workspace.description_focused {
                    if workspace.amend {
                        "AMEND DESCRIPTION · ACTIVE"
                    } else {
                        "COMMIT DESCRIPTION · ACTIVE"
                    }
                } else {
                    if workspace.amend {
                        "AMEND DESCRIPTION · OPTIONAL"
                    } else {
                        "COMMIT DESCRIPTION · OPTIONAL"
                    }
                },
                border,
            )),
        editor[1],
    );
    render_key_hints(frame, sections[2], hints);
}

pub(super) fn render_amend_confirmation(
    frame: &mut Frame<'_>,
    confirmation: AmendConfirmation<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .split(frame.area());
    render_repository_header(
        frame,
        sections[0],
        confirmation.repository,
        confirmation.profile,
        confirmation.branches,
        border,
    );
    let columns = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(sections[1]);
    let mut read_only_selection = ListState::default();
    render_worktree_panel(
        frame,
        columns[0],
        "WORKTREE · REVIEWED",
        confirmation.entries,
        &mut read_only_selection,
        false,
        border,
    );
    let lines = if confirmation.pending {
        vec![Line::raw("Amending HEAD...")]
    } else {
        vec![
            Line::styled(
                "This rewrites HEAD and changes its commit ID.",
                Style::default().fg(Color::Rgb(251, 191, 36)).bold(),
            ),
            Line::styled(
                "Already-pushed commits may need coordination.",
                Style::default().fg(Color::Rgb(251, 191, 36)),
            ),
            Line::raw(format!("\nHEAD: {}", confirmation.head)),
            Line::raw(format!(
                "\nPrevious message:\n{}",
                confirmation.original_message
            )),
            Line::raw(format!(
                "\nReplacement message:\n{}",
                confirmation.new_message
            )),
        ]
    };
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true })
            .block(page_block("CONFIRM AMEND", border)),
        columns[1],
    );
    let hints = if confirmation.pending {
        &[("Working", "amend in progress")][..]
    } else {
        &[("y", "amend HEAD"), ("n/Esc", "cancel")][..]
    };
    render_key_hints(frame, sections[2], hints);
}
