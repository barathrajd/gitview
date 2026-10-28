use gitview::commit::{amend_commit, preview_amend};
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
            "gitview-amend-{}-{}",
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

    fn add_commit(&self, path: &str, contents: &str, message: &str) -> String {
        fs::write(self.path().join(path), contents).unwrap();
        self.git(&["add", "--", path]);
        self.git(&["commit", "-qm", message]);
        String::from_utf8(self.git(&["rev-parse", "HEAD"]))
            .unwrap()
            .trim()
            .to_owned()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn amends_the_head_message_and_staged_snapshot_while_preserving_unstaged_edits() {
    let repo = Repo::new();
    repo.add_commit("notes.txt", "base\n", "initial");
    let original = repo.add_commit("notes.txt", "old commit\n", "old message");
    let parent = String::from_utf8(repo.git(&["rev-parse", "HEAD^"]))
        .unwrap()
        .trim()
        .to_owned();
    fs::write(repo.path().join("notes.txt"), "staged version\n").unwrap();
    repo.git(&["add", "--", "notes.txt"]);
    fs::write(repo.path().join("notes.txt"), "unstaged version\n").unwrap();
    let expected = preview_amend(repo.path()).unwrap();

    amend_commit(
        repo.path(),
        "feat: improve notes\n\nKeep the staged review and working copy separate.",
        &expected,
    )
    .unwrap();

    let head = String::from_utf8(repo.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    let body = String::from_utf8(repo.git(&["log", "-1", "--format=%B"])).unwrap();
    assert_ne!(head, original);
    assert_eq!(
        String::from_utf8(repo.git(&["rev-parse", "HEAD^"]))
            .unwrap()
            .trim(),
        parent
    );
    assert_eq!(
        body.trim_end(),
        "feat: improve notes\n\nKeep the staged review and working copy separate."
    );
    assert_eq!(repo.git(&["show", "HEAD:notes.txt"]), b"staged version\n");
    assert_eq!(
        fs::read(repo.path().join("notes.txt")).unwrap(),
        b"unstaged version\n"
    );
}

#[test]
fn amends_the_message_when_no_files_are_staged() {
    let repo = Repo::new();
    repo.add_commit("notes.txt", "base\n", "old message");
    let original = String::from_utf8(repo.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    let expected = preview_amend(repo.path()).unwrap();

    amend_commit(repo.path(), "docs: clarify notes", &expected).unwrap();

    let head = String::from_utf8(repo.git(&["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    assert_ne!(head, original);
    assert_eq!(
        String::from_utf8(repo.git(&["log", "-1", "--format=%s"]))
            .unwrap()
            .trim(),
        "docs: clarify notes"
    );
}

#[test]
fn refuses_to_amend_when_the_index_changes_after_the_warning_preview() {
    let repo = Repo::new();
    repo.add_commit("notes.txt", "base\n", "old message");
    fs::write(repo.path().join("notes.txt"), "first staged version\n").unwrap();
    repo.git(&["add", "--", "notes.txt"]);
    let expected = preview_amend(repo.path()).unwrap();
    fs::write(repo.path().join("notes.txt"), "second staged version\n").unwrap();
    repo.git(&["add", "--", "notes.txt"]);
    let old_head = repo.git(&["rev-parse", "HEAD"]);

    let error = amend_commit(repo.path(), "feat: amend safely", &expected).unwrap_err();

    assert!(error.contains("changed"));
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), old_head);
}

#[test]
fn refuses_to_amend_when_head_changes_after_the_warning_preview() {
    let repo = Repo::new();
    repo.add_commit("notes.txt", "base\n", "old message");
    let expected = preview_amend(repo.path()).unwrap();
    repo.add_commit("other.txt", "new head\n", "external commit");
    let head_after_external_commit = repo.git(&["rev-parse", "HEAD"]);

    let error = amend_commit(repo.path(), "feat: amend safely", &expected).unwrap_err();

    assert!(error.contains("changed"));
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), head_after_external_commit);
}
