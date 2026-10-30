#![cfg(all(unix, feature = "test-support"))]
//! The real binary with an agent pane, in a real pseudo-terminal: what reaches
//! the program, and that the terminal is restored however the program ends.

#[path = "support/live.rs"]
mod live;

use std::{fs, process::Command};

use live::{Session, read_pid, wait_until_dead};

#[test]
fn quitting_restores_the_terminal_and_ends_the_agent() {
    let pid_file = std::env::temp_dir().join(format!("gitview-live-quit-{}", std::process::id()));
    let script = format!(
        "echo $$ > {}; echo agent up; exec sleep 60",
        pid_file.display()
    );
    let mut session = Session::start(&["sh", "-c", &script]);
    session.wait_for("agent up");
    let pid = read_pid(&pid_file);

    session.send(b"q");

    assert!(session.wait_for_exit(), "gitview should exit successfully");
    session.assert_terminal_restored();
    wait_until_dead(pid);
    session.finish();
    let _ = fs::remove_file(pid_file);
}

#[test]
fn an_agent_that_exits_with_an_error_leaves_gitview_running() {
    let mut session = Session::start(&["sh", "-c", "echo boom; exit 3"]);
    session.wait_for("exited 3");

    // Gitview still answers: `q` quits it and the terminal is restored.
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}

#[test]
fn an_agent_killed_from_outside_is_reported_and_the_terminal_stays_intact() {
    let pid_file = std::env::temp_dir().join(format!("gitview-live-kill-{}", std::process::id()));
    let script = format!(
        "echo $$ > {}; echo agent up; exec sleep 60",
        pid_file.display()
    );
    let mut session = Session::start(&["sh", "-c", &script]);
    session.wait_for("agent up");
    let pid = read_pid(&pid_file);

    Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .unwrap();
    session.wait_for("ended by signal");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
    let _ = fs::remove_file(pid_file);
}

#[test]
fn the_focus_key_and_arrow_keys_cross_a_real_terminal_unchanged() {
    // `cat -v` prints control bytes as ^X, so the screen shows what arrived.
    let mut session = Session::start(&["sh", "-c", "stty raw -echo; echo cat ready; cat -v"]);
    session.wait_for("cat ready");

    // Byte 0x1c is Ctrl+\ on a terminal without the kitty keyboard protocol.
    session.send(&[0x1c]);
    session.wait_for("returns to Gitview");
    session.send(b"\x1b[A\x03\x1bb");
    session.wait_for("^[[A^C^[b");
    session.send(&[0x1c]);
    session.wait_for("focuses the agent");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}

#[test]
fn closing_the_terminal_ends_gitview_and_the_agent() {
    let pid_file = std::env::temp_dir().join(format!("gitview-live-hup-{}", std::process::id()));
    let script = format!(
        "echo $$ > {}; echo agent up; exec sleep 60",
        pid_file.display()
    );
    let mut session = Session::start(&["sh", "-c", &script]);
    session.wait_for("agent up");
    let pid = read_pid(&pid_file);

    session.close_terminal();

    // The hang-up (SIGHUP) ends Gitview, so it does not exit cleanly.
    assert!(!session.wait_for_exit());
    wait_until_dead(pid);
    session.finish();
    let _ = fs::remove_file(pid_file);
}

/// A folder holding an executable `name` that prints its arguments and waits,
/// followed by the real search path so `git` is still found.
fn path_with_fake(name: &str) -> (tempfile::TempDir, std::ffi::OsString) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join(name);
    fs::write(
        &program,
        // Each argument also gets its own line, so a check doesn't depend on
        // where a long line wraps.
        format!(
            "#!/bin/sh\necho \"fake {name} args: $*\"\nprintf 'arg: %s\\n' \"$@\"\nexec sleep 60\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let mut folders = vec![dir.path().to_path_buf()];
    folders.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    (dir, std::env::join_paths(folders).unwrap())
}

#[test]
fn claude_runs_the_claude_program_with_the_gitview_mcp_server_and_the_rest_of_the_line() {
    let (_dir, path) = path_with_fake("claude");
    let mut session =
        Session::start_with(&["--claude", "--resume", "--model", "opus"], Some(&path));

    // Gitview's MCP config comes first, then the user's own arguments.
    session.wait_for("fake claude args: --mcp-config={\"mcpServers\"");
    session.wait_for("arg: --resume");
    session.wait_for("arg: --model");
    session.wait_for("arg: opus");
    session.wait_for("AGENT · claude · running");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}

#[test]
fn codex_runs_the_codex_program() {
    let (_dir, path) = path_with_fake("codex");
    let mut session = Session::start_with(&["--codex"], Some(&path));

    session.wait_for("fake codex args:");
    session.wait_for("AGENT · codex · running");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.finish();
}

#[test]
fn a_missing_program_stops_before_the_terminal_is_touched() {
    let empty = tempfile::tempdir().unwrap();
    let mut session = Session::start_with(&["--claude"], Some(empty.path().as_os_str()));

    let succeeded = session.wait_for_exit();

    assert!(!succeeded);
    let text = session.text();
    assert!(
        text.contains("Claude Code (`claude`) was not found on your PATH"),
        "{text:?}"
    );
    assert!(text.contains("gitview --run <command>"), "{text:?}");
    assert!(
        !text.contains("\x1b[?1049h"),
        "entered the alternate screen: {text:?}"
    );
    assert!(
        !text.contains("\x1b[?2004h"),
        "enabled bracketed paste: {text:?}"
    );
    session.finish();
}

#[test]
fn resizing_the_terminal_moves_between_side_by_side_and_narrow_layouts() {
    // Prints its size at start and again whenever it is told the size changed.
    let script = "trap 'stty size' WINCH; stty size; while :; do sleep 0.05; done";
    let mut session = Session::start(&["sh", "-c", script]);

    // Wide: side by side. 140 columns split in half is 70, minus the border.
    session.wait_for("38 68");
    session.wait_for("AGENT · sh · running");
    assert!(session.screen().contains("Ctrl+\\ focuses the agent"));

    // Narrow: Gitview in front, the agent reduced to a bar and not resized.
    session.resize(80, 24);
    session.wait_for("Ctrl+\\ opens the agent");
    assert!(!session.screen().contains("focuses the agent"));

    // The focus key brings the agent forward, and it learns its new size.
    session.send(&[0x1c]);
    session.wait_for("22 78");
    session.wait_for("returns to Gitview");

    // Wide again: side by side, and the agent is told once more.
    session.resize(140, 40);
    session.wait_for("38 68");
    session.wait_for("returns to Gitview");
    session.send(&[0x1c]);
    session.wait_for("focuses the agent");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}
