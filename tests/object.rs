use std::process::Command;
fn repo() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Test"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        git(root.path(), &args);
    }
    std::fs::write(root.path().join("sample.txt"), "sample text\n").unwrap();
    std::fs::write(root.path().join("binary.dat"), [0, 1, 2]).unwrap();
    git(root.path(), &["add", "--all"]);
    git(root.path(), &["commit", "-qm", "base"]);
    git(
        root.path(),
        &[
            "-c",
            "tag.gpgSign=false",
            "tag",
            "--no-sign",
            "-a",
            "v-test",
            "-m",
            "tag message",
        ],
    );
    root
}
fn git(root: &std::path::Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
#[test]
fn describes_commits_tags_trees_and_blobs_without_changing_the_repository() {
    let root = repo();
    let before = git(root.path(), &["status", "--porcelain=v2"]);
    let commit = gitview::object::inspect(root.path(), "HEAD").unwrap();
    assert_eq!(commit.kind, "commit");
    assert_eq!(commit.description.as_deref(), Some("v-test"));
    assert!(commit.preview.contains("base"));
    assert!(commit.size > 0);
    let tag = gitview::object::inspect(root.path(), "v-test").unwrap();
    assert_eq!(tag.kind, "tag");
    assert!(tag.preview.contains("tag message"));
    let tree = gitview::object::inspect(root.path(), "HEAD^{tree}").unwrap();
    assert_eq!(tree.kind, "tree");
    assert!(tree.preview.contains("sample.txt"));
    let blob = gitview::object::inspect(root.path(), "HEAD:sample.txt").unwrap();
    assert_eq!(blob.kind, "blob");
    assert_eq!(blob.preview, "sample text\n");
    let binary = gitview::object::inspect(root.path(), "HEAD:binary.dat").unwrap();
    assert!(binary.preview.contains("Binary blob"));
    assert!(gitview::object::inspect(root.path(), "--output=bad").is_err());
    assert_eq!(git(root.path(), &["status", "--porcelain=v2"]), before);
}
#[test]
fn bounds_large_object_previews() {
    let root = repo();
    std::fs::write(root.path().join("large.txt"), "x".repeat(1_000_010)).unwrap();
    git(root.path(), &["add", "large.txt"]);
    git(root.path(), &["commit", "-qm", "large"]);
    let object = gitview::object::inspect(root.path(), "HEAD:large.txt").unwrap();
    assert!(object.truncated);
    assert!(object.preview.len() <= 1_000_000);
}
