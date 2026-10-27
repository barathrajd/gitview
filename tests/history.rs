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
