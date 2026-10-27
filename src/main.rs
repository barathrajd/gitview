use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm_keybind::{KeyBind, KeyBindTrait};
use gitview::{
    app::{AppAction, AppState, BranchAction, View},
    ui::{self, ASCII_BORDER, HELP, Screen},
};
use ratatui::{DefaultTerminal, widgets::ListState};
use std::{
    env, io,
    process::{Command, ExitCode},
    sync::mpsc,
    time::{Duration, Instant},
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
    let mut search_results = Vec::new();
    let mut search_selection = ListState::default();
    let mut search_debounce: Option<Instant> = None;
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
    {
        let sender = status_sender.clone();
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send((0, gitview::worktree::load_status(&path)));
        });
    }
    {
        let sender = branch_sender.clone();
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send((0, gitview::branches::load_branches(&path)));
        });
    }
    let (event_sender, event_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(event) = event::read() {
            if event_sender.send(event).is_err() {
                break;
            }
        }
    });
    loop {
        while let Ok((id, result)) = receiver.try_recv() {
            if id != history_request {
                continue;
            }
            if result.as_ref().is_ok_and(|rows| !rows.is_empty()) {
                selection.select(Some(0));
            }
            history = Some(result);
            if searching {
                search_debounce = Some(Instant::now());
            }
        }
        if searching && search_debounce.is_some_and(|deadline| Instant::now() >= deadline) {
            update_search_results(
                history.as_ref(),
                &query,
                &mut search_results,
                &mut search_selection,
            );
            search_debounce = None;
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
            let screen = if help {
                Screen::Help
            } else if branch_view && branch_input.is_some() {
                Screen::BranchInput(branch_input.as_deref().unwrap_or(""))
            } else if branch_view && (mutation_pending || branch_confirm.is_some()) {
                Screen::BranchConfirmation {
                    pending: mutation_pending,
                    action: branch_confirm.as_ref(),
                }
            } else if branch_view {
                Screen::Branches {
                    branches: branches.as_ref(),
                    feedback: &branch_feedback,
                    selection: &mut branch_selection,
                }
            } else if let Some(diff) = &diff {
                Screen::Diff {
                    diff,
                    scroll: diff_scroll,
                }
            } else if let Some(details) = &details {
                Screen::CommitDetails {
                    details,
                    selection: &mut file_selection,
                }
            } else if searching {
                Screen::Search {
                    history: history.as_ref(),
                    matches: &search_results,
                    query: &query,
                    pending: search_debounce.is_some(),
                    selection: &mut search_selection,
                }
            } else {
                Screen::Cockpit(ui::Cockpit {
                    repository: root,
                    branches: branches.as_ref(),
                    history: history.as_ref(),
                    status: status.as_ref(),
                    focused: app_state.view(),
                    history_selection: &mut selection,
                    working_tree_selection: &mut status_selection,
                })
            };
            ui::render(frame, screen, border);
        })?;
        let event = match event_receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            if searching {
                match key.code {
                    KeyCode::Esc => {
                        searching = false;
                        search_debounce = None;
                    }
                    KeyCode::Backspace => {
                        if query.pop().is_some() {
                            search_debounce = Some(Instant::now() + Duration::from_millis(180));
                        }
                    }
                    KeyCode::Enter => {
                        if search_debounce.is_some() {
                            update_search_results(
                                history.as_ref(),
                                &query,
                                &mut search_results,
                                &mut search_selection,
                            );
                        }
                        search_debounce = None;
                        if let Some((index, id)) = selected_search_commit(
                            history.as_ref(),
                            &search_results,
                            search_selection.selected(),
                        ) {
                            selection.select(Some(index));
                            request_id += 1;
                            let path = root.to_path_buf();
                            let sender = details_sender.clone();
                            let current = request_id;
                            file_selection.select(None);
                            details = Some((current, None));
                            std::thread::spawn(move || {
                                let _ = sender
                                    .send((current, gitview::details::load_commit(&path, &id)));
                            });
                            searching = false;
                        }
                    }
                    KeyCode::Up => {
                        search_selection.select(gitview::app::move_up(search_selection.selected()));
                    }
                    KeyCode::Down => {
                        search_selection.select(gitview::app::move_down(
                            search_selection.selected(),
                            search_results.len(),
                        ));
                    }
                    KeyCode::Char(c) => {
                        query.push(c);
                        search_debounce = Some(Instant::now() + Duration::from_millis(180));
                    }
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
                        search_results = history
                            .as_ref()
                            .and_then(|result| result.as_ref().ok())
                            .map(|rows| gitview::history::search_matches(rows, ""))
                            .unwrap_or_default();
                        let selected = selection.selected().unwrap_or(0);
                        let result_selection = search_results
                            .iter()
                            .position(|index| *index == selected)
                            .unwrap_or(0);
                        search_selection
                            .select((!search_results.is_empty()).then_some(result_selection));
                        search_debounce = history.is_none().then_some(Instant::now());
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
                KeyCode::Esc if help => help = false,
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

fn update_search_results(
    history: Option<&Result<Vec<String>, String>>,
    query: &str,
    results: &mut Vec<usize>,
    selection: &mut ListState,
) {
    let previously_selected = results.get(selection.selected().unwrap_or(0)).copied();
    *results = history
        .and_then(|result| result.as_ref().ok())
        .map(|rows| gitview::history::search_matches(rows, query))
        .unwrap_or_default();
    let selected = previously_selected
        .and_then(|previous| results.iter().position(|index| *index == previous))
        .or_else(|| (!results.is_empty()).then_some(0));
    selection.select(selected);
}

fn selected_search_commit(
    history: Option<&Result<Vec<String>, String>>,
    results: &[usize],
    selected: Option<usize>,
) -> Option<(usize, String)> {
    let index = *results.get(selected.unwrap_or(0))?;
    let id = history
        .and_then(|result| result.as_ref().ok())?
        .get(index)
        .and_then(|row| gitview::history::commit_id(row))?
        .to_owned();
    Some((index, id))
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

    #[test]
    fn search_selection_resolves_to_the_matching_commit_for_direct_open() {
        let rows = Ok(vec![
            "* abcdef0 First result\u{1f}abcdef0".to_owned(),
            "* 1234567 Second result\u{1f}1234567".to_owned(),
        ]);
        assert_eq!(
            selected_search_commit(Some(&rows), &[1], Some(0)),
            Some((1, "1234567".to_owned()))
        );
        assert_eq!(selected_search_commit(Some(&rows), &[], None), None);
    }
}
