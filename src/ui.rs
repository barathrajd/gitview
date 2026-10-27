use crate::{
    app::{BranchAction, View},
    branches::{Branches, HeadState},
    details::CommitDetails,
    history,
    worktree::{Entry, Section},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::path::Path;

pub const ASCII_BORDER: ratatui::symbols::border::Set<'static> = ratatui::symbols::border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

pub const HELP: &str = "Navigation: Up/Down or j/k move; Enter/l open; Esc/h back\nGlobal: q quit and ? help outside text/confirmation prompts\nCockpit: Tab focus; b branches; / search; n next match in history\nWorking tree: s Stage selected file; u Unstage selected file; r Refresh\nBranches: Enter/l switch; c create; y confirm; n or Esc cancel\nSearch: Enter find; Esc cancel; Backspace edit\nCommit files: Up/Down or j/k select; Enter diff; Esc/h back\nDiff: Up/Down or j/k scroll; Esc/h back";

const ACCENT: Color = Color::Rgb(94, 234, 212);
const BORDER: Color = Color::Rgb(71, 85, 105);
const MUTED: Color = Color::Rgb(148, 163, 184);

fn page_block(
    title: impl Into<Line<'static>>,
    border: ratatui::symbols::border::Set<'static>,
) -> Block<'static> {
    Block::bordered()
        .border_set(border)
        .border_style(Style::default().fg(BORDER))
        .title(title.into().style(Style::default().fg(ACCENT).bold()))
}

fn render_key_hints(frame: &mut Frame<'_>, area: ratatui::layout::Rect, hints: &[(&str, &str)]) {
    let mut spans = Vec::with_capacity(hints.len() * 2);
    for (key, description) in hints {
        spans.push(Span::styled(
            format!("{key} "),
            Style::default().fg(ACCENT).bold(),
        ));
        spans.push(Span::styled(
            format!("{description}  "),
            Style::default().fg(MUTED),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_message_page(
    frame: &mut Frame<'_>,
    title: &'static str,
    message: &str,
    border: ratatui::symbols::border::Set<'static>,
    hints: &[(&str, &str)],
) {
    let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    frame.render_widget(
        Paragraph::new(message)
            .wrap(Wrap { trim: true })
            .block(page_block(title, border)),
        areas[0],
    );
    render_key_hints(frame, areas[1], hints);
}

pub struct Cockpit<'a> {
    pub repository: &'a Path,
    pub branches: Option<&'a Result<Branches, String>>,
    pub history: Option<&'a Result<Vec<String>, String>>,
    pub status: Option<&'a Result<Vec<Entry>, String>>,
    pub focused: View,
    pub history_selection: &'a mut ListState,
    pub working_tree_selection: &'a mut ListState,
}

pub enum Screen<'a> {
    Cockpit(Cockpit<'a>),
    Help,
    BranchInput(&'a str),
    BranchConfirmation {
        pending: bool,
        action: Option<&'a BranchAction>,
    },
    Branches {
        branches: Option<&'a Result<Branches, String>>,
        feedback: &'a str,
        selection: &'a mut ListState,
    },
    Diff {
        diff: &'a (u64, Option<Result<String, String>>),
        scroll: u16,
    },
    CommitDetails {
        details: &'a (u64, Option<Result<CommitDetails, String>>),
        selection: &'a mut ListState,
    },
    WorkingTree {
        status: Option<&'a Result<Vec<Entry>, String>>,
        selection: &'a mut ListState,
    },
    History {
        history: Option<&'a Result<Vec<String>, String>>,
        searching: bool,
        query: &'a str,
        search_message: &'a str,
        root: &'a Path,
        selection: &'a mut ListState,
    },
}

struct HistoryScreen<'a> {
    history: Option<&'a Result<Vec<String>, String>>,
    searching: bool,
    query: &'a str,
    search_message: &'a str,
    root: &'a Path,
    selection: &'a mut ListState,
}

pub fn render(
    frame: &mut Frame<'_>,
    screen: Screen<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    // Clear cells left by the previous full-screen view before rendering the
    // next one, including when switching from a wide pane layout to a dialog.
    frame.render_widget(Clear, frame.area());
    match screen {
        Screen::Cockpit(cockpit) => render_cockpit(frame, cockpit, border),
        Screen::Help => frame.render_widget(
            Paragraph::new(HELP)
                .wrap(Wrap { trim: true })
                .block(page_block("KEYBOARD SHORTCUTS", border)),
            frame.area(),
        ),
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
        Screen::BranchConfirmation { pending, action } => {
            let text = if pending {
                "Running branch operation...".to_owned()
            } else {
                match action {
                    Some(BranchAction::Switch(name)) => {
                        format!("Switch to local branch {name:?}?")
                    }
                    Some(BranchAction::Create { name, base }) => {
                        format!("Create and switch to local branch {name:?} from commit {base}?")
                    }
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
        Screen::Branches {
            branches,
            feedback,
            selection,
        } => render_branches(frame, branches, feedback, selection, border),
        Screen::Diff { diff, scroll } => render_diff(frame, diff, scroll, border),
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
    }
}

fn render_cockpit(
    frame: &mut Frame<'_>,
    cockpit: Cockpit<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let Cockpit {
        repository,
        branches,
        history,
        status,
        focused,
        history_selection,
        working_tree_selection,
    } = cockpit;
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(2),
    ])
    .split(frame.area());

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
    ]);
    frame.render_widget(
        Paragraph::new(header).block(
            Block::bordered()
                .border_set(border)
                .border_style(Style::default().fg(BORDER)),
        ),
        sections[0],
    );

    if frame.area().width < 88 {
        match focused {
            View::History => {
                render_history_panel(frame, sections[1], history, history_selection, true, border)
            }
            View::WorkingTree => render_worktree_panel(
                frame,
                sections[1],
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
                focused,
            },
            border,
        );
    }

    let (staged, unstaged, untracked) = status_counts(status);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" STAGED ", Style::default().fg(Color::Rgb(94, 234, 212))),
            Span::raw(staged.to_string()),
            Span::styled("   CHANGES ", Style::default().fg(Color::Rgb(251, 191, 36))),
            Span::raw(unstaged.to_string()),
            Span::styled(
                "   UNTRACKED ",
                Style::default().fg(Color::Rgb(148, 163, 184)),
            ),
            Span::raw(untracked.to_string()),
        ])),
        sections[2],
    );
    let action_hints = match focused {
        View::History => &[
            ("Tab", "pane"),
            ("Enter", "details"),
            ("b", "branches"),
            ("/", "search"),
            ("n", "next match"),
        ][..],
        View::WorkingTree => &[
            ("Tab", "pane"),
            ("Enter", "diff"),
            ("s", "stage"),
            ("u", "unstage"),
            ("r", "refresh"),
        ][..],
    };
    render_key_hints(frame, sections[3], action_hints);
    render_key_hints(
        frame,
        ratatui::layout::Rect {
            y: sections[3].y + 1,
            height: 1,
            ..sections[3]
        },
        &[("j/k", "move"), ("?", "help"), ("q", "quit")],
    );
}

fn render_worktree_panel(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    status: Option<&Result<Vec<Entry>, String>>,
    selection: &mut ListState,
    focused: bool,
    border: ratatui::symbols::border::Set<'static>,
) {
    let items = match status {
        None => vec![ListItem::new("Loading working tree...")],
        Some(Err(error)) => vec![ListItem::new(error.as_str())],
        Some(Ok(entries)) if entries.is_empty() => vec![ListItem::new("Working tree clean")],
        Some(Ok(entries)) => entries
            .iter()
            .map(|entry| {
                let (label, color) = match entry.section {
                    Section::Staged => ("STAGED", Color::Rgb(94, 234, 212)),
                    Section::Unstaged => ("EDITED", Color::Rgb(251, 191, 36)),
                    Section::Untracked => ("NEW", Color::Rgb(147, 197, 253)),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{label} "), Style::default().fg(color).bold()),
                    Span::raw(entry.path.to_string_lossy().into_owned()),
                ]))
            })
            .collect(),
    };
    render_panel_list(frame, area, "WORKTREE", items, selection, focused, border);
}

fn render_history_panel(
    frame: &mut Frame<'_>,
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
    render_panel_list(frame, area, "HISTORY", items, selection, focused, border);
}

struct Inspector<'a> {
    repository: &'a Path,
    branches: Option<&'a Result<Branches, String>>,
    history: Option<&'a Result<Vec<String>, String>>,
    status: Option<&'a Result<Vec<Entry>, String>>,
    selected: Option<usize>,
    focused: View,
}

fn render_inspector(
    frame: &mut Frame<'_>,
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
        focused,
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
    let enter_hint = if focused == View::History {
        "details"
    } else {
        "diff"
    };
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
        Line::raw(""),
        Line::from(vec![
            Span::styled("Enter ", Style::default().fg(ACCENT).bold()),
            Span::styled(format!("opens {enter_hint}"), Style::default().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("Tab ", Style::default().fg(ACCENT).bold()),
            Span::styled("changes focus", Style::default().fg(MUTED)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(page_block("INSPECTOR", border)),
        area,
    );
}

fn render_panel_list(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    title: &'static str,
    items: Vec<ListItem<'_>>,
    selection: &mut ListState,
    focused: bool,
    border: ratatui::symbols::border::Set<'static>,
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
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 43, 52))
                .fg(Color::Rgb(241, 245, 249))
                .bold(),
        );
    frame.render_stateful_widget(list, area, selection);
}

fn status_counts(status: Option<&Result<Vec<Entry>, String>>) -> (usize, usize, usize) {
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

fn render_branches(
    frame: &mut Frame<'_>,
    branches: Option<&Result<Branches, String>>,
    feedback: &str,
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    let (heading, lines): (String, Vec<String>) = match branches {
        None => (
            "Local branches - loading".into(),
            vec!["Loading branches...".into()],
        ),
        Some(Err(error)) => ("Local branches - error".into(), vec![error.clone()]),
        Some(Ok(data)) => {
            let heading = match &data.head {
                HeadState::Attached(name) => {
                    format!("Local branches - current: {}", name.to_string_lossy())
                }
                HeadState::Detached => "Local branches - detached HEAD".into(),
                HeadState::Unborn(name) => {
                    format!("Local branches - unborn: {}", name.to_string_lossy())
                }
            };
            let lines = if data.branches.is_empty() {
                vec!["No local branches".into()]
            } else {
                data.branches
                    .iter()
                    .map(|name| {
                        let current = matches!(
                            &data.head,
                            HeadState::Attached(head) if head == name
                        );
                        format!(
                            "{} {}",
                            if current { "*" } else { " " },
                            name.to_string_lossy()
                        )
                    })
                    .collect()
            };
            (heading, lines)
        }
    };
    let available = frame.area().height.saturating_sub(4);
    let width = usize::from(frame.area().width.max(1));
    let feedback_lines = feedback
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(width))
        .sum::<usize>()
        .max(1);
    let feedback_height = u16::try_from(feedback_lines)
        .unwrap_or(u16::MAX)
        .min(available)
        .min(8);
    let areas = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(feedback_height),
        Constraint::Length(1),
    ])
    .split(frame.area());
    let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>())
        .block(page_block(heading, border))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 43, 52))
                .fg(Color::Rgb(241, 245, 249))
                .bold(),
        );
    frame.render_stateful_widget(list, areas[0], selection);
    frame.render_widget(
        Paragraph::new(feedback).wrap(Wrap { trim: false }),
        areas[1],
    );
    render_key_hints(
        frame,
        areas[2],
        &[
            ("j/k", "move"),
            ("Enter/l", "switch"),
            ("c", "create"),
            ("Esc/h", "back"),
            ("q", "quit"),
            ("?", "help"),
        ],
    );
}

fn render_diff(
    frame: &mut Frame<'_>,
    diff: &(u64, Option<Result<String, String>>),
    scroll: u16,
    border: ratatui::symbols::border::Set<'static>,
) {
    let message = match &diff.1 {
        None => "Loading diff...",
        Some(Err(error)) => error,
        Some(Ok(text)) if text.is_empty() => "No textual diff",
        Some(Ok(text)) => text,
    };
    let sections =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    if frame.area().width >= 120 && matches!(diff.1, Some(Ok(_))) {
        let panes = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(sections[0]);
        let rows = crate::diff::side_by_side(message);
        let left: Vec<Line> = rows
            .iter()
            .map(|(line, _)| styled_diff_line(line))
            .collect();
        let right: Vec<Line> = rows
            .iter()
            .map(|(_, line)| styled_diff_line(line))
            .collect();
        frame.render_widget(
            Paragraph::new(Text::from(left))
                .scroll((scroll, 0))
                .block(page_block("BEFORE", border)),
            panes[0],
        );
        frame.render_widget(
            Paragraph::new(Text::from(right))
                .scroll((scroll, 0))
                .block(page_block("AFTER", border)),
            panes[1],
        );
    } else {
        let lines: Vec<Line> = message.lines().map(styled_diff_line).collect();
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .scroll((scroll, 0))
                .block(page_block("DIFF", border)),
            sections[0],
        );
    }
    render_key_hints(
        frame,
        sections[1],
        &[
            ("Up/Down or j/k", "scroll"),
            ("Esc/h", "back"),
            ("q", "quit"),
            ("?", "help"),
        ],
    );
}

fn render_commit_details(
    frame: &mut Frame<'_>,
    details: &(u64, Option<Result<CommitDetails, String>>),
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    match &details.1 {
        None => render_message_page(
            frame,
            "COMMIT",
            "Loading commit...",
            border,
            &[("Esc/h", "back"), ("q", "quit"), ("?", "help")],
        ),
        Some(Err(error)) => render_message_page(
            frame,
            "COMMIT ERROR",
            error,
            border,
            &[("Esc/h", "back"), ("q", "quit"), ("?", "help")],
        ),
        Some(Ok(commit)) => {
            let sections = Layout::vertical([
                Constraint::Length(6),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(frame.area());
            let meta = format!(
                "{}\nAuthor: {}\nDate: {}\nCommit: {}",
                commit.subject, commit.author, commit.date, commit.hash
            );
            frame.render_widget(
                Paragraph::new(meta).block(page_block("COMMIT", border)),
                sections[0],
            );
            let items = commit
                .files
                .iter()
                .map(|file| ListItem::new(file.to_string_lossy().into_owned()))
                .collect::<Vec<_>>();
            let list = List::new(items)
                .block(page_block("CHANGED FILES", border))
                .highlight_symbol("› ")
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(25, 43, 52))
                        .fg(Color::Rgb(241, 245, 249))
                        .bold(),
                );
            frame.render_stateful_widget(list, sections[1], selection);
            render_key_hints(
                frame,
                sections[2],
                &[
                    ("j/k", "select"),
                    ("Enter", "diff"),
                    ("Esc/h", "back"),
                    ("q", "quit"),
                    ("?", "help"),
                ],
            );
        }
    }
}

fn render_working_tree(
    frame: &mut Frame<'_>,
    status: Option<&Result<Vec<Entry>, String>>,
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    let lines: Vec<String> = match status {
        None => vec!["Loading working tree...".to_owned()],
        Some(Err(error)) => vec![error.clone()],
        Some(Ok(entries)) if entries.is_empty() => vec!["Working tree clean".to_owned()],
        Some(Ok(entries)) => entries
            .iter()
            .map(|entry| format!("{:?}  {}", entry.section, entry.path.to_string_lossy()))
            .collect(),
    };
    let sections =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>())
        .block(page_block("WORKING TREE", border))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 43, 52))
                .fg(Color::Rgb(241, 245, 249))
                .bold(),
        );
    frame.render_stateful_widget(list, sections[0], selection);
    render_key_hints(
        frame,
        sections[1],
        &[
            ("j/k", "move"),
            ("Enter", "diff"),
            ("s", "stage"),
            ("u", "unstage"),
            ("r", "refresh"),
            ("Tab", "history"),
            ("q", "quit"),
        ],
    );
}

fn render_history(
    frame: &mut Frame<'_>,
    screen: HistoryScreen<'_>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let lines = match screen.history {
        None => vec!["Loading history...".to_string()],
        Some(Err(error)) => vec![error.clone()],
        Some(Ok(rows)) if rows.is_empty() => vec!["No commits yet".to_string()],
        Some(Ok(rows)) => rows
            .iter()
            .map(|row| history::display_row(row).to_owned())
            .collect(),
    };
    let items = lines.into_iter().map(ListItem::new).collect::<Vec<_>>();
    let sections =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    let title = if screen.searching {
        format!("SEARCH: {}_", screen.query)
    } else if !screen.search_message.is_empty() {
        screen.search_message.to_owned()
    } else {
        format!("HISTORY - {}", screen.root.display())
    };
    let list = List::new(items)
        .block(page_block(title, border))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 43, 52))
                .fg(Color::Rgb(241, 245, 249))
                .bold(),
        );
    frame.render_stateful_widget(list, sections[0], screen.selection);
    let hints = if screen.searching {
        &[("Enter", "find"), ("Esc", "cancel"), ("Backspace", "edit")][..]
    } else {
        &[
            ("j/k", "move"),
            ("Enter", "details"),
            ("b", "branches"),
            ("/", "search"),
            ("n", "next match"),
            ("Tab", "working tree"),
            ("q", "quit"),
        ][..]
    };
    render_key_hints(frame, sections[1], hints);
}

fn styled_diff_line(line: &str) -> Line<'static> {
    let color = if line.starts_with('+') && !line.starts_with("+++") {
        Color::Green
    } else if line.starts_with('-') && !line.starts_with("---") {
        Color::Red
    } else {
        Color::Reset
    };
    Line::styled(line.to_owned(), Style::default().fg(color))
}
