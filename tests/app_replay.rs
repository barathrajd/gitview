//! Characterization tests for the application loop: history, search, diff, and
//! help, driven by scripted keys. They describe how the app behaves today so
//! later refactors can be checked against it.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Backspace, Down, Enter, Esc, PageDown, PageUp, Tab};
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
        Key(Tab),
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
        Key(Tab),
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

#[test]
fn history_motions_jump_and_count_through_real_keys() {
    let repository = Repo::with_history();
    let cases: [(&[char], &str); 4] = [
        (&['G'], "Add readme"),
        (&['G', 'g', 'g'], "Change library"),
        (&['2', 'j'], "Add readme"),
        (&['G', '1', 'k'], "Add library"),
    ];
    for (keys, subject) in cases {
        let mut steps = history_steps();
        steps.extend(keys.iter().map(|key| Char(*key)));
        let screen = replay(&repository.path(), steps);
        assert!(history_selects(&screen, subject), "{keys:?}\n{screen}");
    }
}

/// Forty commits, `Commit 01` (oldest) through `Commit 40` (newest).
fn long_history() -> Repo {
    let repository = Repo::new();
    for number in 1..=40 {
        repository.write("count.txt", &format!("{number}\n"));
        repository.commit_all(&format!("Commit {number:02}"));
    }
    repository
}

/// The first history row drawn on screen.
fn first_visible_commit(screen: &str) -> Option<&str> {
    screen
        .match_indices("Commit ")
        .map(|(index, _)| &screen[index..index + 9])
        .next()
}

#[test]
fn history_page_keys_move_by_the_visible_rows() {
    let repository = long_history();
    let cases = [
        (vec![Ctrl('d')], "Commit 34"),
        (vec![Ctrl('d'), Ctrl('u')], "Commit 40"),
        (vec![Ctrl('f')], "Commit 27"),
        (vec![Key(PageDown), Key(PageDown), Key(PageUp)], "Commit 27"),
        (vec![Char('G'), Ctrl('b')], "Commit 14"),
    ];
    for (keys, subject) in cases {
        let mut steps = vec![WaitFor("Commit 40"), Key(Tab)];
        steps.extend(keys);
        let screen = replay_sized(&repository.path(), 140, 20, steps);
        assert!(history_selects(&screen, subject), "{subject}\n{screen}");
    }
}

#[test]
fn zz_scrolls_the_selected_commit_to_the_middle() {
    let repository = long_history();
    let mut steps = vec![WaitFor("Commit 40"), Key(Tab)];
    steps.extend([Char('2'), Char('0'), Char('j'), Char('z'), Char('z')]);
    let screen = replay_sized(&repository.path(), 140, 20, steps);

    assert!(history_selects(&screen, "Commit 20"), "{screen}");
    assert_eq!(first_visible_commit(&screen), Some("Commit 26"), "{screen}");
}

/// One commit touching `notes-01.txt` through `notes-30.txt`.
fn wide_commit() -> Repo {
    let repository = Repo::with_history();
    for number in 1..=30 {
        repository.write(&format!("notes-{number:02}.txt"), "note\n");
    }
    repository.commit_all("Add notes");
    repository
}

/// The changed file the commit details mark as selected.
fn selected_file(screen: &str) -> Option<&str> {
    let line = screen
        .lines()
        .find(|line| line.contains("› ") && line.contains("notes-"))?;
    let start = line.find("notes-")?;
    line.get(start..start + 8)
}

#[test]
fn commit_file_list_accepts_motions() {
    let repository = wide_commit();
    let cases = [
        (vec![Char('G')], "notes-30"),
        (vec![Char('G'), Char('g'), Char('g')], "notes-01"),
        (vec![Char('5'), Char('j')], "notes-06"),
        (vec![Char('G'), Char('3'), Char('k')], "notes-27"),
    ];
    for (keys, file) in cases {
        let mut steps = vec![
            WaitFor("Add notes"),
            Key(Tab),
            Key(Enter),
            WaitFor("CHANGED FILES"),
        ];
        steps.extend(keys);
        let screen = replay_sized(&repository.path(), 140, 24, steps);
        assert_eq!(selected_file(&screen), Some(file), "{screen}");
    }
}

#[test]
fn diff_scrolls_by_counts_pages_and_gg() {
    let repository = Repo::with_history();
    let mut steps = diff_steps();
    steps.extend([
        Char('5'),
        Char('j'),
        WaitForGone("first changed"),
        Char('g'),
        Char('g'),
        WaitFor("first changed"),
        Ctrl('d'),
        WaitForGone("first changed"),
        // A commit diff has no selected hunk, so `zz` keeps the scroll.
        Text("zz"),
        Pause(std::time::Duration::from_millis(200)),
        WaitForGone("first changed"),
        Ctrl('u'),
        WaitFor("first changed"),
    ]);
    // Short enough that the collapsed diff doesn't fit; scrolling stops at
    // its end.
    replay_sized(&repository.path(), 140, 8, steps);
}

#[test]
fn search_results_page_without_typing_into_the_query() {
    let repository = long_history();
    let open = || {
        vec![
            WaitFor("Commit 40"),
            Key(Tab),
            Char('/'),
            // Tab at the inline prompt opens the full results page.
            Key(Tab),
            Text("Commit"),
            Pause(std::time::Duration::from_millis(300)),
        ]
    };
    let cases = [
        // 20 rows leave 17 for results: half a page is 8, a page 17.
        (vec![Ctrl('d')], "Commit 32"),
        (vec![Ctrl('d'), Ctrl('u')], "Commit 40"),
        (vec![Key(PageDown)], "Commit 23"),
        (vec![Ctrl('f'), Ctrl('b')], "Commit 40"),
        // Other Ctrl letters neither move nor type.
        (vec![Ctrl('x'), Key(Down)], "Commit 39"),
    ];
    for (keys, subject) in cases {
        let mut steps = open();
        steps.extend(keys);
        let screen = replay_sized(&repository.path(), 140, 20, steps);
        assert!(screen.contains("SEARCH  Commit_"), "{screen}");
        let selected = screen.lines().find(|line| line.contains('›'));
        assert!(
            selected.is_some_and(|line| line.contains(subject)),
            "{subject}\n{screen}"
        );
    }
}

#[test]
fn diff_g_reaches_the_end_and_zz_centers_the_selected_hunk() {
    let repository = Repo::with_history();
    let mut text = "first edited\n".to_owned();
    for number in 2..60 {
        text.push_str(&format!("line {number}\n"));
    }
    text.push_str("last edited\n");
    repository.write("src/lib.rs", &text);

    replay_sized(
        &repository.path(),
        140,
        9,
        [
            WaitFor("src/lib.rs"),
            Key(Enter),
            WaitFor("HUNK 1/2"),
            WaitFor("first edited"),
            Char('G'),
            WaitFor("last edited"),
            WaitForGone("first edited"),
            Text("gg"),
            WaitFor("first edited"),
            // `]` puts the second hunk at the top; `zz` brings the first
            // hunk's last context line back into view above it.
            Char(']'),
            WaitFor("HUNK 2/2"),
            WaitForGone("line 4 "),
            Text("zz"),
            WaitFor("line 4 "),
            WaitFor("HUNK 2/2"),
        ],
    );
}

#[test]
fn n_and_shift_n_move_between_search_matches_in_history_order() {
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
    // `/alpha` jumps to the first match below; Enter stays there.
    let search = || {
        vec![
            WaitFor("Fifth beta"),
            Key(Tab),
            Char('/'),
            Text("alpha"),
            Key(Enter),
        ]
    };
    let cases: [(&[char], &str); 5] = [
        (&[], "Fourth alpha"),
        (&['n'], "Second alpha"),
        // Past the last match, `n` wraps to the first.
        (&['n', 'n'], "Fourth alpha"),
        (&['N'], "Second alpha"),
        // Opening search and cancelling it keeps the submitted query.
        (&['/', 'n'], "Second alpha"),
    ];
    for (keys, subject) in cases {
        let mut steps = search();
        for key in keys {
            steps.push(Char(*key));
            // `/` opens search; Esc cancels it.
            if *key == '/' {
                steps.push(Key(Esc));
            }
        }
        steps.push(Pause(std::time::Duration::from_millis(200)));
        let screen = replay_sized(&repository.path(), 140, 20, steps);
        assert!(
            history_selects(&screen, subject),
            "{keys:?} {subject}\n{screen}"
        );
    }
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
fn slash_searches_inline_and_esc_restores_the_selection() {
    let repository = alpha_beta_history();
    // Start on Third beta, the third row.
    let start = || {
        vec![
            WaitFor("Fifth beta"),
            Key(Tab),
            Text("gg2j"),
            Pause(std::time::Duration::from_millis(100)),
            Char('/'),
            Text("alpha"),
            Pause(std::time::Duration::from_millis(200)),
        ]
    };

    // While typing, History stays and the selection jumps to the first match
    // at or below the start.
    let screen = replay_sized(&repository.path(), 140, 20, start());
    assert!(history_selects(&screen, "Second alpha"), "{screen}");
    assert!(screen.contains("/alpha_"), "{screen}");
    assert!(screen.contains("match 2 of 2"), "{screen}");
    assert!(screen.contains("HISTORY"), "{screen}");
    assert!(!screen.contains("SEARCH  alpha"), "{screen}");

    // Esc puts the selection back.
    let mut steps = start();
    steps.extend([Key(Esc), Pause(std::time::Duration::from_millis(200))]);
    let screen = replay_sized(&repository.path(), 140, 20, steps);
    assert!(history_selects(&screen, "Third beta"), "{screen}");
    assert!(!screen.contains("/alpha_"), "{screen}");

    // Enter keeps the match selected without opening the commit.
    let mut steps = start();
    steps.extend([Key(Enter), Pause(std::time::Duration::from_millis(200))]);
    let screen = replay_sized(&repository.path(), 140, 20, steps);
    assert!(history_selects(&screen, "Second alpha"), "{screen}");
    assert!(!screen.contains("CHANGED FILES"), "{screen}");

    // Tab opens the full ranked page with the same query.
    let mut steps = start();
    steps.extend([Key(Tab), WaitFor("SEARCH  alpha_")]);
    replay_sized(&repository.path(), 140, 20, steps);

    // No match leaves the selection where it was.
    let mut steps = start();
    steps.extend([Text("zzz"), Pause(std::time::Duration::from_millis(200))]);
    let screen = replay_sized(&repository.path(), 140, 20, steps);
    assert!(screen.contains("no match"), "{screen}");
    assert!(history_selects(&screen, "Third beta"), "{screen}");
}
