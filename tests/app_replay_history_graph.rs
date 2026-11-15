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

/// A linear history of `count` commits, `c0` oldest, built in one
/// `git fast-import` run.
fn long_history(count: usize) -> Repo {
    use std::io::Write;
    let repository = Repo::new();
    let mut script = String::new();
    for index in 0..count {
        let message = format!("c{index}");
        script.push_str(&format!(
            "commit refs/heads/main\ncommitter T <t@e> {} +0000\ndata {}\n{message}\n",
            1_600_000_000 + index * 60,
            message.len()
        ));
        script.push_str(&format!(
            "M 644 inline f\ndata {}\n{index}\n\n",
            index.to_string().len()
        ));
    }
    let mut child = std::process::Command::new("git")
        .current_dir(repository.path())
        .args(["fast-import", "--quiet"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    repository.git(&["reset", "-q", "--hard", "main"]);
    repository
}

#[test]
fn history_loads_more_commits_past_the_first_page() {
    let repository = long_history(650);
    // G reaches the last loaded commit, which loads the next page; G again
    // reaches the oldest commit.
    let screen = replay_sized(
        &repository.path(),
        160,
        20,
        [
            WaitFor("c649"),
            Key(Tab),
            Char('G'),
            WaitFor("c150"),
            Pause(std::time::Duration::from_millis(500)),
            Char('G'),
            WaitFor("c0 "),
        ],
    );
    assert!(!screen.contains("truncated"), "{screen}");
    let selected = screen
        .lines()
        .find(|line| line.contains("› ●"))
        .unwrap_or("");
    assert!(selected.contains(" c0 "), "{screen}");
}

#[test]
fn v_switches_to_the_raw_graph_and_back_keeping_the_commit() {
    let repository = merged();
    let select_feature = || {
        vec![
            WaitFor("Feature work"),
            Key(Tab),
            Char('/'),
            Text("feature work"),
            Key(Enter),
        ]
    };
    let selected = |screen: &str| {
        screen
            .lines()
            .find(|line| line.contains("› "))
            .unwrap_or("")
            .to_owned()
    };

    let mut steps = select_feature();
    steps.extend([Char('v'), WaitFor("HISTORY · RAW")]);
    let raw = replay_sized(&repository.path(), 160, 20, steps);
    // Raw `git log --graph` characters, and the same commit selected.
    assert!(raw.contains("|\\"), "{raw}");
    assert!(!raw.contains('●'), "{raw}");
    assert!(selected(&raw).contains("› | * "), "{raw}");
    assert!(selected(&raw).contains("Feature work"), "{raw}");

    let mut steps = select_feature();
    steps.extend([
        Char('v'),
        WaitFor("HISTORY · RAW"),
        Char('v'),
        WaitFor("●─╮"),
    ]);
    let structured = replay_sized(&repository.path(), 160, 20, steps);
    assert!(!structured.contains("HISTORY · RAW"), "{structured}");
    assert!(
        selected(&structured).contains("Feature work"),
        "{structured}"
    );
    assert!(selected(&structured).contains("● "), "{structured}");
}
