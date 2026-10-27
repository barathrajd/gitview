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
