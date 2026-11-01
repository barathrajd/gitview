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

/// Five commits, newest first: Fifth beta, Fourth alpha, Third beta, Second
/// alpha, First beta.
fn alpha_beta_history() -> Repo {
    let repository = Repo::new();
    for subject in [
        "First beta",
        "Second alpha",
        "Third beta",
        "Fourth alpha",
        "Fifth beta",
    ] {
        repository.write("count.txt", &format!("{subject}\n"));
        repository.commit_all(subject);
    }
    repository
}

#[test]
fn n_and_shift_n_repeat_a_filtered_search_through_its_results() {
    let repository = alpha_beta_history();
    let search = || {
        vec![
            WaitFor("Fifth beta"),
            Key(Tab),
            Char('/'),
            Text("msg:alpha"),
            WaitFor("2 matches"),
            Key(Enter),
            WaitFor("CHANGED FILES"),
            Key(Esc),
            WaitForGone("CHANGED FILES"),
        ]
    };
    let cases: [(&[char], &str, &str); 3] = [
        (&['n'], "Second alpha", "match 2 of 2"),
        (&['n', 'n'], "Fourth alpha", "match 1 of 2"),
        (&['N'], "Second alpha", "match 2 of 2"),
    ];
    for (keys, subject, status) in cases {
        let mut steps = search();
        steps.extend(keys.iter().map(|key| Char(*key)));
        steps.push(Pause(std::time::Duration::from_millis(200)));
        let screen = replay::replay_sized(&repository.path(), 140, 20, steps);
        let selected = screen
            .lines()
            .any(|line| line.contains("› * ") && line.contains(subject));
        assert!(selected, "{keys:?} {subject}\n{screen}");
        assert!(screen.contains(status), "{keys:?} {status}\n{screen}");
    }
}

#[test]
fn filtered_results_refresh_when_the_repository_changes() {
    let repository = Repo::with_history();
    let root = repository.path();
    let mut steps = vec![
        WaitFor("Change library"),
        Key(Tab),
        Char('/'),
        Text("msg:library"),
        WaitFor("2 matches"),
        // The file watcher starts in the background, as in
        // `app_replay_refresh.rs`.
        Pause(std::time::Duration::from_millis(2500)),
    ];
    steps.push(Do(Box::new(move || {
        std::fs::write(root.join("notes.txt"), "library notes\n").unwrap();
        for args in [
            &["add", "-A"][..],
            &["commit", "-q", "-m", "Document library"],
        ] {
            let status = std::process::Command::new("git")
                .current_dir(&root)
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        }
    })));
    steps.extend([WaitFor("3 matches"), WaitFor("Document library")]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("SEARCH  msg:library_"), "{screen}");
}
