//! The control channel in the real application loop: an agent started by
//! Gitview reaches it through `gitview mcp` and the socket in GITVIEW_SOCKET.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{self, Esc};
use replay::{Repo, Step::*, replay_with_agent};

const WIDE: (u16, u16) = (160, 40);

/// An agent that sends one MCP tool call through the real `gitview mcp`.
fn agent_calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"printf '%s\n' '{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}' | '{}' mcp; sleep 30"#,
        env!("CARGO_BIN_EXE_gitview")
    )
}

fn run(repository: &Repo, script: &str, steps: impl IntoIterator<Item = replay::Step>) -> String {
    replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", script]),
        WIDE.0,
        WIDE.1,
        steps,
    )
}

#[test]
fn the_agent_is_told_where_the_private_socket_is() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        r#"ls -l "$GITVIEW_SOCKET" | cut -c1-10; sleep 30"#,
        [WaitFor("srw-------")],
    );

    assert!(screen.contains("srw-------"), "{screen}");
}

#[test]
fn the_agent_reads_status_through_gitview() {
    let repository = Repo::with_changes();

    run(
        &repository,
        &agent_calling("get_status", "{}"),
        [WaitFor("notes.txt\\\""), WaitFor("\"isError\":false")],
    );
}

#[test]
fn the_agent_opens_a_file_diff_in_gitview() {
    let repository = Repo::with_changes();

    // README.md's only change adds line 2.
    run(
        &repository,
        &agent_calling("select_file", r#"{"path":"README.md"}"#),
        [WaitFor("AFTER · HUNK"), WaitFor("2 more")],
    );
}

#[test]
fn the_agent_opens_a_commit_in_gitview() {
    let repository = Repo::with_history();

    let screen = run(
        &repository,
        &agent_calling("select_commit", r#"{"revision":"HEAD~2"}"#),
        [WaitFor("\"isError\":false"), WaitFor("Author: Test")],
    );

    assert!(screen.contains("│Add readme"), "{screen}");
}

#[test]
fn the_agent_moves_focus_to_gitview() {
    let repository = Repo::with_history();

    // The agent waits for a line typed into it, so the call can't land before
    // the agent has focus.
    let script = format!(
        "read line; {}",
        agent_calling("focus_pane", r#"{"pane":"gitview"}"#)
    );
    run(
        &repository,
        &script,
        [
            Ctrl('\\'),
            WaitFor("Ctrl+\\ returns to Gitview"),
            Text("go"),
            Key(KeyCode::Enter),
            WaitFor("Ctrl+\\ focuses the agent"),
        ],
    );
}

#[test]
fn the_agent_opens_a_file_diff_inside_a_commit() {
    let repository = Repo::with_history();
    // Two files, so the one asked for isn't the first and selected anyway.
    repository.write("README.md", "# Demo\nagain\n");
    repository.write("src/lib.rs", "first rewritten\n");
    repository.commit_all("Change both");

    run(
        &repository,
        &agent_calling(
            "select_commit",
            r#"{"revision":"HEAD","path":"src/lib.rs"}"#,
        ),
        [
            WaitFor("first rewritten"),
            Key(Esc),
            // Back on the commit, with that file selected.
            WaitFor("› src/lib.rs"),
            WaitFor("  README.md"),
        ],
    );
}
