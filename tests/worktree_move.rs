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
            "gitview-move-{}-{}",
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
fn moves_a_clean_tracked_file_and_stages_the_rename() {
    let repo = Repo::new();
    repo.commit_file("old name.txt", "content\n");
    let preview = worktree::preview_move_path(
        &repo.0,
        Path::new("old name.txt"),
        Path::new("new name.txt"),
    )
    .unwrap();

    worktree::move_path(&repo.0, &preview).unwrap();

    assert!(!repo.0.join("old name.txt").exists());
    assert_eq!(fs::read(repo.0.join("new name.txt")).unwrap(), b"content\n");
    let summary = String::from_utf8(repo.git(&["diff", "--cached", "--summary"])).unwrap();
    assert!(
        summary.contains("rename old name.txt => new name.txt"),
        "{summary}"
    );
}

#[test]
fn rejects_dirty_sources_and_paths_outside_the_repository() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "base\n");
    fs::write(repo.0.join("tracked.txt"), "local edit\n").unwrap();

    assert!(
        worktree::preview_move_path(&repo.0, Path::new("tracked.txt"), Path::new("moved.txt"))
            .unwrap_err()
            .contains("uncommitted changes")
    );
    assert!(
        worktree::preview_move_path(&repo.0, Path::new("tracked.txt"), Path::new("../moved.txt"))
            .is_err()
    );
}

#[test]
fn refuses_a_destination_that_is_already_tracked() {
    let repo = Repo::new();
    repo.commit_file("source.txt", "source\n");
    fs::write(repo.0.join("destination.txt"), "destination\n").unwrap();
    repo.git(&["add", "destination.txt"]);
    repo.git(&["commit", "-qm", "second"]);

    assert!(
        worktree::preview_move_path(
            &repo.0,
            Path::new("source.txt"),
            Path::new("destination.txt")
        )
        .unwrap_err()
        .contains("already tracked")
    );
}
