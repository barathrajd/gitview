use gitview::worktree;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gitview-remove-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        repo.git(&["init", "-q"]);
        repo.git(&["config", "user.name", "Gitview Test"]);
        repo.git(&["config", "user.email", "gitview@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn commit_file(&self, path: &str, body: &str) {
        if let Some(parent) = self.0.join(path).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(self.0.join(path), body).unwrap();
        self.git(&["add", "--", path]);
        self.git(&["commit", "-qm", "initial"]);
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn removes_a_confirmed_clean_tracked_path_and_stages_its_deletion() {
    let repo = Repo::new();
    repo.commit_file("folder/file with spaces.txt", "keep?\n");
    let preview =
        worktree::preview_remove_path(repo.0.as_path(), Path::new("folder/file with spaces.txt"))
            .unwrap();

    worktree::remove_path(repo.0.as_path(), &preview).unwrap();

    assert!(!repo.0.join("folder/file with spaces.txt").exists());
    assert!(
        String::from_utf8(repo.git(&["diff", "--cached", "--name-status"]))
            .unwrap()
            .contains("D\tfolder/file with spaces.txt")
    );
}

#[test]
fn refuses_to_remove_a_path_with_unstaged_edits_and_keeps_the_working_copy() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "original\n");
    fs::write(repo.0.join("tracked.txt"), "unreviewed edit\n").unwrap();
    let preview =
        worktree::preview_remove_path(repo.0.as_path(), Path::new("tracked.txt")).unwrap();

    let error = worktree::remove_path(repo.0.as_path(), &preview).unwrap_err();

    assert!(error.contains("Git operation failed"));
    assert_eq!(
        fs::read(repo.0.join("tracked.txt")).unwrap(),
        b"unreviewed edit\n"
    );
}

#[test]
fn rejects_untracked_and_outside_repository_paths() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "base\n");
    fs::write(repo.0.join("untracked.txt"), "new\n").unwrap();

    assert!(worktree::preview_remove_path(repo.0.as_path(), Path::new("untracked.txt")).is_err());
    assert!(worktree::preview_remove_path(repo.0.as_path(), Path::new("../outside.txt")).is_err());
}

#[test]
fn refuses_a_stale_remove_preview_without_deleting_the_newer_file() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "original\n");
    let preview =
        worktree::preview_remove_path(repo.0.as_path(), Path::new("tracked.txt")).unwrap();
    fs::write(repo.0.join("tracked.txt"), "newer edit\n").unwrap();

    let error = worktree::remove_path(repo.0.as_path(), &preview).unwrap_err();

    assert!(error.contains("changed"));
    assert_eq!(
        fs::read(repo.0.join("tracked.txt")).unwrap(),
        b"newer edit\n"
    );
}
