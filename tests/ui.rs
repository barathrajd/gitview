use gitview::{
    app::BranchAction,
    branches::{Branches, HeadState},
    config::FolderProfile,
    details::CommitDetails,
    ui::{HelpPage, Screen, render, render_help_overlay},
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
fn contextual_help_opens_as_a_page_specific_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let diff = (1, Some(Ok("@@ -1 +1 @@\n-before\n+after\n".to_owned())));
    let entry = Entry {
        section: Section::Unstaged,
        path: OsString::from("tracked"),
        original: None,
    };
    terminal
        .draw(|frame| {
            render(
                frame,
                Screen::Diff {
                    diff: &diff,
                    scroll: 0,
                    selected_hunk: 0,
                    raw: false,
                    side_by_side: true,
                    worktree_entry: Some(&entry),
                },
                ratatui::symbols::border::PLAIN,
            );
            render_help_overlay(
                frame,
                HelpPage::WorktreeDiff,
                ratatui::symbols::border::PLAIN,
            );
        })
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("HELP · Working tree diff"));
    assert!(rendered.contains("Stage or unstage selected hunk"));
    assert!(rendered.contains("Esc or ? closes help"));
}

#[test]
fn commit_message_shows_staged_and_unstaged_files_before_single_enter_commit() {
    let input = draw(
        Screen::CommitInput {
            repository: Path::new("/repo/gitview"),
            profile: "Primary",
            branches: None,
            message: "Fix the parser",
            description: "Explain the behavior change\nAdd regression context",
            description_focused: true,
            amend: false,
            feedback: "",
            entries: Some(&Ok(vec![
                Entry {
                    section: Section::Staged,
                    path: OsString::from("src/ready.rs"),
                    original: None,
                },
                Entry {
                    section: Section::Unstaged,
                    path: OsString::from("src/wip.rs"),
                    original: None,
                },
                Entry {
                    section: Section::Untracked,
                    path: OsString::from("notes.txt"),
                    original: None,
                },
            ])),
            pending: false,
        },
        100,
        18,
    );
    assert!(input.contains("COMMIT MESSAGE"));
    assert!(input.contains("COMMIT DESCRIPTION"));
    assert!(input.contains("Fix the parser"));
    assert!(input.contains("Explain the behavior change"));
    assert!(input.contains("Add regression context"));
    assert!(input.contains("GITVIEW"));
    assert!(input.contains("/repo/gitview"));
    assert!(input.contains("WORKTREE"));
    assert!(input.contains("READ ONLY"));
    assert!(input.contains("STAGED CHANGES"));
    assert!(input.contains("CHANGES"));
    assert!(input.contains("src/ready.rs"));
    assert!(input.contains("src/wip.rs"));
    assert!(input.contains("notes.txt"));
    assert!(input.contains("commit"));
    assert!(input.contains("Ctrl+J"));
    assert!(!input.contains("CONFIRM COMMIT"));

    let amend = draw(
        Screen::CommitInput {
            repository: Path::new("/repo/gitview"),
            profile: "Primary",
            branches: None,
            message: "Old commit subject",
            description: "Existing commit body",
            description_focused: false,
            amend: true,
            feedback: "",
            entries: None,
            pending: false,
        },
        100,
        18,
    );
    assert!(amend.contains("AMEND MESSAGE"));
    assert!(amend.contains("Old commit subject"));
    assert!(amend.contains("Existing commit body"));
}

#[test]
fn untrack_confirmation_explains_that_the_working_copy_is_kept() {
    let entry = Entry {
        section: Section::Staged,
        path: OsString::from("src/keep-local.rs"),
        original: None,
    };
    let rendered = draw(
        Screen::UntrackConfirmation {
            entry: &entry,
            pending: false,
        },
        80,
        18,
    );
    assert!(rendered.contains("CONFIRM STOP TRACKING"));
    assert!(rendered.contains("keep the working copy"));
    assert!(rendered.contains("src/keep-local.rs"));
    assert!(rendered.contains("staged deletion"));
}

#[test]
fn ignore_path_prompt_accepts_arbitrary_repository_relative_paths() {
    let rendered = draw(
        Screen::IgnorePathInput {
            path: "build/generated file.log",
            feedback: "",
        },
        80,
        18,
    );
    assert!(rendered.contains("INSPECT IGNORE RULE"));
    assert!(rendered.contains("build/generated file.log"));
    assert!(rendered.contains("inspect path"));
}

#[test]
fn tracked_path_removal_prompt_and_confirmation_show_the_exact_path() {
    let prompt = draw(
        Screen::RemovePathInput {
            path: "src/old file.rs",
            feedback: "",
        },
        80,
        18,
    );
    assert!(prompt.contains("REMOVE TRACKED PATH"));
    assert!(prompt.contains("src/old file.rs"));

    let confirmation = draw(
        Screen::RemovePathConfirmation {
            path: std::ffi::OsStr::new("src/old file.rs"),
            pending: false,
        },
        80,
        18,
    );
    assert!(confirmation.contains("CONFIRM TRACKED PATH REMOVAL"));
    assert!(confirmation.contains("src/old file.rs"));
    assert!(confirmation.contains("normal safety checks"));
}

#[test]
fn tracked_path_move_prompt_and_confirmation_show_both_paths() {
    let prompt = draw(
        Screen::MovePathInput {
            source: "src/old file.rs",
            destination: "src/new file.rs",
            destination_focused: true,
            feedback: "",
        },
        80,
        18,
    );
    assert!(prompt.contains("MOVE TRACKED PATH"));
    assert!(prompt.contains("src/old file.rs"));
    assert!(prompt.contains("src/new file.rs"));

    let confirmation = draw(
        Screen::MovePathConfirmation {
            source: std::ffi::OsStr::new("src/old file.rs"),
            destination: std::ffi::OsStr::new("src/new file.rs"),
            pending: false,
        },
        80,
        18,
    );
    assert!(confirmation.contains("CONFIRM PATH MOVE"));
    assert!(confirmation.contains("src/old file.rs"));
    assert!(confirmation.contains("src/new file.rs"));
}

#[test]
fn amend_confirmation_explains_the_history_rewrite_and_shows_old_and_new_messages() {
    let entries = vec![Entry {
        section: Section::Staged,
        path: OsString::from("src/change.rs"),
        original: None,
    }];
    let rendered = draw(
        Screen::AmendConfirmation {
            repository: Path::new("/repo/gitview"),
            profile: "Primary",
            branches: None,
            entries: Some(&Ok(entries)),
            head: "abcdef1234567890",
            original_message: "Old subject",
            new_message: "feat: new subject\n\nWhy the change matters",
            pending: false,
        },
        120,
        24,
    );
    assert!(rendered.contains("CONFIRM AMEND"));
    assert!(rendered.contains("rewrites HEAD"));
    assert!(rendered.contains("abcdef1234567890"));
    assert!(rendered.contains("Old subject"));
    assert!(rendered.contains("Why the change matters"));
    assert!(rendered.contains("src/change.rs"));
    assert!(rendered.contains("amend HEAD"));
    assert!(rendered.contains("cancel"));
}

#[test]
fn discard_dialog_names_the_selected_file_and_requires_confirmation() {
    let entry = Entry {
        section: Section::Untracked,
        path: OsString::from("new file.rs"),
        original: None,
    };
    let rendered = draw(
        Screen::DiscardConfirmation {
            entry: &entry,
            pending: false,
        },
        60,
        12,
    );
    assert!(rendered.contains("CONFIRM DISCARD"));
    assert!(rendered.contains("new file.rs"));
    assert!(rendered.contains("Permanently delete this untracked file?"));
    assert!(rendered.contains("discard file"));
    assert!(rendered.contains("cancel"));

    let rename = Entry {
        section: Section::Staged,
        path: OsString::from("new name.rs"),
        original: Some(OsString::from("old name.rs")),
    };
    let rendered = draw(
        Screen::DiscardConfirmation {
            entry: &rename,
            pending: false,
        },
        80,
        14,
    );
    assert!(rendered.contains("Permanently discard this staged rename"));
    assert!(rendered.contains("old name.rs"));
    assert!(rendered.contains("new name.rs"));
    assert!(rendered.contains("unstaged content"));
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
fn profile_page_shows_active_and_disabled_folder_mappings() {
    let profiles = vec![
        FolderProfile {
            name: "client-office".into(),
            worktree_root: Path::new("/work/clients/office").to_path_buf(),
            config_path: Path::new("/home/user/.config/gitview/office.gitconfig").to_path_buf(),
            enabled: true,
        },
        FolderProfile {
            name: "old-contract".into(),
            worktree_root: Path::new("/work/contracts/old").to_path_buf(),
            config_path: Path::new("/home/user/.config/gitview/old.gitconfig").to_path_buf(),
            enabled: false,
        },
    ];
    let rendered = draw(
        Screen::Profiles {
            profiles: &profiles,
            selected: Some(0),
            active_profile: "client-office",
            registry_path: Path::new("/home/user/.config/gitview/profiles.json"),
            feedback: "",
        },
        100,
        24,
    );

    assert!(rendered.contains("GIT CONFIG PROFILES"));
    assert!(rendered.contains("client-office"));
    assert!(rendered.contains("ACTIVE"));
    assert!(rendered.contains("DISABLED"));
    assert!(rendered.contains("profiles.json"));
    assert!(rendered.contains("edit registry"));
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
fn selected_worktree_rows_use_status_specific_highlight_colors() {
    let cases = [
        (
            Section::Staged,
            "S",
            ratatui::style::Color::Rgb(94, 234, 212),
            ratatui::style::Color::Rgb(20, 65, 63),
        ),
        (
            Section::Unstaged,
            "E",
            ratatui::style::Color::Rgb(251, 191, 36),
            ratatui::style::Color::Rgb(74, 55, 20),
        ),
        (
            Section::Untracked,
            "N",
            ratatui::style::Color::Rgb(147, 197, 253),
            ratatui::style::Color::Rgb(30, 58, 88),
        ),
    ];

    for (section, label, foreground, background) in cases {
        let status = Ok(vec![Entry {
            section,
            path: OsString::from("file"),
            original: None,
        }]);
        let mut selection = ListState::default();
        selection.select(Some(0));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    Screen::WorkingTree {
                        status: Some(&status),
                        selection: &mut selection,
                    },
                    ratatui::symbols::border::PLAIN,
                )
            })
            .unwrap();

        assert!(terminal.backend().buffer().content().iter().any(|cell| {
            cell.symbol() == label && cell.fg == foreground && cell.bg == background
        }));
    }
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
            selected_hunk: 0,
            raw: false,
            side_by_side: true,
            worktree_entry: None,
        },
        80,
        24,
    );
    assert!(diff_error.contains("Diff failed"));
}

#[test]
fn renders_ide_diff_by_default_and_raw_patch_on_request() {
    let diff = (1, Some(Ok("@@ -10 +20 @@\n-before\n+after\n".to_owned())));
    let narrow = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
            selected_hunk: 0,
            raw: false,
            side_by_side: true,
            worktree_entry: None,
        },
        119,
        24,
    );
    assert!(narrow.contains("BEFORE"));
    assert!(narrow.contains("AFTER"));
    assert!(narrow.contains("10"));
    assert!(narrow.contains("20"));

    let wide = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
            selected_hunk: 0,
            raw: true,
            side_by_side: false,
            worktree_entry: None,
        },
        120,
        24,
    );
    assert!(wide.contains("RAW PATCH"));
    assert!(!wide.contains("BEFORE"));
    assert!(!wide.contains("AFTER"));
    assert!(wide.contains("10"));
    assert!(wide.contains("20"));
    assert!(wide.contains("Up/Down or j/k"));
    assert!(wide.contains("Esc/h"));
}

#[test]
fn working_tree_diff_shows_hunk_controls_and_selection() {
    let diff = (
        1,
        Some(Ok(
            "@@ -2 +2 @@\n-old\n+new\n@@ -8 +8 @@\n-old2\n+new2\n".to_owned()
        )),
    );
    let entry = Entry {
        section: Section::Unstaged,
        path: OsString::from("tracked"),
        original: None,
    };
    let rendered = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
            selected_hunk: 1,
            raw: false,
            side_by_side: true,
            worktree_entry: Some(&entry),
        },
        56,
        20,
    );
    assert!(rendered.contains("HUNK 2/2"));
    assert!(rendered.contains("stage"));
    assert!(rendered.contains("unstage"));
    assert!(rendered.contains("stage all"));
    assert!(rendered.contains("unstage all"));
    assert!(!rendered.contains("quit"));
    assert!(!rendered.contains("help"));
    assert!(!rendered.contains("history"));
    assert!(!rendered.contains("scroll"));
    let cells: Vec<_> = rendered.chars().collect();
    let rows = cells
        .chunks(56)
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>();
    let action_row = rows.iter().find(|row| row.contains("stage all")).unwrap();
    assert!(action_row.contains("unstage all"));
    assert!(action_row.contains("unstage"));
}

#[test]
fn staged_worktree_diff_shows_unstage_all_controls() {
    let diff = (1, Some(Ok("@@ -2 +2 @@\n-old\n+new\n".to_owned())));
    let entry = Entry {
        section: Section::Staged,
        path: OsString::from("tracked"),
        original: None,
    };
    let rendered = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
            selected_hunk: 0,
            raw: false,
            side_by_side: true,
            worktree_entry: Some(&entry),
        },
        120,
        20,
    );
    assert!(rendered.contains("stage hunk"));
    assert!(rendered.contains("unstage hunk"));
    assert!(rendered.contains("stage all"));
    assert!(rendered.contains("unstage all"));
}

#[test]
fn wide_side_by_side_diff_wraps_long_lines() {
    let long_line = format!("+{}tail-marker", "x".repeat(90));
    let diff = (1, Some(Ok(format!("@@ -1 +1 @@\n{long_line}\n"))));
    let rendered = draw(
        Screen::Diff {
            diff: &diff,
            scroll: 0,
            selected_hunk: 0,
            raw: false,
            side_by_side: true,
            worktree_entry: None,
        },
        120,
        24,
    );
    assert!(rendered.contains("BEFORE"));
    assert!(rendered.contains("AFTER"));
    assert!(rendered.contains("tail-marker"));
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
    assert!(!branch_list.contains("quit"));
    assert!(!branch_list.contains("help"));

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

/// Whether the cells under `text` in the first row containing it are underlined.
fn underlined(buffer: &ratatui::buffer::Buffer, text: &str) -> Vec<bool> {
    let width = usize::from(buffer.area.width);
    let cells = buffer.content();
    let rows: Vec<String> = cells
        .chunks(width)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect())
        .collect();
    let (y, row) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.contains(text))
        .unwrap_or_else(|| panic!("{text} not drawn"));
    let x = row[..row.find(text).unwrap()].chars().count();
    (0..text.chars().count())
        .map(|offset| {
            cells[y * width + x + offset]
                .modifier
                .contains(ratatui::style::Modifier::UNDERLINED)
        })
        .collect()
}

#[test]
fn search_page_underlines_the_matched_characters() {
    let history = Ok(vec![
        "* 1234567 Fix search results - Sam\u{1f}1234567".to_owned(),
    ]);
    let matches = vec![0];
    let mut selection = ListState::default();
    selection.select(Some(0));
    let mut terminal = Terminal::new(TestBackend::new(120, 24)).unwrap();
    terminal
        .draw(|frame| {
            render(
                frame,
                Screen::Search {
                    history: Some(&history),
                    matches: &matches,
                    query: "search",
                    pending: false,
                    selection: &mut selection,
                },
                ratatui::symbols::border::PLAIN,
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        underlined(buffer, "Fix search"),
        [
            false, false, false, false, true, true, true, true, true, true
        ]
    );
}
