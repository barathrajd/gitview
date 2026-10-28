use std::{path::Path, process::Command};
#[test]
fn filters_history_to_a_literal_path_with_spaces_and_glob_characters() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .current_dir(root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "commit.gpgsign", "false"]);
    std::fs::write(root.path().join("[one] file.txt"), "selected\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "selected path"]);
    std::fs::write(root.path().join("o file.txt"), "other\n").unwrap();
    git(&["add", "--all"]);
    git(&["commit", "-qm", "unrelated path"]);
    let rows =
        gitview::history::load_history_for_path(root.path(), Some(Path::new("[one] file.txt")))
            .unwrap();
    assert!(rows.iter().any(|row| row.contains("selected path")));
    assert!(!rows.iter().any(|row| row.contains("unrelated path")));
    let rows = gitview::history::load_history_for_path(root.path(), None).unwrap();
    assert!(rows.iter().any(|row| row.contains("unrelated path")));
    assert!(
        gitview::history::load_history_for_path(root.path(), Some(Path::new("../outside")))
            .is_err()
    );
}
