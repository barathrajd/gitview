use gitview::worktree::{self, Section};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_REPO_ID: AtomicU64 = AtomicU64::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gitview-ignore-{}-{}",
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
fn reports_the_ignore_rule_matching_a_selected_tracked_file() {
    let repo = Repo::new();
    repo.commit_file("generated file.log", "before\n");
    fs::write(repo.path().join(".gitignore"), "*.log\n").unwrap();
    fs::write(repo.path().join("generated file.log"), "after\n").unwrap();
    let entry = worktree::load_status(repo.path())
        .unwrap()
        .into_iter()
        .find(|entry| entry.path == "generated file.log" && entry.section == Section::Unstaged)
        .unwrap();

    let rule = worktree::ignore_rule(repo.path(), &entry).unwrap().unwrap();

    assert!(rule.source.ends_with(".gitignore"));
    assert_eq!(rule.line, "1");
    assert_eq!(rule.pattern, "*.log");
}

#[test]
fn reports_when_no_ignore_rule_matches_a_selected_path() {
    let repo = Repo::new();
    repo.commit_file("tracked.txt", "before\n");
    fs::write(repo.path().join("tracked.txt"), "after\n").unwrap();
    let entry = worktree::load_status(repo.path()).unwrap().remove(0);

    assert!(
        worktree::ignore_rule(repo.path(), &entry)
            .unwrap()
            .is_none()
    );
}
