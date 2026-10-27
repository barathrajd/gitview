use gitview::branches::{HeadState, load_branches};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fixture() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gitview-branches-{}-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
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

#[test]
fn switches_to_an_existing_local_branch() {
    use gitview::branches::switch_branch;
    let root = fixture();
    commit(&root);
    git(&root, &["branch", "topic"]);
    switch_branch(&root, std::ffi::OsStr::new("topic")).unwrap();
    assert_eq!(
        load_branches(&root).unwrap().head,
        HeadState::Attached("topic".into())
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_a_switch_that_would_overwrite_local_changes() {
    use gitview::branches::switch_branch;
    let root = fixture();
    fs::write(root.join("file"), "base\n").unwrap();
    git(&root, &["add", "file"]);
    commit(&root);
    git(&root, &["switch", "-qc", "topic"]);
    fs::write(root.join("file"), "branch\n").unwrap();
    git(&root, &["add", "file"]);
    commit(&root);
    git(&root, &["switch", "-q", "-"]);
    let before = load_branches(&root).unwrap().head;
    fs::write(root.join("file"), "local\n").unwrap();
    assert!(switch_branch(&root, std::ffi::OsStr::new("topic")).is_err());
    assert_eq!(load_branches(&root).unwrap().head, before);
    assert_eq!(fs::read(root.join("file")).unwrap(), b"local\n");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_remote_or_option_like_names() {
    use gitview::branches::switch_branch;
    let root = fixture();
    commit(&root);
    assert!(switch_branch(&root, std::ffi::OsStr::new("--discard-changes")).is_err());
    assert!(switch_branch(&root, std::ffi::OsStr::new("missing")).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preserves_nonconflicting_changes_and_respects_other_worktrees() {
    use gitview::branches::switch_branch;
    let root = fixture();
    commit(&root);
    git(&root, &["branch", "topic"]);
    fs::write(root.join("local"), "untracked").unwrap();
    switch_branch(&root, std::ffi::OsStr::new("topic")).unwrap();
    assert_eq!(fs::read(root.join("local")).unwrap(), b"untracked");
    git(&root, &["switch", "-q", "-"]);
    let other = root.with_extension("other");
    git(
        &root,
        &["worktree", "add", "-q", other.to_str().unwrap(), "topic"],
    );
    assert!(switch_branch(&root, std::ffi::OsStr::new("topic")).is_err());
    assert_eq!(fs::read(root.join("local")).unwrap(), b"untracked");
    git(
        &root,
        &["worktree", "remove", "--force", other.to_str().unwrap()],
    );
    fs::remove_dir_all(root).unwrap();
}

fn head_id(root: &Path) -> String {
    let out = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

#[test]
fn creates_and_switches_to_a_valid_branch_from_confirmed_head() {
    use gitview::branches::create_branch;
    let root = fixture();
    commit(&root);
    let base = head_id(&root);
    create_branch(&root, "feature/topic", &base).unwrap();
    assert_eq!(
        load_branches(&root).unwrap().head,
        HeadState::Attached("feature/topic".into())
    );
    assert_eq!(head_id(&root), base);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_duplicate_and_stale_creation_requests() {
    use gitview::branches::create_branch;
    let root = fixture();
    assert!(create_branch(&root, "new", "").is_err());
    commit(&root);
    let base = head_id(&root);
    let before = load_branches(&root).unwrap().head;
    for name in ["bad name", "-danger", "@{-1}", "topic..bad"] {
        assert!(
            create_branch(&root, name, &base).is_err(),
            "accepted {name}"
        );
    }
    git(&root, &["branch", "existing"]);
    assert!(create_branch(&root, "existing", &base).is_err());
    assert!(create_branch(&root, "stale", "0000000000000000000000000000000000000000").is_err());
    assert_eq!(load_branches(&root).unwrap().head, before);
    assert!(
        !load_branches(&root)
            .unwrap()
            .branches
            .iter()
            .any(|b| b == "stale")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn creates_from_detached_head() {
    use gitview::branches::create_branch;
    let root = fixture();
    commit(&root);
    git(&root, &["switch", "--detach", "-q"]);
    let base = head_id(&root);
    create_branch(&root, "from-detached", &base).unwrap();
    assert_eq!(
        load_branches(&root).unwrap().head,
        HeadState::Attached("from-detached".into())
    );
    fs::remove_dir_all(root).unwrap();
}
