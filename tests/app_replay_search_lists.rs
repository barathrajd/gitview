//! Inline `/` search on lists other than History: it jumps to matches and
//! highlights them, `n`/`N` step through them, and every row stays visible.

#[path = "support/replay.rs"]
mod replay;

use std::time::Duration;

use crossterm::event::KeyCode::{Enter, Esc};
use replay::{Repo, Step::*, replay_sized};

/// The path on the row the worktree panel (the left column) marks as
/// selected.
fn selected_row(screen: &str) -> Option<String> {
    screen.lines().find_map(|line| {
        let panel = line.strip_prefix("│›")?;
        let panel = panel.split('│').next()?;
        panel.split_whitespace().last().map(str::to_owned)
    })
}

/// A staged `z-alpha.txt`, shown first under STAGED CHANGES, then new files
/// `a-alpha.txt`, `b-beta.txt`, `c-alpha.txt`, `d-beta.txt` under CHANGES.
fn worktree() -> Repo {
    let repository = Repo::with_history();
    repository.write("z-alpha.txt", "staged\n");
    repository.git(&["add", "z-alpha.txt"]);
    for name in ["a-alpha.txt", "b-beta.txt", "c-alpha.txt", "d-beta.txt"] {
        repository.write(name, "new\n");
    }
    repository
}

fn worktree_screen(repository: &Repo, keys: Vec<replay::Step>) -> String {
    let mut steps = vec![WaitFor("d-beta.txt")];
    steps.extend(keys);
    steps.push(Pause(Duration::from_millis(200)));
    replay_sized(&repository.path(), 140, 24, steps)
}

#[test]
fn slash_jumps_through_worktree_files_and_esc_restores_the_selection() {
    let repository = worktree();
    // Start on b-beta.txt, the third file shown.
    let start = || vec![Text("gg2j"), Char('/'), Text("alpha")];

    // The first match at or below the start, in display order.
    let screen = worktree_screen(&repository, start());
    assert_eq!(
        selected_row(&screen).as_deref(),
        Some("c-alpha.txt"),
        "{screen}"
    );
    assert!(screen.contains("/alpha_"), "{screen}");
    assert!(screen.contains("match 3 of 3"), "{screen}");
    // Rows that don't match stay visible.
    assert!(screen.contains("d-beta.txt"), "{screen}");

    let mut keys = start();
    keys.push(Key(Esc));
    let screen = worktree_screen(&repository, keys);
    assert_eq!(
        selected_row(&screen).as_deref(),
        Some("b-beta.txt"),
        "{screen}"
    );
    assert!(!screen.contains("/alpha_"), "{screen}");

    // Enter keeps the match without opening its diff.
    let mut keys = start();
    keys.push(Key(Enter));
    let screen = worktree_screen(&repository, keys);
    assert_eq!(
        selected_row(&screen).as_deref(),
        Some("c-alpha.txt"),
        "{screen}"
    );
    assert!(!screen.contains("/alpha_"), "{screen}");

    let mut keys = start();
    keys.push(Text("zzz"));
    let screen = worktree_screen(&repository, keys);
    assert!(screen.contains("no match"), "{screen}");
    assert_eq!(
        selected_row(&screen).as_deref(),
        Some("b-beta.txt"),
        "{screen}"
    );
}

#[test]
fn n_and_shift_n_step_through_worktree_matches_in_display_order() {
    let repository = worktree();
    let cases: [(&str, &str); 4] = [
        ("", "z-alpha.txt"),
        ("n", "a-alpha.txt"),
        // Past the last match, `n` wraps to the first.
        ("nnn", "z-alpha.txt"),
        ("N", "c-alpha.txt"),
    ];
    for (keys, file) in cases {
        let mut steps = vec![Text("gg"), Char('/'), Text("alpha"), Key(Enter)];
        if !keys.is_empty() {
            steps.push(Text(keys));
        }
        let screen = worktree_screen(&repository, steps);
        assert_eq!(
            selected_row(&screen).as_deref(),
            Some(file),
            "{keys:?}\n{screen}"
        );
    }
}

#[test]
fn worktree_search_matches_the_full_path() {
    let repository = Repo::with_history();
    repository.write("docs/guide.txt", "new\n");
    repository.write("src/main.txt", "new\n");
    repository.git(&["add", "docs/guide.txt", "src/main.txt"]);
    let mut steps = vec![WaitFor("main.txt"), Text("gg"), Char('/'), Text("docs")];
    steps.push(Pause(Duration::from_millis(200)));
    let screen = replay_sized(&repository.path(), 140, 24, steps);
    assert_eq!(
        selected_row(&screen).as_deref(),
        Some("docs/guide.txt"),
        "{screen}"
    );
    assert!(screen.contains("match 1 of 1"), "{screen}");
}
