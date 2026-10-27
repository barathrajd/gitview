use gitview::{
    app::BranchAction,
    branches::{Branches, HeadState},
    details::CommitDetails,
    ui::{Screen, render},
    worktree::{Entry, Section},
};
use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
use std::{ffi::OsString, path::Path};

fn draw(screen: Screen<'_>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render(frame, screen, ratatui::symbols::border::PLAIN))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn renders_help_and_loading_or_empty_history_states() {
    let help = draw(Screen::Help, 80, 24);
    assert!(help.contains("KEYBOARD SHORTCUTS"));
    assert!(help.contains("KEY"));
    assert!(help.contains("DETAILS"));
    assert!(gitview::ui::HELP.contains("j/k"));
    assert!(gitview::ui::HELP.contains("Enter"));
    assert!(gitview::ui::HELP.contains(" h "));
    assert!(gitview::ui::HELP.contains(" l "));
    assert!(gitview::ui::HELP.contains("q                   Quit outside prompts"));

    let history = None;
    let mut selection = ListState::default();
    let loading = draw(
        Screen::History {
            history,
            searching: false,
            query: "",
            search_message: "",
            root: Path::new("/repo"),
            selection: &mut selection,
        },
        80,
        24,
    );
    assert!(loading.contains("Loading history"));

    let history = Some(Ok(Vec::new()));
    let empty = draw(
        Screen::History {
            history: history.as_ref(),
            searching: false,
            query: "",
            search_message: "",
            root: Path::new("/repo"),
            selection: &mut selection,
        },
        80,
        24,
    );
    assert!(empty.contains("No commits yet"));
}

#[test]
fn renders_debounced_search_result_list() {
    let history = Ok(vec![
        "* abcdef0 Unrelated change - Alex\u{1f}abcdef0".to_owned(),
        "* 1234567 Fix search results - Sam\u{1f}1234567".to_owned(),
    ]);
    let matches = vec![1];
    let mut selection = ListState::default();
    selection.select(Some(0));
    let search = draw(
        Screen::Search {
            history: Some(&history),
            matches: &matches,
            query: "search",
            pending: false,
            selection: &mut selection,
        },
        120,
        24,
    );
    assert!(search.contains("SEARCH"));
    assert!(search.contains("search"));
    assert!(search.contains("Fix search results"));
    assert!(!search.contains("Unrelated change"));
    assert!(search.contains("open"));
}

#[test]
fn renders_working_tree_files_and_branch_confirmation() {
    let status = Ok(vec![Entry {
        section: Section::Untracked,
        path: OsString::from("new-file"),
        original: None,
    }]);
    let mut selection = ListState::default();
    let working_tree = draw(
        Screen::WorkingTree {
            status: Some(&status),
            selection: &mut selection,
        },
        80,
        24,
    );
    assert!(working_tree.contains("new-file"));

    let action = BranchAction::Switch(OsString::from("topic"));
    let confirmation = draw(
        Screen::BranchConfirmation {
            pending: false,
            action: Some(&action),
        },
        80,
        24,
    );
    assert!(confirmation.contains("Switch to local branch"));
    assert!(confirmation.contains("topic"));
}

#[test]
fn renders_error_states_for_git_backed_views() {
    let status = Err("Git status failed".to_owned());
    let mut selection = ListState::default();
    let working_tree = draw(
        Screen::WorkingTree {
            status: Some(&status),
            selection: &mut selection,
        },
        80,
        24,
    );
    assert!(working_tree.contains("Git status failed"));

    let branches = Err("Branch listing failed".to_owned());
    let branch_error = draw(
        Screen::Branches {
            branches: Some(&branches),
            feedback: "",
            selection: &mut selection,
        },
        80,
        24,
    );
    assert!(branch_error.contains("Branch listing failed"));

    let diff = (1, Some(Err("Diff failed".to_owned())));
    let diff_error = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
        },
        80,
        24,
    );
    assert!(diff_error.contains("Diff failed"));
}

#[test]
fn renders_diff_titles_for_narrow_and_wide_terminals() {
    let diff = (1, Some(Ok("@@ -10 +20 @@\n-before\n+after\n".to_owned())));
    let narrow = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
        },
        119,
        24,
    );
    assert!(narrow.contains("DIFF"));
    assert!(!narrow.contains("Before"));
    assert!(narrow.contains("10"));
    assert!(narrow.contains("20"));

    let wide = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
        },
        120,
        24,
    );
    assert!(wide.contains("BEFORE"));
    assert!(wide.contains("AFTER"));
    assert!(wide.contains("10"));
    assert!(wide.contains("20"));
    assert!(wide.contains("Up/Down or j/k"));
    assert!(wide.contains("Esc/h"));
}

#[test]
fn renders_branch_and_commit_details() {
    let branches = Ok(Branches {
        head: HeadState::Attached(OsString::from("main")),
        branches: vec![OsString::from("main"), OsString::from("topic")],
        head_commit: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
    });
    let mut branch_selection = ListState::default();
    let branch_list = draw(
        Screen::Branches {
            branches: Some(&branches),
            feedback: "",
            selection: &mut branch_selection,
        },
        100,
        24,
    );
    assert!(branch_list.contains("Local branches"));
    assert!(branch_list.contains("topic"));
    assert!(branch_list.contains("Enter/l"));
    assert!(branch_list.contains("quit"));

    let details = (
        1,
        Some(Ok(CommitDetails {
            hash: "0123456789abcdef".to_owned(),
            author: "Developer".to_owned(),
            date: "2026-10-27".to_owned(),
            subject: "Add a feature".to_owned(),
            files: vec![OsString::from("src/main.rs")],
        })),
    );
    let mut file_selection = ListState::default();
    let commit = draw(
        Screen::CommitDetails {
            details: &details,
            selection: &mut file_selection,
        },
        100,
        24,
    );
    assert!(commit.contains("Add a feature"));
    assert!(commit.contains("src/main.rs"));
    assert!(commit.contains("j/k"));
    assert!(commit.contains("Enter/l"));
    assert!(commit.contains("diff"));
}
