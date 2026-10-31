//! Git-side search filters, driven through the app by scripted keys.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Enter, Esc, Tab};
use replay::{Repo, Step::*, replay};

fn history_steps() -> Vec<replay::Step> {
    vec![WaitFor("Change library"), Key(Tab)]
}

#[test]
fn a_filter_typed_inline_opens_the_search_page() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('/'),
        Text("msg:readme"),
        WaitFor("SEARCH  msg:readme_"),
        WaitFor("1 matches"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Add readme"), "{screen}");
    assert!(!screen.contains("Add library"), "{screen}");
}

#[test]
fn free_text_ranks_within_filtered_commits_and_enter_opens_one() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('/'),
        Text("msg:library change"),
        WaitFor("SEARCH  msg:library change_"),
        WaitFor("1 matches"),
        Key(Enter),
        WaitFor("CHANGED FILES"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Change library"), "{screen}");
}

#[test]
fn an_unknown_ref_shows_the_git_error() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('/'),
        Text("ref:nosuch"),
        WaitFor("Unable to search history"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("nosuch"), "{screen}");
}

#[test]
fn editing_the_filter_reruns_the_search() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([
        Char('/'),
        Text("msg:readme"),
        WaitFor("1 matches"),
        Key(crossterm::event::KeyCode::Backspace),
        Key(crossterm::event::KeyCode::Backspace),
        Key(crossterm::event::KeyCode::Backspace),
        Key(crossterm::event::KeyCode::Backspace),
        Key(crossterm::event::KeyCode::Backspace),
        Key(crossterm::event::KeyCode::Backspace),
        Text("library"),
        WaitFor("SEARCH  msg:library_"),
        WaitFor("2 matches"),
        Key(Esc),
        WaitFor("HISTORY"),
    ]);

    replay(&repository.path(), steps);
}

#[test]
fn typing_a_filter_key_opens_the_search_page_before_its_value() {
    let repository = Repo::with_history();
    let mut steps = history_steps();
    steps.extend([Char('/'), Text("path:"), WaitFor("SEARCH  path:_")]);

    let screen = replay(&repository.path(), steps);

    assert!(!screen.contains("no match"), "{screen}");
    assert!(screen.contains("Add readme"), "{screen}");
}
