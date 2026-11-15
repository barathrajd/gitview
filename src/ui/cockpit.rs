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
    let mut workspace = crate::workspace::Workspace::new(false);
    workspace.focus(match cockpit.focused {
        View::WorkingTree => crate::workspace::Panel::Sidebar,
        View::History => crate::workspace::Panel::Center,
    });
    render_workspace_cockpit(frame, cockpit, &workspace, border);
}

pub fn render_workspace_cockpit(
    frame: &mut Viewport<'_, '_>,
    cockpit: Cockpit<'_>,
    workspace: &crate::workspace::Workspace,
    border: ratatui::symbols::border::Set<'static>,
) {
    let Cockpit {
        repository,
        profile,
        branches,
        history,
        history_entries,
        status,
        focused,
        feedback,
        search_prompt,
        history_selection,
        working_tree_selection,
    } = cockpit;
    // Only the focused list is searched.
    let query = search_prompt.as_ref().map_or("", |prompt| prompt.query);
    let (highlight, worktree_highlight) = match focused {
        View::History => (query, ""),
        View::WorkingTree => ("", query),
    };
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
            ("/", "search"),
        ],
    };
    hints.extend([("j/k", "move"), ("?", "help"), ("q", "quit")]);
    if search_prompt.is_some() {
        hints = vec![("Enter", "stay"), ("Esc", "back")];
        if focused == View::History {
            hints.push(("Tab", "full list"));
        }
    }
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_repository_header(frame, sections[0], repository, profile, branches, border);

    // The model reserves one bottom row, already owned by the status line.
    let workspace_layout = workspace.gitview_layout(ratatui::layout::Rect {
        height: sections[1].height.saturating_add(1),
        ..sections[1]
    });
    let side_by_side = workspace_layout.sidebar.zip(workspace_layout.center);
    if side_by_side.is_none() {
        if let Some(area) = workspace_layout.center {
            render_history_panel(
                frame,
                area,
                history,
                history_entries,
                history_selection,
                true,
                highlight,
                border,
            );
        }
        if let Some(area) = workspace_layout.sidebar {
            render_worktree_panel(
                frame,
                area,
                "WORKTREE",
                status,
                working_tree_selection,
                true,
                worktree_highlight,
                border,
            );
        }
    } else if let Some((sidebar, center)) = side_by_side.filter(|_| frame.area().width < 128) {
        let panes = [sidebar, center];
        render_worktree_panel(
            frame,
            panes[0],
            "WORKTREE",
            status,
            working_tree_selection,
            focused == View::WorkingTree,
            worktree_highlight,
            border,
        );
        render_history_panel(
            frame,
            panes[1],
            history,
            history_entries,
            history_selection,
            focused == View::History,
            highlight,
            border,
        );
    } else if let Some((sidebar, center)) = side_by_side {
        let center_panes =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(center);
        let panes = [sidebar, center_panes[0], center_panes[1]];
        render_worktree_panel(
            frame,
            panes[0],
            "WORKTREE",
            status,
            working_tree_selection,
            focused == View::WorkingTree,
            worktree_highlight,
            border,
        );
        render_history_panel(
            frame,
            panes[1],
            history,
            history_entries,
            history_selection,
            focused == View::History,
            highlight,
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
    let status_line = if let Some(prompt) = &search_prompt {
        Line::styled(format!(" {}", prompt.line), Style::default().fg(ACCENT))
    } else if feedback.is_empty() {
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
    highlight: &str,
    border: ratatui::symbols::border::Set<'static>,
) {
    let (items, selection_index) = worktree_items(status, highlight);
    let highlight = worktree_selection_style(status, selection.selected());
    let mut display_selection = ListState::default();
    display_selection.select(
        selection
            .selected()
            .and_then(|index| selection_index.get(index).copied().flatten()),
    );
    // `selection` keeps the scroll offset in display rows between frames.
    *display_selection.offset_mut() = selection.offset();
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
    // A loading or empty list draws one placeholder row, which clamps the
    // offset to zero; keep the files' position for when they return.
    if !selection_index.is_empty() {
        *selection.offset_mut() = display_selection.offset();
    }
}

/// The list row that shows entry `index`, counting the section headings.
pub fn worktree_display_row(
    status: Option<&Result<Vec<Entry>, String>>,
    index: usize,
) -> Option<usize> {
    worktree_items(status, "").1.get(index).copied().flatten()
}

/// Entry indices in the order the list shows them, staged files first.
pub fn worktree_display_order(status: Option<&Result<Vec<Entry>, String>>) -> Vec<usize> {
    let rows = worktree_items(status, "").1;
    let mut order: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].is_some()).collect();
    order.sort_by_key(|&index| rows[index]);
    order
}

pub(super) fn worktree_items(
    status: Option<&Result<Vec<Entry>, String>>,
    highlight: &str,
) -> (Vec<ListItem<'static>>, Vec<Option<usize>>) {
    let entries = match status {
        None => return (vec![ListItem::new("Loading working tree...")], Vec::new()),
        Some(Err(error)) => return (vec![ListItem::new(error.clone())], Vec::new()),
        Some(Ok(entries)) => entries,
    };
    if entries.is_empty() {
        return (vec![ListItem::new("Working tree clean")], Vec::new());
    }

    let mut finder = history::MatchFinder::new(highlight);
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
            let path = entry.path.to_string_lossy();
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(format!("{label} "), Style::default().fg(color).bold()),
            ];
            spans.extend(crate::ui::highlighted_spans(
                &path,
                &finder.positions(&path),
            ));
            items.push(ListItem::new(Line::from(spans)));
        }
    }
    (items, entry_rows)
}

pub(super) fn render_history_panel(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    history: Option<&Result<Vec<String>, String>>,
    entries: &[Option<history::Entry>],
    selection: &mut ListState,
    focused: bool,
    highlight: &str,
    border: ratatui::symbols::border::Set<'static>,
) {
    let style = Style::default()
        .bg(Color::Rgb(25, 43, 52))
        .fg(Color::Rgb(241, 245, 249))
        .bold();
    let rows = match history {
        None => vec![ListItem::new("Loading history...")],
        Some(Err(error)) => vec![ListItem::new(error.as_str())],
        Some(Ok(rows)) if rows.is_empty() => vec![ListItem::new("No commits yet")],
        Some(Ok(rows)) => {
            // Build only the rows that fit, so long History draws quickly;
            // `selection` keeps the scroll offset over all rows.
            let height = usize::from(area.height.saturating_sub(2)).max(1);
            let window = visible_window(selection, rows.len(), height);
            let structured = entries.len() == rows.len();
            let shown = || {
                window
                    .clone()
                    .filter_map(|index| entries.get(index)?.as_ref())
            };
            let mut finder = history::MatchFinder::new(highlight);
            // Borders and the selection marker take four columns.
            let layout = crate::ui::history_rows::RowLayout {
                graph_width: shown()
                    .map(|entry| entry.graph.cells.len())
                    .max()
                    .unwrap_or(0),
                width: usize::from(area.width.saturating_sub(4)),
                author_width: shown()
                    .map(|entry| {
                        unicode_width::UnicodeWidthStr::width(entry.commit.author.as_str())
                    })
                    .max()
                    .unwrap_or(0),
                ascii: border == crate::ui::ASCII_BORDER,
                now: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |elapsed| elapsed.as_secs() as i64),
            };
            let items = window
                .clone()
                .map(|index| match entries.get(index) {
                    Some(Some(entry)) if structured => {
                        ListItem::new(crate::ui::history_rows::history_line(
                            &rows[index],
                            entry,
                            &layout,
                            &mut finder,
                        ))
                    }
                    // A notice row, or raw rows without parsed commits.
                    _ => ListItem::new(crate::ui::highlighted_history_row(
                        &rows[index],
                        &mut finder,
                    )),
                })
                .collect();
            let mut shown_selection = ListState::default();
            shown_selection.select(selection.selected().map(|index| index - window.start));
            render_panel_list(
                frame,
                area,
                "HISTORY",
                items,
                &mut shown_selection,
                focused,
                border,
                style,
            );
            return;
        }
    };
    render_panel_list(
        frame, area, "HISTORY", rows, selection, focused, border, style,
    );
}

/// The rows a list of `len` rows shows in `height` rows: the stored offset,
/// moved just enough to keep the selection visible, as ratatui's `List`
/// scrolls. Updates the offset in `selection`.
pub(super) fn visible_window(
    selection: &mut ListState,
    len: usize,
    height: usize,
) -> std::ops::Range<usize> {
    let selected = selection
        .selected()
        .map(|index| index.min(len.saturating_sub(1)));
    let mut offset = selection.offset().min(len.saturating_sub(1));
    if let Some(selected) = selected {
        if selected < offset {
            offset = selected;
        } else if selected >= offset + height {
            offset = selected + 1 - height;
        }
    }
    *selection.offset_mut() = offset;
    if let Some(selected) = selected {
        selection.select(Some(selected));
    }
    offset..(offset + height).min(len)
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
