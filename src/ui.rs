use crate::{
    app::{BranchAction, View},
    branches::{Branches, HeadState},
    config::FolderProfile,
    details::CommitDetails,
    history,
    worktree::{Entry, Section},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, Wrap},
};
use std::{
    cell::RefCell,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    sync::OnceLock,
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::{SyntaxReference, SyntaxSet},
};
use unicode_width::UnicodeWidthChar;

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

pub const HELP: &str = concat!(
    "KEY                 DETAILS                           KEY                 DETAILS\n",
    "Up/Down             Move list selection                j/k                 Move list selection\n",
    "Enter               Open selected item                 l                   Open selected item\n",
    "Esc                 Back or cancel                     h                   Back or cancel\n",
    "Tab                 Switch history/worktree pane       q                   Quit outside prompts\n",
    "H                   Open history outside prompts\n",
    "W                   Open working tree outside prompts\n",
    "?                   Help outside prompts               b                   Browse local branches\n",
    "f                   Filter history by a literal path; empty clears\n",
    "/                   Search commit history               s                   Stage selected file\n",
    "u                   Unstage selected file               r                   Refresh worktree\n",
    "S                   Stage all worktree changes\n",
    "U                   Unstage all staged changes\n",
    "p                   Git config profiles\n",
    "a                   Amend the current commit in the worktree\n",
    "c                   Commit in worktree / create branch in branch list\n",
    "i                   Inspect the selected path's ignore rule\n",
    "I                   Inspect an arbitrary repository-relative path\n",
    "R                   Remove and stage deletion of a tracked path\n",
    "M                   Move a clean tracked path and stage its rename\n",
    "x                   Stop tracking the selected path and keep its working copy\n",
    "n                   Cancel branch operation            Backspace           Edit search query\n",
    "Diff: [/] selects hunks; s stages an unstaged hunk and u unstages a staged hunk.\n",
    "S stages all changes in the file; U unstages all staged changes in the file.\n",
    "In search, Up/Down select matches and Enter opens commit details. Backspace edits; Esc returns.\n",
    "In input fields, regular letters are text; ? opens page help."
);

const HELP_ROWS: &[(&str, &str, &str, &str)] = &[
    (
        "Up/Down",
        "Move list selection",
        "j/k",
        "Move list selection",
    ),
    ("Enter", "Open selected item", "l", "Open selected item"),
    ("Esc", "Back or cancel", "h", "Back or cancel"),
    ("Tab", "Switch pane", "q", "Quit outside prompts"),
    ("?", "Help outside prompts", "b", "Browse local branches"),
    ("p", "Git config profiles", "H", "Open history"),
    ("/", "Search history", "s", "Stage selected file"),
    ("u", "Unstage selected file", "r", "Refresh worktree"),
    (
        "i",
        "Inspect selected path ignore rule",
        "x",
        "Stop tracking and keep working copy",
    ),
    (
        "I",
        "Inspect ignore rule for a path",
        "R",
        "Remove tracked path and stage deletion",
    ),
    (
        "M",
        "Move clean tracked path and stage rename",
        "d",
        "Discard selected unstaged or untracked file",
    ),
    (
        "c",
        "Commit or create branch by page",
        "y",
        "Confirm branch",
    ),
    (
        "n",
        "Cancel branch operation",
        "Backspace",
        "Edit search query",
    ),
    (
        "[/]",
        "Select diff hunk",
        "S/U",
        "Stage/unstage whole file in diff",
    ),
    ("H", "Open history", "W", "Open working tree"),
    (
        "S",
        "Stage all worktree changes",
        "U",
        "Unstage all staged changes",
    ),
];

const ACCENT: Color = Color::Rgb(94, 234, 212);
const BORDER: Color = Color::Rgb(71, 85, 105);
const MUTED: Color = Color::Rgb(148, 163, 184);
const BAR_BACKGROUND: Color = Color::Rgb(30, 41, 59);

fn page_block(
    title: impl Into<Line<'static>>,
    border: ratatui::symbols::border::Set<'static>,
) -> Block<'static> {
    Block::bordered()
        .border_set(border)
        .border_style(Style::default().fg(BORDER))
        .title(title.into().style(Style::default().fg(ACCENT).bold()))
}

fn hint_rows<'a>(
    hints: &'a [(&'a str, &'a str)],
    width: u16,
    compact: bool,
) -> Vec<Vec<(&'a str, &'a str)>> {
    let width = usize::from(width.max(1));
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut row_width = 0usize;
    for hint in hints {
        let hint_width = hint.0.len() + hint.1.len() + if compact { 1 } else { 4 };
        let separator = usize::from(!row.is_empty()) * 2;
        if !row.is_empty() && row_width + separator + hint_width > width {
            rows.push(std::mem::take(&mut row));
            row_width = 0;
        }
        row_width += usize::from(!row.is_empty()) * 2 + hint_width;
        row.push(*hint);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

fn render_key_hints(frame: &mut Frame<'_>, area: ratatui::layout::Rect, hints: &[(&str, &str)]) {
    render_key_hints_with_spacing(frame, area, hints, area.width <= 80);
}

fn render_key_hints_padded(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    hints: &[(&str, &str)],
) {
    render_key_hints_with_spacing(frame, area, hints, false);
}

fn render_key_hints_with_spacing(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    hints: &[(&str, &str)],
    compact: bool,
) {
    let rows = hint_rows(hints, area.width, compact);
    let lines = rows
        .iter()
        .map(|row| {
            let mut spans = Vec::with_capacity(row.len() * 3);
            for (index, (key, details)) in row.iter().enumerate() {
                if index > 0 {
                    spans.push(Span::styled(
                        "  ",
                        Style::default().fg(BORDER).bg(BAR_BACKGROUND),
                    ));
                }
                spans.push(Span::styled(
                    if compact {
                        (*key).to_owned()
                    } else {
                        format!(" {key} ")
                    },
                    Style::default()
                        .fg(Color::Rgb(12, 24, 30))
                        .bg(ACCENT)
                        .bold(),
                ));
                spans.push(Span::styled(
                    if compact {
                        format!(" {details}")
                    } else {
                        format!(" {details} ")
                    },
                    Style::default()
                        .fg(Color::Rgb(226, 232, 240))
                        .bg(BAR_BACKGROUND),
                ));
            }
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Text::from(lines)).style(Style::default().bg(BAR_BACKGROUND)),
        area,
    );
}

fn hint_row_count(hints: &[(&str, &str)], width: u16) -> u16 {
    u16::try_from(hint_rows(hints, width, width <= 80).len()).unwrap_or(u16::MAX)
}

fn render_help(frame: &mut Frame<'_>, border: ratatui::symbols::border::Set<'static>) {
    let rows = HELP_ROWS
        .iter()
        .map(|(key1, details1, key2, details2)| {
            Row::new(vec![
                Cell::from(*key1).style(Style::default().fg(ACCENT).bold()),
                Cell::from(*details1).style(Style::default().fg(MUTED)),
                Cell::from(*key2).style(Style::default().fg(ACCENT).bold()),
                Cell::from(*details2).style(Style::default().fg(MUTED)),
            ])
        })
        .collect::<Vec<_>>();
    let sections =
        Layout::vertical([Constraint::Min(12), Constraint::Length(3)]).split(frame.area());
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(["KEY", "DETAILS", "KEY", "DETAILS"]).style(Style::default().fg(ACCENT).bold()),
    )
    .column_spacing(1)
    .block(page_block("KEYBOARD SHORTCUTS", border));
    frame.render_widget(table, sections[0]);
    frame.render_widget(
        Paragraph::new(
            "Esc/? closes help. H opens history outside prompts. Search: Up/Down selects; Enter opens; Backspace edits; Esc returns.\nIn input fields, regular letters are text; ? opens page help.",
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(MUTED)),
        sections[1],
    );
}

pub fn render_help_overlay(
    frame: &mut Frame<'_>,
    page: HelpPage,
    border: ratatui::symbols::border::Set<'static>,
) {
    const HISTORY: &[(&str, &str)] = &[
        ("Up/Down, j/k", "Move through commits"),
        ("Enter, l", "Open selected commit"),
        ("/", "Search commit history"),
        ("f", "Filter history by literal path; empty clears"),
        ("b", "Browse local branches"),
        ("Tab", "Open working tree"),
        ("W", "Open working tree"),
        ("H", "Return to history"),
        ("q", "Quit Gitview"),
    ];
    const WORKTREE: &[(&str, &str)] = &[
        ("Up/Down, j/k", "Select a changed file"),
        ("Enter", "Open file diff"),
        ("c", "Commit staged files"),
        ("a", "Amend the current commit"),
        ("d", "Discard selected unstaged or untracked file"),
        ("x", "Stop tracking selected file and keep working copy"),
        ("i", "Inspect ignore rule for selected path"),
        ("I", "Inspect ignore rule for an entered path"),
        ("R", "Remove a tracked path and stage its deletion"),
        ("M", "Move a clean tracked path and stage its rename"),
        ("s", "Stage selected file"),
        ("u", "Unstage selected file"),
        ("S", "Stage all working tree changes"),
        ("U", "Unstage all staged changes"),
        ("r", "Refresh working tree"),
        ("b", "Browse local branches"),
        ("Tab, H", "Open history"),
        ("q", "Quit Gitview"),
    ];
    const BRANCHES: &[(&str, &str)] = &[
        ("Up/Down, j/k", "Select a local branch"),
        ("Enter, l", "Switch to selected branch"),
        ("c", "Create a branch"),
        ("t", "Open local tags"),
        ("r", "Rename the selected local branch"),
        ("-", "Review previous checkout"),
        ("d", "Delete selected branch if Git permits"),
        ("y", "Confirm branch operation"),
        ("n, Esc", "Cancel branch operation"),
        ("H", "Open history"),
        ("W", "Open working tree"),
        ("q", "Quit Gitview"),
    ];
    const BRANCH_INPUT: &[(&str, &str)] = &[
        ("Type", "Enter a local branch name"),
        ("Backspace", "Delete a character"),
        ("Enter", "Continue to confirmation"),
        ("Esc", "Cancel branch name entry"),
    ];
    const IGNORE_PATH_INPUT: &[(&str, &str)] = &[
        ("Type", "Enter a repository-relative path"),
        ("Enter", "Inspect its ignore rule"),
        ("Backspace", "Delete a character"),
        ("Esc", "Cancel"),
    ];
    const BRANCH_CONFIRMATION: &[(&str, &str)] =
        &[("y", "Confirm operation"), ("n, Esc", "Cancel operation")];
    const COMMIT: &[(&str, &str)] = &[
        ("Up/Down, j/k", "Select a changed file"),
        ("Enter, l", "Open selected file diff"),
        ("Esc, h", "Return to history"),
        ("H", "Return to history"),
        ("W", "Open working tree"),
        ("q", "Quit Gitview"),
    ];
    const COMMIT_MESSAGE: &[(&str, &str)] = &[
        ("Type", "Write a commit message"),
        ("Tab", "Switch subject and description"),
        ("Ctrl+J", "Add a description line"),
        ("Ctrl+E", "Edit message in configured Git editor"),
        ("Enter", "Create commit"),
        ("Backspace", "Delete a character"),
        ("Esc", "Cancel commit"),
    ];
    const AMEND_CONFIRMATION: &[(&str, &str)] = &[
        (
            "y",
            "Rewrite HEAD with the reviewed message and staged files",
        ),
        ("n, Esc", "Return to the amend editor"),
    ];
    const WORKTREE_DIFF: &[(&str, &str)] = &[
        ("[, ]", "Select previous or next hunk"),
        ("v", "Toggle IDE or raw diff"),
        ("Tab", "Toggle side-by-side or unified layout"),
        ("s, u", "Stage or unstage selected hunk"),
        ("S, U", "Stage or unstage all file changes"),
        ("Up/Down, j/k", "Scroll diff"),
        ("Esc, h", "Return to working tree"),
        ("H", "Open history"),
        ("W", "Open working tree"),
        ("q", "Quit Gitview"),
    ];
    const COMMIT_DIFF: &[(&str, &str)] = &[
        ("v", "Toggle IDE or raw diff"),
        ("Tab", "Toggle side-by-side or unified layout"),
        ("Up/Down, j/k", "Scroll diff"),
        ("Esc, h", "Return to commit files"),
        ("H", "Return to history"),
        ("W", "Open working tree"),
        ("q", "Quit Gitview"),
    ];
    const SEARCH: &[(&str, &str)] = &[
        ("Type", "Edit search query"),
        ("Up/Down", "Select a matching commit"),
        ("Enter", "Open selected commit"),
        ("Backspace", "Delete a character"),
        ("Esc", "Return to history"),
    ];
    const PROFILES: &[(&str, &str)] = &[
        ("e", "Edit the profile registry in your editor"),
        ("r", "Reload and validate profile mappings"),
        ("Esc, h", "Return to the repository"),
        ("?", "Close this help panel"),
        ("q", "Quit Gitview"),
    ];
    let (title, hints) = match page {
        HelpPage::History => ("History", HISTORY),
        HelpPage::WorkingTree => ("Working tree", WORKTREE),
        HelpPage::Branches => ("Branches", BRANCHES),
        HelpPage::BranchInput => ("Branch name", BRANCH_INPUT),
        HelpPage::IgnorePathInput => ("Ignore-rule path", IGNORE_PATH_INPUT),
        HelpPage::BranchConfirmation => ("Confirm branch", BRANCH_CONFIRMATION),
        HelpPage::Commit => ("Commit", COMMIT),
        HelpPage::CommitMessage => ("Commit message", COMMIT_MESSAGE),
        HelpPage::AmendConfirmation => ("Confirm amend", AMEND_CONFIRMATION),
        HelpPage::WorktreeDiff => ("Working tree diff", WORKTREE_DIFF),
        HelpPage::CommitDiff => ("Commit diff", COMMIT_DIFF),
        HelpPage::Search => ("Search", SEARCH),
        HelpPage::Profiles => ("Git config profiles", PROFILES),
    };

    let screen = frame.area();
    let width = screen.width.clamp(1, 84);
    let height = screen
        .height
        .min(u16::try_from(hints.len() + 5).unwrap_or(u16::MAX))
        .max(1);
    let popup = ratatui::layout::Rect::new(
        screen.x + screen.width.saturating_sub(width) / 2,
        screen.y + screen.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let block = page_block(format!("HELP · {title}"), border);
    let content = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    let padded = ratatui::layout::Rect::new(
        content.x.saturating_add(1),
        content.y.saturating_add(1),
        content.width.saturating_sub(2),
        content.height.saturating_sub(2),
    );
    let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(padded);
    let rows = hints
        .iter()
        .map(|(key, details)| {
            Row::new(vec![
                Cell::from(*key).style(Style::default().fg(ACCENT).bold()),
                Cell::from(*details).style(Style::default().fg(MUTED)),
            ])
        })
        .collect::<Vec<_>>();
    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(12)]).column_spacing(1);
    frame.render_widget(table, areas[0]);
    frame.render_widget(
        Paragraph::new("Esc or ? closes help").style(Style::default().fg(MUTED)),
        areas[1],
    );
}

fn render_message_page(
    frame: &mut Frame<'_>,
    title: &'static str,
    message: &str,
    border: ratatui::symbols::border::Set<'static>,
    hints: &[(&str, &str)],
) {
    let areas = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(hint_row_count(hints, frame.area().width)),
    ])
    .split(frame.area());
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
    pub profile: &'a str,
    pub branches: Option<&'a Result<Branches, String>>,
    pub history: Option<&'a Result<Vec<String>, String>>,
    pub status: Option<&'a Result<Vec<Entry>, String>>,
    pub feedback: &'a str,
    pub focused: View,
    pub history_selection: &'a mut ListState,
    pub working_tree_selection: &'a mut ListState,
}

pub enum Screen<'a> {
    PathHistoryInput {
        path: &'a str,
    },
    Tags(&'a crate::tag_page::TagPage),
    Cockpit(Cockpit<'a>),
    Help,
    BranchInput(&'a str),
    RenameBranchInput {
        input: &'a str,
        name: &'a std::ffi::OsStr,
    },
    IgnorePathInput {
        path: &'a str,
        feedback: &'a str,
    },
    RemovePathInput {
        path: &'a str,
        feedback: &'a str,
    },
    RemovePathConfirmation {
        path: &'a std::ffi::OsStr,
        pending: bool,
    },
    MovePathInput {
        source: &'a str,
        destination: &'a str,
        destination_focused: bool,
        feedback: &'a str,
    },
    MovePathConfirmation {
        source: &'a std::ffi::OsStr,
        destination: &'a std::ffi::OsStr,
        pending: bool,
    },
    BranchConfirmation {
        pending: bool,
        action: Option<&'a BranchAction>,
    },
    CommitInput {
        repository: &'a Path,
        profile: &'a str,
        branches: Option<&'a Result<Branches, String>>,
        message: &'a str,
        description: &'a str,
        description_focused: bool,
        amend: bool,
        feedback: &'a str,
        entries: Option<&'a Result<Vec<Entry>, String>>,
        pending: bool,
    },
    AmendConfirmation {
        repository: &'a Path,
        profile: &'a str,
        branches: Option<&'a Result<Branches, String>>,
        entries: Option<&'a Result<Vec<Entry>, String>>,
        head: &'a str,
        original_message: &'a str,
        new_message: &'a str,
        pending: bool,
    },
    DiscardConfirmation {
        entry: &'a Entry,
        pending: bool,
    },
    UntrackConfirmation {
        entry: &'a Entry,
        pending: bool,
    },
    Branches {
        branches: Option<&'a Result<Branches, String>>,
        feedback: &'a str,
        selection: &'a mut ListState,
    },
    Diff {
        diff: &'a (u64, Option<Result<String, String>>),
        scroll: u16,
        selected_hunk: usize,
        raw: bool,
        side_by_side: bool,
        worktree_entry: Option<&'a Entry>,
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
    Search {
        history: Option<&'a Result<Vec<String>, String>>,
        matches: &'a [usize],
        query: &'a str,
        pending: bool,
        selection: &'a mut ListState,
    },
    Profiles {
        profiles: &'a [FolderProfile],
        selected: Option<usize>,
        active_profile: &'a str,
        registry_path: &'a Path,
        feedback: &'a str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpPage {
    History,
    WorkingTree,
    Branches,
    BranchInput,
    BranchConfirmation,
    Commit,
    CommitMessage,
    AmendConfirmation,
    WorktreeDiff,
    CommitDiff,
    Search,
    Profiles,
    IgnorePathInput,
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

fn render_profiles(
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

struct CommitWorkspace<'a> {
    repository: &'a Path,
    profile: &'a str,
    branches: Option<&'a Result<Branches, String>>,
    message: &'a str,
    description: &'a str,
    description_focused: bool,
    amend: bool,
    feedback: &'a str,
    entries: Option<&'a Result<Vec<Entry>, String>>,
    pending: bool,
}

struct AmendConfirmation<'a> {
    repository: &'a Path,
    profile: &'a str,
    branches: Option<&'a Result<Branches, String>>,
    entries: Option<&'a Result<Vec<Entry>, String>>,
    head: &'a str,
    original_message: &'a str,
    new_message: &'a str,
    pending: bool,
}

fn render_repository_header(
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

fn render_commit_workspace(
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

fn render_amend_confirmation(
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

fn render_cockpit(
    frame: &mut Frame<'_>,
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

fn render_worktree_panel(
    frame: &mut Frame<'_>,
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

fn worktree_items(
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

struct Inspector<'a> {
    repository: &'a Path,
    branches: Option<&'a Result<Branches, String>>,
    history: Option<&'a Result<Vec<String>, String>>,
    status: Option<&'a Result<Vec<Entry>, String>>,
    selected: Option<usize>,
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

fn render_panel_list(
    frame: &mut Frame<'_>,
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
    let hints = [
        ("j/k", "move"),
        ("Enter/l", "switch"),
        ("c", "create"),
        ("t", "tags"),
        ("r", "rename"),
        ("-", "previous checkout"),
        ("d", "delete"),
        ("Esc/h", "back"),
    ];
    let hint_height = hint_row_count(&hints, frame.area().width);
    let available = frame.area().height.saturating_sub(3 + hint_height + 1);
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
        Constraint::Length(hint_height),
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
    render_key_hints(frame, areas[2], &hints);
}

fn render_diff(
    frame: &mut Frame<'_>,
    diff: &(u64, Option<Result<String, String>>),
    scroll: u16,
    selected_hunk: usize,
    raw: bool,
    side_by_side: bool,
    worktree_entry: Option<&Entry>,
    border: ratatui::symbols::border::Set<'static>,
) {
    let message = match &diff.1 {
        None => "Loading diff...",
        Some(Err(error)) => error,
        Some(Ok(text)) if text.is_empty() => "No textual diff",
        Some(Ok(text)) => text,
    };
    let hunk_count = diff
        .1
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|text| text.lines().filter(|line| line.starts_with("@@ ")).count())
        .unwrap_or(0);
    let selectable_hunks =
        worktree_entry.is_some_and(|entry| entry.section != Section::Untracked) && hunk_count > 0;
    let hints = if worktree_entry.is_some() {
        let compact = frame.area().width <= 80;
        let mut hints = vec![
            (
                "v",
                if raw {
                    if compact { "IDE" } else { "IDE diff" }
                } else if compact {
                    "raw"
                } else {
                    "raw patch"
                },
            ),
            (
                "Tab",
                if side_by_side {
                    if compact { "unified" } else { "unified view" }
                } else if compact {
                    "split"
                } else {
                    "side-by-side"
                },
            ),
            ("s", if compact { "stage" } else { "stage hunk" }),
            ("u", if compact { "unstage" } else { "unstage hunk" }),
            ("S", "stage all"),
            ("U", "unstage all"),
        ];
        if selectable_hunks {
            hints.insert(0, ("[/]", "hunk"));
        }
        hints
    } else {
        vec![
            ("v", if raw { "IDE diff" } else { "raw patch" }),
            (
                "Tab",
                if side_by_side {
                    "unified view"
                } else {
                    "side-by-side"
                },
            ),
            ("Up/Down or j/k", "scroll"),
            ("Esc/h", "back"),
        ]
    };
    let sections = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(hint_row_count(&hints, frame.area().width)),
    ])
    .split(frame.area());
    if let Some(Ok(text)) = &diff.1 {
        if side_by_side {
            let panes =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(sections[0]);
            let content_width = usize::from(panes[0].width.min(panes[1].width))
                .saturating_sub(2)
                .saturating_sub(6)
                .max(1);
            let (left, right, empty_hint) = if raw {
                let (left, right) = aligned_raw_diff_lines(
                    &crate::diff::numbered_side_by_side(text),
                    content_width,
                );
                (left, right, "No file lines · Tab for unified raw patch")
            } else {
                let rows = cached_ide_diff_rows(diff.0, message, worktree_entry);
                let (left, right) = aligned_ide_diff_lines(&rows, content_width);
                (left, right, "No source lines · press v for raw patch")
            };
            let left = if left.is_empty() {
                vec![Line::raw(empty_hint)]
            } else {
                left
            };
            let right = if right.is_empty() {
                vec![Line::raw(empty_hint)]
            } else {
                right
            };
            frame.render_widget(
                Paragraph::new(Text::from(left))
                    .scroll((scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(page_block(
                        if selectable_hunks {
                            format!(
                                "BEFORE · HUNK {}/{}",
                                selected_hunk.saturating_add(1),
                                hunk_count
                            )
                        } else {
                            "BEFORE".to_owned()
                        },
                        border,
                    )),
                panes[0],
            );
            frame.render_widget(
                Paragraph::new(Text::from(right))
                    .scroll((scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(page_block(
                        if selectable_hunks {
                            format!(
                                "AFTER · HUNK {}/{}",
                                selected_hunk.saturating_add(1),
                                hunk_count
                            )
                        } else {
                            "AFTER".to_owned()
                        },
                        border,
                    )),
                panes[1],
            );
        } else {
            let content_width = usize::from(sections[0].width)
                .saturating_sub(2)
                .saturating_sub(14)
                .max(1);
            let (lines, title) = if raw {
                (
                    crate::diff::numbered_lines(text)
                        .iter()
                        .map(|line| {
                            styled_numbered_unified(line.old_number, line.new_number, line.text)
                        })
                        .collect(),
                    "RAW PATCH",
                )
            } else {
                let rows = cached_ide_diff_rows(diff.0, message, worktree_entry);
                (
                    aligned_ide_unified_lines(&rows, content_width),
                    "UNIFIED IDE DIFF",
                )
            };
            frame.render_widget(
                Paragraph::new(Text::from(lines))
                    .scroll((scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(page_block(title, border)),
                sections[0],
            );
        }
    } else {
        let lines: Vec<Line> = crate::diff::numbered_lines(message)
            .iter()
            .map(|line| styled_numbered_unified(line.old_number, line.new_number, line.text))
            .collect();
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .scroll((scroll, 0))
                .wrap(Wrap { trim: false })
                .block(page_block(
                    if raw && selectable_hunks {
                        format!(
                            "RAW PATCH · HUNK {}/{}",
                            selected_hunk.saturating_add(1),
                            hunk_count
                        )
                    } else if raw {
                        "RAW PATCH".to_owned()
                    } else {
                        "DIFF".to_owned()
                    },
                    border,
                )),
            sections[0],
        );
    }
    render_key_hints(frame, sections[1], &hints);
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
            &[("Esc/h", "back")],
        ),
        Some(Err(error)) => {
            render_message_page(frame, "COMMIT ERROR", error, border, &[("Esc/h", "back")])
        }
        Some(Ok(commit)) => {
            let hints = [
                ("j/k", "select"),
                ("Enter/l", "diff"),
                ("Esc/h", "back"),
                ("H", "history"),
            ];
            let sections = Layout::vertical([
                Constraint::Length(6),
                Constraint::Min(3),
                Constraint::Length(hint_row_count(&hints, frame.area().width)),
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
            render_key_hints(frame, sections[2], &hints);
        }
    }
}

fn render_working_tree(
    frame: &mut Frame<'_>,
    status: Option<&Result<Vec<Entry>, String>>,
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    let (items, selection_index) = worktree_items(status);
    let hints = [
        ("j/k", "move"),
        ("Enter", "diff"),
        ("s", "stage"),
        ("u", "unstage"),
        ("r", "refresh"),
        ("Tab", "history"),
        ("H", "history"),
        ("S", "stage all"),
        ("U", "unstage all"),
    ];
    let sections = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(hint_row_count(&hints, frame.area().width)),
    ])
    .split(frame.area());
    let list = List::new(items)
        .block(page_block("WORKING TREE", border))
        .highlight_symbol("› ")
        .highlight_style(worktree_selection_style(status, selection.selected()));
    let mut display_selection = ListState::default();
    display_selection.select(
        selection
            .selected()
            .and_then(|index| selection_index.get(index).copied().flatten()),
    );
    frame.render_stateful_widget(list, sections[0], &mut display_selection);
    render_key_hints(frame, sections[1], &hints);
}

fn worktree_selection_style(
    status: Option<&Result<Vec<Entry>, String>>,
    selected: Option<usize>,
) -> Style {
    let section = status
        .and_then(|result| result.as_ref().ok())
        .and_then(|entries| selected.and_then(|index| entries.get(index)))
        .map(|entry| entry.section);
    let background = match section {
        Some(Section::Staged) => Color::Rgb(20, 65, 63),
        Some(Section::Unstaged) => Color::Rgb(74, 55, 20),
        Some(Section::Untracked) => Color::Rgb(30, 58, 88),
        None => Color::Rgb(30, 41, 59),
    };
    Style::default().bg(background).bold()
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
    let hints = if screen.searching {
        &[("Enter", "find"), ("Esc", "cancel"), ("Backspace", "edit")][..]
    } else {
        &[
            ("j/k", "move"),
            ("Enter/l", "details"),
            ("b", "branches"),
            ("/", "search"),
            ("Tab", "working tree"),
        ][..]
    };
    let sections = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(hint_row_count(hints, frame.area().width)),
    ])
    .split(frame.area());
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
    render_key_hints(frame, sections[1], hints);
}

fn render_search(
    frame: &mut Frame<'_>,
    history: Option<&Result<Vec<String>, String>>,
    matches: &[usize],
    query: &str,
    pending: bool,
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    let items = match history {
        None => vec![ListItem::new("Loading commit history...")],
        Some(Err(error)) => vec![ListItem::new(error.as_str())],
        Some(Ok(_)) if pending && matches.is_empty() => {
            vec![ListItem::new("Waiting for search results...")]
        }
        Some(Ok(_)) if matches.is_empty() => vec![ListItem::new("No matching commits")],
        Some(Ok(rows)) => matches
            .iter()
            .filter_map(|index| rows.get(*index))
            .map(|row| ListItem::new(history::display_row(row)))
            .collect(),
    };
    let title = if pending {
        format!("SEARCH  {query}_   updating")
    } else {
        format!("SEARCH  {query}_   {} matches", matches.len())
    };
    let hints = [
        ("Up/Down", "select"),
        ("Enter", "open"),
        ("Backspace", "edit"),
        ("Esc", "back"),
    ];
    let sections = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(hint_row_count(&hints, frame.area().width)),
    ])
    .split(frame.area());
    let list = List::new(items)
        .block(page_block(title, border))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 43, 52))
                .fg(Color::Rgb(241, 245, 249))
                .bold(),
        );
    frame.render_stateful_widget(list, sections[0], selection);
    render_key_hints(frame, sections[1], &hints);
}

fn diff_line_style(line: &str) -> Style {
    if line.starts_with('+') && !line.starts_with("+++") {
        Style::default()
            .fg(Color::Rgb(134, 239, 172))
            .bg(Color::Rgb(24, 58, 43))
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default()
            .fg(Color::Rgb(252, 165, 165))
            .bg(Color::Rgb(63, 39, 44))
    } else {
        Style::default()
    }
}

#[derive(Clone)]
struct IdeDiffLine {
    number: Option<u32>,
    code: String,
    spans: Vec<(String, Style)>,
    background: Option<Color>,
    inline_change: Option<(usize, usize)>,
    inline_background: Option<Color>,
}

#[derive(Clone)]
struct IdeDiffRow {
    before: IdeDiffLine,
    after: IdeDiffLine,
}

thread_local! {
    static IDE_DIFF_CACHE: RefCell<Option<(u64, usize, String, Rc<Vec<IdeDiffRow>>)>> = const { RefCell::new(None) };
}

fn syntax_assets() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    (
        SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines),
        THEMES.get_or_init(ThemeSet::load_defaults),
    )
}

fn cached_ide_diff_rows(
    id: u64,
    message: &str,
    worktree_entry: Option<&Entry>,
) -> Rc<Vec<IdeDiffRow>> {
    let path = diff_path(message, worktree_entry);
    let path_key = path.to_string_lossy().into_owned();
    let message_pointer = message.as_ptr() as usize;

    IDE_DIFF_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((cached_id, cached_pointer, cached_path, rows)) = cache.as_ref()
            && *cached_id == id
            && *cached_pointer == message_pointer
            && *cached_path == path_key
        {
            return Rc::clone(rows);
        }

        let rows = Rc::new(build_ide_diff_rows(message, &path));
        *cache = Some((id, message_pointer, path_key, Rc::clone(&rows)));
        rows
    })
}

fn diff_path(message: &str, worktree_entry: Option<&Entry>) -> PathBuf {
    if let Some(entry) = worktree_entry {
        return PathBuf::from(&entry.path);
    }

    message
        .lines()
        .filter_map(|line| {
            line.strip_prefix("+++ ")
                .or_else(|| line.strip_prefix("--- "))
        })
        .find(|path| *path != "/dev/null")
        .map(|path| {
            let path = path.trim_matches('"');
            PathBuf::from(
                path.strip_prefix("b/")
                    .or_else(|| path.strip_prefix("a/"))
                    .unwrap_or(path),
            )
        })
        .unwrap_or_default()
}

fn diff_syntax<'a>(path: &Path, syntaxes: &'a SyntaxSet) -> &'a SyntaxReference {
    let extension = if path.file_name().is_some_and(|name| name == "Cargo.lock") {
        Some("toml")
    } else {
        path.extension().and_then(|extension| extension.to_str())
    };
    extension
        .and_then(|extension| syntaxes.find_syntax_by_extension(extension))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
}

fn build_ide_diff_rows(message: &str, path: &Path) -> Vec<IdeDiffRow> {
    let (syntaxes, themes) = syntax_assets();
    let syntax = diff_syntax(path, syntaxes);
    let theme = themes
        .themes
        .get("base16-ocean.dark")
        .or_else(|| themes.themes.values().next())
        .expect("Syntect default themes are present");
    let mut before_highlighter = HighlightLines::new(syntax, theme);
    let mut after_highlighter = HighlightLines::new(syntax, theme);
    let mut rows = Vec::new();

    for row in crate::diff::numbered_side_by_side(message) {
        if row.old_text.starts_with("@@ ") || row.new_text.starts_with("@@ ") {
            before_highlighter = HighlightLines::new(syntax, theme);
            after_highlighter = HighlightLines::new(syntax, theme);
            continue;
        }
        if row.old_number.is_none() && row.new_number.is_none() {
            continue;
        }

        let mut before = highlighted_ide_line(
            row.old_number,
            row.old_text,
            &mut before_highlighter,
            syntaxes,
        );
        let mut after = highlighted_ide_line(
            row.new_number,
            row.new_text,
            &mut after_highlighter,
            syntaxes,
        );
        if before.background.is_some() && after.background.is_some() {
            let (before_range, after_range) = inline_change_ranges(&before.code, &after.code);
            before.inline_change = before_range;
            after.inline_change = after_range;
            before.inline_background = before.background.map(inline_diff_background);
            after.inline_background = after.background.map(inline_diff_background);
        }
        rows.push(IdeDiffRow { before, after });
    }
    rows
}

fn highlighted_ide_line(
    number: Option<u32>,
    diff_text: &str,
    highlighter: &mut HighlightLines<'_>,
    syntaxes: &SyntaxSet,
) -> IdeDiffLine {
    let (code, background) = source_line(diff_text);
    if number.is_none() {
        return IdeDiffLine {
            number,
            code: String::new(),
            spans: Vec::new(),
            background,
            inline_change: None,
            inline_background: None,
        };
    }

    let input = format!("{code}\n");
    let spans = highlighter
        .highlight_line(&input, syntaxes)
        .map(|ranges| {
            ranges
                .into_iter()
                .filter_map(|(style, text)| {
                    let text = text.trim_end_matches('\n').trim_end_matches('\r');
                    (!text.is_empty()).then(|| {
                        (
                            text.to_owned(),
                            Style::default().fg(Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            )),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_else(|_| vec![(code.to_owned(), Style::default())]);

    IdeDiffLine {
        number,
        code: code.to_owned(),
        spans,
        background,
        inline_change: None,
        inline_background: None,
    }
}

fn inline_diff_background(background: Color) -> Color {
    match background {
        Color::Rgb(63, 39, 44) => Color::Rgb(112, 54, 64),
        Color::Rgb(24, 58, 43) => Color::Rgb(39, 112, 74),
        _ => background,
    }
}

fn inline_change_ranges(
    before: &str,
    after: &str,
) -> (Option<(usize, usize)>, Option<(usize, usize)>) {
    let before: Vec<_> = before.chars().collect();
    let after: Vec<_> = after.chars().collect();
    let mut prefix = 0;
    while prefix < before.len() && prefix < after.len() && before[prefix] == after[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < before.len().saturating_sub(prefix)
        && suffix < after.len().saturating_sub(prefix)
        && before[before.len() - suffix - 1] == after[after.len() - suffix - 1]
    {
        suffix += 1;
    }

    let before_end = before.len() - suffix;
    let after_end = after.len() - suffix;
    (
        (prefix < before_end).then_some((prefix, before_end)),
        (prefix < after_end).then_some((prefix, after_end)),
    )
}

fn source_line(diff_text: &str) -> (&str, Option<Color>) {
    if diff_text.starts_with('+') && !diff_text.starts_with("+++") {
        (&diff_text[1..], Some(Color::Rgb(24, 58, 43)))
    } else if diff_text.starts_with('-') && !diff_text.starts_with("---") {
        (&diff_text[1..], Some(Color::Rgb(63, 39, 44)))
    } else if diff_text.starts_with(' ') {
        (&diff_text[1..], None)
    } else {
        (diff_text, None)
    }
}

fn aligned_ide_diff_lines(
    rows: &[IdeDiffRow],
    content_width: usize,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();

    for row in rows {
        let old_wrapped = wrap_styled_line(&row.before, content_width);
        let new_wrapped = wrap_styled_line(&row.after, content_width);
        let height = old_wrapped.len().max(new_wrapped.len());

        for line_index in 0..height {
            old_lines.push(ide_line(
                &row.before,
                (line_index == 0).then_some(row.before.number).flatten(),
                old_wrapped.get(line_index),
            ));
            new_lines.push(ide_line(
                &row.after,
                (line_index == 0).then_some(row.after.number).flatten(),
                new_wrapped.get(line_index),
            ));
        }
    }

    (old_lines, new_lines)
}

fn aligned_raw_diff_lines(
    rows: &[crate::diff::NumberedDiffRow<'_>],
    content_width: usize,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut before = Vec::new();
    let mut after = Vec::new();

    for row in rows
        .iter()
        .filter(|row| row.old_number.is_some() || row.new_number.is_some())
    {
        let old_wrapped = wrap_diff_line(row.old_text, content_width);
        let new_wrapped = wrap_diff_line(row.new_text, content_width);
        let height = old_wrapped.len().max(new_wrapped.len());
        let old_style = diff_line_style(row.old_text);
        let new_style = diff_line_style(row.new_text);

        for line_index in 0..height {
            before.push(numbered_wrapped_line(
                (line_index == 0).then_some(row.old_number).flatten(),
                old_wrapped.get(line_index).map_or("", String::as_str),
                old_style,
            ));
            after.push(numbered_wrapped_line(
                (line_index == 0).then_some(row.new_number).flatten(),
                new_wrapped.get(line_index).map_or("", String::as_str),
                new_style,
            ));
        }
    }

    (before, after)
}

fn numbered_wrapped_line(number: Option<u32>, text: &str, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format_number(number), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(text.to_owned(), style),
    ])
}

fn aligned_ide_unified_lines(rows: &[IdeDiffRow], content_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for row in rows {
        if row.before.number.is_some() && row.after.number.is_some() {
            lines.extend(ide_unified_code_lines(
                &row.before,
                row.before.number,
                row.after.number,
                " ",
                content_width,
            ));
        } else {
            if row.before.number.is_some() {
                lines.extend(ide_unified_code_lines(
                    &row.before,
                    row.before.number,
                    None,
                    "-",
                    content_width,
                ));
            }
            if row.after.number.is_some() {
                lines.extend(ide_unified_code_lines(
                    &row.after,
                    None,
                    row.after.number,
                    "+",
                    content_width,
                ));
            }
        }
    }
    lines
}

fn ide_unified_code_lines(
    source: &IdeDiffLine,
    old_number: Option<u32>,
    new_number: Option<u32>,
    marker: &str,
    content_width: usize,
) -> Vec<Line<'static>> {
    let wrapped = wrap_styled_line(source, content_width);
    wrapped
        .iter()
        .enumerate()
        .map(|(index, spans)| {
            let old_number = (index == 0).then_some(old_number).flatten();
            let new_number = (index == 0).then_some(new_number).flatten();
            let marker_style = match marker {
                "+" => Style::default().fg(Color::Green),
                "-" => Style::default().fg(Color::Red),
                _ => Style::default(),
            };
            let background = source.background;
            let with_background = |style: Style| match background {
                Some(color) if style.bg.is_none() => style.bg(color),
                None => style,
                Some(_) => style,
            };
            let mut line = vec![
                Span::styled(
                    format_number(old_number),
                    with_background(Style::default().fg(MUTED)),
                ),
                Span::styled(" ", with_background(Style::default())),
                Span::styled(
                    format_number(new_number),
                    with_background(Style::default().fg(MUTED)),
                ),
                Span::styled(" ", with_background(Style::default())),
                Span::styled(
                    if index == 0 { marker } else { " " }.to_owned(),
                    with_background(marker_style),
                ),
                Span::styled(" ", with_background(Style::default())),
            ];
            line.extend(
                spans
                    .iter()
                    .map(|(text, style)| Span::styled(text.clone(), with_background(*style))),
            );
            Line::from(line)
        })
        .collect()
}

fn ide_line(
    source: &IdeDiffLine,
    number: Option<u32>,
    wrapped: Option<&Vec<(String, Style)>>,
) -> Line<'static> {
    let background = source.background;
    let with_background = |style: Style| match background {
        Some(color) if style.bg.is_none() => style.bg(color),
        None => style,
        Some(_) => style,
    };
    let mut spans = vec![
        Span::styled(
            format_number(number),
            with_background(Style::default().fg(MUTED)),
        ),
        Span::styled(" ", with_background(Style::default())),
    ];
    if let Some(wrapped) = wrapped {
        spans.extend(
            wrapped
                .iter()
                .map(|(text, style)| Span::styled(text.clone(), with_background(*style))),
        );
    }
    Line::from(spans)
}

fn wrap_styled_line(line: &IdeDiffLine, width: usize) -> Vec<Vec<(String, Style)>> {
    let ranges = wrap_line_ranges(&line.code, width);
    let mut lines = Vec::with_capacity(ranges.len());
    let mut character_index = 0;
    let mut byte_cursor = 0;
    let mut span_index = 0;
    let mut span_offset = 0;

    for range in ranges {
        let mut wrapped: Vec<(String, Style)> = Vec::new();
        while byte_cursor < range.end {
            let Some((text, base_style)) = line.spans.get(span_index) else {
                break;
            };
            let available = text.len().saturating_sub(span_offset);
            let take = available.min(range.end - byte_cursor);
            if take == 0 {
                span_index += 1;
                span_offset = 0;
                continue;
            }
            let segment = &text[span_offset..span_offset + take];
            for character in segment.chars() {
                let style = match line.inline_change {
                    Some((start, end)) if start <= character_index && character_index < end => {
                        base_style.bg(line.inline_background.unwrap_or(Color::Reset))
                    }
                    _ => *base_style,
                };
                if let Some((last_text, last_style)) = wrapped.last_mut()
                    && *last_style == style
                {
                    last_text.push(character);
                } else {
                    wrapped.push((character.to_string(), style));
                }
                character_index += 1;
            }
            span_offset += take;
            byte_cursor += take;
            if span_offset == text.len() {
                span_index += 1;
                span_offset = 0;
            }
        }
        lines.push(wrapped);
    }
    lines
}

pub fn diff_hunk_line_positions(
    diff: &str,
    terminal_width: u16,
    raw: bool,
    side_by_side: bool,
) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut visual_line = 0;

    if side_by_side {
        let content_width = usize::from(terminal_width / 2).saturating_sub(8).max(1);
        for row in crate::diff::numbered_side_by_side(diff) {
            if row.old_text.starts_with("@@ ") || row.new_text.starts_with("@@ ") {
                positions.push(visual_line);
            } else if row.old_number.is_some() || row.new_number.is_some() {
                let before = if raw {
                    row.old_text
                } else {
                    source_line(row.old_text).0
                };
                let after = if raw {
                    row.new_text
                } else {
                    source_line(row.new_text).0
                };
                visual_line += wrap_line_ranges(before, content_width)
                    .len()
                    .max(wrap_line_ranges(after, content_width).len());
            }
        }
    } else if raw {
        let content_width = usize::from(terminal_width).saturating_sub(14).max(1);
        for line in diff.lines() {
            if line.starts_with("@@ ") {
                positions.push(visual_line);
            }
            visual_line += wrap_line_ranges(line, content_width).len();
        }
    } else {
        let content_width = usize::from(terminal_width).saturating_sub(16).max(1);
        for row in crate::diff::numbered_side_by_side(diff) {
            if row.old_text.starts_with("@@ ") || row.new_text.starts_with("@@ ") {
                positions.push(visual_line);
            } else if row.old_number.is_some() || row.new_number.is_some() {
                let before = source_line(row.old_text).0;
                let after = source_line(row.new_text).0;
                if row.old_number.is_some() && row.new_number.is_some() {
                    visual_line += wrap_line_ranges(before, content_width).len();
                } else {
                    if row.old_number.is_some() {
                        visual_line += wrap_line_ranges(before, content_width).len();
                    }
                    if row.new_number.is_some() {
                        visual_line += wrap_line_ranges(after, content_width).len();
                    }
                }
            }
        }
    }
    positions
}

fn wrap_diff_line(text: &str, width: usize) -> Vec<String> {
    wrap_line_ranges(text, width)
        .into_iter()
        .map(|range| text[range].to_owned())
        .collect()
}

fn wrap_line_ranges(text: &str, width: usize) -> Vec<Range<usize>> {
    if text.is_empty() {
        return vec![0..0];
    }

    let width = width.max(1);
    let mut ranges = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let mut used_width = 0;
        let mut last_fit = start;
        let mut last_whitespace = None;
        let mut overflow = false;

        for (relative_index, character) in text[start..].char_indices() {
            let index = start + relative_index;
            let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
            if used_width + character_width > width {
                overflow = true;
                break;
            }
            used_width += character_width;
            last_fit = index + character.len_utf8();
            if character.is_whitespace() && index > start {
                last_whitespace = Some(last_fit);
            }
        }

        if !overflow {
            ranges.push(start..text.len());
            break;
        }

        let mut split_at = last_whitespace.unwrap_or(last_fit);
        if split_at <= start {
            split_at = start + text[start..].chars().next().unwrap().len_utf8();
        }
        ranges.push(start..split_at);
        start = split_at;
    }

    ranges
}

fn styled_numbered_unified(old: Option<u32>, new: Option<u32>, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format_number(old), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(format_number(new), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(text.to_owned(), diff_line_style(text)),
    ])
}

fn format_number(number: Option<u32>) -> String {
    number.map_or_else(|| "     ".to_owned(), |number| format!("{number:>5}"))
}
