use gitview::worktree;
use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_REPO_ID: AtomicU64 = AtomicU64::new(0);

struct Repo(std::path::PathBuf);

impl Repo {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gitview-discard-{}-{}",
            std::process::id(),
            NEXT_REPO_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        repo.git(&["init", "-q"]);
        repo.git(&["config", "user.name", "Gitview Test"]);
        repo.git(&["config", "user.email", "gitview@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn git(&self, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .current_dir(self.path())
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
        fs::write(self.path().join(path), body).unwrap();
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
fn restores_only_the_confirmed_unstaged_file_to_the_index_version() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "base\n");
    fs::write(repo.path().join("tracked.txt"), "staged\n").unwrap();
    repo.git(&["add", "--", "tracked.txt"]);
    fs::write(repo.path().join("tracked.txt"), "unstaged\n").unwrap();
    fs::write(repo.path().join("other.txt"), "keep me\n").unwrap();
    let entry = worktree::load_status(repo.path())
        .unwrap()
        .into_iter()
        .find(|entry| entry.path == "tracked.txt" && entry.section == worktree::Section::Unstaged)
        .unwrap();
    let preview = worktree::preview_discard(repo.path(), &entry).unwrap();

    worktree::discard(repo.path(), &preview).unwrap();

    assert_eq!(
        fs::read(repo.path().join("tracked.txt")).unwrap(),
        b"staged\n"
    );
    assert_eq!(
        fs::read(repo.path().join("other.txt")).unwrap(),
        b"keep me\n"
    );
    let remaining = worktree::load_status(repo.path()).unwrap();
    assert!(remaining.iter().any(|entry| {
        entry.path == "tracked.txt" && entry.section == worktree::Section::Staged
    }));
    assert!(!remaining.iter().any(|entry| {
        entry.path == "tracked.txt" && entry.section == worktree::Section::Unstaged
    }));
}

#[test]
fn removes_only_the_confirmed_untracked_path_even_when_it_starts_with_a_dash() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "base\n");
    fs::write(repo.path().join("-remove me.txt"), "remove\n").unwrap();
    fs::write(repo.path().join("keep me.txt"), "keep\n").unwrap();
    let entry = worktree::load_status(repo.path())
        .unwrap()
        .into_iter()
        .find(|entry| entry.path == "-remove me.txt")
        .unwrap();
    let preview = worktree::preview_discard(repo.path(), &entry).unwrap();

    worktree::discard(repo.path(), &preview).unwrap();

    assert!(!repo.path().join("-remove me.txt").exists());
    assert!(repo.path().join("keep me.txt").exists());
}

#[test]
fn discards_unstaged_edits_without_reverting_a_staged_rename() {
    let repo = Repo::new();
    repo.commit_file("old name.txt", "original content\n");
    fs::rename(
        repo.path().join("old name.txt"),
        repo.path().join("new name.txt"),
    )
    .unwrap();
    repo.git(&["add", "-A"]);
    fs::write(repo.path().join("new name.txt"), "renamed and edited\n").unwrap();

    let entry = worktree::load_status(repo.path())
        .unwrap()
        .into_iter()
        .find(|entry| entry.section == worktree::Section::Unstaged)
        .unwrap();
    assert_eq!(entry.path, "new name.txt");
    assert_eq!(
        entry.original.as_deref(),
        Some(std::ffi::OsStr::new("old name.txt"))
    );
    let preview = worktree::preview_discard(repo.path(), &entry).unwrap();

    worktree::discard(repo.path(), &preview).unwrap();

    assert!(!repo.path().join("old name.txt").exists());
    assert_eq!(
        fs::read(repo.path().join("new name.txt")).unwrap(),
        b"original content\n"
    );
    assert!(
        worktree::load_status(repo.path())
            .unwrap()
            .iter()
            .any(|entry| {
                entry.section == worktree::Section::Staged
                    && entry.path == "new name.txt"
                    && entry.original.as_deref() == Some(std::ffi::OsStr::new("old name.txt"))
            })
    );
}

#[test]
fn refuses_a_stale_preview_without_removing_the_newer_working_copy() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "base\n");
    fs::write(repo.path().join("tracked.txt"), "first edit\n").unwrap();
    let entry = worktree::load_status(repo.path()).unwrap().remove(0);
    let preview = worktree::preview_discard(repo.path(), &entry).unwrap();
    fs::write(repo.path().join("tracked.txt"), "newer edit\n").unwrap();

    let error = worktree::discard(repo.path(), &preview).unwrap_err();

    assert!(error.contains("changed"));
    assert_eq!(
        fs::read(repo.path().join("tracked.txt")).unwrap(),
        b"newer edit\n"
    );
}

#[test]
fn refuses_to_discard_a_staged_file() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "base\n");
    fs::write(repo.path().join("tracked.txt"), "staged edit\n").unwrap();
    repo.git(&["add", "--", "tracked.txt"]);
    let entry = worktree::load_status(repo.path()).unwrap().remove(0);

    let error = worktree::preview_discard(repo.path(), &entry).unwrap_err();

    assert!(error.contains("unstaged or untracked"));
    assert_eq!(
        fs::read(repo.path().join("tracked.txt")).unwrap(),
        b"staged edit\n"
    );
}
