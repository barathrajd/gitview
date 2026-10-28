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
    assert_eq!(comparison.files.len(), 2);
    assert_eq!(comparison.files[0].path, "[one] file.txt");
    assert_eq!(comparison.files[0].additions, Some(1));
    assert_eq!(comparison.files[0].deletions, Some(1));
    assert!(comparison.files[0].patch.contains("+after"));
    assert!(!comparison.files[0].patch.contains("unrelated"));
    assert_eq!(comparison.files[1].path, "other.txt");
    assert!(comparison.files[1].patch.contains("+unrelated"));
}

#[test]
fn groups_binary_deletions_and_quoted_paths_without_losing_file_boundaries() {
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
    std::fs::write(root.path().join("binary.dat"), [0, 1, 2]).unwrap();
    std::fs::write(root.path().join("deleted.txt"), "removed\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "base"]);
    std::fs::write(root.path().join("binary.dat"), [0, 3, 4]).unwrap();
    std::fs::remove_file(root.path().join("deleted.txt")).unwrap();
    std::fs::write(root.path().join("quoted\" file.txt"), "created\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "target"]);
    let comparison = gitview::comparison::load(root.path(), "HEAD^", "HEAD", None).unwrap();
    assert_eq!(comparison.files.len(), 3);
    let binary = comparison
        .files
        .iter()
        .find(|file| file.path == "binary.dat")
        .unwrap();
    assert_eq!(binary.additions, None);
    assert!(binary.patch.contains("Binary files"));
    let deleted = comparison
        .files
        .iter()
        .find(|file| file.path == "deleted.txt")
        .unwrap();
    assert_eq!(deleted.deletions, Some(1));
    assert!(deleted.patch.contains("-removed"));
    let quoted = comparison
        .files
        .iter()
        .find(|file| file.path == "quoted\" file.txt")
        .unwrap();
    assert_eq!(quoted.additions, Some(1));
    assert!(quoted.patch.contains("+created"));
    assert!(!comparison.truncated);
}
