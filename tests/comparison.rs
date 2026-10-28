use std::{path::Path, process::Command};
#[test]
fn compares_fixed_commit_snapshots_with_literal_path_and_summary() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(root.path().join("[one] file.txt"), "before\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "base"]);
    std::fs::write(root.path().join("[one] file.txt"), "after\n").unwrap();
    std::fs::write(root.path().join("other.txt"), "unrelated\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "target"]);
    let comparison = gitview::comparison::load(
        root.path(),
        "HEAD^",
        "HEAD",
        Some(Path::new("[one] file.txt")),
    )
    .unwrap();
    assert_ne!(comparison.base, comparison.target);
    assert!(comparison.patch.contains("-before"));
    assert!(comparison.patch.contains("+after"));
    assert!(!comparison.patch.contains("unrelated"));
    assert!(comparison.summary.contains("[one] file.txt"));
    assert!(gitview::comparison::load(root.path(), "--output=bad", "HEAD", None).is_err());
    assert!(
        gitview::comparison::load(root.path(), "HEAD^", "HEAD", Some(Path::new("../outside")))
            .is_err()
    );
    git(&["update-ref", "refs/remotes/origin/local", "HEAD^"]);
    let comparison = gitview::comparison::load(root.path(), "origin/local", "HEAD", None).unwrap();
    assert!(comparison.patch.contains("unrelated"));
}
