use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use crossterm_keybind::{KeyBind, KeyBindTrait};
use gitview::{
    app::{AppAction, AppState, BranchAction, View},
    ui::{self, ASCII_BORDER, HELP, Screen},
};
use ratatui::{DefaultTerminal, widgets::ListState};
use std::{
    env, io,
    process::{Command, ExitCode},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

struct BracketedPasteMode {
    enabled: bool,
}

impl BracketedPasteMode {
    fn enable() -> io::Result<Self> {
        crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
        Ok(Self { enabled: true })
    }

    fn disable(&mut self) -> io::Result<()> {
        if self.enabled {
            crossterm::execute!(io::stdout(), DisableBracketedPaste)?;
            self.enabled = false;
        }
        Ok(())
    }

    fn resume(&mut self) -> io::Result<()> {
        if !self.enabled {
            crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
            self.enabled = true;
        }
        Ok(())
    }
}

impl Drop for BracketedPasteMode {
    fn drop(&mut self) {
        let _ = self.disable();
    }
}

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
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.iter().any(|arg| arg == "--shell-integration") {
        print_shell_integration();
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--profile-config-for") {
        let folder = args
            .get(index + 1)
            .ok_or("--profile-config-for requires a folder path")?;
        let config_dir = gitview::config::config_dir()?;
        if let Some(profile) =
            gitview::config::profile_for_worktree(std::path::Path::new(folder), &config_dir)?
        {
            println!("{}", profile.config_path.display());
        }
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Usage: gitview [--ascii] [--version]\n       gitview init [directory]\n       gitview clone <repository> [directory]\n       gitview --shell-integration\n\nOpen the current Git repository. Use init or clone to set up a repository.\nUse --ascii for plain terminal borders. Folder-specific Git config profiles are loaded automatically. Run --shell-integration for the optional zsh hook.\n\n{HELP}"
        );
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("gitview {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "init") {
        return run_init(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "clone") {
        return run_clone(&args[1..]);
    }
    Navigation::init_and_load_file(None)
        .map_err(|e| format!("Could not initialize keybindings: {e}"))?;
    let ascii = args.iter().any(|arg| arg == "--ascii");
    let cwd =
        env::current_dir().map_err(|error| format!("Could not read current folder: {error}"))?;
    let config_dir = gitview::config::config_dir()?;
    let selected_profile = gitview::config::profile_for_worktree(&cwd, &config_dir)?;
    let (profile_label, config_context) = match selected_profile {
        Some(profile) => (
            profile.name.clone(),
            gitview::config::GitConfigContext::for_profile(profile.name, profile.config_path),
        ),
        None => (
            "Primary".to_owned(),
            gitview::config::GitConfigContext::new(None),
        ),
    };
    let mut profiles = gitview::config::load_profiles(&config_dir)?;
    gitview::process::install_git_config_context(config_context)?;
    let output = gitview::process::git_command()
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Could not run Git; install Git and ensure it is in PATH: {e}"))?;
    if !output.status.success() {
        return Err("Not in a Git repository. Run gitview from a repository.".into());
    }
    let root = gitview::repository::parse_root(&output.stdout);
    let mut terminal =
        ratatui::try_init().map_err(|e| format!("Could not initialize terminal: {e}"))?;
    let mut paste_mode = match BracketedPasteMode::enable() {
        Ok(mode) => mode,
        Err(error) => {
            ratatui::restore();
            return Err(format!("Could not enable bracketed paste: {error}"));
        }
    };
    let result = app(
        &mut terminal,
        &mut paste_mode,
        &root,
        ascii,
        profile_label,
        &config_dir,
        &mut profiles,
    );
    let paste_result = paste_mode.disable();
    ratatui::restore();
    result.map_err(|e| format!("Terminal error: {e}"))?;
    paste_result.map_err(|error| format!("Could not disable bracketed paste: {error}"))?;
    Ok(())
}

fn run_init(args: &[std::ffi::OsString]) -> Result<(), String> {
    if args.len() > 1 {
        return Err("Usage: gitview init [directory]".into());
    }
    let directory = args.first().map(std::path::PathBuf::from).unwrap_or(
        env::current_dir().map_err(|error| format!("Could not read current folder: {error}"))?,
    );
    gitview::repository::initialize(&directory)?;
    println!("Initialized Git repository at {}", directory.display());
    Ok(())
}

fn run_clone(args: &[std::ffi::OsString]) -> Result<(), String> {
    if args.is_empty() || args.len() > 2 {
        return Err("Usage: gitview clone <repository> [directory]".into());
    }
    let source = std::path::Path::new(&args[0]);
    let destination = args.get(1).map(std::path::Path::new);
    println!("Cloning {}...", source.display());
    gitview::repository::clone_repository(source, destination)?;
    println!("Clone complete.");
    Ok(())
}

fn print_shell_integration() {
    print!(
        r#"# Optional Gitview zsh integration. Add this output to ~/.zshrc to enable; remove it to disable.
git() {{
  local profile_config
  profile_config="$(command gitview --profile-config-for "$PWD")" || return $?
  if [[ -n "$profile_config" ]]; then
    GIT_CONFIG_GLOBAL="$profile_config" command git "$@"
  else
    command git "$@"
  fi
}}
"#
    );
}

fn reload_profile_context(
    root: &std::path::Path,
    config_dir: &std::path::Path,
    profiles: &mut Vec<gitview::config::FolderProfile>,
    profile_label: &mut String,
) -> Result<(), String> {
    let updated_profiles = gitview::config::load_profiles(config_dir)?;
    let selected = gitview::config::profile_for_worktree(root, config_dir)?;
    let (label, context) = match selected {
        Some(profile) => (
            profile.name.clone(),
            gitview::config::GitConfigContext::for_profile(profile.name, profile.config_path),
        ),
        None => (
            "Primary".to_owned(),
            gitview::config::GitConfigContext::new(None),
        ),
    };
    gitview::process::update_git_config_context(context)?;
    *profiles = updated_profiles;
    *profile_label = label;
    Ok(())
}

fn edit_profile_registry(
    terminal: &mut DefaultTerminal,
    paste_mode: &mut BracketedPasteMode,
    root: &std::path::Path,
    config_dir: &std::path::Path,
    profiles: &mut Vec<gitview::config::FolderProfile>,
    profile_label: &mut String,
    feedback: &mut String,
) -> io::Result<()> {
    let registry_path = config_dir.join("profiles.json");
    if !registry_path.exists()
        && let Err(error) = gitview::config::save_profiles(config_dir, profiles)
    {
        *feedback = error;
        return Ok(());
    }
    let editor = env::var_os("VISUAL").or_else(|| env::var_os("EDITOR"));
    let Some(editor) = editor else {
        *feedback = "Set VISUAL or EDITOR to edit profiles.json.".into();
        return Ok(());
    };

    paste_mode.disable()?;
    ratatui::restore();
    let editor_result = Command::new(editor).arg(&registry_path).status();
    match ratatui::try_init() {
        Ok(restored) => {
            *terminal = restored;
            paste_mode.resume()?;
        }
        Err(error) => {
            return Err(io::Error::other(format!(
                "Could not restore terminal: {error}"
            )));
        }
    }

    match editor_result {
        Ok(status) if status.success() => {
            match reload_profile_context(root, config_dir, profiles, profile_label) {
                Ok(()) => *feedback = "Git config profiles reloaded.".into(),
                Err(error) => *feedback = error,
            }
        }
        Ok(status) => *feedback = format!("Profile editor exited with {status}."),
        Err(error) => *feedback = format!("Could not open profile editor: {error}"),
    }
    Ok(())
}

enum CommitReadResult {
    Create(Result<gitview::commit::CommitPreview, String>),
    Amend(Result<gitview::commit::AmendPreview, String>),
}

fn app(
    terminal: &mut DefaultTerminal,
    paste_mode: &mut BracketedPasteMode,
    root: &std::path::Path,
    ascii: bool,
    mut profile_label: String,
    config_dir: &std::path::Path,
    profiles: &mut Vec<gitview::config::FolderProfile>,
) -> io::Result<()> {
    let border = if ascii {
        ASCII_BORDER
    } else {
        ratatui::symbols::border::PLAIN
    };
    let mut help = false;
    let mut searching = false;
    let mut history_filter: Option<std::path::PathBuf> = None;
    let mut history_path_input: Option<String> = None;
    let mut query = String::new();
    let mut search_results = Vec::new();
    let mut search_selection = ListState::default();
    let mut search_debounce: Option<Instant> = None;
    let (history_sender, receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<String>, String>)>();
    let mut history_request = 0u64;
    let sender = history_sender.clone();
    let path = root.to_path_buf();
    let requested_history_path = history_filter.clone();
    std::thread::spawn(move || {
        let _ = sender.send((
            0,
            gitview::history::load_history_for_path(&path, requested_history_path.as_deref()),
        ));
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
    let mut diff_raw_view = false;
    let mut diff_side_by_side = true;
    let mut diff_entry: Option<gitview::worktree::Entry> = None;
    let mut selected_hunk = 0usize;
    let mut branch_view = false;
    let mut object_page: Option<gitview::object_page::ObjectPage> = None;
    let mut comparison_page: Option<gitview::comparison_page::ComparisonPage> = None;
    let mut tag_page: Option<gitview::tag_page::TagPage> = None;
    let mut profiles_view = false;
    let mut profile_selection = ListState::default();
    let mut profile_feedback = String::new();
    let profile_registry_path = config_dir.join("profiles.json");
    let (branch_sender, branch_receiver) =
        std::sync::mpsc::channel::<(u64, Result<gitview::branches::Branches, String>)>();
    let mut branch_request = 0u64;
    let (operation_sender, operation_receiver) =
        std::sync::mpsc::channel::<(String, Result<(), String>)>();
    let mut branch_confirm: Option<BranchAction> = None;
    let mut branch_input: Option<String> = None;
    let mut branch_rename: Option<gitview::branches::BranchPreview> = None;
    let mut branch_feedback = String::new();
    let mut ignore_path_input: Option<String> = None;
    let mut remove_path_input: Option<String> = None;
    let mut remove_confirm: Option<gitview::worktree::RemovePathPreview> = None;
    let mut move_path_input = false;
    let mut move_source = String::new();
    let mut move_destination = String::new();
    let mut move_destination_focused = false;
    let mut move_confirm: Option<gitview::worktree::MovePathPreview> = None;
    let mut commit_preview_mode: Option<bool> = None;
    let preview_root = root.to_path_buf();
    let commit_preview_worker = gitview::read_worker::LatestRead::new(move |amend: bool| {
        if amend {
            CommitReadResult::Amend(gitview::commit::preview_amend(&preview_root))
        } else {
            CommitReadResult::Create(gitview::commit::preview_commit(&preview_root))
        }
    });
    let mut commit_message: Option<String> = None;
    let mut commit_description = String::new();
    let mut commit_description_focused = false;
    let mut commit_amend = false;
    let mut amend_confirm = false;
    let mut commit_expected: Option<gitview::commit::CommitPreview> = None;
    let mut amend_expected: Option<gitview::commit::AmendPreview> = None;
    let mut commit_feedback = String::new();
    let (commit_sender, commit_receiver) =
        mpsc::channel::<(String, String, bool, Result<(), String>)>();
    let mut discard_confirm: Option<gitview::worktree::DiscardPreview> = None;
    let (discard_sender, discard_receiver) = mpsc::channel::<Result<(), String>>();
    let mut untrack_confirm: Option<gitview::worktree::UntrackPreview> = None;
    let (untrack_sender, untrack_receiver) = mpsc::channel::<Result<(), String>>();
    let (remove_sender, remove_receiver) = mpsc::channel::<Result<(), String>>();
    let (move_sender, move_receiver) = mpsc::channel::<Result<(), String>>();
    let (ignore_sender, ignore_receiver) =
        mpsc::channel::<Result<Option<gitview::worktree::IgnoreRule>, String>>();
    let mut mutation_pending = false;
    let mut status_mutation: Option<u64> = None;
    let mut diff_entry_refresh: Option<(gitview::worktree::Section, gitview::worktree::Section)> =
        None;
    let mut branches: Option<Result<gitview::branches::Branches, String>> = None;
    let mut branch_selection = ListState::default();
    let mut app_state = AppState::default();
    let (status_sender, status_receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<gitview::worktree::Entry>, String>)>();
    let (status_watch_sender, status_watch_receiver) =
        std::sync::mpsc::sync_channel::<Vec<gitview::worktree::Entry>>(1);
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
        let sender = status_watch_sender;
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let mut previous = None;
            loop {
                std::thread::sleep(Duration::from_millis(750));
                let Ok(entries) = gitview::worktree::load_status(&path) else {
                    continue;
                };
                if previous.as_ref() != Some(&entries) {
                    match sender.try_send(entries.clone()) {
                        Ok(()) => previous = Some(entries),
                        Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
                    }
                }
            }
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
    let event_paused = Arc::new(AtomicBool::new(false));
    let event_gate = Arc::new(Mutex::new(()));
    let reader_paused = event_paused.clone();
    let reader_gate = event_gate.clone();
    std::thread::spawn(move || {
        loop {
            if reader_paused.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            let Ok(_guard) = reader_gate.lock() else {
                break;
            };
            if reader_paused.load(Ordering::Acquire) {
                continue;
            }
            match event::poll(Duration::from_millis(50)) {
                Ok(true) => match event::read() {
                    Ok(event) => {
                        if event_sender.send(event).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });
    let mut needs_redraw = true;
    loop {
        if let Some(page) = &mut object_page {
            needs_redraw |= page.tick();
        }
        if let Some(page) = &mut comparison_page {
            needs_redraw |= page.tick();
        }
        if let Some(page) = &mut tag_page {
            needs_redraw |= page.tick();
        }
        while let Ok(entries) = status_watch_receiver.try_recv() {
            needs_redraw = true;
            if mutation_pending {
                continue;
            }
            let unchanged = status
                .as_ref()
                .and_then(|current| current.as_ref().ok())
                .is_some_and(|current| current == &entries);
            if unchanged {
                continue;
            }
            let selected_entry = status
                .as_ref()
                .and_then(|current| current.as_ref().ok())
                .and_then(|current| status_selection.selected().and_then(|i| current.get(i)))
                .cloned();
            let selected = selected_entry
                .and_then(|selected| entries.iter().position(|entry| entry == &selected))
                .or_else(|| {
                    (!entries.is_empty()).then(|| {
                        status_selection
                            .selected()
                            .unwrap_or(0)
                            .min(entries.len() - 1)
                    })
                });
            status_selection.select(selected);
            status = Some(Ok(entries));
        }
        while let Ok((id, result)) = receiver.try_recv() {
            needs_redraw = true;
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
            needs_redraw = true;
        }
        while let Ok((id, result)) = details_receiver.try_recv() {
            needs_redraw = true;
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
            needs_redraw = true;
            if let Some((active, value)) = &mut diff
                && *active == id
            {
                *value = Some(result);
                selected_hunk = 0;
                diff_scroll = 0;
            }
        }
        while let Ok((id, result)) = status_receiver.try_recv() {
            needs_redraw = true;
            let is_mutation = status_mutation == Some(id);
            if is_mutation {
                mutation_pending = false;
                status_mutation = None;
                if let (Some(entry), Some((preferred, fallback)), Ok(entries)) =
                    (&diff_entry, diff_entry_refresh, &result)
                {
                    diff_entry = entries
                        .iter()
                        .find(|current| {
                            current.section == preferred
                                && current.path == entry.path
                                && current.original == entry.original
                        })
                        .or_else(|| {
                            entries.iter().find(|current| {
                                current.section == fallback
                                    && current.path == entry.path
                                    && current.original == entry.original
                            })
                        })
                        .cloned();
                }
                diff_entry_refresh = None;
            }
            if id != status_request {
                continue;
            }
            // Keep the last known-good list visible if a Git mutation fails.
            if is_mutation && result.is_err() {
                continue;
            }
            if let Ok(entries) = &result {
                let previous_entry = status
                    .as_ref()
                    .and_then(|current| current.as_ref().ok())
                    .and_then(|current| {
                        status_selection
                            .selected()
                            .and_then(|index| current.get(index))
                    });
                let prefer_staged = previous_entry
                    .is_some_and(|previous| previous.section == gitview::worktree::Section::Staged);
                let preferred_index = entries
                    .iter()
                    .position(|entry| {
                        let in_preferred_section = if prefer_staged {
                            entry.section == gitview::worktree::Section::Staged
                        } else {
                            entry.section != gitview::worktree::Section::Staged
                        };
                        in_preferred_section
                            && previous_entry.is_some_and(|previous| {
                                entry.path == previous.path && entry.original == previous.original
                            })
                    })
                    .or_else(|| {
                        entries.iter().position(|entry| {
                            if prefer_staged {
                                entry.section == gitview::worktree::Section::Staged
                            } else {
                                entry.section != gitview::worktree::Section::Staged
                            }
                        })
                    });
                let fallback_index = entries.iter().position(|entry| {
                    if prefer_staged {
                        entry.section != gitview::worktree::Section::Staged
                    } else {
                        entry.section == gitview::worktree::Section::Staged
                    }
                });
                let same_entry = previous_entry.and_then(|previous| {
                    entries.iter().position(|entry| {
                        entry.section == previous.section
                            && entry.path == previous.path
                            && entry.original == previous.original
                    })
                });
                status_selection.select(if entries.is_empty() {
                    None
                } else if is_mutation {
                    preferred_index
                        .or(fallback_index)
                        .or(same_entry)
                        .or(Some(0))
                } else {
                    same_entry.or(Some(0))
                });
            } else {
                status_selection.select(None);
            }
            status = Some(result);
        }
        while let Ok((id, result)) = branch_receiver.try_recv() {
            needs_redraw = true;
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
        while let Ok((success_message, result)) = operation_receiver.try_recv() {
            needs_redraw = true;
            mutation_pending = false;
            branch_feedback = match result {
                Ok(()) => success_message,
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
            let requested_history_path = history_filter.clone();
            std::thread::spawn(move || {
                let _ = sender.send((
                    id,
                    gitview::history::load_history_for_path(
                        &path,
                        requested_history_path.as_deref(),
                    ),
                ));
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
        while let Ok((message, description, was_amend, result)) = commit_receiver.try_recv() {
            needs_redraw = true;
            mutation_pending = false;
            amend_confirm = false;
            match result {
                Ok(()) => {
                    commit_message = None;
                    commit_description.clear();
                    commit_description_focused = false;
                    commit_expected = None;
                    amend_expected = None;
                    commit_amend = false;
                    commit_feedback = if was_amend {
                        "Commit amended; HEAD now has a new ID.".into()
                    } else {
                        "Commit created.".into()
                    };
                    history_request += 1;
                    history = None;
                    let id = history_request;
                    let sender = history_sender.clone();
                    let path = root.to_path_buf();
                    let requested_history_path = history_filter.clone();
                    std::thread::spawn(move || {
                        let _ = sender.send((
                            id,
                            gitview::history::load_history_for_path(
                                &path,
                                requested_history_path.as_deref(),
                            ),
                        ));
                    });
                    status_request += 1;
                    status = None;
                    status_mutation = Some(status_request);
                    let id = status_request;
                    let sender = status_sender.clone();
                    let path = root.to_path_buf();
                    std::thread::spawn(move || {
                        let result = gitview::worktree::load_status(&path);
                        let _ = sender.send((id, result));
                    });
                }
                Err(error) => {
                    commit_feedback = error;
                    commit_message = Some(message);
                    commit_description = description;
                    commit_amend = was_amend;
                    if was_amend {
                        amend_expected = gitview::commit::preview_amend(root).ok();
                    } else {
                        commit_expected = gitview::commit::preview_commit(root).ok();
                    }
                }
            }
        }
        while let Ok(result) = discard_receiver.try_recv() {
            needs_redraw = true;
            mutation_pending = false;
            discard_confirm = None;
            commit_feedback = match result {
                Ok(()) => "Selected file discarded.".into(),
                Err(error) => error,
            };
            status_request += 1;
            let id = status_request;
            status_mutation = Some(id);
            let sender = status_sender.clone();
            let path = root.to_path_buf();
            status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = untrack_receiver.try_recv() {
            needs_redraw = true;
            mutation_pending = false;
            untrack_confirm = None;
            commit_feedback = match result {
                Ok(()) => "Stopped tracking the selected file; kept its working copy.".into(),
                Err(error) => error,
            };
            status_request += 1;
            let id = status_request;
            status_mutation = Some(id);
            let sender = status_sender.clone();
            let path = root.to_path_buf();
            status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = remove_receiver.try_recv() {
            needs_redraw = true;
            mutation_pending = false;
            remove_confirm = None;
            commit_feedback = match result {
                Ok(()) => "Removed the tracked path and staged its deletion.".into(),
                Err(error) => error,
            };
            status_request += 1;
            let id = status_request;
            status_mutation = Some(id);
            let sender = status_sender.clone();
            let path = root.to_path_buf();
            status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = move_receiver.try_recv() {
            needs_redraw = true;
            mutation_pending = false;
            move_confirm = None;
            move_path_input = false;
            commit_feedback = match result {
                Ok(()) => "Moved the tracked path and staged its rename.".into(),
                Err(error) => error,
            };
            status_request += 1;
            let id = status_request;
            status_mutation = Some(id);
            let sender = status_sender.clone();
            let path = root.to_path_buf();
            status = None;
            std::thread::spawn(move || {
                let _ = sender.send((id, gitview::worktree::load_status(&path)));
            });
        }
        while let Ok(result) = ignore_receiver.try_recv() {
            needs_redraw = true;
            commit_feedback = match result {
                Ok(Some(rule)) => {
                    format!(
                        "Ignored by {}:{} ({:?}).",
                        rule.source, rule.line, rule.pattern
                    )
                }
                Ok(None) => "No ignore rule matches the selected path.".into(),
                Err(error) => error,
            };
        }
        if let Some(result) = commit_preview_worker.take_result()
            && commit_preview_mode.take().is_some()
        {
            needs_redraw = true;
            match result {
                CommitReadResult::Create(result) => match result {
                    Ok(preview) => {
                        commit_expected = Some(preview);
                        amend_expected = None;
                        amend_confirm = false;
                        commit_amend = false;
                        commit_feedback.clear();
                        commit_description.clear();
                        commit_description_focused = false;
                        commit_message = Some(String::new());
                    }
                    Err(error) => {
                        commit_feedback = error;
                    }
                },
                CommitReadResult::Amend(result) => match result {
                    Ok(preview) => {
                        let (subject, description) = split_commit_message(&preview.message);
                        amend_expected = Some(preview);
                        commit_expected = None;
                        amend_confirm = false;
                        commit_amend = true;
                        commit_feedback.clear();
                        commit_description = description;
                        commit_description_focused = false;
                        commit_message = Some(subject);
                    }
                    Err(error) => commit_feedback = error,
                },
            }
        }
        if needs_redraw {
            terminal.draw(|frame| {
                let workspace_feedback = if app_state.view() == View::History {
                    history_filter
                        .as_ref()
                        .map(|path| {
                            format!(
                                "History path: {} · f change/clear. {}",
                                path.display(),
                                commit_feedback
                            )
                        })
                        .unwrap_or_else(|| commit_feedback.clone())
                } else {
                    commit_feedback.clone()
                };
                let composed_message = combined_commit_message(
                    commit_message.as_deref().unwrap_or_default(),
                    &commit_description,
                );
                let help_page = if profiles_view {
                    ui::HelpPage::Profiles
                } else if amend_confirm {
                    ui::HelpPage::AmendConfirmation
                } else if commit_message.is_some() {
                    ui::HelpPage::CommitMessage
                } else if move_path_input || move_confirm.is_some() {
                    ui::HelpPage::WorkingTree
                } else if remove_path_input.is_some() || remove_confirm.is_some() {
                    ui::HelpPage::WorkingTree
                } else if ignore_path_input.is_some() {
                    ui::HelpPage::IgnorePathInput
                } else if branch_view && branch_input.is_some() {
                    ui::HelpPage::BranchInput
                } else if branch_view && branch_confirm.is_some() {
                    ui::HelpPage::BranchConfirmation
                } else if branch_view {
                    ui::HelpPage::Branches
                } else if diff_entry.is_some() && diff.is_some() {
                    ui::HelpPage::WorktreeDiff
                } else if diff.is_some() {
                    ui::HelpPage::CommitDiff
                } else if details.is_some() {
                    ui::HelpPage::Commit
                } else if searching {
                    ui::HelpPage::Search
                } else if app_state.view() == View::WorkingTree {
                    ui::HelpPage::WorkingTree
                } else {
                    ui::HelpPage::History
                };
                let screen = if let Some(amend) = commit_preview_mode {
                    Screen::CommitPreviewLoading { amend }
                } else if let Some(page) = &object_page {
                    Screen::Object(page)
                } else if let Some(page) = &comparison_page {
                    Screen::Comparison(page)
                } else if let Some(path) = &history_path_input {
                    Screen::PathHistoryInput { path }
                } else if let Some(page) = &tag_page {
                    Screen::Tags(page)
                } else if profiles_view {
                    Screen::Profiles {
                        profiles,
                        selected: profile_selection.selected(),
                        active_profile: &profile_label,
                        registry_path: &profile_registry_path,
                        feedback: &profile_feedback,
                    }
                } else if move_path_input {
                    Screen::MovePathInput {
                        source: &move_source,
                        destination: &move_destination,
                        destination_focused: move_destination_focused,
                        feedback: &commit_feedback,
                    }
                } else if let Some(preview) = &move_confirm {
                    Screen::MovePathConfirmation {
                        source: preview.source(),
                        destination: preview.destination(),
                        pending: mutation_pending,
                    }
                } else if let Some(path) = &remove_path_input {
                    Screen::RemovePathInput {
                        path,
                        feedback: &commit_feedback,
                    }
                } else if let Some(preview) = &remove_confirm {
                    Screen::RemovePathConfirmation {
                        path: preview.path(),
                        pending: mutation_pending,
                    }
                } else if let Some(path) = &ignore_path_input {
                    Screen::IgnorePathInput {
                        path,
                        feedback: &commit_feedback,
                    }
                } else if amend_confirm {
                    let preview = amend_expected.as_ref();
                    Screen::AmendConfirmation {
                        repository: root,
                        profile: &profile_label,
                        branches: branches.as_ref(),
                        entries: status.as_ref(),
                        head: preview.map_or("unavailable", |preview| preview.head.as_str()),
                        original_message: preview
                            .map_or("Unavailable", |preview| preview.message.as_str()),
                        new_message: &composed_message,
                        pending: mutation_pending,
                    }
                } else if let Some(preview) = &discard_confirm {
                    Screen::DiscardConfirmation {
                        entry: &preview.entry,
                        pending: mutation_pending,
                    }
                } else if let Some(preview) = &untrack_confirm {
                    Screen::UntrackConfirmation {
                        entry: &preview.entry,
                        pending: mutation_pending,
                    }
                } else if let Some(message) = &commit_message {
                    Screen::CommitInput {
                        repository: root,
                        profile: &profile_label,
                        branches: branches.as_ref(),
                        message,
                        description: &commit_description,
                        description_focused: commit_description_focused,
                        amend: commit_amend,
                        feedback: &commit_feedback,
                        entries: status.as_ref(),
                        pending: mutation_pending,
                    }
                } else if branch_view && branch_input.is_some() {
                    if let Some(preview) = &branch_rename {
                        Screen::RenameBranchInput {
                            input: branch_input.as_deref().unwrap_or(""),
                            name: preview.name(),
                        }
                    } else {
                        Screen::BranchInput(branch_input.as_deref().unwrap_or(""))
                    }
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
                        selected_hunk,
                        raw: diff_raw_view,
                        side_by_side: diff_side_by_side,
                        worktree_entry: diff_entry.as_ref(),
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
                        profile: &profile_label,
                        branches: branches.as_ref(),
                        history: history.as_ref(),
                        status: status.as_ref(),
                        feedback: &workspace_feedback,
                        focused: app_state.view(),
                        history_selection: &mut selection,
                        working_tree_selection: &mut status_selection,
                    })
                };
                ui::render(frame, screen, border);
                if help {
                    ui::render_help_overlay(frame, help_page, border);
                }
            })?;
            needs_redraw = false;
        }
        let event = match event_receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };
        needs_redraw = true;
        if let Event::Paste(pasted) = &event {
            if let Some(page) = &mut object_page {
                page.paste(pasted);
                continue;
            }
            if let Some(page) = &mut comparison_page {
                page.paste(pasted);
                continue;
            }
            if let Some(path) = &mut history_path_input {
                if !help {
                    path.push_str(pasted);
                }
                continue;
            }
            if let Some(page) = &mut tag_page {
                page.paste(pasted);
                continue;
            }
            if move_path_input && !help {
                if move_destination_focused {
                    move_destination.push_str(pasted);
                } else {
                    move_source.push_str(pasted);
                }
            }
            if let Some(path) = &mut remove_path_input
                && !help
            {
                path.push_str(pasted);
            }
            if let Some(path) = &mut ignore_path_input
                && !help
            {
                path.push_str(pasted);
            }
            if commit_message.is_some() && !amend_confirm && !mutation_pending && !help {
                let pasted = normalize_commit_paste(pasted, commit_description_focused);
                if commit_description_focused {
                    commit_description.push_str(&pasted);
                } else if let Some(message) = &mut commit_message {
                    message.push_str(&pasted);
                }
                commit_feedback.clear();
            }
            continue;
        }
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            if help {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                    help = false;
                }
                continue;
            }
            if commit_preview_mode.is_some() {
                if key.code == KeyCode::Esc {
                    commit_preview_worker.cancel();
                    commit_preview_mode = None;
                    commit_feedback.clear();
                }
                continue;
            }
            if let Some(page) = &mut object_page {
                if !page.key(root, key.code) {
                    object_page = None;
                }
                continue;
            }
            if let Some(page) = &mut comparison_page {
                if !page.key(root, key.code) {
                    comparison_page = None;
                }
                continue;
            }
            if let Some(input) = &mut history_path_input {
                match key.code {
                    KeyCode::Esc => history_path_input = None,
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                    KeyCode::Enter => {
                        history_filter =
                            (!input.is_empty()).then(|| std::path::PathBuf::from(&*input));
                        history_path_input = None;
                        history_request += 1;
                        let id = history_request;
                        let sender = history_sender.clone();
                        let path = root.to_path_buf();
                        let filter = history_filter.clone();
                        history = None;
                        selection.select(None);
                        std::thread::spawn(move || {
                            let _ = sender.send((
                                id,
                                gitview::history::load_history_for_path(&path, filter.as_deref()),
                            ));
                        });
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(page) = &mut tag_page {
                if !page.key(root, key.code) {
                    tag_page = None;
                    history_request += 1;
                    let id = history_request;
                    let sender = history_sender.clone();
                    let path = root.to_path_buf();
                    history = None;
                    let requested_history_path = history_filter.clone();
                    std::thread::spawn(move || {
                        let _ = sender.send((
                            id,
                            gitview::history::load_history_for_path(
                                &path,
                                requested_history_path.as_deref(),
                            ),
                        ));
                    });
                }
                continue;
            }
            if profiles_view {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('h') => profiles_view = false,
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('e') => edit_profile_registry(
                        terminal,
                        paste_mode,
                        root,
                        config_dir,
                        profiles,
                        &mut profile_label,
                        &mut profile_feedback,
                    )?,
                    KeyCode::Char('r') => {
                        match reload_profile_context(root, config_dir, profiles, &mut profile_label)
                        {
                            Ok(()) => profile_feedback = "Git config profiles reloaded.".into(),
                            Err(error) => profile_feedback = error,
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        profile_selection
                            .select(gitview::app::move_up(profile_selection.selected()));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        profile_selection.select(gitview::app::move_down(
                            profile_selection.selected(),
                            profiles.len(),
                        ));
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(path_text) = ignore_path_input.as_mut() {
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Esc => {
                        ignore_path_input = None;
                        commit_feedback.clear();
                    }
                    KeyCode::Backspace => {
                        path_text.pop();
                    }
                    KeyCode::Enter => {
                        let user_path = path_text.clone();
                        ignore_path_input = None;
                        let sender = ignore_sender.clone();
                        let root = root.to_path_buf();
                        std::thread::spawn(move || {
                            let result = gitview::worktree::ignore_rule_for_path(
                                &root,
                                std::path::Path::new(&user_path),
                            );
                            let _ = sender.send(result);
                        });
                    }
                    KeyCode::Char(ch) if !ch.is_control() => path_text.push(ch),
                    _ => {}
                }
                continue;
            }
            if move_path_input {
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Esc => {
                        move_path_input = false;
                        move_source.clear();
                        move_destination.clear();
                        commit_feedback.clear();
                    }
                    KeyCode::Tab => move_destination_focused = !move_destination_focused,
                    KeyCode::Backspace => {
                        if move_destination_focused {
                            move_destination.pop();
                        } else {
                            move_source.pop();
                        }
                    }
                    KeyCode::Enter if !move_destination_focused => {
                        if move_source.is_empty() {
                            commit_feedback = "Enter the tracked source path first.".into();
                        } else {
                            move_destination_focused = true;
                            commit_feedback.clear();
                        }
                    }
                    KeyCode::Enter => {
                        match gitview::worktree::preview_move_path(
                            root,
                            std::path::Path::new(&move_source),
                            std::path::Path::new(&move_destination),
                        ) {
                            Ok(preview) => {
                                move_path_input = false;
                                move_confirm = Some(preview);
                                commit_feedback.clear();
                            }
                            Err(error) => commit_feedback = error,
                        }
                    }
                    KeyCode::Char(ch) if !ch.is_control() => {
                        if move_destination_focused {
                            move_destination.push(ch);
                        } else {
                            move_source.push(ch);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(path_text) = remove_path_input.as_mut() {
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Esc => {
                        remove_path_input = None;
                        commit_feedback.clear();
                    }
                    KeyCode::Backspace => {
                        path_text.pop();
                    }
                    KeyCode::Enter => {
                        match gitview::worktree::preview_remove_path(
                            root,
                            std::path::Path::new(path_text),
                        ) {
                            Ok(preview) => {
                                remove_path_input = None;
                                remove_confirm = Some(preview);
                                commit_feedback.clear();
                            }
                            Err(error) => commit_feedback = error,
                        }
                    }
                    KeyCode::Char(ch) if !ch.is_control() => path_text.push(ch),
                    _ => {}
                }
                continue;
            }
            if amend_confirm {
                if mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('y') => {
                        let subject = commit_message.clone().unwrap_or_default();
                        let description = commit_description.clone();
                        let message = combined_commit_message(&subject, &description);
                        let expected = amend_expected.clone();
                        let path = root.to_path_buf();
                        let sender = commit_sender.clone();
                        mutation_pending = true;
                        std::thread::spawn(move || {
                            let result = expected
                                .as_ref()
                                .ok_or_else(|| "Amend preview is unavailable".to_owned())
                                .and_then(|preview| {
                                    gitview::commit::amend_commit(&path, &message, preview)
                                });
                            let _ = sender.send((subject, description, true, result));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => amend_confirm = false,
                    _ => {}
                }
                continue;
            }
            if commit_message.is_some() {
                if mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => {
                        commit_message = None;
                        commit_description.clear();
                        commit_description_focused = false;
                        commit_expected = None;
                        amend_expected = None;
                        commit_amend = false;
                        commit_feedback.clear();
                    }
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let message = combined_commit_message(
                            commit_message.as_deref().unwrap_or_default(),
                            &commit_description,
                        );
                        match gitview::editor::configured_editor(root) {
                            Err(error) => commit_feedback = error,
                            Ok(editor) => {
                                paste_mode.disable()?;
                                event_paused.store(true, Ordering::Release);
                                let guard = event_gate
                                    .lock()
                                    .map_err(|_| io::Error::other("Input reader unavailable"))?;
                                ratatui::restore();
                                let result =
                                    gitview::editor::edit_with_command(root, &editor, &message);
                                let restored = ratatui::try_init();
                                while event_receiver.try_recv().is_ok() {}
                                drop(guard);
                                event_paused.store(false, Ordering::Release);
                                *terminal = restored?;
                                paste_mode.resume()?;
                                terminal.clear()?;
                                match result {
                                    Ok(edited) => {
                                        let (subject, description) = split_commit_message(&edited);
                                        commit_message = Some(subject);
                                        commit_description = description;
                                        commit_feedback = "Returned from editor; review the message before committing.".into();
                                    }
                                    Err(error) => commit_feedback = error,
                                }
                            }
                        }
                    }
                    KeyCode::Tab => commit_description_focused = !commit_description_focused,
                    KeyCode::Backspace => {
                        if commit_description_focused {
                            commit_description.pop();
                        } else if let Some(message) = &mut commit_message {
                            message.pop();
                        }
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if commit_description_focused {
                            commit_description.push('\n');
                        }
                    }
                    KeyCode::Enter | KeyCode::Char('\r' | '\n') => {
                        let message = commit_message.as_deref().unwrap_or_default();
                        if message.trim().is_empty() {
                            commit_feedback = "Enter a commit message.".into();
                        } else if (commit_amend && amend_expected.is_none())
                            || (!commit_amend && commit_expected.is_none())
                        {
                            commit_feedback = if commit_amend {
                                "Amend preview is unavailable; refresh and try again.".into()
                            } else {
                                "Stage changes before committing.".into()
                            };
                        } else {
                            let subject = message.to_owned();
                            let description = commit_description.clone();
                            if commit_amend {
                                amend_confirm = true;
                                commit_feedback.clear();
                            } else {
                                let message = combined_commit_message(&subject, &description);
                                let staged = commit_expected.clone();
                                let path = root.to_path_buf();
                                let sender = commit_sender.clone();
                                mutation_pending = true;
                                commit_feedback.clear();
                                std::thread::spawn(move || {
                                    let result = staged
                                        .as_ref()
                                        .ok_or_else(|| "Staged changes are unavailable".to_owned())
                                        .and_then(|preview| {
                                            gitview::commit::create_commit(&path, &message, preview)
                                        });
                                    let _ = sender.send((subject, description, false, result));
                                });
                            }
                        }
                    }
                    KeyCode::Char(character) if !character.is_control() => {
                        if commit_description_focused {
                            commit_description.push(character);
                        } else if let Some(message) = &mut commit_message {
                            message.push(character);
                        }
                        commit_feedback.clear();
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = discard_confirm.as_ref() {
                if mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('y') => {
                        mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = discard_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(gitview::worktree::discard(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => discard_confirm = None,
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = untrack_confirm.as_ref() {
                if mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('y') => {
                        mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = untrack_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(gitview::worktree::untrack(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => untrack_confirm = None,
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = remove_confirm.as_ref() {
                if mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('y') => {
                        mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = remove_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(gitview::worktree::remove_path(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => remove_confirm = None,
                    _ => {}
                }
                continue;
            }
            if let Some(preview) = move_confirm.as_ref() {
                if mutation_pending {
                    continue;
                }
                match key.code {
                    KeyCode::Char('?') => help = true,
                    KeyCode::Char('y') => {
                        mutation_pending = true;
                        let preview = preview.clone();
                        let path = root.to_path_buf();
                        let sender = move_sender.clone();
                        std::thread::spawn(move || {
                            let _ = sender.send(gitview::worktree::move_path(&path, &preview));
                        });
                    }
                    KeyCode::Char('n') | KeyCode::Esc => move_confirm = None,
                    _ => {}
                }
                continue;
            }
            if searching {
                match key.code {
                    KeyCode::Esc => {
                        searching = false;
                        search_debounce = None;
                    }
                    KeyCode::Char('?') => help = true,
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
            if key.code == KeyCode::Char('W')
                && !help
                && !mutation_pending
                && (!branch_view || (branch_input.is_none() && branch_confirm.is_none()))
            {
                let _ = app_state.handle_key(KeyCode::Char('W'), false);
                branch_view = false;
                diff = None;
                diff_entry = None;
                details = None;
                continue;
            }
            if branch_view && !help {
                if mutation_pending {
                    continue;
                }
                if let Some(input) = &mut branch_input {
                    match key.code {
                        KeyCode::Char('?') => help = true,
                        KeyCode::Esc => {
                            branch_input = None;
                            branch_rename = None;
                        }
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                        KeyCode::Enter => {
                            let name = input.clone();
                            branch_input = None;
                            if let Some(preview) = branch_rename.take() {
                                branch_confirm = Some(BranchAction::Rename { preview, name });
                                continue;
                            }
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
                        KeyCode::Char('?') => help = true,
                        KeyCode::Char('y') => {
                            mutation_pending = true;
                            let action = action.clone();
                            let path = root.to_path_buf();
                            let sender = operation_sender.clone();
                            std::thread::spawn(move || {
                                let (success_message, result) = match action {
                                    BranchAction::Previous(preview) => (
                                        "Returned to previous checkout.".to_owned(),
                                        gitview::branches::switch_previous_checkout(
                                            &path, &preview,
                                        ),
                                    ),
                                    BranchAction::Switch(name) => (
                                        "Branch switched successfully.".to_owned(),
                                        gitview::branches::switch_branch(&path, &name),
                                    ),
                                    BranchAction::Rename { preview, name } => (
                                        "Branch renamed successfully.".to_owned(),
                                        gitview::branches::rename_branch(&path, &preview, &name),
                                    ),
                                    BranchAction::Delete(preview) => (
                                        "Branch deleted successfully.".to_owned(),
                                        gitview::branches::delete_branch(&path, &preview),
                                    ),
                                    BranchAction::Create { name, base } => (
                                        "Branch created and switched successfully.".to_owned(),
                                        gitview::branches::create_branch(&path, &name, &base),
                                    ),
                                };
                                let _ = sender.send((success_message, result));
                            });
                        }
                        KeyCode::Char('n') | KeyCode::Esc => branch_confirm = None,
                        _ => {}
                    }
                    continue;
                }
                match navigation_key(key) {
                    KeyCode::Char('t') => {
                        tag_page = Some(gitview::tag_page::TagPage::open(root));
                    }
                    KeyCode::Char('-') => {
                        match gitview::branches::preview_previous_checkout(root) {
                            Ok(preview) => {
                                branch_confirm = Some(BranchAction::Previous(preview));
                                branch_feedback.clear();
                            }
                            Err(error) => branch_feedback = error,
                        }
                    }
                    KeyCode::Char('r' | 'd') => {
                        if let (Some(Ok(data)), Some(index)) =
                            (&branches, branch_selection.selected())
                            && let Some(name) = data.branches.get(index)
                        {
                            match gitview::branches::preview_branch(root, name) {
                                Ok(preview) if key.code == KeyCode::Char('r') => {
                                    branch_rename = Some(preview);
                                    branch_input = Some(String::new());
                                    branch_feedback.clear();
                                }
                                Ok(preview) => branch_confirm = Some(BranchAction::Delete(preview)),
                                Err(error) => branch_feedback = error,
                            }
                        }
                    }
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
                    KeyCode::Char('H') => {
                        branch_view = false;
                        let _ = app_state.handle_key(KeyCode::Char('H'), false);
                    }
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
            if key.code == KeyCode::Char('H') && !help && !mutation_pending {
                let _ = app_state.handle_key(KeyCode::Char('H'), false);
                searching = false;
                diff = None;
                diff_entry = None;
                details = None;
                continue;
            }
            if key.code == KeyCode::Char('p')
                && !mutation_pending
                && details.is_none()
                && diff.is_none()
                && !branch_view
            {
                profile_selection.select(
                    profiles
                        .iter()
                        .position(|profile| profile.name == profile_label),
                );
                profile_feedback.clear();
                profiles_view = true;
                continue;
            }
            if key.code != KeyCode::Char('c') {
                commit_feedback.clear();
            }
            let blocked = help || mutation_pending || details.is_some() || diff.is_some();
            if let Some(action) = app_state.handle_key(navigation_key(key), blocked) {
                match action {
                    AppAction::LoadWorkingTree => {
                        // Keep the cached working-tree status while switching panes.
                        // The initial load, mutations, and explicit `r` refresh it.
                    }
                    AppAction::ReturnToHistory => {}
                    AppAction::InspectObject => {
                        let revision = history
                            .as_ref()
                            .and_then(|rows| rows.as_ref().ok())
                            .and_then(|rows| selection.selected().and_then(|index| rows.get(index)))
                            .and_then(|row| gitview::history::commit_id(row))
                            .unwrap_or("HEAD");
                        object_page = Some(gitview::object_page::ObjectPage::open(revision));
                    }
                    AppAction::OpenComparison => {
                        let target = history
                            .as_ref()
                            .and_then(|rows| rows.as_ref().ok())
                            .and_then(|rows| selection.selected().and_then(|index| rows.get(index)))
                            .and_then(|row| gitview::history::commit_id(row))
                            .unwrap_or("HEAD");
                        comparison_page = Some(gitview::comparison_page::ComparisonPage::open(
                            target,
                            history_filter.as_deref(),
                        ));
                    }
                    AppAction::StartPathHistory => {
                        history_path_input = Some(
                            history_filter
                                .as_ref()
                                .map(|path| path.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        );
                    }
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
                    AppAction::StartCommit | AppAction::StartAmend => {
                        let amend = action == AppAction::StartAmend;
                        commit_preview_mode = Some(amend);
                        commit_feedback.clear();
                        commit_preview_worker.request(amend);
                    }
                    AppAction::PreviewDiscardSelected => {
                        if let (Some(Ok(entries)), Some(index)) =
                            (&status, status_selection.selected())
                            && let Some(entry) = entries.get(index)
                        {
                            match gitview::worktree::preview_discard(root, entry) {
                                Ok(preview) => {
                                    discard_confirm = Some(preview);
                                    commit_feedback.clear();
                                }
                                Err(error) => commit_feedback = error,
                            }
                        }
                    }
                    AppAction::PreviewUntrackSelected => {
                        if let (Some(Ok(entries)), Some(index)) =
                            (&status, status_selection.selected())
                            && let Some(entry) = entries.get(index)
                        {
                            match gitview::worktree::preview_untrack(root, entry) {
                                Ok(preview) => {
                                    untrack_confirm = Some(preview);
                                    commit_feedback.clear();
                                }
                                Err(error) => commit_feedback = error,
                            }
                        }
                    }
                    AppAction::InspectIgnoreRule => {
                        if let (Some(Ok(entries)), Some(index)) =
                            (&status, status_selection.selected())
                            && let Some(entry) = entries.get(index)
                        {
                            let entry = entry.clone();
                            let sender = ignore_sender.clone();
                            let path = root.to_path_buf();
                            std::thread::spawn(move || {
                                let _ = sender.send(gitview::worktree::ignore_rule(&path, &entry));
                            });
                        }
                    }
                    AppAction::StartIgnorePathInput => {
                        ignore_path_input = Some(String::new());
                        commit_feedback.clear();
                    }
                    AppAction::StartRemovePathInput => {
                        remove_path_input = Some(String::new());
                        commit_feedback.clear();
                    }
                    AppAction::StartMovePathInput => {
                        move_source.clear();
                        move_destination.clear();
                        move_destination_focused = false;
                        move_path_input = true;
                        commit_feedback.clear();
                    }
                    AppAction::StageAll => {
                        status_request += 1;
                        let id = status_request;
                        let sender = status_sender.clone();
                        let path = root.to_path_buf();
                        mutation_pending = true;
                        status_mutation = Some(id);
                        std::thread::spawn(move || {
                            let result = gitview::worktree::stage_all(&path);
                            let _ = sender.send((
                                id,
                                result.and_then(|()| gitview::worktree::load_status(&path)),
                            ));
                        });
                    }
                    AppAction::UnstageAll => {
                        status_request += 1;
                        let id = status_request;
                        let sender = status_sender.clone();
                        let path = root.to_path_buf();
                        mutation_pending = true;
                        status_mutation = Some(id);
                        std::thread::spawn(move || {
                            let result = gitview::worktree::unstage_all(&path);
                            let _ = sender.send((
                                id,
                                result.and_then(|()| gitview::worktree::load_status(&path)),
                            ));
                        });
                    }
                    AppAction::StageSelected | AppAction::UnstageSelected => {
                        if let (Some(Ok(entries)), Some(index)) =
                            (&status, status_selection.selected())
                            && let Some(entry) = entries.get(index)
                        {
                            let entry = entry.clone();
                            let stage = action == AppAction::StageSelected;
                            let eligible = if stage {
                                entry.section != gitview::worktree::Section::Staged
                            } else {
                                entry.section == gitview::worktree::Section::Staged
                            };
                            if eligible {
                                status_request += 1;
                                let id = status_request;
                                let sender = status_sender.clone();
                                let path = root.to_path_buf();
                                mutation_pending = true;
                                status_mutation = Some(id);
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
                KeyCode::Tab if !help && diff.is_some() => {
                    diff_side_by_side = !diff_side_by_side;
                    diff_scroll = 0;
                }
                KeyCode::Tab
                | KeyCode::Char('b')
                | KeyCode::Char('/')
                | KeyCode::Char('n')
                | KeyCode::Char('r') => {}
                KeyCode::Esc if !help => {
                    if diff.is_some() {
                        diff = None;
                        diff_entry = None;
                    } else {
                        details = None;
                    }
                }
                KeyCode::Char('v') if !help && diff.is_some() => {
                    diff_raw_view = !diff_raw_view;
                    diff_scroll = 0;
                }
                KeyCode::Enter
                    if !help && app_state.view() == View::WorkingTree && diff.is_none() =>
                {
                    if let (Some(Ok(entries)), Some(index)) = (&status, status_selection.selected())
                        && let Some(entry) = entries.get(index)
                    {
                        let entry = entry.clone();
                        diff_entry = Some(entry.clone());
                        request_id += 1;
                        let current = request_id;
                        let sender = diff_sender.clone();
                        let path = std::path::PathBuf::from(root);
                        diff = Some((current, None));
                        diff_scroll = 0;
                        selected_hunk = 0;
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
                        diff_entry = None;
                        selected_hunk = 0;
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
                KeyCode::Char(']')
                    if !help
                        && diff_entry.as_ref().is_some_and(|entry| {
                            entry.section != gitview::worktree::Section::Untracked
                        }) =>
                {
                    if let Some((_, Some(Ok(text)))) = &diff {
                        let positions = hunk_line_positions(
                            text,
                            diff_raw_view,
                            diff_side_by_side,
                            terminal.size().map_or(80, |size| size.width),
                        );
                        if !positions.is_empty() {
                            selected_hunk = (selected_hunk + 1).min(positions.len() - 1);
                            diff_scroll = positions[selected_hunk] as u16;
                        }
                    }
                }
                KeyCode::Char('[')
                    if !help
                        && diff_entry.as_ref().is_some_and(|entry| {
                            entry.section != gitview::worktree::Section::Untracked
                        }) =>
                {
                    if let Some((_, Some(Ok(text)))) = &diff {
                        let positions = hunk_line_positions(
                            text,
                            diff_raw_view,
                            diff_side_by_side,
                            terminal.size().map_or(80, |size| size.width),
                        );
                        if !positions.is_empty() {
                            selected_hunk = selected_hunk.saturating_sub(1);
                            diff_scroll = positions[selected_hunk] as u16;
                        }
                    }
                }
                KeyCode::Char('s' | 'u' | 'S' | 'U') if !help && !mutation_pending => {
                    let stage = matches!(key.code, KeyCode::Char('s' | 'S'));
                    let all_hunks = matches!(key.code, KeyCode::Char('S' | 'U'));
                    if let (Some(entry), Some((_, Some(Ok(text))))) = (&diff_entry, &diff)
                        && ((stage
                            && (entry.section == gitview::worktree::Section::Unstaged
                                || (all_hunks
                                    && entry.section == gitview::worktree::Section::Untracked)))
                            || (!stage && entry.section == gitview::worktree::Section::Staged))
                        && (all_hunks
                            || selected_hunk
                                < hunk_line_positions(
                                    text,
                                    diff_raw_view,
                                    diff_side_by_side,
                                    terminal.size().map_or(80, |size| size.width),
                                )
                                .len())
                    {
                        let entry = entry.clone();
                        let hunk = selected_hunk;
                        let preferred_section = if stage {
                            if all_hunks {
                                gitview::worktree::Section::Staged
                            } else {
                                gitview::worktree::Section::Unstaged
                            }
                        } else if all_hunks {
                            gitview::worktree::Section::Unstaged
                        } else {
                            gitview::worktree::Section::Staged
                        };
                        let fallback_section = if stage {
                            gitview::worktree::Section::Staged
                        } else {
                            gitview::worktree::Section::Unstaged
                        };
                        diff_entry_refresh = Some((preferred_section, fallback_section));
                        status_request += 1;
                        let status_id = status_request;
                        status_mutation = Some(status_id);
                        mutation_pending = true;
                        let diff_id = diff.as_ref().map(|(id, _)| *id).unwrap_or_default();
                        let status_sender = status_sender.clone();
                        let diff_sender = diff_sender.clone();
                        let path = root.to_path_buf();
                        std::thread::spawn(move || {
                            let result = if all_hunks && stage {
                                gitview::worktree::stage(&path, &entry)
                            } else if all_hunks {
                                gitview::worktree::unstage(&path, &entry)
                            } else if stage {
                                gitview::worktree::stage_hunk(&path, &entry, hunk)
                            } else {
                                gitview::worktree::unstage_hunk(&path, &entry, hunk)
                            };
                            let status_result =
                                result.and_then(|()| gitview::worktree::load_status(&path));
                            let diff_result = match &status_result {
                                Ok(entries) => entries
                                    .iter()
                                    .find(|current| {
                                        current.section == preferred_section
                                            && current.path == entry.path
                                            && current.original == entry.original
                                    })
                                    .or_else(|| {
                                        entries.iter().find(|current| {
                                            current.section == fallback_section
                                                && current.path == entry.path
                                                && current.original == entry.original
                                        })
                                    })
                                    .map_or_else(
                                        || Ok(String::new()),
                                        |current| gitview::worktree::load_diff(&path, current),
                                    ),
                                Err(error) => Err(error.clone()),
                            };
                            let _ = status_sender.send((status_id, status_result));
                            let _ = diff_sender.send((diff_id, diff_result));
                        });
                    }
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

fn hunk_line_positions(
    diff: &str,
    raw: bool,
    side_by_side: bool,
    terminal_width: u16,
) -> Vec<usize> {
    ui::diff_hunk_line_positions(diff, terminal_width, raw, side_by_side)
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

fn normalize_commit_paste(pasted: &str, multiline: bool) -> String {
    pasted
        .chars()
        .filter_map(|character| match character {
            '\r' => None,
            '\n' if multiline => Some('\n'),
            '\n' | '\t' => Some(' '),
            character if character.is_control() => None,
            character => Some(character),
        })
        .collect()
}

fn combined_commit_message(subject: &str, description: &str) -> String {
    if description.trim().is_empty() {
        subject.to_owned()
    } else {
        format!("{}\n\n{}", subject.trim_end(), description.trim_end())
    }
}

fn split_commit_message(message: &str) -> (String, String) {
    match message.split_once('\n') {
        Some((subject, description)) => (
            subject.to_owned(),
            description.trim_start_matches('\n').trim_end().to_owned(),
        ),
        None => (message.to_owned(), String::new()),
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
    fn commit_paste_keeps_description_newlines_and_flattens_subject_lines() {
        assert_eq!(
            normalize_commit_paste("feat: add editor\r\nwith paste", false),
            "feat: add editor with paste"
        );
        assert_eq!(
            normalize_commit_paste("First line\r\nSecond\tline", true),
            "First line\nSecond line"
        );
    }

    #[test]
    fn splits_and_recombines_existing_commit_subject_and_description() {
        let (subject, description) =
            split_commit_message("fix: preserve files\n\nExplain the safety rule.\n");
        assert_eq!(subject, "fix: preserve files");
        assert_eq!(description, "Explain the safety rule.");
        assert_eq!(
            combined_commit_message(&subject, &description),
            "fix: preserve files\n\nExplain the safety rule."
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
