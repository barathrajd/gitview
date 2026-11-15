//! The workspace sidebar: Branches stacked above the working tree, focus
//! moving with Tab and Shift-Tab, and branch prompts opening in the centre
//! panel with the sidebar still visible.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{BackTab, Enter, Esc, Tab};
use replay::{Repo, Step::*, replay_sized};

fn repository() -> Repo {
    let repository = Repo::with_changes();
    repository.git(&["branch", "feature"]);
    repository
}

/// The title of the panel whose border is drawn in the accent colour can't
/// be read from text, so focus shows as the row marked `›` in the sidebar.
fn sidebar_selection(screen: &str) -> Option<String> {
    let line = screen.lines().find(|line| line.starts_with("│›"))?;
    let panel = line.split('│').nth(1)?;
    Some(panel.trim_start_matches('›').trim().to_owned())
}

#[test]
fn the_sidebar_stacks_branches_above_the_working_tree() {
    let repository = repository();
    let screen = replay_sized(&repository.path(), 140, 30, [WaitFor("BRANCHES (2)")]);
    let branches = screen.find("BRANCHES (2)").unwrap();
    let worktree = screen.find("WORKTREE").unwrap();
    assert!(branches < worktree, "{screen}");
    assert!(screen.contains("* main"), "{screen}");
    assert!(screen.contains("HISTORY"), "{screen}");
    assert!(!screen.contains("INSPECTOR"), "{screen}");
}

#[test]
fn tab_moves_down_the_sidebar_and_shift_tab_moves_up() {
    let repository = repository();
    let start = || vec![WaitFor("BRANCHES (2)"), WaitFor("staged.txt")];
    let after = |keys: &[crossterm::event::KeyCode]| {
        let mut steps = start();
        steps.extend(keys.iter().map(|&key| Key(key)));
        steps.push(Pause(std::time::Duration::from_millis(200)));
        replay_sized(&repository.path(), 140, 30, steps)
    };

    // Files first: the worktree's row is marked.
    let screen = after(&[]);
    assert!(
        sidebar_selection(&screen).unwrap().contains("staged.txt"),
        "{screen}"
    );
    // Tab: History, below the sidebar's last section.
    let screen = after(&[Tab]);
    assert!(screen.contains("Enter/l  details"), "{screen}");
    // Tab again wraps to Sessions at the top, then Branches.
    let screen = after(&[Tab, Tab]);
    assert!(screen.contains("Enter  review"), "{screen}");
    let screen = after(&[Tab, Tab, Tab]);
    assert_eq!(
        sidebar_selection(&screen).as_deref(),
        Some("feature"),
        "{screen}"
    );
    assert!(screen.contains("Enter/l  switch"), "{screen}");

    // Shift-Tab from Files goes up to Branches, then Sessions.
    let screen = after(&[BackTab]);
    assert!(screen.contains("Enter/l  switch"), "{screen}");
    let screen = after(&[BackTab, BackTab]);
    assert!(screen.contains("Enter  review"), "{screen}");
}

#[test]
fn a_branch_prompt_opens_in_the_centre_and_esc_returns_to_history() {
    let repository = repository();
    let screen = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("BRANCHES (2)"),
            Char('b'),
            Char('c'),
            WaitFor("New local branch name:"),
        ],
    );
    // The sidebar stays visible beside the prompt.
    assert!(screen.contains("BRANCHES (2)"), "{screen}");
    assert!(screen.contains("WORKTREE"), "{screen}");
    assert!(!screen.contains("HISTORY"), "{screen}");

    let screen = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("BRANCHES (2)"),
            Char('b'),
            Key(Enter),
            WaitFor("Switch to local branch \"feature\"?"),
            Char('n'),
            WaitForGone("Switch to local branch"),
            Key(Esc),
            Pause(std::time::Duration::from_millis(200)),
        ],
    );
    assert!(screen.contains("HISTORY"), "{screen}");
    assert!(screen.contains("Enter/l  details"), "{screen}");
}

#[test]
fn sessions_sit_above_branches_and_a_focuses_them() {
    let repository = repository();
    let session = gitview::sessions::create(&repository.path()).unwrap();
    let screen = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("SESSIONS (1)"),
            Char('A'),
            Pause(std::time::Duration::from_millis(200)),
        ],
    );
    let sessions = screen.find("SESSIONS (1)").unwrap();
    let branches = screen.find("BRANCHES (").unwrap();
    assert!(sessions < branches, "{screen}");
    assert!(screen.contains("clean"), "{screen}");
    assert!(
        sidebar_selection(&screen)
            .unwrap()
            .starts_with(&session.name),
        "{screen}"
    );
    assert!(screen.contains("Enter  review"), "{screen}");

    // Esc hands focus back to History, and Tab passes through the list.
    let screen = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("SESSIONS (1)"),
            Char('A'),
            Key(Esc),
            Pause(std::time::Duration::from_millis(200)),
        ],
    );
    assert!(screen.contains("Enter/l  details"), "{screen}");
    let screen = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("SESSIONS (1)"),
            Char('A'),
            Key(Tab),
            Pause(std::time::Duration::from_millis(200)),
        ],
    );
    // Tab goes down to Branches.
    assert!(screen.contains("Enter/l  switch"), "{screen}");
}
