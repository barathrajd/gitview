use crate::ui::Viewport;
use crate::{
    app::View,
    branches::{Branches, HeadState},
    history,
    worktree::{Entry, Section},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use std::path::Path;

use super::commit::render_repository_header;
use super::hints::render_key_hints_padded;
use super::panels::worktree_selection_style;
use super::screen::Cockpit;
use super::theme::{ACCENT, BORDER, MUTED, page_block};

pub(super) fn render_cockpit(
    frame: &mut Viewport<'_, '_>,
    cockpit: Cockpit<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let Cockpit {
        repository,
        profile,
        branches,
        history,
        status,
        focused,
        feedback,
        history_selection,
        working_tree_selection,
    } = cockpit;
    let mut hints = match focused {
        View::History => vec![
            ("Tab", "pane"),
            ("W", "worktree"),
            ("Enter/l", "details"),
            ("f", "path history"),
            ("D", "compare"),
            ("o", "object"),
            ("b", "branches"),
            ("p", "profiles"),
            ("/", "search"),
        ],
        View::WorkingTree => vec![
            ("Tab", "pane"),
            ("Enter", "diff"),
            ("s", "stage"),
            ("u", "unstage"),
            ("S", "stage all"),
            ("U", "unstage all"),
            ("c", "commit"),
            ("a", "amend HEAD"),
            ("r", "refresh"),
            ("p", "profiles"),
        ],
    };
    hints.extend([("j/k", "move"), ("?", "help"), ("q", "quit")]);
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_repository_header(frame, sections[0], repository, profile, branches, border);

    if frame.area().width < 88 {
        match focused {
            View::History => {
                render_history_panel(frame, sections[1], history, history_selection, true, border)
            }
            View::WorkingTree => render_worktree_panel(
                frame,
                sections[1],
                "WORKTREE",
                status,
                working_tree_selection,
                true,
                border,
            ),
        }
    } else if frame.area().width < 128 {
        let panes = Layout::horizontal([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(sections[1]);
        render_worktree_panel(
            frame,
            panes[0],
            "WORKTREE",
            status,
            working_tree_selection,
            focused == View::WorkingTree,
            border,
        );
        render_history_panel(
            frame,
            panes[1],
            history,
            history_selection,
            focused == View::History,
            border,
        );
    } else {
        let panes = Layout::horizontal([
            Constraint::Percentage(29),
            Constraint::Percentage(43),
            Constraint::Percentage(28),
        ])
        .split(sections[1]);
        render_worktree_panel(
            frame,
            panes[0],
            "WORKTREE",
            status,
            working_tree_selection,
            focused == View::WorkingTree,
            border,
        );
        render_history_panel(
            frame,
            panes[1],
            history,
            history_selection,
            focused == View::History,
            border,
        );
        render_inspector(
            frame,
            panes[2],
            Inspector {
                repository,
                branches,
                history,
                status,
                selected: history_selection.selected(),
            },
            border,
        );
    }

    let (staged, unstaged, untracked) = status_counts(status);
    let status_line = if feedback.is_empty() {
        Line::from(vec![
            Span::styled(" STAGED ", Style::default().fg(Color::Rgb(94, 234, 212))),
            Span::raw(staged.to_string()),
            Span::styled("   CHANGES ", Style::default().fg(Color::Rgb(251, 191, 36))),
            Span::raw(unstaged.to_string()),
            Span::styled(
                "   UNTRACKED ",
                Style::default().fg(Color::Rgb(148, 163, 184)),
            ),
            Span::raw(untracked.to_string()),
        ])
    } else {
        Line::styled(feedback, Style::default().fg(Color::Rgb(94, 234, 212)))
    };
    frame.render_widget(Paragraph::new(status_line), sections[2]);
    render_key_hints_padded(frame, sections[3], &hints);
}

pub(super) fn render_worktree_panel(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    title: &'static str,
    status: Option<&Result<Vec<Entry>, String>>,
    selection: &mut ListState,
    focused: bool,
    border: ratatui::symbols::border::Set<'static>,
) {
    let (items, selection_index) = worktree_items(status);
    let highlight = worktree_selection_style(status, selection.selected());
    let mut display_selection = ListState::default();
    display_selection.select(
        selection
            .selected()
            .and_then(|index| selection_index.get(index).copied().flatten()),
    );
    render_panel_list(
        frame,
        area,
        title,
        items,
        &mut display_selection,
        focused,
        border,
        highlight,
    );
}

pub(super) fn worktree_items(
    status: Option<&Result<Vec<Entry>, String>>,
) -> (Vec<ListItem<'static>>, Vec<Option<usize>>) {
    let entries = match status {
        None => return (vec![ListItem::new("Loading working tree...")], Vec::new()),
        Some(Err(error)) => return (vec![ListItem::new(error.clone())], Vec::new()),
        Some(Ok(entries)) => entries,
    };
    if entries.is_empty() {
        return (vec![ListItem::new("Working tree clean")], Vec::new());
    }

    let mut items = Vec::with_capacity(entries.len() + 2);
    let mut entry_rows = vec![None; entries.len()];
    for (title, section, color) in [
        ("STAGED CHANGES", Section::Staged, ACCENT),
        ("CHANGES", Section::Unstaged, Color::Rgb(251, 191, 36)),
    ] {
        let indices = entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (entry.section == section
                    || (section == Section::Unstaged && entry.section == Section::Untracked))
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        if indices.is_empty() {
            continue;
        }
        items.push(ListItem::new(Line::styled(
            format!("{title} ({})", indices.len()),
            Style::default().fg(color).bold(),
        )));
        for index in indices {
            let entry = &entries[index];
            let (label, color) = match entry.section {
                Section::Staged => ("STAGED", ACCENT),
                Section::Unstaged => ("EDITED", Color::Rgb(251, 191, 36)),
                Section::Untracked => ("NEW", Color::Rgb(147, 197, 253)),
            };
            entry_rows[index] = Some(items.len());
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{label} "), Style::default().fg(color).bold()),
                Span::raw(entry.path.to_string_lossy().into_owned()),
            ])));
        }
    }
    (items, entry_rows)
}

pub(super) fn render_history_panel(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    history: Option<&Result<Vec<String>, String>>,
    selection: &mut ListState,
    focused: bool,
    border: ratatui::symbols::border::Set<'static>,
) {
    let items = match history {
        None => vec![ListItem::new("Loading history...")],
        Some(Err(error)) => vec![ListItem::new(error.as_str())],
        Some(Ok(rows)) if rows.is_empty() => vec![ListItem::new("No commits yet")],
        Some(Ok(rows)) => rows
            .iter()
            .map(|row| ListItem::new(history::display_row(row)))
            .collect(),
    };
    render_panel_list(
        frame,
        area,
        "HISTORY",
        items,
        selection,
        focused,
        border,
        Style::default()
            .bg(Color::Rgb(25, 43, 52))
            .fg(Color::Rgb(241, 245, 249))
            .bold(),
    );
}

pub(super) struct Inspector<'a> {
    pub(super) repository: &'a Path,
    pub(super) branches: Option<&'a Result<Branches, String>>,
    pub(super) history: Option<&'a Result<Vec<String>, String>>,
    pub(super) status: Option<&'a Result<Vec<Entry>, String>>,
    pub(super) selected: Option<usize>,
}

pub(super) fn render_inspector(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    inspector: Inspector<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let Inspector {
        repository,
        branches,
        history,
        status,
        selected,
    } = inspector;
    let branch = match branches {
        Some(Ok(data)) => match &data.head {
            HeadState::Attached(name) => name.to_string_lossy().into_owned(),
            HeadState::Detached => "detached HEAD".to_owned(),
            HeadState::Unborn(name) => format!("{} (unborn)", name.to_string_lossy()),
        },
        Some(Err(_)) => "Unavailable".to_owned(),
        None => "Loading...".to_owned(),
    };
    let (staged, unstaged, untracked) = status_counts(status);
    let selected_commit = history
        .and_then(|rows| rows.as_ref().ok())
        .and_then(|rows| selected.and_then(|index| rows.get(index)));
    let commit = selected_commit
        .map(|row| history::display_row(row))
        .unwrap_or("No commit selected");
    let content = vec![
        Line::styled(
            "REPOSITORY",
            Style::default().fg(Color::Rgb(94, 234, 212)).bold(),
        ),
        Line::raw(repository.display().to_string()),
        Line::raw(""),
        Line::styled(
            "BRANCH",
            Style::default().fg(Color::Rgb(94, 234, 212)).bold(),
        ),
        Line::raw(branch),
        Line::raw(""),
        Line::styled(
            "SELECTED COMMIT",
            Style::default().fg(Color::Rgb(94, 234, 212)).bold(),
        ),
        Line::raw(commit),
        Line::raw(""),
        Line::styled(
            "WORKTREE SUMMARY",
            Style::default().fg(Color::Rgb(94, 234, 212)).bold(),
        ),
        Line::raw(format!("{staged} staged")),
        Line::raw(format!("{unstaged} changed")),
        Line::raw(format!("{untracked} untracked")),
    ];
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(page_block("INSPECTOR", border)),
        area,
    );
}

pub(super) fn render_panel_list(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    title: &'static str,
    items: Vec<ListItem<'_>>,
    selection: &mut ListState,
    focused: bool,
    border: ratatui::symbols::border::Set<'static>,
    highlight: Style,
) {
    let accent = if focused { ACCENT } else { BORDER };
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_set(border)
                .border_style(Style::default().fg(accent))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(if focused { ACCENT } else { MUTED })
                        .bold(),
                )),
        )
        .highlight_symbol("› ")
        .highlight_style(highlight);
    frame.render_stateful_widget(list, area, selection);
}

pub(super) fn status_counts(status: Option<&Result<Vec<Entry>, String>>) -> (usize, usize, usize) {
    status
        .and_then(|result| result.as_ref().ok())
        .map_or((0, 0, 0), |entries| {
            entries.iter().fold((0, 0, 0), |mut counts, entry| {
                match entry.section {
                    Section::Staged => counts.0 += 1,
                    Section::Unstaged => counts.1 += 1,
                    Section::Untracked => counts.2 += 1,
                }
                counts
            })
        })
}
