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

#[test]
fn workspace_can_hide_and_restore_the_worktree_panel() {
    let repository = Repo::with_changes();
    let hidden = replay_sized(
        &repository.path(),
        119,
        30,
        [
            WaitFor("notes.txt"),
            Ctrl('w'),
            Char('h'),
            Ctrl('w'),
            Char('x'),
        ],
    );
    assert!(hidden.contains("HISTORY"));
    assert!(!hidden.contains("WORKTREE"));
    let restored = replay_sized(
        &repository.path(),
        119,
        30,
        [
            WaitFor("notes.txt"),
            Ctrl('w'),
            Char('h'),
            Ctrl('w'),
            Char('x'),
            Ctrl('w'),
            Char('='),
        ],
    );
    assert!(restored.contains("WORKTREE"));
    assert!(restored.contains("HISTORY"));
}

#[test]
fn workspace_sidebar_width_persists_across_redraws() {
    let repository = Repo::with_changes();
    let before = replay_sized(&repository.path(), 140, 30, [WaitFor("notes.txt")]);
    let after = replay_sized(
        &repository.path(),
        140,
        30,
        [WaitFor("notes.txt"), Ctrl('w'), Char('+')],
    );
    let edge = |screen: &str| {
        screen
            .lines()
            .find(|line| line.contains("WORKTREE"))
            .unwrap()
            .find('┐')
            .unwrap()
    };
    assert!(edge(&after) > edge(&before));
}

#[test]
fn default_sidebar_keeps_ordinary_filenames_readable() {
    let repository = Repo::with_changes();
    repository.write("made-by-agent.txt", "new\n");
    let screen = replay_sized(&repository.path(), 140, 30, [WaitFor("notes.txt")]);
    assert!(screen.contains("made-by-agent.txt"));
}
