//! The views follow the repository while Gitview is open: an agent (here, the
//! test) edits files, stages, commits, and switches branches, and nothing is
//! pressed to see it.

#[path = "support/replay.rs"]
mod replay;

use std::{
    cell::Cell,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};

use crossterm::event::KeyCode::{Enter, Tab};
use replay::{Repo, Step, Step::*, replay, replay_sized};

/// How long Gitview takes to start watching: the watcher starts in the
/// background and a full refresh follows when it is ready.
const WATCHER_READY: Duration = Duration::from_millis(2500);

fn outside_git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn outside_write(path: PathBuf, contents: &'static str) -> Step {
    Do(Box::new(move || {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }))
}

fn outside_git_step(root: PathBuf, args: &'static [&'static str]) -> Step {
    Do(Box::new(move || outside_git(&root, args)))
}

fn history_steps() -> Vec<Step> {
    // Wait for the commits themselves: a key pressed before they load is lost.
    vec![WaitFor("Change library"), Key(Tab)]
}

#[test]
fn a_file_created_outside_appears_in_the_working_tree_within_a_second() {
    let repository = Repo::with_history();
    let changed_at = Rc::new(Cell::new(None::<Instant>));
    let latency = Rc::new(Cell::new(None::<Duration>));
    let (start, stop) = (Rc::clone(&changed_at), Rc::clone(&latency));
    let root = repository.path();

    replay(
        &root,
        [
            WaitFor("Working tree clean"),
            Pause(WATCHER_READY),
            Do(Box::new({
                let file = root.join("made-by-agent.txt");
                move || {
                    start.set(Some(Instant::now()));
                    std::fs::write(file, "hello\n").unwrap();
                }
            })),
            WaitFor("made-by-agent.txt"),
            Do(Box::new(move || {
                stop.set(changed_at.get().map(|started| started.elapsed()));
            })),
        ],
    );

    let latency = latency.get().expect("latency was measured");
    assert!(
        latency < Duration::from_millis(1000),
        "the view took {latency:?} to follow an edit"
    );
}

#[test]
fn a_commit_made_outside_appears_in_history_and_the_selection_stays_put() {
    let repository = Repo::with_history();
    let root = repository.path();
    let mut steps = history_steps();
    steps.extend([
        Char('j'),
        WaitFor("› * "),
        Pause(WATCHER_READY),
        outside_write(root.join("agent.txt"), "from the agent\n"),
        outside_git_step(root.clone(), &["add", "-A"]),
        outside_git_step(root.clone(), &["commit", "-q", "-m", "Made by the agent"]),
        WaitFor("Made by the agent"),
    ]);

    let screen = replay(&root, steps);

    let selected = screen
        .lines()
        .find(|line| line.contains("› * "))
        .expect("a row is selected");
    assert!(
        selected.contains("Add library"),
        "the selection jumped to {selected:?}\n{screen}"
    );
}

#[test]
fn a_branch_switched_outside_updates_the_header() {
    let repository = Repo::with_history();
    let root = repository.path();

    replay(
        &root,
        [
            WaitFor("branch: main"),
            Pause(WATCHER_READY),
            outside_git_step(root.clone(), &["checkout", "-q", "-b", "agent-branch"]),
            WaitFor("branch: agent-branch"),
        ],
    );
}

#[test]
fn the_open_diff_follows_edits_and_keeps_its_place() {
    let repository = Repo::with_history();
    let root = repository.path();
    let lines = |first: &str, last: &str| {
        let mut text = format!("{first}\n");
        for number in 2..30 {
            text.push_str(&format!("line {number}\n"));
        }
        text.push_str(&format!("{last}\n"));
        text
    };
    repository.write("src/lib.rs", &lines("first edited", "last edited"));
    let rewritten = Box::leak(lines("first edited", "last REWRITTEN").into_boxed_str());

    // On a short terminal the second hunk is off screen until `]` scrolls to it.
    let screen = replay_sized(
        &root,
        140,
        9,
        [
            WaitFor("src/lib.rs"),
            Key(Enter),
            WaitFor("HUNK 1/2"),
            Char(']'),
            WaitFor("HUNK 2/2"),
            WaitFor("last edited"),
            Pause(WATCHER_READY),
            outside_write(root.join("src/lib.rs"), rewritten),
            WaitFor("last REWRITTEN"),
        ],
    );

    // Still on the second hunk, not thrown back to the first.
    assert!(screen.contains("HUNK 2/2"), "{screen}");
    assert!(!screen.contains("first edited"), "{screen}");
}

#[test]
fn an_open_diff_follows_its_file_when_it_is_unstaged_outside() {
    let repository = Repo::with_changes();
    let root = repository.path();
    // README.md is staged with "more" and then changed again in the working
    // copy, so its staged and unstaged diffs differ.
    repository.write("README.md", "# Demo\nmore\n");
    repository.git(&["add", "README.md"]);
    repository.write("README.md", "# Demo\nother\n");

    // Open the staged diff, then unstage the file from outside. The staged entry
    // disappears; the diff must move to the file's unstaged entry and show its
    // current content, not go blank and not keep showing the old staged text.
    let screen = replay(
        &root,
        [
            // README.md sorts before staged.txt, so it is the first staged entry.
            WaitFor("STAGED CHANGES (2)"),
            Key(Enter),
            WaitFor("more"),
            Pause(WATCHER_READY),
            outside_git_step(root.clone(), &["reset", "-q", "README.md"]),
            WaitFor("other"),
        ],
    );

    assert!(
        !screen.contains("more"),
        "the old staged text stayed:\n{screen}"
    );
}

#[test]
fn an_open_diff_empties_when_its_changes_are_committed_outside() {
    let repository = Repo::with_changes();
    let root = repository.path();

    let screen = replay(
        &root,
        [
            WaitFor("EDITED README.md"),
            Char('j'),
            Key(Enter),
            WaitFor("more"),
            Pause(WATCHER_READY),
            outside_git_step(root.clone(), &["add", "README.md"]),
            outside_git_step(root.clone(), &["commit", "-q", "-m", "Commit README"]),
            WaitForGone("more"),
        ],
    );

    assert!(!screen.contains("more"), "{screen}");
}

#[test]
fn a_burst_of_edits_ends_on_the_last_one() {
    let repository = Repo::with_history();
    let root = repository.path();
    repository.write("src/lib.rs", "start\n");
    let file = root.join("src/lib.rs");

    // Stale results must never overwrite newer ones: after sixty rewrites the
    // diff has to settle on the last.
    let screen = replay(
        &root,
        [
            WaitFor("src/lib.rs"),
            Key(Enter),
            WaitFor("start"),
            Pause(WATCHER_READY),
            Do(Box::new(move || {
                for index in 0..60 {
                    std::fs::write(&file, format!("revision {index}\n")).unwrap();
                    std::thread::sleep(Duration::from_millis(5));
                }
                std::fs::write(&file, "FINAL revision\n").unwrap();
            })),
            WaitFor("FINAL revision"),
        ],
    );

    assert!(!screen.contains("revision 59"), "{screen}");
}

#[test]
fn what_an_agent_in_the_pane_does_shows_up_in_the_views() {
    let repository = Repo::with_history();
    let root = repository.path();
    let script = "sleep 2.5; echo hi > by-agent.txt; git add by-agent.txt; \
                  git commit -qm 'Committed by the agent'; sleep 30";
    let mut steps = vec![WaitFor("Working tree clean")];
    steps.extend([Key(Tab), WaitFor("Committed by the agent")]);

    // 80 columns leaves Gitview one pane at a time; Tab shows history.
    let screen = replay::replay_with_agent(&root, Some(&["sh", "-c", script]), 140, 40, steps);

    assert!(screen.contains("Committed by the agent"), "{screen}");
}
