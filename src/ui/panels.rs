use crate::ui::Viewport;
use crate::{
    details::CommitDetails,
    history,
    worktree::{Entry, Section},
};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{List, ListItem, ListState, Paragraph},
};

use super::cockpit::worktree_items;
use super::help::render_message_page;
use super::hints::{hint_row_count, render_key_hints};
use super::screen::HistoryScreen;
use super::theme::page_block;

pub(super) fn render_commit_details(
    frame: &mut Viewport<'_, '_>,
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

pub(super) fn render_working_tree(
    frame: &mut Viewport<'_, '_>,
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

pub(super) fn worktree_selection_style(
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

pub(super) fn render_history(
    frame: &mut Viewport<'_, '_>,
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

pub(super) fn render_search(
    frame: &mut Viewport<'_, '_>,
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
            .map({
                let mut finder = history::MatchFinder::new(query);
                move |row| ListItem::new(crate::ui::highlighted_history_row(row, &mut finder))
            })
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
