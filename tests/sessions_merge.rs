//! Reviewing a session's work and merging it back into its base branch with a
//! merge commit, refusing whenever the reviewed state no longer holds.

use gitview::sessions::{self, Session};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn repository() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gitview-sessions-merge-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    // In the repository's config, not only on the test's own commands:
    // Gitview's merge commits too, and CI has no global identity.
    git(&root, &["config", "user.name", "T"]);
    git(&root, &["config", "user.email", "t@e"]);
    fs::write(root.join("a.txt"), "one\n").unwrap();
    git(&root, &["add", "a.txt"]);
    git(&root, &["commit", "-qm", "base"]);
    root
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=T",
            "-c",
            "user.email=t@e",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// A session with one commit that adds `agent.txt`.
fn session_with_work(root: &Path) -> Session {
    let session = sessions::create(root).unwrap();
    fs::write(session.path.join("agent.txt"), "from the agent\n").unwrap();
    git(&session.path, &["add", "agent.txt"]);
    git(&session.path, &["commit", "-qm", "Agent adds a file"]);
    session
}

#[test]
fn review_shows_the_sessions_commits_and_diff() {
    let root = repository();
    let session = session_with_work(&root);

    let review = sessions::review(&root, &session).unwrap();

    assert_eq!(review.commits.len(), 1);
    assert!(
        review.commits[0].ends_with("Agent adds a file"),
        "{:?}",
        review.commits
    );
    assert!(review.diff.contains("+from the agent"), "{}", review.diff);
}

#[test]
fn merge_brings_the_work_back_with_a_merge_commit() {
    let root = repository();
    let base = git(&root, &["rev-parse", "HEAD"]);
    let session = session_with_work(&root);
    let tip = git(&session.path, &["rev-parse", "HEAD"]);
    let review = sessions::review(&root, &session).unwrap();

    sessions::merge(&root, &review).unwrap();

    let parents = git(&root, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(
        parents.split(' ').skip(1).collect::<Vec<_>>(),
        [base.as_str(), tip.as_str()]
    );
    assert!(git(&root, &["log", "-1", "--format=%s"]).contains(&session.name));
    assert_eq!(
        fs::read_to_string(root.join("agent.txt")).unwrap(),
        "from the agent\n"
    );
}

#[test]
fn merge_refuses_a_dirty_main_worktree() {
    let root = repository();
    let review = sessions::review(&root, &session_with_work(&root)).unwrap();
    fs::write(root.join("a.txt"), "user edit\n").unwrap();

    let error = sessions::merge(&root, &review).unwrap_err();

    assert!(error.contains("uncommitted changes"), "{error}");
    assert_eq!(
        fs::read_to_string(root.join("a.txt")).unwrap(),
        "user edit\n"
    );
}

#[test]
fn merge_refuses_when_either_branch_moved_since_review() {
    let root = repository();
    let session = session_with_work(&root);
    let review = sessions::review(&root, &session).unwrap();
    git(
        &session.path,
        &["commit", "-q", "--allow-empty", "-m", "more agent work"],
    );
    let error = sessions::merge(&root, &review).unwrap_err();
    assert!(error.contains("changed since you reviewed"), "{error}");

    let review = sessions::review(&root, &session).unwrap();
    git(&root, &["commit", "-q", "--allow-empty", "-m", "user work"]);
    let error = sessions::merge(&root, &review).unwrap_err();
    assert!(error.contains("changed since you reviewed"), "{error}");
}

#[test]
fn merge_refuses_uncommitted_session_work() {
    let root = repository();
    let session = session_with_work(&root);
    fs::write(session.path.join("draft.txt"), "unfinished\n").unwrap();

    let review = sessions::review(&root, &session).unwrap();
    let error = sessions::merge(&root, &review).unwrap_err();

    assert!(error.contains("uncommitted"), "{error}");
}

#[test]
fn merge_refuses_when_the_base_branch_isnt_checked_out() {
    let root = repository();
    let review = sessions::review(&root, &session_with_work(&root)).unwrap();
    git(&root, &["switch", "-q", "-c", "elsewhere"]);

    let error = sessions::merge(&root, &review).unwrap_err();

    assert!(error.contains("Switch to main"), "{error}");
}

#[test]
fn a_conflict_aborts_and_leaves_main_as_it_was() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    fs::write(session.path.join("a.txt"), "agent\n").unwrap();
    git(&session.path, &["commit", "-qam", "agent edit"]);
    fs::write(root.join("a.txt"), "user\n").unwrap();
    git(&root, &["commit", "-qam", "user edit"]);
    let head = git(&root, &["rev-parse", "HEAD"]);
    let review = sessions::review(&root, &session).unwrap();

    let error = sessions::merge(&root, &review).unwrap_err();

    assert!(error.contains("conflict"), "{error}");
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&root, &["status", "--porcelain"]), "");
    assert!(!root.join(".git/MERGE_HEAD").exists());
}

#[test]
fn merge_from_inside_the_session_lands_in_the_main_worktree() {
    let root = repository();
    let session = session_with_work(&root);
    let review = sessions::review(&session.path, &session).unwrap();

    sessions::merge(&session.path, &review).unwrap();

    assert!(git(&root, &["log", "-1", "--format=%s"]).contains(&session.name));
    assert_eq!(
        fs::read_to_string(root.join("agent.txt")).unwrap(),
        "from the agent\n"
    );
}
