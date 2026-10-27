use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm_keybind::{KeyBind, KeyBindTrait};
use gitview::app::{AppAction, AppState, View};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    env, io,
    process::{Command, ExitCode},
};

#[derive(KeyBind)]
enum Navigation {
    #[keybindings["j"]]
    Down,
    #[keybindings["k"]]
    Up,
    #[keybindings["h"]]
    Back,
    #[keybindings["l"]]
    Open,
}

#[derive(Clone)]
enum BranchAction {
    Switch(std::ffi::OsString),
    Create { name: String, base: String },
}

const ASCII_BORDER: ratatui::symbols::border::Set<'static> = ratatui::symbols::border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

const HELP: &str = "Up/Down or j/k  Navigate and scroll\nEnter / Esc or h/l  Open / back (l / h)\nTab      Switch history / working tree\nb        Browse local branches\nc        Create a branch (in branch view)\ny / n    Confirm / cancel branch operation\n/        Search message, author or hash\nn        Next search match\ns        Stage selected file\nu        Unstage selected file\nr        Refresh working tree\n?        Toggle help\nq        Quit";

fn run() -> Result<(), String> {
    if env::args()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: gitview [--ascii] [--version]\n\nOpen the current Git repository. Use --ascii for plain terminal borders.\n\n{HELP}"
        );
        return Ok(());
    }
    if env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("gitview {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    Navigation::init_and_load_file(None)
        .map_err(|e| format!("Could not initialize keybindings: {e}"))?;
    let ascii = env::args().any(|arg| arg == "--ascii");
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Could not run Git; install Git and ensure it is in PATH: {e}"))?;
    if !output.status.success() {
        return Err("Not in a Git repository. Run gitview from a repository.".into());
    }
    let root = gitview::repository::parse_root(&output.stdout);
    let mut terminal =
        ratatui::try_init().map_err(|e| format!("Could not initialize terminal: {e}"))?;
    let result = app(&mut terminal, &root, ascii);
    ratatui::restore();
    result.map_err(|e| format!("Terminal error: {e}"))
}

fn app(terminal: &mut DefaultTerminal, root: &std::path::Path, ascii: bool) -> io::Result<()> {
    let border = if ascii {
        ASCII_BORDER
    } else {
        ratatui::symbols::border::PLAIN
    };
    let mut help = false;
    let mut searching = false;
    let mut query = String::new();
    let mut search_message = String::new();
    let (history_sender, receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<String>, String>)>();
    let mut history_request = 0u64;
    let sender = history_sender.clone();
    let path = root.to_path_buf();
    std::thread::spawn(move || {
        let _ = sender.send((0, gitview::history::load_history(&path)));
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
    let mut branch_view = false;
    let (branch_sender, branch_receiver) =
        std::sync::mpsc::channel::<(u64, Result<gitview::branches::Branches, String>)>();
    let mut branch_request = 0u64;
    let (operation_sender, operation_receiver) =
        std::sync::mpsc::channel::<(bool, Result<(), String>)>();
    let mut branch_confirm: Option<BranchAction> = None;
    let mut branch_input: Option<String> = None;
    let mut branch_feedback = String::new();
    let mut mutation_pending = false;
    let mut status_mutation: Option<u64> = None;
    let mut branches: Option<Result<gitview::branches::Branches, String>> = None;
    let mut branch_selection = ListState::default();
    let mut app_state = AppState::default();
    let (status_sender, status_receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<gitview::worktree::Entry>, String>)>();
    let mut status: Option<Result<Vec<gitview::worktree::Entry>, String>> = None;
    let mut status_request = 0u64;
    let mut status_selection = ListState::default();
    loop {
        while let Ok((id, result)) = receiver.try_recv() {
            if id != history_request {
                continue;
            }
            if result.as_ref().is_ok_and(|rows| !rows.is_empty()) {
                selection.select(Some(0));
            }
            history = Some(result);
        }
        while let Ok((id, result)) = details_receiver.try_recv() {
            if let Some((active, value)) = &mut details
                && *active == id
            {
                if result.as_ref().is_ok_and(|commit| !commit.files.is_empty()) {
                    file_selection.select(Some(0));
                }
                *value = Some(result);
            }
        }
        while let Ok((id, result)) = diff_receiver.try_recv() {
            if let Some((active, value)) = &mut diff
                && *active == id
            {
                *value = Some(result);
            }
        }
        while let Ok((id, result)) = status_receiver.try_recv() {
            if status_mutation == Some(id) {
                mutation_pending = false;
                status_mutation = None;
            }
            if id != status_request {
                continue;
            }
            if result.as_ref().is_ok_and(|entries| !entries.is_empty()) {
                status_selection.select(Some(0));
            } else {
                status_selection.select(None);
            }
            status = Some(result);
        }
        while let Ok((id, result)) = branch_receiver.try_recv() {
            if id != branch_request {
                continue;
            }
            if result.as_ref().is_ok_and(|data| !data.branches.is_empty()) {
                branch_selection.select(Some(0));
            } else {
                branch_selection.select(None);
            }
            branches = Some(result);
        }
        while let Ok((created, result)) = operation_receiver.try_recv() {
            mutation_pending = false;
            branch_feedback = match result {
                Ok(()) if created => "Branch created and switched successfully.".to_owned(),
                Ok(()) => "Branch switched successfully.".to_owned(),
                Err(error) => {
                    format!("{error} (branch state refreshed; a hook may have changed HEAD)")
                }
            };
            branch_confirm = None;
            request_id += 1;
            details = None;
            diff = None;
            history_request += 1;
            history = None;
            let id = history_request;
            let sender = history_sender.clone();
            let path = root.to_path_buf();
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::history::load_history(&path)));
            });
            status_request += 1;
            status = None;
            let id = status_request;
            let sender = status_sender.clone();
            let path = root.to_path_buf();
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::worktree::load_status(&path)));
            });
            branch_request += 1;
            branches = None;
            let id = branch_request;
            let sender = branch_sender.clone();
            let path = root.to_path_buf();
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::branches::load_branches(&path)));
            });
        }
        terminal.draw(|frame| {
            if help {
                frame.render_widget(
                    Paragraph::new(HELP).block(Block::bordered().border_set(border).title("Help")),
                    frame.area(),
                );
            } else if branch_view && branch_input.is_some() {
                let text = format!("New local branch name: {}_\n\nEnter: continue   Esc: cancel", branch_input.as_deref().unwrap_or(""));
                frame.render_widget(Paragraph::new(text).block(Block::bordered().border_set(border).title("Create branch")), frame.area());
            } else if branch_view && (mutation_pending || branch_confirm.is_some()) {
                let text = if mutation_pending { "Running branch operation...".to_owned() } else {
                    match branch_confirm.as_ref().unwrap() {
                        BranchAction::Switch(name) => format!("Switch to local branch {name:?}?\n\ny: confirm    n / Esc: cancel"),
                        BranchAction::Create { name, base } => format!("Create and switch to local branch {name:?} from commit {base}?\n\ny: confirm    n / Esc: cancel"),
                    }
                };
                frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }).block(Block::bordered().border_set(border).title("Confirm branch operation")), frame.area());
            } else if branch_view {
                use gitview::branches::HeadState;
                let (heading, lines): (String, Vec<String>) = match &branches {
                    None => ("Local branches - loading".into(), vec!["Loading branches...".into()]),
                    Some(Err(error)) => ("Local branches - error".into(), vec![error.clone()]),
                    Some(Ok(data)) => {
                        let heading = match &data.head {
                            HeadState::Attached(name) => format!("Local branches - current: {}", name.to_string_lossy()),
                            HeadState::Detached => "Local branches - detached HEAD".into(),
                            HeadState::Unborn(name) => format!("Local branches - unborn: {}", name.to_string_lossy()),
                        };
                        let lines = if data.branches.is_empty() { vec!["No local branches".into()] } else { data.branches.iter().map(|name| {
                            let current = matches!(&data.head, HeadState::Attached(head) if head == name);
                            format!("{} {}", if current { "*" } else { " " }, name.to_string_lossy())
                        }).collect() };
                        (heading, lines)
                    }
                };
                let available = frame.area().height.saturating_sub(3);
                let width = usize::from(frame.area().width.max(1));
                let feedback_lines = branch_feedback.lines().map(|line| line.chars().count().max(1).div_ceil(width)).sum::<usize>().max(1);
                let feedback_height = u16::try_from(feedback_lines).unwrap_or(u16::MAX).min(available).min(8);
                let areas = Layout::vertical([Constraint::Min(3), Constraint::Length(feedback_height)]).split(frame.area());
                let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>()).block(Block::bordered().border_set(border).title(format!("{heading} | Enter/l switch | c create | Esc/h back"))).highlight_symbol("> ");
                frame.render_stateful_widget(list, areas[0], &mut branch_selection);
                frame.render_widget(Paragraph::new(branch_feedback.as_str()).wrap(Wrap { trim: false }), areas[1]);
            } else if let Some((_, value)) = &diff {
                let message = match value {
                    None => "Loading diff...",
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
                            .block(
                                Block::bordered()
                                    .border_set(border)
                                    .title("Before - up/down or j/k scroll | Esc back"),
                            ),
                        panes[0],
                    );
                    frame.render_widget(
                        Paragraph::new(Text::from(right))
                            .scroll((diff_scroll, 0))
                            .block(Block::bordered().border_set(border).title("After")),
                        panes[1],
                    );
                } else {
                    let lines: Vec<Line> = message.lines().map(styled_diff_line).collect();
                    frame.render_widget(
                        Paragraph::new(Text::from(lines))
                            .scroll((diff_scroll, 0))
                            .block(
                                Block::bordered()
                                    .border_set(border)
                                    .title("Diff - up/down or j/k scroll | Esc back"),
                            ),
                        frame.area(),
                    );
                }
            } else if let Some((_, value)) = &details {
                match value {
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
                            Layout::vertical([Constraint::Length(6), Constraint::Min(3)])
                                .split(frame.area());
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
                        frame.render_stateful_widget(list, sections[1], &mut file_selection);
                    }
                }
            } else if app_state.view() == View::WorkingTree {
                let lines: Vec<String> = match &status {
                    None => vec!["Loading working tree...".to_owned()],
                    Some(Err(error)) => vec![error.clone()],
                    Some(Ok(entries)) if entries.is_empty() => {
                        vec!["Working tree clean".to_owned()]
                    }
                    Some(Ok(entries)) => entries
                        .iter()
                        .map(|entry| {
                            format!("{:?}  {}", entry.section, entry.path.to_string_lossy())
                        })
                        .collect(),
                };
                let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>())
                    .block(
                        Block::bordered()
                            .border_set(border)
                            .title("Working tree - Enter diff | s stage | u unstage | Tab history"),
                    )
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, frame.area(), &mut status_selection);
            } else {
                let lines = match &history {
                    None => vec!["Loading history...".to_string()],
                    Some(Err(error)) => vec![error.clone()],
                    Some(Ok(rows)) if rows.is_empty() => vec!["No commits yet".to_string()],
                    Some(Ok(rows)) => rows
                        .iter()
                        .map(|row| gitview::history::display_row(row).to_owned())
                        .collect(),
                };
                let items = lines.into_iter().map(ListItem::new).collect::<Vec<_>>();
                let title = if searching {
                    format!("Search: {query}_ (Enter find | Esc cancel)")
                } else if !search_message.is_empty() {
                    search_message.clone()
                } else {
                    format!(
                        "Gitview - b branches | / search | Tab changes | ? help - {}",
                        root.display()
                    )
                };
                let list = List::new(items)
                    .block(Block::bordered().border_set(border).title(title))
                    .highlight_symbol("> ");
                frame.render_stateful_widget(list, frame.area(), &mut selection);
            }
        })?;
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if searching {
                match key.code {
                    KeyCode::Esc => searching = false,
                    KeyCode::Backspace => {
                        query.pop();
                    }
                    KeyCode::Enter => {
                        searching = false;
                        if let Some(Ok(rows)) = &history {
                            if let Some(index) = gitview::history::find_next(
                                rows,
                                &query,
                                selection.selected().unwrap_or(0),
                            ) {
                                selection.select(Some(index));
                                search_message = format!("Search: {query} - n next | / new search");
                            } else {
                                search_message = format!("No match for: {query}");
                            }
                        }
                    }
                    KeyCode::Char(c) => query.push(c),
                    _ => {}
                }
                continue;
            }
            if branch_view && !help {
                if mutation_pending {
                    continue;
                }
                if let Some(input) = &mut branch_input {
                    match key.code {
                        KeyCode::Esc => branch_input = None,
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                        KeyCode::Enter => {
                            let name = input.clone();
                            branch_input = None;
                            if let Some(Ok(data)) = &branches {
                                if let Some(base) = &data.head_commit {
                                    if name.is_empty() {
                                        branch_feedback = "Enter a branch name.".into();
                                    } else {
                                        branch_confirm = Some(BranchAction::Create {
                                            name,
                                            base: base.clone(),
                                        });
                                    }
                                } else {
                                    branch_feedback =
                                        "Create the first commit before creating a branch.".into();
                                }
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                if let Some(action) = &branch_confirm {
                    match key.code {
                        KeyCode::Char('y') => {
                            mutation_pending = true;
                            let action = action.clone();
                            let path = root.to_path_buf();
                            let sender = operation_sender.clone();
                            std::thread::spawn(move || {
                                let (created, result) = match action {
                                    BranchAction::Switch(name) => {
                                        (false, gitview::branches::switch_branch(&path, &name))
                                    }
                                    BranchAction::Create { name, base } => (
                                        true,
                                        gitview::branches::create_branch(&path, &name, &base),
                                    ),
                                };
                                let _ = sender.send((created, result));
                            });
                        }
                        KeyCode::Char('n') | KeyCode::Esc => branch_confirm = None,
                        _ => {}
                    }
                    continue;
                }
                match navigation_key(key) {
                    KeyCode::Char('c') => {
                        if let Some(Ok(data)) = &branches {
                            if data.head_commit.is_some() {
                                branch_input = Some(String::new());
                                branch_feedback.clear();
                            } else {
                                branch_feedback =
                                    "Create the first commit before creating a branch.".into();
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let (Some(Ok(data)), Some(index)) =
                            (&branches, branch_selection.selected())
                            && let Some(name) = data.branches.get(index)
                        {
                            if matches!(&data.head, gitview::branches::HeadState::Attached(current) if current == name)
                            {
                                branch_feedback = "Already on this branch.".into();
                            } else {
                                branch_confirm = Some(BranchAction::Switch(name.clone()));
                                branch_feedback.clear();
                            }
                        }
                    }
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => help = true,
                    KeyCode::Esc | KeyCode::Char('b') => branch_view = false,
                    KeyCode::Down => {
                        if let Some(Ok(data)) = &branches
                            && !data.branches.is_empty()
                        {
                            branch_selection.select(Some(
                                (branch_selection.selected().unwrap_or(0) + 1)
                                    .min(data.branches.len() - 1),
                            ));
                        }
                    }
                    KeyCode::Up => {
                        if let Some(index) = branch_selection.selected() {
                            branch_selection.select(Some(index.saturating_sub(1)));
                        }
                    }
                    _ => {}
                }
                continue;
            }
            let blocked = help || mutation_pending || details.is_some() || diff.is_some();
            if let Some(action) = app_state.handle_key(navigation_key(key), blocked) {
                match action {
                    AppAction::LoadWorkingTree => {
                        status = None;
                        status_request += 1;
                        let id = status_request;
                        let sender = status_sender.clone();
                        let path = root.to_path_buf();
                        std::thread::spawn(move || {
                            let _ = sender.send((id, gitview::worktree::load_status(&path)));
                        });
                    }
                    AppAction::ReturnToHistory => {}
                    AppAction::StartSearch => {
                        searching = true;
                        query.clear();
                        search_message.clear();
                    }
                    AppAction::NextMatch => {
                        if let Some(Ok(rows)) = &history
                            && let Some(index) = gitview::history::find_next(
                                rows,
                                &query,
                                selection.selected().unwrap_or(0),
                            )
                        {
                            selection.select(Some(index));
                        }
                    }
                    AppAction::OpenBranches => {
                        branch_view = true;
                        branches = None;
                        branch_feedback.clear();
                        branch_request += 1;
                        let id = branch_request;
                        let sender = branch_sender.clone();
                        let path = root.to_path_buf();
                        std::thread::spawn(move || {
                            let _ = sender.send((id, gitview::branches::load_branches(&path)));
                        });
                    }
                    AppAction::StageSelected | AppAction::UnstageSelected => {
                        if let (Some(Ok(entries)), Some(index)) =
                            (&status, status_selection.selected())
                            && let Some(entry) = entries.get(index)
                        {
                            let entry = entry.clone();
                            status_request += 1;
                            let id = status_request;
                            let sender = status_sender.clone();
                            let path = root.to_path_buf();
                            status = None;
                            mutation_pending = true;
                            status_mutation = Some(id);
                            let stage = action == AppAction::StageSelected;
                            std::thread::spawn(move || {
                                let result = if stage {
                                    gitview::worktree::stage(&path, &entry)
                                } else {
                                    gitview::worktree::unstage(&path, &entry)
                                };
                                let _ = sender.send((
                                    id,
                                    result.and_then(|()| gitview::worktree::load_status(&path)),
                                ));
                            });
                        }
                    }
                    AppAction::RefreshWorkingTree => {
                        status = None;
                        status_request += 1;
                        let id = status_request;
                        let sender = status_sender.clone();
                        let path = root.to_path_buf();
                        std::thread::spawn(move || {
                            let _ = sender.send((id, gitview::worktree::load_status(&path)));
                        });
                    }
                    AppAction::MoveUp => match app_state.view() {
                        View::History => {
                            selection.select(gitview::app::move_up(selection.selected()))
                        }
                        View::WorkingTree => status_selection
                            .select(gitview::app::move_up(status_selection.selected())),
                    },
                    AppAction::MoveDown => match app_state.view() {
                        View::History => {
                            if let Some(Ok(rows)) = &history {
                                selection.select(gitview::app::move_down(
                                    selection.selected(),
                                    rows.len(),
                                ));
                            }
                        }
                        View::WorkingTree => {
                            if let Some(Ok(entries)) = &status {
                                status_selection.select(gitview::app::move_down(
                                    status_selection.selected(),
                                    entries.len(),
                                ));
                            }
                        }
                    },
                }
                continue;
            }
            match navigation_key(key) {
                KeyCode::Char('q') if !mutation_pending => return Ok(()),
                KeyCode::Char('?') => help = !help,
                KeyCode::Tab
                | KeyCode::Char('b')
                | KeyCode::Char('/')
                | KeyCode::Char('n')
                | KeyCode::Char('s')
                | KeyCode::Char('u')
                | KeyCode::Char('r') => {}
                KeyCode::Esc if !help => {
                    if diff.is_some() {
                        diff = None;
                    } else {
                        details = None;
                    }
                }
                KeyCode::Enter
                    if !help && app_state.view() == View::WorkingTree && diff.is_none() =>
                {
                    if let (Some(Ok(entries)), Some(index)) = (&status, status_selection.selected())
                        && let Some(entry) = entries.get(index)
                    {
                        let entry = entry.clone();
                        request_id += 1;
                        let current = request_id;
                        let sender = diff_sender.clone();
                        let path = std::path::PathBuf::from(root);
                        diff = Some((current, None));
                        diff_scroll = 0;
                        std::thread::spawn(move || {
                            let _ =
                                sender.send((current, gitview::worktree::load_diff(&path, &entry)));
                        });
                    }
                }
                KeyCode::Enter
                    if !help && details.is_none() && app_state.view() == View::History =>
                {
                    if let (Some(Ok(rows)), Some(index)) = (&history, selection.selected())
                        && let Some(id) = rows
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
                            let _ =
                                sender.send((current, gitview::details::load_commit(&path, &id)));
                        });
                    }
                }
                KeyCode::Enter if !help && diff.is_none() => {
                    if let (Some((_, Some(Ok(commit)))), Some(index)) =
                        (&details, file_selection.selected())
                        && let Some(file) = commit.files.get(index)
                    {
                        request_id += 1;
                        let current = request_id;
                        let hash = commit.hash.clone();
                        let file = file.clone();
                        let path = std::path::PathBuf::from(root);
                        let sender = diff_sender.clone();
                        diff = Some((current, None));
                        diff_scroll = 0;
                        std::thread::spawn(move || {
                            let _ = sender
                                .send((current, gitview::details::load_diff(&path, &hash, &file)));
                        });
                    }
                }
                KeyCode::Down if !help && diff.is_some() => {
                    diff_scroll = diff_scroll.saturating_add(1);
                }
                KeyCode::Up if !help && diff.is_some() => {
                    diff_scroll = diff_scroll.saturating_sub(1);
                }
                KeyCode::Down if !help && details.is_some() => {
                    if let Some((_, Some(Ok(commit)))) = &details
                        && !commit.files.is_empty()
                    {
                        file_selection.select(Some(
                            (file_selection.selected().unwrap_or(0) + 1)
                                .min(commit.files.len() - 1),
                        ));
                    }
                }
                KeyCode::Up if !help && details.is_some() => {
                    if let Some(i) = file_selection.selected() {
                        file_selection.select(Some(i.saturating_sub(1)));
                    }
                }
                _ => {}
            }
        }
    }
}

fn navigation_key(key: KeyEvent) -> KeyCode {
    match Navigation::dispatch(&key).first() {
        Some(Navigation::Down) => KeyCode::Down,
        Some(Navigation::Up) => KeyCode::Up,
        Some(Navigation::Back) => KeyCode::Esc,
        Some(Navigation::Open) => KeyCode::Enter,
        None => key.code,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn keybinding_library_maps_vim_navigation_without_capturing_modifiers() {
        Navigation::init_and_load_file(None).unwrap();
        for (letter, expected) in [
            ('j', KeyCode::Down),
            ('k', KeyCode::Up),
            ('h', KeyCode::Esc),
            ('l', KeyCode::Enter),
        ] {
            assert_eq!(
                navigation_key(KeyEvent::new(KeyCode::Char(letter), KeyModifiers::NONE)),
                expected
            );
            assert_eq!(
                navigation_key(KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL)),
                KeyCode::Char(letter)
            );
        }
        assert_eq!(
            navigation_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            KeyCode::Down
        );
    }
}
