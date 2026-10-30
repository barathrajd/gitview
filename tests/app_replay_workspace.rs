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

#[test]
fn agent_can_toggle_its_own_fullscreen_and_restore_the_workspace() {
    let repository = Repo::with_changes();
    let render = |restore| {
        let mut steps = vec![WaitFor("agent-ready"), Ctrl('\\'), Ctrl('w'), Char('o')];
        if restore {
            steps.extend([Ctrl('w'), Char('o')]);
        }
        replay::replay_with_agent(
            &repository.path(),
            Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
            140,
            30,
            steps,
        )
    };
    let full = render(false);
    assert!(full.contains("AGENT"));
    assert!(!full.contains("WORKTREE"));
    let restored = render(true);
    assert!(restored.contains("WORKTREE"));
    assert!(restored.contains("AGENT"));
}

#[test]
fn other_ctrl_w_sequences_still_reach_the_agent() {
    let repository = Repo::with_changes();
    replay::replay_with_agent(
        &repository.path(),
        Some(&[
            "sh",
            "-c",
            "stty raw -echo; printf raw-ready; od -An -tx1 -N2 | tr -s ' '; sleep 30",
        ]),
        140,
        30,
        [
            WaitFor("raw-ready"),
            Ctrl('\\'),
            Ctrl('w'),
            Char('z'),
            WaitFor("17 7a"),
        ],
    );
}

#[test]
fn agent_fullscreen_also_works_when_control_is_held_for_both_keys() {
    let repository = Repo::with_changes();
    let screen = replay::replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
        140,
        30,
        [WaitFor("agent-ready"), Ctrl('\\'), Ctrl('w'), Ctrl('o')],
    );
    assert!(screen.contains("AGENT"));
    assert!(!screen.contains("WORKTREE"));
}

#[test]
fn focused_agent_resizes_in_both_directions() {
    let repository = Repo::with_changes();
    let render = |resize| {
        let mut steps = vec![WaitFor("agent-ready"), Ctrl('\\')];
        if let Some(key) = resize {
            steps.extend([Ctrl('w'), Char(key)]);
        }
        replay::replay_with_agent(
            &repository.path(),
            Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
            140,
            30,
            steps,
        )
    };
    let start = |screen: &str| {
        screen
            .lines()
            .find(|line| line.contains("┌ AGENT"))
            .unwrap()
            .split("┌ AGENT")
            .next()
            .unwrap()
            .chars()
            .count()
    };
    let before = start(&render(None));
    assert!(start(&render(Some('+'))) < before);
    assert!(start(&render(Some('-'))) > before);
}

#[test]
fn agent_can_hide_and_restore_without_ending_the_shell() {
    let repository = Repo::with_changes();
    let render = |restore| {
        let mut steps = vec![
            WaitFor("agent-ready"),
            Ctrl('\\'),
            Ctrl('w'),
            Char('o'),
            Ctrl('w'),
            Char('x'),
        ];
        if restore {
            steps.extend([Ctrl('w'), Char('='), Ctrl('\\')]);
        }
        replay::replay_with_agent(
            &repository.path(),
            Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
            140,
            30,
            steps,
        )
    };
    let hidden = render(false);
    assert!(!hidden.contains("AGENT"));
    assert!(hidden.contains("WORKTREE") || hidden.contains("HISTORY"));
    let restored = render(true);
    assert!(restored.contains("agent-ready"));
    assert!(restored.contains("WORKTREE") || restored.contains("HISTORY"));
    assert!(restored.contains("running"));
}

#[test]
fn agent_workspace_shows_only_the_selected_gitview_pane() {
    let repository = Repo::with_changes();
    for width in [119, 140, 300] {
        let screen = replay::replay_with_agent(
            &repository.path(),
            Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
            width,
            30,
            [WaitFor("agent-ready"), WaitFor("notes.txt")],
        );
        assert!(screen.contains("WORKTREE"), "{width}: {screen}");
        assert!(!screen.contains("HISTORY"), "{width}: {screen}");
        assert!(screen.contains("AGENT"), "{width}: {screen}");
        let history = replay::replay_with_agent(
            &repository.path(),
            Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
            width,
            30,
            [WaitFor("agent-ready"), Key(Tab)],
        );
        assert!(history.contains("HISTORY"), "{width}: {history}");
        assert!(!history.contains("WORKTREE"), "{width}: {history}");
    }
}

#[test]
fn compact_agent_workspace_keeps_common_filenames_readable() {
    let repository = Repo::with_changes();
    repository.write("made-by-agent.txt", "new\n");
    let screen = replay::replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
        140,
        30,
        [WaitFor("agent-ready"), WaitFor("made-by-agent.txt")],
    );
    assert!(!screen.contains("HISTORY"));
}

#[test]
fn agent_receives_mouse_coordinates_inside_its_terminal() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let repository = Repo::with_changes();
    replay::replay_with_agent(
        &repository.path(),
        Some(&[
            "sh",
            "-c",
            r"stty raw -echo; printf '\033[?1000h\033[?1006hmouse-ready'; od -An -tx1 -N9 | tr -s ' '; sleep 30",
        ]),
        140,
        30,
        [
            WaitFor("mouse-ready"),
            Ctrl('\\'),
            Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 73,
                row: 3,
                modifiers: KeyModifiers::NONE,
            }),
            WaitFor("1b 5b 3c 30 3b 33 3b 33 4d"),
        ],
    );
}

#[test]
fn hiding_both_gitview_panels_leaves_the_agent_full_width() {
    let repository = Repo::with_changes();
    let screen = replay::replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", "echo agent-ready; sleep 30"]),
        140,
        30,
        [
            WaitFor("agent-ready"),
            Ctrl('w'),
            Char('x'),
            Ctrl('w'),
            Char('x'),
        ],
    );
    assert!(screen.contains("AGENT"));
    assert!(!screen.contains("WORKTREE"));
    assert!(!screen.contains("HISTORY"));
}

#[test]
fn wheel_scrolls_an_alternate_screen_without_moving_its_input_cursor() {
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};
    let repository = Repo::with_changes();
    replay::replay_with_agent(
        &repository.path(),
        Some(&[
            "sh",
            "-c",
            r"stty raw -echo; printf '\033[?1049hwheel-ready'; od -An -tx1 -N4 | tr -s ' '; sleep 30",
        ]),
        140,
        30,
        [
            WaitFor("wheel-ready"),
            Ctrl('\\'),
            Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 73,
                row: 3,
                modifiers: KeyModifiers::NONE,
            }),
            WaitFor("1b 5b 35 7e"),
        ],
    );
}
