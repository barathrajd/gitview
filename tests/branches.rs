use gitview::branches::{HeadState, load_branches};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn fixture() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gitview-branches-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    git(&path, &["init", "-q"]);
    path
}
fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}
fn commit(root: &Path) {
    git(
        root,
        &[
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
        ],
    );
}

#[test]
fn reports_unborn_branch_without_commits() {
    let root = fixture();
    let data = load_branches(&root).unwrap();
    assert!(matches!(data.head, HeadState::Unborn(_)));
    assert!(data.branches.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lists_local_branches_and_current_or_detached_head() {
    let root = fixture();
    commit(&root);
    git(&root, &["branch", "topic"]);
    let data = load_branches(&root).unwrap();
    assert!(data.branches.iter().any(|name| name == "topic"));
    assert!(matches!(data.head, HeadState::Attached(_)));
    git(&root, &["switch", "--detach", "-q"]);
    let data = load_branches(&root).unwrap();
    assert!(matches!(data.head, HeadState::Detached));
    assert!(data.branches.iter().any(|name| name == "topic"));
    fs::remove_dir_all(root).unwrap();
}
