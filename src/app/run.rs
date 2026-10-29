use std::{
    io,
    sync::mpsc::{self, RecvTimeoutError},
    time::{Duration, Instant},
};

use crate::{
    app::{AppState, BranchAction},
    pane::AgentPane,
    ui::{self, ASCII_BORDER},
};
use crossterm::event::{Event, KeyEventKind};
use ratatui::{Terminal, backend::Backend, widgets::ListState};

pub use super::helpers::Navigation;
use super::{
    agent, drain, draw,
    helpers::{CommitReadResult, backend_error},
    input::{self, Route},
    keys,
    model::App,
    paste,
    refresh::{self, Refresh, Watching},
};

/// What a piece of the event loop decided about the current event.
pub(super) enum Flow {
    /// Not handled here; try the next handler.
    Next,
    /// Handled; wait for the next event.
    Handled,
    /// The user quit.
    Quit,
}

/// The terminal and paths the loop uses besides `App`.
pub(super) struct Env<'a, H: Host> {
    pub(super) terminal: &'a mut Terminal<H::Backend>,
    pub(super) host: &'a mut H,
    pub(super) root: &'a std::path::Path,
    pub(super) config_dir: &'a std::path::Path,
    pub(super) profiles: &'a mut Vec<crate::config::FolderProfile>,
}

/// Runs a handler and acts on its `Flow`: fall through, skip to the next event,
/// or return from the loop.
macro_rules! step {
    ($handler:expr) => {
        match $handler? {
            Flow::Next => {}
            Flow::Handled => continue,
            Flow::Quit => return Ok(()),
        }
    };
}

/// What `run_app` needs from the terminal it runs in. The real terminal and the
/// in-memory test backend each provide one.
pub trait Host {
    type Backend: Backend;

    /// Waits up to `timeout` for the next input event. `terminal` shows the
    /// last frame, so a test host can inspect the screen.
    fn next_event(
        &mut self,
        terminal: &Terminal<Self::Backend>,
        timeout: Duration,
    ) -> Result<Event, RecvTimeoutError>;

    /// Runs an interactive external program with the terminal handed back to it.
    fn run_external<T>(
        &mut self,
        terminal: &mut Terminal<Self::Backend>,
        program: impl FnOnce() -> T,
    ) -> io::Result<T>;
}

pub fn run_app<H: Host>(
    terminal: &mut Terminal<H::Backend>,
    host: &mut H,
    root: &std::path::Path,
    ascii: bool,
    profile_label: String,
    config_dir: &std::path::Path,
    profiles: &mut Vec<crate::config::FolderProfile>,
) -> io::Result<()> {
    run_app_with_agent(
        terminal,
        host,
        root,
        ascii,
        profile_label,
        config_dir,
        profiles,
        None,
    )
}

/// Like `run_app`, with `agent` (a command and its arguments) running in a pane
/// beside Gitview. The program starts in `root` and ends when this returns.
#[allow(clippy::too_many_arguments)]
pub fn run_app_with_agent<H: Host>(
    terminal: &mut Terminal<H::Backend>,
    host: &mut H,
    root: &std::path::Path,
    ascii: bool,
    profile_label: String,
    config_dir: &std::path::Path,
    profiles: &mut Vec<crate::config::FolderProfile>,
    agent: Option<Vec<String>>,
) -> io::Result<()> {
    let pane = match agent {
        Some(command) => {
            let size = terminal.size().map_err(backend_error)?;
            // The first draw corrects this to the pane's real size.
            let rows = size.height.saturating_sub(2).max(1);
            let columns = (size.width / 2).saturating_sub(2).max(1);
            Some(AgentPane::spawn(&command, root, rows, columns).map_err(io::Error::other)?)
        }
        None => None,
    };
    let border = if ascii {
        ASCII_BORDER
    } else {
        ratatui::symbols::border::PLAIN
    };
    let help = false;
    let searching = false;
    let history_filter: Option<std::path::PathBuf> = None;
    let history_path_input: Option<String> = None;
    let query = String::new();
    let search_results = Vec::new();
    let search_selection = ListState::default();
    let search_debounce: Option<Instant> = None;
    let (history_sender, receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<String>, String>)>();
    let history_request = 0u64;
    let sender = history_sender.clone();
    let path = root.to_path_buf();
    let requested_history_path = history_filter.clone();
    std::thread::spawn(move || {
        let _ = sender.send((
            0,
            crate::history::load_history_for_path(&path, requested_history_path.as_deref()),
        ));
    });
    let history: Option<Result<Vec<String>, String>> = None;
    let selection = ListState::default();
    let (details_sender, details_receiver) =
        std::sync::mpsc::channel::<(u64, Result<crate::details::CommitDetails, String>)>();
    let details: Option<(u64, Option<Result<crate::details::CommitDetails, String>>)> = None;
    let request_id = 0u64;
    let file_selection = ListState::default();
    let (diff_sender, diff_receiver) = std::sync::mpsc::channel::<(u64, Result<String, String>)>();
    let diff: Option<(u64, Option<Result<String, String>>)> = None;
    let diff_root = root.to_path_buf();
    let ordinary_diff_worker = crate::read_worker::LatestRead::new(move |request| {
        crate::diff_loader::load(&diff_root, request)
    });
    let prepared_diff: Option<ui::PreparedDiff> = None;
    let diff_scroll = 0u16;
    let diff_raw_view = false;
    let diff_side_by_side = true;
    let diff_entry: Option<crate::worktree::Entry> = None;
    let selected_hunk = 0usize;
    let branch_view = false;
    let object_page: Option<crate::object_page::ObjectPage> = None;
    let comparison_page: Option<crate::comparison_page::ComparisonPage> = None;
    let tag_page: Option<crate::tag_page::TagPage> = None;
    let profiles_view = false;
    let profile_selection = ListState::default();
    let profile_feedback = String::new();
    let profile_registry_path = config_dir.join("profiles.json");
    let (branch_sender, branch_receiver) =
        std::sync::mpsc::channel::<(u64, Result<crate::branches::Branches, String>)>();
    let branch_request = 0u64;
    let (operation_sender, operation_receiver) =
        std::sync::mpsc::channel::<(String, Result<(), String>)>();
    let branch_confirm: Option<BranchAction> = None;
    let branch_input: Option<String> = None;
    let branch_rename: Option<crate::branches::BranchPreview> = None;
    let branch_feedback = String::new();
    let ignore_path_input: Option<String> = None;
    let remove_path_input: Option<String> = None;
    let remove_confirm: Option<crate::worktree::RemovePathPreview> = None;
    let move_path_input = false;
    let move_source = String::new();
    let move_destination = String::new();
    let move_destination_focused = false;
    let move_confirm: Option<crate::worktree::MovePathPreview> = None;
    let preview_root = root.to_path_buf();
    let preview_worker = crate::read_worker::LatestRead::new(move |request| {
        crate::preview::prepare(&preview_root, request)
    });
    let pending_preview: Option<crate::preview::Context> = None;
    let commit_preview_retry = false;
    let commit_preview_mode: Option<bool> = None;
    let preview_root = root.to_path_buf();
    let commit_preview_worker = crate::read_worker::LatestRead::new(move |amend: bool| {
        if amend {
            CommitReadResult::Amend(crate::commit::preview_amend(&preview_root))
        } else {
            CommitReadResult::Create(crate::commit::preview_commit(&preview_root))
        }
    });
    let commit_message: Option<String> = None;
    let commit_description = String::new();
    let commit_description_focused = false;
    let commit_amend = false;
    let amend_confirm = false;
    let commit_expected: Option<crate::commit::CommitPreview> = None;
    let amend_expected: Option<crate::commit::AmendPreview> = None;
    let commit_feedback = String::new();
    let (commit_sender, commit_receiver) =
        mpsc::channel::<(String, String, bool, Result<(), String>)>();
    let discard_confirm: Option<crate::worktree::DiscardPreview> = None;
    let (discard_sender, discard_receiver) = mpsc::channel::<Result<(), String>>();
    let untrack_confirm: Option<crate::worktree::UntrackPreview> = None;
    let (untrack_sender, untrack_receiver) = mpsc::channel::<Result<(), String>>();
    let (remove_sender, remove_receiver) = mpsc::channel::<Result<(), String>>();
    let (move_sender, move_receiver) = mpsc::channel::<Result<(), String>>();
    let mutation_pending = false;
    let status_mutation: Option<u64> = None;
    let diff_entry_refresh: Option<(crate::worktree::Section, crate::worktree::Section)> = None;
    let branches: Option<Result<crate::branches::Branches, String>> = None;
    let branch_selection = ListState::default();
    let app_state = AppState::default();
    let (status_sender, status_receiver) =
        std::sync::mpsc::channel::<(u64, Result<Vec<crate::worktree::Entry>, String>)>();
    let (status_watch_sender, status_watch_receiver) =
        std::sync::mpsc::sync_channel::<Vec<crate::worktree::Entry>>(1);
    let status: Option<Result<Vec<crate::worktree::Entry>, String>> = None;
    let status_request = 0u64;
    let status_selection = ListState::default();
    {
        let sender = status_sender.clone();
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send((0, crate::worktree::load_status(&path)));
        });
    }
    // Changes are noticed by a file watcher that starts in the background, with
    // polling as the fallback.
    let watching = Watching::begin(root, status_watch_sender);
    {
        let sender = branch_sender.clone();
        let path = root.to_path_buf();
        std::thread::spawn(move || {
            let _ = sender.send((0, crate::branches::load_branches(&path)));
        });
    }
    let mut app = App {
        border,
        help,
        searching,
        history_filter,
        history_path_input,
        query,
        search_results,
        search_selection,
        search_debounce,
        history_sender,
        receiver,
        history_request,
        history,
        selection,
        details_sender,
        details_receiver,
        details,
        request_id,
        file_selection,
        diff_sender,
        diff_receiver,
        diff,
        ordinary_diff_worker,
        prepared_diff,
        diff_scroll,
        diff_raw_view,
        diff_side_by_side,
        diff_entry,
        selected_hunk,
        branch_view,
        object_page,
        comparison_page,
        tag_page,
        profiles_view,
        profile_selection,
        profile_feedback,
        profile_registry_path,
        branch_sender,
        branch_receiver,
        branch_request,
        operation_sender,
        operation_receiver,
        branch_confirm,
        branch_input,
        branch_rename,
        branch_feedback,
        ignore_path_input,
        remove_path_input,
        remove_confirm,
        move_path_input,
        move_source,
        move_destination,
        move_destination_focused,
        move_confirm,
        preview_worker,
        pending_preview,
        commit_preview_retry,
        commit_preview_mode,
        commit_preview_worker,
        commit_message,
        commit_description,
        commit_description_focused,
        commit_amend,
        amend_confirm,
        commit_expected,
        amend_expected,
        commit_feedback,
        commit_sender,
        commit_receiver,
        discard_confirm,
        discard_sender,
        discard_receiver,
        untrack_confirm,
        untrack_sender,
        untrack_receiver,
        remove_sender,
        remove_receiver,
        move_sender,
        move_receiver,
        mutation_pending,
        status_mutation,
        diff_entry_refresh,
        branches,
        branch_selection,
        app_state,
        status_sender,
        status_receiver,
        status_watch_receiver,
        status,
        status_request,
        status_selection,
        needs_redraw: true,
        profile_label,
        watching,
        refresh: Refresh::default(),
        diff_refresh: None,
        focus: input::Focus::default(),
        pane,
    };
    let mut env = Env {
        terminal,
        host,
        root,
        config_dir,
        profiles,
    };
    loop {
        step!(drain::drain_results(&mut app, &mut env));
        agent::poll(&mut app);
        refresh::step(&mut app, &mut env);
        step!(draw::draw_screen(&mut app, &mut env));
        let event = match env.host.next_event(env.terminal, Duration::from_millis(50)) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };
        app.needs_redraw = true;
        match input::route(app.focus, app.pane.is_some(), &event) {
            Route::ToggleFocus => {
                agent::toggle_focus(&mut app);
                continue;
            }
            Route::Agent => {
                agent::forward(&mut app, &event);
                continue;
            }
            Route::Gitview => {}
        }
        step!(paste::handle_paste(&mut app, &mut env, &event));
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            step!(keys::overlays::handle(&mut app, &mut env, key));
            step!(keys::prompts::handle(&mut app, &mut env, key));
            step!(keys::commit::handle(&mut app, &mut env, key));
            step!(keys::confirm::handle(&mut app, &mut env, key));
            step!(keys::search::handle(&mut app, &mut env, key));
            step!(keys::branches::handle(&mut app, &mut env, key));
            step!(keys::views::handle(&mut app, &mut env, key));
        }
    }
}
