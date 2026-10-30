//! Workspace shell rendering through the real application loop.
#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::Tab;
use replay::{Repo, Step::*, replay_sized};

#[test]
fn narrow_workspace_shows_only_the_focused_panel() {
    let repository = Repo::with_changes();
    let screen = replay_sized(&repository.path(), 118, 30, [WaitFor("notes.txt")]);
    assert!(screen.contains("WORKTREE"));
    assert!(!screen.contains("HISTORY"));
    let screen = replay_sized(
        &repository.path(),
        118,
        30,
        [WaitFor("notes.txt"), Key(Tab)],
    );
    assert!(screen.contains("HISTORY"));
    assert!(!screen.contains("WORKTREE"));
}

#[test]
fn wide_workspace_shows_worktree_and_history() {
    let repository = Repo::with_changes();
    let screen = replay_sized(&repository.path(), 119, 30, [WaitFor("notes.txt")]);
    assert!(screen.contains("WORKTREE"));
    assert!(screen.contains("HISTORY"));
}
