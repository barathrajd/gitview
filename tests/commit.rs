use gitview::commit::{create_commit, preview_commit};
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
            "gitview-commit-{}-{}",
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
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn creates_a_commit_from_the_confirmed_index_and_keeps_unstaged_edits() {
    let repo = Repo::new();
    fs::write(repo.path().join("notes.txt"), "base\n").unwrap();
    repo.git(&["add", "--", "notes.txt"]);
    repo.git(&["commit", "-qm", "initial"]);

    fs::write(repo.path().join("notes.txt"), "staged version\n").unwrap();
    repo.git(&["add", "--", "notes.txt"]);
    fs::write(repo.path().join("notes.txt"), "working version\n").unwrap();
    let expected = preview_commit(repo.path()).unwrap();

    create_commit(repo.path(), "Review staged work", &expected).unwrap();

    let subject = String::from_utf8(repo.git(&["log", "-1", "--format=%s"])).unwrap();
    let committed = repo.git(&["show", "HEAD:notes.txt"]);
    assert_eq!(subject.trim(), "Review staged work");
    assert_eq!(committed, b"staged version\n");
    assert_eq!(
        fs::read(repo.path().join("notes.txt")).unwrap(),
        b"working version\n"
    );
}

#[test]
fn keeps_the_optional_description_as_the_commit_body() {
    let repo = Repo::new();
    fs::write(repo.path().join("notes.txt"), "body\n").unwrap();
    repo.git(&["add", "--", "notes.txt"]);
    let expected = preview_commit(repo.path()).unwrap();

    create_commit(
        repo.path(),
        "feat: add commit descriptions\n\nExplain the behavior and context.",
        &expected,
    )
    .unwrap();

    let body = String::from_utf8(repo.git(&["log", "-1", "--format=%B"])).unwrap();
    assert_eq!(
        body.trim_end(),
        "feat: add commit descriptions\n\nExplain the behavior and context."
    );
}

#[test]
fn refuses_to_commit_when_the_staged_set_changed_after_confirmation() {
    let repo = Repo::new();
    fs::write(repo.path().join("first.txt"), "first\n").unwrap();
    repo.git(&["add", "--", "first.txt"]);
    let expected = preview_commit(repo.path()).unwrap();
    fs::write(repo.path().join("second.txt"), "second\n").unwrap();
    repo.git(&["add", "--", "second.txt"]);

    let error = create_commit(repo.path(), "Commit selected files", &expected).unwrap_err();

    assert!(error.contains("Staged changes changed"));
    assert!(!repo.git(&["status", "--porcelain"]).is_empty());
}

#[test]
fn refuses_when_staged_content_changes_without_changing_the_path_set() {
    let repo = Repo::new();
    fs::write(repo.path().join("file.txt"), "first staged version\n").unwrap();
    repo.git(&["add", "--", "file.txt"]);
    let expected = preview_commit(repo.path()).unwrap();
    fs::write(repo.path().join("file.txt"), "second staged version\n").unwrap();
    repo.git(&["add", "--", "file.txt"]);

    let error = create_commit(repo.path(), "Commit reviewed version", &expected).unwrap_err();

    assert!(error.contains("Staged changes changed"));
    assert!(!repo.git(&["status", "--porcelain"]).is_empty());
}

#[test]
fn refuses_an_empty_or_whitespace_only_message() {
    let repo = Repo::new();
    fs::write(repo.path().join("file.txt"), "content\n").unwrap();
    repo.git(&["add", "--", "file.txt"]);
    let expected = preview_commit(repo.path()).unwrap();

    let error = create_commit(repo.path(), "  \n", &expected).unwrap_err();

    assert!(error.contains("Commit message cannot be empty"));
    assert!(!repo.git(&["status", "--porcelain"]).is_empty());
}
