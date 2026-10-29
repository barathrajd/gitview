//! The agent pane in the real application loop: layout, focus, forwarding,
//! and what happens when the program ends or Gitview quits.

#[path = "support/replay.rs"]
mod replay;

use std::time::{Duration, Instant};

use crossterm::event::KeyCode::Esc;
use replay::{Repo, Step::*, replay_with_agent};

const WIDE: (u16, u16) = (140, 40);

fn run(
    repository: &Repo,
    agent: &[&str],
    size: (u16, u16),
    steps: impl IntoIterator<Item = replay::Step>,
) -> String {
    replay_with_agent(&repository.path(), Some(agent), size.0, size.1, steps)
}

#[test]
fn the_agent_pane_appears_beside_gitview() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        &["sh", "-c", "echo from the agent; sleep 30"],
        WIDE,
        [WaitFor("from the agent"), WaitFor("Working tree clean")],
    );

    assert!(screen.contains("AGENT · sh · running"), "{screen}");
    assert!(screen.contains("Ctrl+\\ focuses the agent"), "{screen}");
}

#[test]
fn the_agent_is_sized_to_its_pane() {
    let repository = Repo::with_history();

    // 140 columns split in half is 70; minus the border that is 68 by 38.
    run(
        &repository,
        &["sh", "-c", "stty size; sleep 30"],
        WIDE,
        [WaitFor("38 68")],
    );
}

#[test]
fn keys_stay_with_gitview_until_focus_moves() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        &["cat"],
        WIDE,
        [
            WaitFor("Working tree clean"),
            Text("zzz"),
            Ctrl('\\'),
            WaitFor("Ctrl+\\ returns to Gitview"),
            Text("typed\r"),
            WaitFor("typed"),
        ],
    );

    assert!(!screen.contains("zzz"), "{screen}");
}

#[test]
fn while_the_agent_has_focus_gitview_keys_go_to_the_agent() {
    let repository = Repo::with_history();

    // `q` quits Gitview, so the script only finishes if the first `q` was sent
    // to the agent and the second one, after focus returned, quit.
    run(
        &repository,
        &["cat"],
        WIDE,
        [
            WaitFor("Working tree clean"),
            Ctrl('\\'),
            WaitFor("returns to Gitview"),
            Text("quit?"),
            WaitFor("quit?"),
            Key(Esc),
            Ctrl('\\'),
            WaitFor("focuses the agent"),
            Char('q'),
        ],
    );
}

#[test]
fn the_legacy_form_of_the_focus_key_also_toggles() {
    let repository = Repo::with_history();

    run(
        &repository,
        &["cat"],
        WIDE,
        [
            WaitFor("Working tree clean"),
            // A terminal without the kitty protocol reports Ctrl+\ as Ctrl+4.
            Ctrl('4'),
            WaitFor("returns to Gitview"),
            Ctrl('4'),
            WaitFor("focuses the agent"),
        ],
    );
}

#[test]
fn pasted_text_reaches_the_agent_only_while_it_has_focus() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        &["cat"],
        WIDE,
        [
            WaitFor("Working tree clean"),
            Paste("ignored paste\r"),
            Ctrl('\\'),
            WaitFor("returns to Gitview"),
            Paste("forwarded paste\r"),
            WaitFor("forwarded paste"),
        ],
    );

    assert!(!screen.contains("ignored paste"), "{screen}");
}

#[test]
fn focus_returns_to_gitview_when_the_agent_exits() {
    let repository = Repo::with_history();

    // If focus stayed on the ended agent, `?` would be dropped and the help
    // page would never open.
    run(
        &repository,
        &["sh", "-c", "sleep 0.4"],
        WIDE,
        [
            WaitFor("Working tree clean"),
            Ctrl('\\'),
            WaitFor("returns to Gitview"),
            WaitFor("exited 0"),
            Char('?'),
            WaitFor("HELP · Working tree"),
        ],
    );
}

#[test]
fn the_focus_key_does_not_enter_an_agent_that_has_ended() {
    let repository = Repo::with_history();

    run(
        &repository,
        &["sh", "-c", "exit 0"],
        WIDE,
        [
            WaitFor("exited 0"),
            Ctrl('\\'),
            Char('?'),
            WaitFor("HELP · Working tree"),
        ],
    );
}

#[test]
fn a_killed_agent_leaves_gitview_usable() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        &["sh", "-c", "echo last words; kill -9 $$"],
        WIDE,
        [
            WaitFor("ended by signal"),
            Char('?'),
            WaitFor("HELP · Working tree"),
        ],
    );

    assert!(screen.contains("last words"), "{screen}");
}

#[test]
fn a_narrow_terminal_shows_only_gitview_and_keeps_input_with_it() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        &["cat"],
        (60, 24),
        [
            WaitFor("Working tree clean"),
            // With no pane on screen the focus key must not hide input in it.
            Ctrl('\\'),
            Text("zzz"),
            Char('?'),
            WaitFor("HELP"),
        ],
    );

    assert!(!screen.contains("AGENT"), "{screen}");
}

#[test]
fn quitting_gitview_ends_the_agent() {
    let repository = Repo::with_history();
    let pid_file = repository.path().join("agent.pid");
    let script = format!(
        "echo $$ > {}; echo started; exec sleep 30",
        pid_file.display()
    );

    run(
        &repository,
        &["sh", "-c", &script],
        WIDE,
        [WaitFor("started"), Char('q')],
    );

    let pid: u32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
    {
        assert!(Instant::now() < deadline, "agent {pid} outlived Gitview");
        std::thread::sleep(Duration::from_millis(20));
    }
}
