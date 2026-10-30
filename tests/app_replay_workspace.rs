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
        [WaitFor("notes.txt"), Ctrl('w'), Char('x')],
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

#[test]
fn resizing_history_changes_the_focused_panel_in_both_directions() {
    let repository = Repo::with_changes();
    let render = |resize| {
        let mut steps = vec![WaitFor("notes.txt"), Key(Tab)];
        if let Some(key) = resize {
            steps.extend([Ctrl('w'), Char(key)]);
        }
        replay_sized(&repository.path(), 140, 30, steps)
    };
    let width = |screen: &str| {
        let line = screen
            .lines()
            .find(|line| line.contains("┌HISTORY"))
            .unwrap();
        line.split("┌HISTORY")
            .nth(1)
            .unwrap()
            .split('┐')
            .next()
            .unwrap()
            .chars()
            .count()
    };
    let before = width(&render(None));
    assert!(width(&render(Some('+'))) > before);
    assert!(width(&render(Some('-'))) < before);
}

#[test]
fn fullscreen_toggle_restores_layout_and_close_hides_the_panel() {
    let repository = Repo::with_changes();
    let full = replay_sized(
        &repository.path(),
        140,
        30,
        [WaitFor("notes.txt"), Ctrl('w'), Char('o')],
    );
    assert!(full.contains("WORKTREE"));
    assert!(!full.contains("HISTORY"));
    let restored = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("notes.txt"),
            Ctrl('w'),
            Char('o'),
            Ctrl('w'),
            Char('o'),
        ],
    );
    assert!(restored.contains("WORKTREE"));
    assert!(restored.contains("HISTORY"));
    let closed = replay_sized(
        &repository.path(),
        140,
        30,
        [
            WaitFor("notes.txt"),
            Ctrl('w'),
            Char('o'),
            Ctrl('w'),
            Char('x'),
        ],
    );
    assert!(!closed.contains("WORKTREE"));
    assert!(closed.contains("HISTORY"));
}

#[test]
fn fullscreen_mode_can_show_the_agent_and_return_to_gitview() {
    let repository = Repo::with_changes();
    let screen = replay::replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
        140,
        30,
        [WaitFor("agent-ready"), Ctrl('w'), Char('o'), Ctrl('\\')],
    );
    assert!(screen.contains("agent-ready"));
    assert!(!screen.contains("WORKTREE"));
    let restored = replay::replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
        140,
        30,
        [
            WaitFor("agent-ready"),
            Ctrl('w'),
            Char('o'),
            Ctrl('\\'),
            Ctrl('\\'),
            Ctrl('w'),
            Char('o'),
        ],
    );
    assert!(restored.contains("WORKTREE"));
    assert!(restored.contains("AGENT"));
}
