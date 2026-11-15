//! The structured History graph in the real application loop: lanes for a
//! merge, ref badges, aligned columns, and the selection on parsed commits.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Enter, Tab};
use replay::{Repo, Step::*, replay_sized};

/// main: Base, Main work, then a merge of `feature` (Feature work), tagged.
fn merged() -> Repo {
    let repository = Repo::new();
    repository.write("a.txt", "a\n");
    repository.commit_all("Base");
    repository.git(&["switch", "-qc", "feature"]);
    repository.write("f.txt", "f\n");
    repository.commit_all("Feature work");
    repository.git(&["switch", "-q", "main"]);
    repository.write("m.txt", "m\n");
    repository.commit_all("Main work");
    repository.git(&["merge", "-q", "--no-ff", "--no-edit", "feature"]);
    repository.git(&["tag", "v1"]);
    repository
}

/// The HISTORY panel's rows, from the wide layout's middle column.
fn history_rows(screen: &str) -> Vec<String> {
    screen
        .lines()
        .filter_map(|line| {
            let mut panels = line.split("││");
            panels.next()?;
            let history = panels.next()?;
            Some(history.trim_end().to_owned())
        })
        .filter(|row| row.contains('●'))
        .collect()
}

#[test]
fn history_draws_merge_lanes_badges_and_columns() {
    let repository = merged();
    let screen = replay_sized(
        &repository.path(),
        160,
        20,
        [WaitFor("Feature work"), Key(Tab)],
    );
    let rows = history_rows(&screen);
    assert_eq!(rows.len(), 4, "{screen}");
    assert!(rows[0].contains("●─╮"), "{screen}");
    assert!(rows[0].contains("(HEAD -> main)"), "{screen}");
    assert!(rows[0].contains("(tag: v1)"), "{screen}");
    assert!(
        rows[3].contains("●─╯") && rows[3].contains("Base"),
        "{screen}"
    );
    // Hashes line up on every row; subjects line up on rows without
    // badges, which come before the subject.
    let column = |text: &str| {
        rows.iter()
            .find_map(|row| row.find(text).map(|byte| row[..byte].chars().count()))
            .unwrap()
    };
    let hashes: Vec<usize> = rows
        .iter()
        .map(|row| {
            let byte = row
                .char_indices()
                .find(|&(_, c)| c.is_ascii_hexdigit())
                .unwrap()
                .0;
            row[..byte].chars().count()
        })
        .collect();
    assert!(hashes.windows(2).all(|pair| pair[0] == pair[1]), "{screen}");
    assert_eq!(column("Main work"), column("Base"), "{screen}");
    // No raw `git log --graph` characters.
    assert!(!screen.contains("|\\"), "{screen}");
}

#[test]
fn enter_opens_the_selected_parsed_commit() {
    let repository = merged();
    let screen = replay_sized(
        &repository.path(),
        160,
        24,
        [
            WaitFor("Feature work"),
            Key(Tab),
            Text("gg"),
            Char('/'),
            Text("feature work"),
            Key(Enter),
            Key(Enter),
            WaitFor("CHANGED FILES"),
        ],
    );
    assert!(screen.contains("f.txt"), "{screen}");
}
