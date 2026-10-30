//! Characterization tests for the working-tree pane, the commit composer, and
//! the narrow layout. Staging is asynchronous, so each test waits for the
//! status counts and then checks the result against Git itself.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Enter, Esc, Tab};
use replay::{Repo, Step::*, replay, replay_sized};

const START: &str = "STAGED 1   CHANGES 1   UNTRACKED 1";

#[test]
fn worktree_groups_staged_changed_and_untracked_files() {
    let repository = Repo::with_changes();

    let screen = replay(&repository.path(), [WaitFor(START)]);

    for expected in [
        "STAGED CHANGES (1)",
        "STAGED staged.txt",
        "CHANGES (2)",
        "EDITED README.md",
        "NEW notes.txt",
    ] {
        assert!(screen.contains(expected), "missing {expected}:\n{screen}");
    }
}

#[test]
fn s_stages_the_selected_changed_file() {
    let repository = Repo::with_changes();

    replay(
        &repository.path(),
        [
            WaitFor(START),
            Char('j'),
            Char('s'),
            WaitFor("STAGED 2   CHANGES 0   UNTRACKED 1"),
        ],
    );

    let status = repository.output(&["status", "--porcelain"]);
    assert!(status.contains("M  README.md"), "{status}");
    assert!(status.contains("?? notes.txt"), "{status}");
}

#[test]
fn u_unstages_the_selected_staged_file() {
    let repository = Repo::with_changes();

    replay(
        &repository.path(),
        [
            WaitFor(START),
            Char('u'),
            WaitFor("STAGED 0   CHANGES 1   UNTRACKED 2"),
        ],
    );

    let status = repository.output(&["status", "--porcelain"]);
    assert!(status.contains("?? staged.txt"), "{status}");
}

#[test]
fn shift_s_stages_everything_and_shift_u_unstages_everything() {
    let repository = Repo::with_changes();

    replay(
        &repository.path(),
        [
            WaitFor(START),
            Char('S'),
            WaitFor("STAGED 3   CHANGES 0   UNTRACKED 0"),
        ],
    );
    let status = repository.output(&["status", "--porcelain"]);
    assert!(!status.contains("??"), "{status}");

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 3"),
            Char('U'),
            WaitFor("STAGED 0   CHANGES 1   UNTRACKED 2"),
        ],
    );
    let status = repository.output(&["status", "--porcelain"]);
    assert!(!status.contains("A  "), "{status}");
}

#[test]
fn commit_composer_commits_the_staged_files_with_the_typed_subject() {
    let repository = Repo::with_changes();

    replay(
        &repository.path(),
        [
            WaitFor(START),
            Char('c'),
            WaitFor("COMMIT MESSAGE · ACTIVE"),
            Text("Add staged file"),
            Key(Enter),
            WaitFor("Add staged file - Test"),
        ],
    );

    assert_eq!(
        repository.output(&["log", "-1", "--format=%s"]).trim(),
        "Add staged file"
    );
    assert!(
        repository
            .output(&["show", "--stat", "--format=", "HEAD"])
            .contains("staged.txt")
    );
}

#[test]
fn escape_cancels_the_commit_composer_without_committing() {
    let repository = Repo::with_changes();

    replay(
        &repository.path(),
        [
            WaitFor(START),
            Char('c'),
            WaitFor("COMMIT MESSAGE · ACTIVE"),
            Text("Never committed"),
            Key(Esc),
            WaitForGone("COMMIT MESSAGE · ACTIVE"),
        ],
    );

    assert_eq!(
        repository.output(&["log", "-1", "--format=%s"]).trim(),
        "Change library"
    );
}

#[test]
fn narrow_terminals_show_one_pane_and_tab_switches_it() {
    let repository = Repo::with_changes();

    let worktree = replay_sized(&repository.path(), 60, 24, [WaitFor("STAGED CHANGES")]);
    assert!(!worktree.contains("Change library"), "{worktree}");

    let history = replay_sized(
        &repository.path(),
        60,
        24,
        [
            WaitFor("STAGED CHANGES"),
            Key(Tab),
            WaitFor("Change library"),
        ],
    );
    assert!(!history.contains("STAGED CHANGES"), "{history}");
}

#[test]
fn control_u_and_control_d_do_not_run_the_plain_git_keys() {
    let repository = Repo::with_changes();

    let screen = replay(
        &repository.path(),
        [
            WaitFor(START),
            Ctrl('u'),
            Ctrl('d'),
            Char('?'),
            WaitFor("HELP"),
        ],
    );

    assert!(!screen.contains("DISCARD"), "{screen}");
    // A misread Ctrl-u unstages on a worker thread; give it time to land.
    std::thread::sleep(std::time::Duration::from_millis(500));
    let status = repository.output(&["status", "--porcelain"]);
    assert!(status.contains("A  staged.txt"), "{status}");
}

#[test]
fn zz_scrolls_the_selected_file_to_the_middle() {
    let repository = Repo::with_history();
    for number in 1..=40 {
        repository.write(&format!("file-{number:02}.txt"), "new\n");
    }
    let extra = repository.path().join("file-41.txt");

    let screen = replay_sized(
        &repository.path(),
        140,
        20,
        [
            WaitFor("UNTRACKED 40"),
            Char('2'),
            Char('0'),
            Char('j'),
            Char('z'),
            Char('z'),
            // Help and a refresh keep the centered scroll position.
            Char('?'),
            WaitFor("HELP"),
            Char('?'),
            WaitForGone("HELP"),
            Do(Box::new(move || std::fs::write(extra, "new\n").unwrap())),
            Char('r'),
            WaitFor("UNTRACKED 41"),
        ],
    );

    let first = screen.find("file-").map(|index| &screen[index..index + 7]);
    assert_eq!(first, Some("file-15"), "{screen}");
    assert!(
        screen.contains("› ") && screen.contains("file-21"),
        "{screen}"
    );
}
