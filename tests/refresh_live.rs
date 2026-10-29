#![cfg(all(unix, feature = "test-support"))]
//! How often the real binary runs Git, idle and after an edit. A `git` shim on
//! `PATH` logs every invocation, so polling shows up as a count.

#[path = "support/live.rs"]
mod live;

use std::{
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use live::Session;

/// A folder holding a `git` that records its arguments and runs the real one,
/// and the search path that finds it first.
fn counting_git() -> (tempfile::TempDir, OsString, std::path::PathBuf) {
    let real = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    let real = String::from_utf8(real.stdout).unwrap().trim().to_owned();
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("git.log");
    let shim = dir.path().join("git");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{real}' \"$@\"\n",
            log.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    let mut folders = vec![dir.path().to_path_buf()];
    folders.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    (dir, std::env::join_paths(folders).unwrap(), log)
}

fn status_runs(log: &Path) -> usize {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("status"))
        .count()
}

#[test]
fn an_idle_gitview_does_not_poll_git() {
    let (_shim, path, log) = counting_git();
    let session = Session::start_with(&[], Some(&path));
    session.wait_for("Working tree clean");

    // Let start-up and the watcher's first refresh finish, then watch idle time.
    thread::sleep(Duration::from_secs(3));
    let settled = status_runs(&log);
    thread::sleep(Duration::from_secs(5));
    let idle = status_runs(&log) - settled;

    // Polling every 750 ms would run `git status` about seven times here.
    assert_eq!(idle, 0, "idle Gitview ran `git status` {idle} times in 5 s");
    session.finish();
}

#[test]
fn an_edit_shows_up_promptly_and_costs_a_few_git_runs_not_a_stream() {
    let (_shim, path, log) = counting_git();
    let session = Session::start_with(&[], Some(&path));
    session.wait_for("Working tree clean");
    thread::sleep(Duration::from_secs(3));
    let before = status_runs(&log);

    let started = Instant::now();
    fs::write(session.repository.join("made-outside.txt"), "hi\n").unwrap();
    session.wait_for("made-outside.txt");
    let latency = started.elapsed();
    thread::sleep(Duration::from_secs(2));
    let runs = status_runs(&log) - before;

    assert!(latency < Duration::from_millis(1500), "took {latency:?}");
    assert!(
        (1..=3).contains(&runs),
        "{runs} `git status` runs for one edit"
    );
    session.finish();
}

#[test]
fn without_the_watcher_gitview_polls_and_still_follows_edits() {
    let (_shim, path, log) = counting_git();
    let session = Session::start_full(&[], Some(&path), &[("GITVIEW_NO_WATCH", "1".as_ref())]);
    session.wait_for("Working tree clean");
    thread::sleep(Duration::from_secs(1));
    let settled = status_runs(&log);
    thread::sleep(Duration::from_secs(4));
    let polled = status_runs(&log) - settled;
    assert!(polled >= 3, "expected polling, saw {polled} runs in 4 s");

    fs::write(session.repository.join("found-by-polling.txt"), "hi\n").unwrap();
    session.wait_for("found-by-polling.txt");
    session.finish();
}
