use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use std::{
    env, io,
    process::{Command, ExitCode},
};

fn run() -> Result<(), String> {
    if env::args()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        println!("Usage: gitview\n\nOpen the current Git repository.\n\nKeys: ? help, q quit");
        return Ok(());
    }
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Could not run Git; install Git and ensure it is in PATH: {e}"))?;
    if !output.status.success() {
        return Err("Not in a Git repository. Run gitview from a repository.".into());
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let mut terminal =
        ratatui::try_init().map_err(|e| format!("Could not initialize terminal: {e}"))?;
    let result = app(&mut terminal, &root);
    ratatui::restore();
    result.map_err(|e| format!("Terminal error: {e}"))
}

fn app(terminal: &mut DefaultTerminal, root: &str) -> io::Result<()> {
    let mut help = false;
    let (sender, receiver) = std::sync::mpsc::channel();
    let path = std::path::PathBuf::from(root);
    std::thread::spawn(move || {
        let _ = sender.send(gitview::history::load_history(&path));
    });
    let mut history: Option<Result<Vec<String>, String>> = None;
    let mut selection = ListState::default();
    let (details_sender, details_receiver) =
        std::sync::mpsc::channel::<(u64, Result<gitview::details::CommitDetails, String>)>();
    let mut details: Option<(u64, Option<Result<gitview::details::CommitDetails, String>>)> = None;
    let mut request_id = 0u64;
    let mut file_selection = ListState::default();
    let (diff_sender, diff_receiver) = std::sync::mpsc::channel::<(u64, Result<String, String>)>();
    let mut diff: Option<(u64, Option<Result<String, String>>)> = None;
    let mut diff_scroll = 0u16;
    loop {
        if let Ok(result) = receiver.try_recv() {
            if result.as_ref().is_ok_and(|rows| !rows.is_empty()) {
                selection.select(Some(0));
            }
            history = Some(result);
        }
        while let Ok((id, result)) = details_receiver.try_recv() {
            if let Some((active, value)) = &mut details {
                if *active == id {
                    if result.as_ref().is_ok_and(|commit| !commit.files.is_empty()) {
                        file_selection.select(Some(0));
                    }
                    *value = Some(result);
                }
            }
        }
        while let Ok((id, result)) = diff_receiver.try_recv() {
            if let Some((active, value)) = &mut diff {
                if *active == id {
                    *value = Some(result);
                }
            }
        }
        terminal.draw(|frame| {
            if help {
                frame.render_widget(
                    Paragraph::new(
                        "Up/Down Navigate   Enter Open commit   Esc Back   ? Close help   q Quit",
                    )
                    .block(Block::bordered().title("Help")),
                    frame.area(),
                );
            } else if let Some((_, value)) = &diff {
                let message = match value {
                    None => "Loading diff…",
                    Some(Err(error)) => error,
                    Some(Ok(text)) if text.is_empty() => "No textual diff",
                    Some(Ok(text)) => text,
                };
                if frame.area().width >= 120 && matches!(value, Some(Ok(_))) {
                    let panes = Layout::horizontal([
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(frame.area());
                    let rows = gitview::diff::side_by_side(message);
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
                            .scroll((diff_scroll, 0))
                            .block(Block::bordered().title("Before — up/down scroll • Esc back")),
                        panes[0],
                    );
                    frame.render_widget(
                        Paragraph::new(Text::from(right))
                            .scroll((diff_scroll, 0))
                            .block(Block::bordered().title("After")),
                        panes[1],
                    );
                } else {
                    let lines: Vec<Line> = message.lines().map(styled_diff_line).collect();
                    frame.render_widget(
                        Paragraph::new(Text::from(lines))
                            .scroll((diff_scroll, 0))
                            .block(Block::bordered().title("Diff — up/down scroll • Esc back")),
                        frame.area(),
                    );
                }
            } else if let Some((_, value)) = &details {
                match value {
                    None => frame.render_widget(
                        Paragraph::new("Loading commit…").block(Block::bordered().title("Commit")),
                        frame.area(),
                    ),
                    Some(Err(error)) => frame.render_widget(
                        Paragraph::new(error.as_str())
                            .block(Block::bordered().title("Commit error")),
                        frame.area(),
                    ),
                    Some(Ok(commit)) => {
                        let sections =
                            Layout::vertical([Constraint::Length(6), Constraint::Min(3)])
                                .split(frame.area());
                        let meta = format!(
                            "{}\nAuthor: {}\nDate: {}\nCommit: {}",
                            commit.subject, commit.author, commit.date, commit.hash
                        );
                        frame.render_widget(
                            Paragraph::new(meta)
                                .block(Block::bordered().title("Commit — Esc back")),
                            sections[0],
                        );
                        let items = commit
                            .files
                            .iter()
                            .map(|file| ListItem::new(file.as_str()))
                            .collect::<Vec<_>>();
                        let list = List::new(items)
                            .block(Block::bordered().title("Files — up/down select • Enter diff"))
                            .highlight_symbol("> ");
                        frame.render_stateful_widget(list, sections[1], &mut file_selection);
                    }
                }
            } else {
                let lines = match &history {
                    None => vec!["Loading history…".to_string()],
                    Some(Err(error)) => vec![error.clone()],
                    Some(Ok(rows)) if rows.is_empty() => vec!["No commits yet".to_string()],
                    Some(Ok(rows)) => rows.clone(),
                };
                let items = lines.into_iter().map(ListItem::new).collect::<Vec<_>>();
                let list = List::new(items)
                    .block(Block::bordered().title(format!(
                        "Gitview — {root} — up/down navigate • ? help • q quit"
                    )))
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, frame.area(), &mut selection);
            }
        })?;
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => help = !help,
                    KeyCode::Esc if !help => {
                        if diff.is_some() {
                            diff = None;
                        } else {
                            details = None;
                        }
                    }
                    KeyCode::Enter if !help && details.is_none() => {
                        if let (Some(Ok(rows)), Some(index)) = (&history, selection.selected()) {
                            if let Some(id) = rows
                                .get(index)
                                .and_then(|row| gitview::history::commit_id(row))
                            {
                                request_id += 1;
                                let id = id.to_owned();
                                let path = std::path::PathBuf::from(root);
                                let sender = details_sender.clone();
                                let current = request_id;
                                details = Some((current, None));
                                std::thread::spawn(move || {
                                    let _ = sender
                                        .send((current, gitview::details::load_commit(&path, &id)));
                                });
                            }
                        }
                    }
                    KeyCode::Enter if !help && diff.is_none() => {
                        if let (Some((_, Some(Ok(commit)))), Some(index)) =
                            (&details, file_selection.selected())
                        {
                            if let Some(file) = commit.files.get(index) {
                                request_id += 1;
                                let current = request_id;
                                let hash = commit.hash.clone();
                                let file = file.clone();
                                let path = std::path::PathBuf::from(root);
                                let sender = diff_sender.clone();
                                diff = Some((current, None));
                                diff_scroll = 0;
                                std::thread::spawn(move || {
                                    let _ = sender.send((
                                        current,
                                        gitview::details::load_diff(&path, &hash, &file),
                                    ));
                                });
                            }
                        }
                    }
                    KeyCode::Down if !help && diff.is_some() => {
                        diff_scroll = diff_scroll.saturating_add(1);
                    }
                    KeyCode::Up if !help && diff.is_some() => {
                        diff_scroll = diff_scroll.saturating_sub(1);
                    }
                    KeyCode::Down if !help && details.is_some() => {
                        if let Some((_, Some(Ok(commit)))) = &details {
                            if !commit.files.is_empty() {
                                file_selection.select(Some(
                                    (file_selection.selected().unwrap_or(0) + 1)
                                        .min(commit.files.len() - 1),
                                ));
                            }
                        }
                    }
                    KeyCode::Up if !help && details.is_some() => {
                        if let Some(i) = file_selection.selected() {
                            file_selection.select(Some(i.saturating_sub(1)));
                        }
                    }
                    KeyCode::Down if !help && details.is_none() => {
                        if let Some(Ok(rows)) = &history {
                            if !rows.is_empty() {
                                selection.select(Some(
                                    (selection.selected().unwrap_or(0) + 1).min(rows.len() - 1),
                                ));
                            }
                        }
                    }
                    KeyCode::Up if !help && details.is_none() => {
                        if let Some(i) = selection.selected() {
                            selection.select(Some(i.saturating_sub(1)));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
