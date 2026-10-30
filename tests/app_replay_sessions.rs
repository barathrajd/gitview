//! The Sessions page in the real application loop: list, review, merge, and
//! discard agent sessions.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Enter, Esc};
use replay::{Repo, Step::*, replay};

/// A repository with one session whose agent committed `agent.txt`.
fn with_session() -> (Repo, gitview::sessions::Session) {
    let repository = Repo::with_history();
    let session = gitview::sessions::create(&repository.path()).unwrap();
    std::fs::write(session.path.join("agent.txt"), "from the agent\n").unwrap();
    for args in [
        &["add", "agent.txt"][..],
        &[
            "-c",
            "user.name=T",
            "-c",
            "user.email=t@e",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "Agent adds a file",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .current_dir(&session.path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    (repository, session)
}

#[test]
fn the_page_lists_sessions_with_their_state() {
    let (repository, session) = with_session();

    let screen = replay(
        &repository.path(),
        [
            Char('A'),
            WaitFor("AGENT SESSIONS"),
            WaitFor(session.name.clone().leak()),
        ],
    );

    assert!(screen.contains("1 commit ahead of main"), "{screen}");
}

#[test]
fn review_then_merge_brings_the_work_into_main() {
    let (repository, session) = with_session();

    replay(
        &repository.path(),
        [
            Char('A'),
            WaitFor(session.name.clone().leak()),
            Key(Enter),
            WaitFor("Agent adds a file"),
            WaitFor("+from the agent"),
            Char('m'),
            WaitFor("Merge"),
            Char('y'),
            WaitFor("Merged"),
            Key(Esc),
        ],
    );

    let subject = repository.output(&["log", "-1", "--format=%s"]);
    assert!(subject.contains(&session.name), "{subject}");
}

#[test]
fn discard_shows_what_is_lost_and_removes_the_session() {
    let (repository, session) = with_session();

    replay(
        &repository.path(),
        [
            Char('A'),
            WaitFor(session.name.clone().leak()),
            Char('d'),
            WaitFor("Agent adds a file"),
            WaitFor("will be lost"),
            Char('y'),
            WaitFor("Discarded"),
            WaitFor("No agent sessions"),
        ],
    );

    assert!(!session.path.exists());
    assert!(
        gitview::sessions::list(&repository.path())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_refused_merge_explains_why() {
    let (repository, session) = with_session();
    repository.write("README.md", "# Edited by the user\n");

    replay(
        &repository.path(),
        [
            Char('A'),
            WaitFor(session.name.clone().leak()),
            Key(Enter),
            WaitFor("Agent adds a file"),
            Char('m'),
            Char('y'),
            WaitFor("uncommitted changes"),
        ],
    );

    assert!(
        !repository
            .output(&["log", "-1", "--format=%s"])
            .contains(&session.name)
    );
}
