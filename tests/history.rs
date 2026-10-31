use gitview::history::load_history;
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn loads_a_commit_and_its_branch_from_a_repository() {
    let path = std::env::temp_dir().join(format!(
        "gitview-history-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&path)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "user.email=test@example.com",
        "commit",
        "--allow-empty",
        "-qm",
        "first commit",
    ]);
    let rows = load_history(&path).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.contains("first commit") && row.contains("HEAD"))
    );
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn empty_repository_has_no_history() {
    let path = std::env::temp_dir().join(format!(
        "gitview-empty-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&path)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    assert!(load_history(&path).unwrap().is_empty());
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn shows_detached_head_even_when_no_branch_points_to_it() {
    let path = std::env::temp_dir().join(format!(
        "gitview-detached-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&path)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "base",
    ]);
    git(&["checkout", "--detach", "-q"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "detached commit",
    ]);
    assert!(
        load_history(&path)
            .unwrap()
            .iter()
            .any(|line| line.contains("detached commit"))
    );
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn reports_when_history_is_truncated() {
    use std::io::Write;
    use std::process::Stdio;
    let path = std::env::temp_dir().join(format!(
        "gitview-limit-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&path)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    let mut importer = Command::new("git")
        .current_dir(&path)
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = importer.stdin.as_mut().unwrap();
    for i in 1..=201 {
        write!(stdin, "commit refs/heads/test\nmark :{i}\ncommitter Test <test@example.com> {i} +0000\ndata 1\nx\n").unwrap();
        if i > 1 {
            writeln!(stdin, "from :{}", i - 1).unwrap();
        }
        writeln!(stdin).unwrap();
    }
    drop(importer.stdin.take());
    assert!(importer.wait().unwrap().success());
    let rows = load_history(&path).unwrap();
    assert!(rows.iter().any(|row| row.contains("history truncated")));
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn displays_merge_lanes_branches_and_tags() {
    use std::{io::Write, process::Stdio};
    let path = std::env::temp_dir().join(format!(
        "gitview-merge-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&path)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    let mut importer = Command::new("git")
        .current_dir(&path)
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    write!(importer.stdin.as_mut().unwrap(), "commit refs/heads/main\nmark :1\ncommitter Test <test@example.com> 1 +0000\ndata 4\nbase\n\ncommit refs/heads/side\nmark :2\ncommitter Test <test@example.com> 2 +0000\ndata 4\nside\nfrom :1\n\ncommit refs/heads/main\nmark :3\ncommitter Test <test@example.com> 3 +0000\ndata 4\nmain\nfrom :1\n\ncommit refs/heads/main\nmark :4\ncommitter Test <test@example.com> 4 +0000\ndata 5\nmerge\nfrom :3\nmerge :2\n\n").unwrap();
    drop(importer.stdin.take());
    assert!(importer.wait().unwrap().success());
    assert!(
        Command::new("git")
            .current_dir(&path)
            .args(["tag", "v1", "main"])
            .status()
            .unwrap()
            .success()
    );
    let rows = load_history(&path).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.contains("tag: v1") && row.contains("merge"))
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("side") && row.contains("*"))
    );
    assert!(
        rows.iter()
            .any(|row| row.contains('|') || row.contains('\\'))
    );
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn search_ranks_fuzzy_matches_best_first() {
    let rows = vec![
        "* 1111111 Spread each address record - Haruki\u{1f}1111111".to_owned(),
        "* 2222222 Fix search debounce - Bob\u{1f}2222222".to_owned(),
        "| [history truncated]".to_owned(),
        "* 3333333 Add search results - Cy\u{1f}3333333".to_owned(),
    ];

    // Letters in order but scattered still match; a contiguous word ranks
    // first, and equal matches keep history order.
    assert_eq!(
        gitview::history::search_matches(&rows, "search"),
        vec![1, 3, 0]
    );
    // Words match in any order, each anywhere in the row.
    assert_eq!(
        gitview::history::search_matches(&rows, "debounce fix"),
        vec![1]
    );
    assert_eq!(gitview::history::search_matches(&rows, "fxsrch"), vec![1]);
    // Non-commit rows never match, and case is ignored as before.
    assert_eq!(
        gitview::history::search_matches(&rows, "HISTORY"),
        Vec::<usize>::new()
    );
    assert_eq!(gitview::history::search_matches(&rows, "BOB"), vec![1]);
    // An empty query lists every commit in history order.
    assert_eq!(gitview::history::search_matches(&rows, ""), vec![0, 1, 3]);
}

#[test]
fn search_matches_filters_commit_rows_ignoring_case() {
    let rows = vec![
        "* abcdef0 Add search results - Alice\u{1f}abcdef0".to_owned(),
        "| [history truncated]".to_owned(),
        "* 1234567 Fix search debounce - Bob\u{1f}1234567".to_owned(),
        "* 7654321 Other change - Alice\u{1f}7654321".to_owned(),
    ];

    assert_eq!(
        gitview::history::search_matches(&rows, "SEARCH"),
        vec![0, 2]
    );
    assert_eq!(gitview::history::search_matches(&rows, "alice"), vec![0, 3]);
    assert_eq!(
        gitview::history::search_matches(&rows, "missing"),
        Vec::<usize>::new()
    );
}

#[test]
fn displays_short_hashes_but_searches_the_full_hash() {
    use gitview::history::display_row;
    let root = std::env::temp_dir().join(format!(
        "gitview-hash-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap()
    };
    assert!(git(&["init", "-q"]).status.success());
    assert!(
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-qm",
            "sample"
        ])
        .status
        .success()
    );
    let full = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_owned();
    let rows = load_history(&root).unwrap();
    let row = rows.iter().find(|row| row.contains("sample")).unwrap();
    assert!(!display_row(row).contains(&full));
    let row_index = rows
        .iter()
        .position(|candidate| candidate.contains("sample"))
        .unwrap();
    assert_eq!(
        gitview::history::search_matches(&rows, &full),
        vec![row_index]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn match_positions_mark_each_matched_byte() {
    let row = "* a2 Fix search debounce - Bob\u{1f}a2ffff";
    let marked = |query: &str| -> String {
        let positions = gitview::history::match_positions(row, query);
        row.char_indices()
            .map(|(index, _)| if positions.contains(&index) { '^' } else { ' ' })
            .collect::<String>()
            .trim_end()
            .to_owned()
    };
    assert_eq!(marked("debounce fix"), "     ^^^        ^^^^^^^^");
    assert_eq!(marked("BOB"), "                           ^^^");
    assert_eq!(marked("fxsrch"), "     ^ ^ ^  ^^^");
    assert_eq!(marked(""), "");
    assert_eq!(marked("zzz"), "");
}
