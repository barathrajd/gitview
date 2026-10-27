use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
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
    let (details_sender, details_receiver) = std::sync::mpsc::channel();
    let mut details: Option<(u64, Option<Result<gitview::details::CommitDetails, String>>)> = None;
    let mut request_id = 0u64;
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
            } else if let Some((_, value)) = &details {
                let text = match value {
                    None => "Loading commit…".to_string(),
                    Some(Err(error)) => error.clone(),
                    Some(Ok(commit)) => format!(
                        "{}\nAuthor: {}\nDate: {}\nCommit: {}\n\nChanged files:\n{}",
                        commit.subject,
                        commit.author,
                        commit.date,
                        commit.hash,
                        if commit.files.is_empty() {
                            "(none)".to_string()
                        } else {
                            commit.files.join("\n")
                        }
                    ),
                };
                frame.render_widget(
                    Paragraph::new(text)
                        .block(Block::bordered().title("Commit — Esc back • ? help • q quit")),
                    frame.area(),
                );
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
                        details = None;
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
