//! Characterization tests for the application loop: history, search, diff, and
//! help, driven by scripted keys. They describe how the app behaves today so
//! later refactors can be checked against it.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Backspace, Enter, Esc, Tab};
use replay::{Repo, Step::*, replay, replay_sized};

/// True when the history list marks the row for `subject` as selected.
fn history_selects(screen: &str, subject: &str) -> bool {
    screen
        .lines()
        .any(|line| line.contains("› * ") && line.contains(subject))
}

/// Opens the History pane (the app starts on the working tree).
fn history_steps() -> Vec<replay::Step> {
    vec![WaitFor("Change library"), Key(Tab)]
}

/// History pane, then the "Change library" commit's `src/lib.rs` diff.
fn diff_steps() -> Vec<replay::Step> {
    let mut steps = history_steps();
    steps.extend([
        Key(Enter),
        WaitFor("CHANGED FILES"),
        Key(Enter),
        WaitFor("first changed"),
    ]);
    steps
}

#[test]
fn help_opens_and_closes_from_scripted_keys() {
    let repository = Repo::with_history();

    replay(
        &repository.path(),
        [
            WaitFor("Change library"),
            Char('?'),
            WaitFor("HELP · Working tree"),
            Key(Esc),
            WaitForGone("HELP · Working tree"),
        ],
    );
}

#[test]
fn help_page_follows_the_current_view_and_question_mark_toggles_it() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('?'),
        WaitFor("HELP · History"),
        Char('?'),
        WaitForGone("HELP · History"),
        Key(Enter),
        WaitFor("CHANGED FILES"),
        Key(Enter),
        WaitFor("first changed"),
        Char('?'),
        WaitFor("HELP · Commit diff"),
        Key(Esc),
        WaitForGone("HELP · Commit diff"),
    ]);

    replay(&repository.path(), steps);
}

#[test]
fn quit_key_ends_the_session() {
    let repository = Repo::with_history();

    // `replay` fails if keys remain unread, so reaching the end proves `q`
    // returned from the loop.
    replay(&repository.path(), [WaitFor("Change library"), Char('q')]);
}

#[test]
fn history_selection_moves_with_j_and_k() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([Char('j'), Char('j')]);
    let screen = replay(&repository.path(), steps);
    assert!(history_selects(&screen, "Add readme"), "{screen}");

    let mut steps = history_steps();
    steps.extend([Char('j'), Char('j'), Char('k')]);
    let screen = replay(&repository.path(), steps);
    assert!(history_selects(&screen, "Add library"), "{screen}");
}

#[test]
fn enter_opens_commit_details_and_escape_returns_to_history() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('j'),
        Key(Enter),
        WaitFor("CHANGED FILES"),
        Key(Esc),
        WaitForGone("CHANGED FILES"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(history_selects(&screen, "Add library"), "{screen}");
}

#[test]
fn commit_details_list_the_changed_files() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Key(Enter),
        WaitFor("CHANGED FILES"),
        WaitFor("› src/lib.rs"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Change library"), "{screen}");
    assert!(screen.contains("Author: Test"), "{screen}");
}

#[test]
fn search_narrows_history_and_enter_opens_the_match() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('/'),
        Text("readme"),
        WaitForGone("Add library"),
        WaitFor("1 matches"),
        Key(Enter),
        WaitFor("CHANGED FILES"),
        WaitFor("› README.md"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Add readme"), "{screen}");
}

#[test]
fn escape_leaves_search_with_the_match_selected() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('/'),
        Text("readme"),
        WaitForGone("Add library"),
        Key(Enter),
        WaitFor("CHANGED FILES"),
        Key(Esc),
        WaitForGone("CHANGED FILES"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(history_selects(&screen, "Add readme"), "{screen}");
    assert!(screen.contains("Add library"), "{screen}");
}

#[test]
fn path_filter_limits_history_to_commits_touching_the_path() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('f'),
        Text("README.md"),
        Key(Enter),
        WaitFor("History path: README.md"),
        WaitForGone("Loading history"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Add readme"), "{screen}");
    assert!(!screen.contains("Add library"), "{screen}");
    assert!(!screen.contains("Change library"), "{screen}");
}

#[test]
fn path_filter_clears_when_submitted_empty() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('f'),
        Text("README.md"),
        Key(Enter),
        WaitFor("History path: README.md"),
        WaitForGone("Loading history"),
        Char('f'),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Backspace),
        Key(Enter),
        WaitFor("Add library"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Add readme"), "{screen}");
    assert!(screen.contains("Change library"), "{screen}");
}

#[test]
fn commit_diff_starts_side_by_side() {
    let repository = Repo::with_history();

    let screen = replay(&repository.path(), diff_steps());

    assert!(screen.contains("BEFORE"), "{screen}");
    assert!(screen.contains("AFTER"), "{screen}");
    assert!(screen.contains("1 first original"), "{screen}");
    assert!(screen.contains("1 first changed"), "{screen}");
}

#[test]
fn tab_switches_the_commit_diff_to_unified() {
    let repository = Repo::with_history();
    let mut steps = diff_steps();
    steps.extend([Key(Tab), WaitFor("UNIFIED IDE DIFF")]);

    let screen = replay(&repository.path(), steps);

    assert!(!screen.contains("BEFORE"), "{screen}");
    assert!(screen.contains("Tab  side-by-side"), "{screen}");
}

#[test]
fn v_switches_the_commit_diff_to_the_raw_patch() {
    let repository = Repo::with_history();
    let mut steps = diff_steps();
    steps.extend([Char('v'), WaitFor("v  IDE diff")]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("1 -first original"), "{screen}");
    assert!(screen.contains("1 +first changed"), "{screen}");
}

#[test]
fn escape_closes_the_diff_and_then_the_commit_details() {
    let repository = Repo::with_history();
    let mut steps = diff_steps();
    steps.extend([
        Key(Esc),
        WaitForGone("first changed"),
        WaitFor("CHANGED FILES"),
        Key(Esc),
        WaitForGone("CHANGED FILES"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(history_selects(&screen, "Change library"), "{screen}");
}

#[test]
fn hunk_keys_scroll_a_working_tree_diff_to_each_change() {
    let repository = Repo::with_history();
    let lines = |first: &str, last: &str| {
        let mut text = format!("{first}\n");
        for number in 2..30 {
            text.push_str(&format!("line {number}\n"));
        }
        text.push_str(&format!("{last}\n"));
        text
    };
    repository.write("src/lib.rs", &lines("first edited", "last edited"));

    // On a short terminal the second hunk is off screen until `]` scrolls to it.
    let screen = replay_sized(
        &repository.path(),
        140,
        9,
        [
            WaitFor("src/lib.rs"),
            Key(Enter),
            WaitFor("HUNK 1/2"),
            WaitFor("first edited"),
            Char(']'),
            WaitFor("HUNK 2/2"),
            WaitFor("last edited"),
            Char('['),
            WaitFor("HUNK 1/2"),
            WaitFor("first edited"),
        ],
    );

    assert!(!screen.contains("last edited"), "{screen}");
}
