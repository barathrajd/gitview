use crate::ui::Viewport;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Cell, Clear, Paragraph, Row, Table, Wrap},
};

use super::hints::{hint_row_count, render_key_hints};
use super::screen::HelpPage;
use super::theme::{ACCENT, MUTED, page_block};

pub const HELP: &str = concat!(
    "KEY                 DETAILS                           KEY                 DETAILS\n",
    "Up/Down             Move list selection                j/k                 Move list selection\n",
    "Enter               Open selected item                 l                   Open selected item\n",
    "Esc                 Back or cancel                     h                   Back or cancel\n",
    "Tab                 Switch history/worktree pane       q                   Quit outside prompts\n",
    "H                   Open history outside prompts\n",
    "W                   Open working tree outside prompts\n",
    "Ctrl-w +/-          Grow/shrink focused panel (Tab changes focus)\n",
    "Ctrl-w x            Hide focused panel; Ctrl-w = restores both\n",
    "Ctrl-w o            Toggle pane full width (agent too)\n",
    "?                   Help outside prompts               b                   Browse local branches\n",
    "o                   Inspect a local Git object\n",
    "A                   Agent sessions: review, merge, or discard\n",
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

pub(super) const HELP_ROWS: &[(&str, &str, &str, &str)] = &[
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

pub(super) fn render_help(
    frame: &mut Viewport<'_, '_>,
    border: ratatui::symbols::border::Set<'static>,
) {
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

pub fn render_help_overlay_in(
    frame: &mut Viewport<'_, '_>,
    page: HelpPage,
    border: ratatui::symbols::border::Set<'static>,
) {
    const HISTORY: &[(&str, &str)] = &[
        ("Ctrl-w +/-", "Grow/shrink focused panel"),
        ("Ctrl-w o", "Toggle pane full width (agent too)"),
        ("Ctrl-w x/=", "Hide panel / restore both"),
        ("Up/Down, j/k", "Move through commits"),
        ("Enter, l", "Open selected commit"),
        ("/", "Search commit history"),
        ("o", "Inspect a local Git object"),
        ("D", "Review comparisons with foldable files"),
        ("f", "Filter history by literal path; empty clears"),
        ("b", "Browse local branches"),
        ("Tab", "Open working tree"),
        ("W", "Open working tree"),
        ("H", "Return to history"),
        ("q", "Quit Gitview"),
    ];
    const WORKTREE: &[(&str, &str)] = &[
        ("Ctrl-w +/-", "Grow/shrink focused panel"),
        ("Ctrl-w o", "Toggle pane full width (agent too)"),
        ("Ctrl-w x/=", "Hide panel / restore both"),
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
        ("Ctrl+E", "Edit in Git editor; saving commits"),
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

pub(super) fn render_message_page(
    frame: &mut Viewport<'_, '_>,
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

/// Renders the help overlay over the whole frame.
pub fn render_help_overlay(
    frame: &mut Frame<'_>,
    page: HelpPage,
    border: ratatui::symbols::border::Set<'static>,
) {
    let area = frame.area();
    render_help_overlay_in(&mut Viewport::new(frame, area), page, border);
}
