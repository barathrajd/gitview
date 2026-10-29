//! Reading the working-tree status must not write to the repository: a file
//! watcher on `.git/index` would otherwise see Gitview's own reads as changes
//! and refresh in a loop.

use std::{fs, process::Command, thread, time::Duration};

use gitview::worktree::{Entry, Section, load_diff, load_status};

fn git(root: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[test]
fn loading_the_status_leaves_the_index_untouched() {
    let repository = tempfile::tempdir().unwrap();
    let root = repository.path();
    git(root, &["init", "-q", "-b", "main"]);
    for index in 0..20 {
        fs::write(root.join(format!("file{index}.txt")), "content\n").unwrap();
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "init"]);
    // Rewrite every file with identical content: the cached stat data in the
    // index is now stale, so a plain `git status` would refresh and rewrite it.
    thread::sleep(Duration::from_millis(1100));
    for index in 0..20 {
        fs::write(root.join(format!("file{index}.txt")), "content\n").unwrap();
    }
    let index_path = root.join(".git/index");
    let before = fs::metadata(&index_path).unwrap().modified().unwrap();
    let before_bytes = fs::read(&index_path).unwrap();

    let entries = load_status(root).unwrap();

    assert!(entries.is_empty(), "contents are unchanged: {entries:?}");
    assert_eq!(
        fs::metadata(&index_path).unwrap().modified().unwrap(),
        before
    );
    assert_eq!(fs::read(&index_path).unwrap(), before_bytes);
}

#[test]
fn loading_a_diff_leaves_the_index_untouched() {
    let repository = tempfile::tempdir().unwrap();
    let root = repository.path();
    git(root, &["init", "-q", "-b", "main"]);
    for index in 0..20 {
        fs::write(root.join(format!("file{index}.txt")), "content\n").unwrap();
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "init"]);
    thread::sleep(Duration::from_millis(1100));
    for index in 0..20 {
        fs::write(root.join(format!("file{index}.txt")), "content\n").unwrap();
    }
    fs::write(root.join("file3.txt"), "changed\n").unwrap();
    let index_path = root.join(".git/index");
    let before = fs::read(&index_path).unwrap();
    let modified = fs::metadata(&index_path).unwrap().modified().unwrap();
    let entry = Entry {
        section: Section::Unstaged,
        path: "file3.txt".into(),
        original: None,
    };

    let diff = load_diff(root, &entry).unwrap();

    assert!(diff.contains("+changed"), "{diff}");
    assert_eq!(fs::read(&index_path).unwrap(), before);
    assert_eq!(
        fs::metadata(&index_path).unwrap().modified().unwrap(),
        modified
    );
}
