use crate::ui::Viewport;
use crate::{
    app::View,
    history,
    worktree::{Entry, Section},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use super::commit::render_repository_header;
use super::hints::render_key_hints_padded;
use super::panels::worktree_selection_style;
use super::screen::Cockpit;
use super::theme::{ACCENT, BORDER, MUTED};

pub(super) fn render_cockpit(
    frame: &mut Viewport<'_, '_>,
    cockpit: Cockpit<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let mut workspace = crate::workspace::Workspace::new(false);
    workspace.focus(cockpit.focused.panel());
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
        branch_selection,
        sessions,
        page_open,
    } = cockpit;
    // Only the focused list is searched.
    let query = search_prompt.as_ref().map_or("", |prompt| prompt.query);
    let (highlight, worktree_highlight, branch_highlight) = match focused {
        View::History => (query, "", ""),
        View::WorkingTree => ("", query, ""),
        View::Branches => ("", "", query),
        View::Sessions => ("", "", ""),
    };
    let mut hints = workspace_hints(focused);
    if page_open {
        // The page in the centre hands its keys to this bar instead.
        hints.clear();
    } else if search_prompt.is_some() {
        hints = vec![("Enter", "stay"), ("Esc", "back")];
        if focused == View::History {
            hints.push(("Tab", "full list"));
        }
    }
    // One row of hints; those that don't fit fold into `? more`.
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
            render_sidebar(
                frame,
                area,
                Sidebar {
                    sessions: sessions.as_ref(),
                    branches,
                    branch_selection,
                    branch_highlight,
                    status,
                    working_tree_selection,
                    worktree_highlight,
                    focused,
                    // The only panel shown is the focused one.
                    sidebar_focused: true,
                },
                border,
            );
        }
    } else if let Some((sidebar, center)) = side_by_side {
        render_sidebar(
            frame,
            sidebar,
            Sidebar {
                sessions: sessions.as_ref(),
                branches,
                branch_selection,
                branch_highlight,
                status,
                working_tree_selection,
                worktree_highlight,
                focused,
                sidebar_focused: focused != View::History,
            },
            border,
        );
        render_history_panel(
            frame,
            center,
            history,
            history_entries,
            history_selection,
            focused == View::History,
            highlight,
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
    if !hints.is_empty() {
        render_key_hints_padded(frame, sections[3], &hints);
    }
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
    // Only the focused panel marks its selection.
    display_selection.select(
        selection
            .selected()
            .filter(|_| focused)
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
            // Raw `git log --graph` rows carry no parsed commits.
            let title = if !entries.is_empty() && entries.iter().all(Option::is_none) {
                "HISTORY · RAW"
            } else {
                "HISTORY"
            };
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
                title,
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

pub(super) fn render_panel_list(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    title: &str,
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

/// What the sidebar's stacked sections draw.
struct Sidebar<'a, 'b> {
    sessions: Option<&'a crate::sessions_page::SessionSection>,
    branches: Option<&'a Result<crate::branches::Branches, String>>,
    branch_selection: &'b mut ListState,
    branch_highlight: &'a str,
    status: Option<&'a Result<Vec<Entry>, String>>,
    working_tree_selection: &'b mut ListState,
    worktree_highlight: &'a str,
    focused: View,
    /// The sidebar holds the focus, so its focused section is marked.
    sidebar_focused: bool,
}

/// Rows a sidebar section needs: its rows plus two borders, at least one row.
fn section_rows(rows: usize) -> u16 {
    u16::try_from(rows.max(1) + 2).unwrap_or(u16::MAX)
}

/// The sidebar: Branches stacked above the working tree. Each section takes
/// what its rows need up to a share of the height, and the focused section's
/// share is larger; the working tree takes the rest.
fn render_sidebar(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    sidebar: Sidebar<'_, '_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let Sidebar {
        sessions,
        branches,
        branch_selection,
        branch_highlight,
        status,
        working_tree_selection,
        worktree_highlight,
        focused,
        sidebar_focused,
    } = sidebar;
    let session_rows = sessions.map_or(1, |section| section.rows.len());
    let branch_count = match branches {
        Some(Ok(data)) => data.branches.len(),
        _ => 1,
    };
    let [session_area, branch_area, files_area] =
        sidebar_sections(area, session_rows, branch_count, focused);
    render_session_section(
        frame,
        session_area,
        sessions,
        sidebar_focused && focused == View::Sessions,
        border,
    );
    render_branch_section(
        frame,
        branch_area,
        branches,
        branch_selection,
        sidebar_focused && focused == View::Branches,
        branch_highlight,
        border,
    );
    render_worktree_panel(
        frame,
        files_area,
        "WORKTREE",
        status,
        working_tree_selection,
        sidebar_focused && focused == View::WorkingTree,
        worktree_highlight,
        border,
    );
}

/// The Branches section: local branches, the current one marked `*`.
fn render_branch_section(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    branches: Option<&Result<crate::branches::Branches, String>>,
    selection: &mut ListState,
    focused: bool,
    highlight: &str,
    border: ratatui::symbols::border::Set<'static>,
) {
    use crate::branches::HeadState;
    let mut finder = history::MatchFinder::new(highlight);
    let (title, items) = match branches {
        None => (
            "BRANCHES".to_owned(),
            vec![ListItem::new("Loading branches...")],
        ),
        Some(Err(error)) => ("BRANCHES".to_owned(), vec![ListItem::new(error.clone())]),
        Some(Ok(data)) if data.branches.is_empty() => (
            "BRANCHES".to_owned(),
            vec![ListItem::new("No local branches")],
        ),
        Some(Ok(data)) => (
            format!("BRANCHES ({})", data.branches.len()),
            data.branches
                .iter()
                .map(|name| {
                    let current = matches!(&data.head, HeadState::Attached(head) if head == name);
                    let name = name.to_string_lossy();
                    let mut spans = vec![if current {
                        Span::styled("* ", Style::default().fg(ACCENT).bold())
                    } else {
                        Span::raw("  ")
                    }];
                    spans.extend(crate::ui::highlighted_spans(
                        &name,
                        &finder.positions(&name),
                    ));
                    ListItem::new(Line::from(spans))
                })
                .collect(),
        ),
    };
    // An unfocused section hides its selection, so the sidebar shows one
    // marker; the selection itself is kept for when focus returns.
    let mut hidden = ListState::default();
    *hidden.offset_mut() = selection.offset();
    render_panel_list(
        frame,
        area,
        &title,
        items,
        if focused { selection } else { &mut hidden },
        focused,
        border,
        Style::default()
            .bg(Color::Rgb(25, 43, 52))
            .fg(Color::Rgb(241, 245, 249))
            .bold(),
    );
}

/// Where a page opens inside the workspace in `area`: the centre panel, or
/// the sidebar when only the sidebar is shown, or the panels' whole area.
pub fn cockpit_centre_area(
    area: ratatui::layout::Rect,
    workspace: &crate::workspace::Workspace,
) -> ratatui::layout::Rect {
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    let body = sections[1];
    let layout = workspace.gitview_layout(ratatui::layout::Rect {
        height: body.height.saturating_add(1),
        ..body
    });
    layout.center.or(layout.sidebar).unwrap_or(body)
}

/// The Sessions section: each agent session's name and state.
fn render_session_section(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    section: Option<&crate::sessions_page::SessionSection>,
    focused: bool,
    border: ratatui::symbols::border::Set<'static>,
) {
    let mut selection = ListState::default();
    let (title, items) = match section {
        None => (
            "SESSIONS".to_owned(),
            vec![ListItem::new("Loading sessions...")],
        ),
        Some(section) if section.loading => (
            "SESSIONS".to_owned(),
            vec![ListItem::new("Loading sessions...")],
        ),
        Some(section) if section.rows.is_empty() => (
            "SESSIONS".to_owned(),
            vec![ListItem::new(Line::styled(
                "No agent sessions",
                Style::default().fg(MUTED),
            ))],
        ),
        Some(section) => {
            let query = section
                .search
                .as_ref()
                .map_or("", |(query, _)| query.as_str());
            let mut finder = history::MatchFinder::new(query);
            let items: Vec<ListItem<'static>> = section
                .rows
                .iter()
                .map(|(name, state)| {
                    let mut spans = crate::ui::highlighted_spans(name, &finder.positions(name));
                    spans.push(Span::styled(
                        format!("  {state}"),
                        Style::default().fg(MUTED),
                    ));
                    ListItem::new(Line::from(spans))
                })
                .collect();
            if focused {
                selection.select(Some(section.selected));
            }
            (format!("SESSIONS ({})", section.rows.len()), items)
        }
    };
    render_panel_list(
        frame,
        area,
        &title,
        items,
        &mut selection,
        focused,
        border,
        Style::default()
            .bg(Color::Rgb(25, 43, 52))
            .fg(Color::Rgb(241, 245, 249))
            .bold(),
    );
}

/// The workspace hint bar for the focused view, before an inline search or
/// an open page replaces it.
pub fn workspace_hints(focused: View) -> Vec<(&'static str, &'static str)> {
    let mut hints = match focused {
        View::History => vec![
            ("Tab", "pane"),
            ("W", "worktree"),
            ("Enter/l", "details"),
            ("f", "path history"),
            ("D", "compare"),
            ("o", "object"),
            ("v", "raw graph"),
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
        View::Branches => vec![
            ("Tab", "pane"),
            ("Enter/l", "switch"),
            ("c", "create"),
            ("r", "rename"),
            ("d", "delete"),
            ("-", "previous checkout"),
            ("t", "tags"),
            ("/", "search"),
        ],
        View::Sessions => vec![
            ("Tab", "pane"),
            ("Enter", "review"),
            ("d", "discard"),
            ("r", "refresh"),
            ("/", "search"),
        ],
    };
    hints.extend([("j/k", "move"), ("?", "help"), ("q", "quit")]);
    hints
}

/// The sidebar's Sessions, Branches, and working-tree areas: Sessions and
/// Branches each take what their rows need up to a share of the height
/// that doubles while focused; the working tree takes the rest.
pub fn sidebar_sections(
    area: ratatui::layout::Rect,
    session_rows: usize,
    branch_rows: usize,
    focused: View,
) -> [ratatui::layout::Rect; 3] {
    let fit = |rows: usize, share: u16, focus: View| {
        let share = if focused == focus { share * 2 } else { share };
        section_rows(rows).min(area.height * share / 100).max(3)
    };
    Layout::vertical([
        Constraint::Length(fit(session_rows, 20, View::Sessions)),
        Constraint::Length(fit(branch_rows, 30, View::Branches)),
        Constraint::Min(3),
    ])
    .areas(area)
}
