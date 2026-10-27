use gitview::details::load_commit;
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn loads_commit_metadata_and_changed_file_names() {
    let path = std::env::temp_dir().join(format!(
        "gitview-details-{}",
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
    fs::write(path.join("file with space.txt"), "hello\n").unwrap();
    git(&["add", "--", "file with space.txt"]);
    git(&[
        "-c",
        "user.name=Example User",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "Example change",
    ]);
    let details = load_commit(&path, "HEAD").unwrap();
    assert_eq!(details.subject, "Example change");
    assert_eq!(details.author, "Example User");
    assert!(!details.hash.is_empty() && !details.date.is_empty());
    assert!(
        details
            .files
            .iter()
            .any(|file| file == "file with space.txt")
    );
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn extracts_commit_ids_only_from_graph_commit_rows() {
    use gitview::history::commit_id;
    assert_eq!(
        commit_id("| * abc1234 (HEAD -> main) change"),
        Some("abc1234")
    );
    assert_eq!(commit_id("| |\\"), None);
    assert_eq!(commit_id("[history truncated to 200 commits]"), None);
}

#[test]
fn loads_the_selected_files_unified_diff() {
    use gitview::details::load_diff;
    let path = std::env::temp_dir().join(format!(
        "gitview-diff-{}",
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
    fs::write(path.join("a file.txt"), "new line\n").unwrap();
    git(&["add", "--", "a file.txt"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "add file",
    ]);
    let diff = load_diff(&path, "HEAD", "a file.txt").unwrap();
    assert!(diff.contains("+new line"));
    fs::remove_dir_all(path).unwrap();
}
