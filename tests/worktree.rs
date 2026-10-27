use gitview::worktree::{Section, load_status};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn separates_staged_unstaged_and_untracked_changes() {
    let root = std::env::temp_dir().join(format!(
        "gitview-status-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("both file"), "base\n").unwrap();
    git(&["add", "--", "both file"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::write(root.join("both file"), "staged\n").unwrap();
    git(&["add", "--", "both file"]);
    fs::write(root.join("both file"), "unstaged\n").unwrap();
    fs::write(root.join("new file"), "new\n").unwrap();
    let entries = load_status(&root).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Staged && entry.path == "both file")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Unstaged && entry.path == "both file")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Untracked && entry.path == "new file")
    );
    fs::remove_dir_all(root).unwrap();
}
