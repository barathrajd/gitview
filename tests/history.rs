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
fn search_matches_returns_filtered_commit_rows_in_history_order() {
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
