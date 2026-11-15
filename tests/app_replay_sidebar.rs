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
fn tab_and_shift_tab_cycle_files_branches_and_history() {
    let repository = repository();
    let start = || vec![WaitFor("BRANCHES (2)"), WaitFor("staged.txt")];

    // Files first: the worktree's row is marked.
    let screen = replay_sized(&repository.path(), 140, 30, start());
    assert!(
        sidebar_selection(&screen).unwrap().contains("staged.txt"),
        "{screen}"
    );

    // Tab: Branches, whose row is now the sidebar's only mark.
    let mut steps = start();
    steps.push(Key(Tab));
    steps.push(Pause(std::time::Duration::from_millis(200)));
    let screen = replay_sized(&repository.path(), 140, 30, steps);
    assert_eq!(
        sidebar_selection(&screen).as_deref(),
        Some("feature"),
        "{screen}"
    );
    assert!(screen.contains("Enter/l  switch"), "{screen}");

    // Tab again: History.
    let mut steps = start();
    steps.extend([
        Key(Tab),
        Key(Tab),
        Pause(std::time::Duration::from_millis(200)),
    ]);
    let screen = replay_sized(&repository.path(), 140, 30, steps);
    assert!(screen.contains("Enter/l  details"), "{screen}");

    // Shift-Tab from Files goes straight to History.
    let mut steps = start();
    steps.extend([Key(BackTab), Pause(std::time::Duration::from_millis(200))]);
    let screen = replay_sized(&repository.path(), 140, 30, steps);
    assert!(screen.contains("Enter/l  details"), "{screen}");
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
