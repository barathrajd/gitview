use crate::{
    app::BranchAction,
    branches::{Branches, HeadState},
    details::CommitDetails,
    history,
    worktree::Entry,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
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

pub const HELP: &str = "Up/Down or j/k  Navigate and scroll\nEnter / Esc or h/l  Open / back (l / h)\nTab      Switch history / working tree\nb        Browse local branches\nc        Create a branch (in branch view)\ny / n    Confirm / cancel branch operation\n/        Search message, author or hash\nn        Next search match\ns        Stage selected file\nu        Unstage selected file\nr        Refresh working tree\n?        Toggle help\nq        Quit";

pub enum Screen<'a> {
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
    match screen {
        Screen::Help => frame.render_widget(
            Paragraph::new(HELP).block(Block::bordered().border_set(border).title("Help")),
            frame.area(),
        ),
        Screen::BranchInput(input) => {
            let text = format!("New local branch name: {input}_\n\nEnter: continue   Esc: cancel");
            frame.render_widget(
                Paragraph::new(text)
                    .block(Block::bordered().border_set(border).title("Create branch")),
                frame.area(),
            );
        }
        Screen::BranchConfirmation { pending, action } => {
            let text = if pending {
                "Running branch operation...".to_owned()
            } else {
                match action {
                    Some(BranchAction::Switch(name)) => {
                        format!("Switch to local branch {name:?}?\n\ny: confirm    n / Esc: cancel")
                    }
                    Some(BranchAction::Create { name, base }) => format!(
                        "Create and switch to local branch {name:?} from commit {base}?\n\ny: confirm    n / Esc: cancel"
                    ),
                    None => "Branch operation unavailable".to_owned(),
                }
            };
            frame.render_widget(
                Paragraph::new(text).wrap(Wrap { trim: false }).block(
                    Block::bordered()
                        .border_set(border)
                        .title("Confirm branch operation"),
                ),
                frame.area(),
            );
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
    let available = frame.area().height.saturating_sub(3);
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
    let areas = Layout::vertical([Constraint::Min(3), Constraint::Length(feedback_height)])
        .split(frame.area());
    let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>())
        .block(Block::bordered().border_set(border).title(format!(
            "{heading} | Enter/l switch | c create | Esc/h back"
        )))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, areas[0], selection);
    frame.render_widget(
        Paragraph::new(feedback).wrap(Wrap { trim: false }),
        areas[1],
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
    if frame.area().width >= 120 && matches!(diff.1, Some(Ok(_))) {
        let panes = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());
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
            Paragraph::new(Text::from(left)).scroll((scroll, 0)).block(
                Block::bordered()
                    .border_set(border)
                    .title("Before - up/down or j/k scroll | Esc back"),
            ),
            panes[0],
        );
        frame.render_widget(
            Paragraph::new(Text::from(right))
                .scroll((scroll, 0))
                .block(Block::bordered().border_set(border).title("After")),
            panes[1],
        );
    } else {
        let lines: Vec<Line> = message.lines().map(styled_diff_line).collect();
        frame.render_widget(
            Paragraph::new(Text::from(lines)).scroll((scroll, 0)).block(
                Block::bordered()
                    .border_set(border)
                    .title("Diff - up/down or j/k scroll | Esc back"),
            ),
            frame.area(),
        );
    }
}

fn render_commit_details(
    frame: &mut Frame<'_>,
    details: &(u64, Option<Result<CommitDetails, String>>),
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    match &details.1 {
        None => frame.render_widget(
            Paragraph::new("Loading commit...")
                .block(Block::bordered().border_set(border).title("Commit")),
            frame.area(),
        ),
        Some(Err(error)) => frame.render_widget(
            Paragraph::new(error.as_str())
                .block(Block::bordered().border_set(border).title("Commit error")),
            frame.area(),
        ),
        Some(Ok(commit)) => {
            let sections =
                Layout::vertical([Constraint::Length(6), Constraint::Min(3)]).split(frame.area());
            let meta = format!(
                "{}\nAuthor: {}\nDate: {}\nCommit: {}",
                commit.subject, commit.author, commit.date, commit.hash
            );
            frame.render_widget(
                Paragraph::new(meta).block(
                    Block::bordered()
                        .border_set(border)
                        .title("Commit - Esc back"),
                ),
                sections[0],
            );
            let items = commit
                .files
                .iter()
                .map(|file| ListItem::new(file.to_string_lossy().into_owned()))
                .collect::<Vec<_>>();
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .border_set(border)
                        .title("Files - up/down or j/k select | Enter diff"),
                )
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, sections[1], selection);
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
    let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>())
        .block(
            Block::bordered()
                .border_set(border)
                .title("Working tree - Enter diff | s stage | u unstage | Tab history"),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, frame.area(), selection);
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
    let title = if screen.searching {
        format!("Search: {}_ (Enter find | Esc cancel)", screen.query)
    } else if !screen.search_message.is_empty() {
        screen.search_message.to_owned()
    } else {
        format!(
            "Gitview - b branches | / search | Tab changes | ? help - {}",
            screen.root.display()
        )
    };
    let list = List::new(items)
        .block(Block::bordered().border_set(border).title(title))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, frame.area(), screen.selection);
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
