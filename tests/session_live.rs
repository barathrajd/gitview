#![cfg(all(unix, feature = "test-support"))]
//! `gitview --session` in a real pseudo-terminal: the agent and Gitview both
//! work in a new session worktree, which outlives Gitview.

#[path = "support/live.rs"]
mod live;

use live::Session;

#[test]
fn session_runs_the_agent_and_gitview_in_a_new_worktree() {
    let mut terminal = Session::start_with(
        &[
            "--session",
            "--run",
            "sh",
            "-c",
            "echo on $(git symbolic-ref --short HEAD); echo x > made-by-agent.txt; sleep 30",
        ],
        None,
    );

    terminal.wait_for("on gitview/session-");
    // Gitview shows the session's worktree, where the agent made a file.
    terminal.wait_for("made-by-agent.txt");
    terminal.send(b"q");
    assert!(terminal.wait_for_exit());
    terminal.assert_terminal_restored();

    let listed = gitview::sessions::list(&terminal.repository).unwrap();
    assert_eq!(listed.len(), 1, "the session outlives Gitview");
    assert_eq!(listed[0].session.base_branch, "main");
    assert_eq!(listed[0].health, gitview::sessions::Health::Ready);
    assert!(listed[0].dirty);
    assert!(!terminal.repository.join("made-by-agent.txt").exists());
    terminal.finish();
}
