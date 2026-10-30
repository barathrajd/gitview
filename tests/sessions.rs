//! Agent sessions: each runs in its own worktree on its own branch, kept under
//! the repository's Git folder and recorded in a manifest.

use gitview::sessions::{self, Health};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

/// A repository on `main` with one commit, in a folder whose name has a space.
fn repository() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gitview sessions-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
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

#[test]
fn a_session_gets_its_own_worktree_and_branch_from_head() {
    let root = repository();
    let head = git(&root, &["rev-parse", "HEAD"]);

    let session = sessions::create(&root).unwrap();

    assert!(session.name.starts_with("session-"), "{}", session.name);
    assert_eq!(session.branch, format!("gitview/{}", session.name));
    assert_eq!(session.base_branch, "main");
    assert_eq!(session.base_commit, head);
    let sessions_dir = fs::canonicalize(root.join(".git/gitview/sessions")).unwrap();
    assert_eq!(session.path, sessions_dir.join(&session.name));
    assert_eq!(
        git(&session.path, &["symbolic-ref", "--short", "HEAD"]),
        session.branch
    );
    assert_eq!(git(&session.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read_to_string(session.path.join("a.txt")).unwrap(),
        "one\n"
    );
    // The main worktree doesn't see the session's folder as a change.
    assert_eq!(
        git(&root, &["status", "--porcelain", "--untracked-files=all"]),
        ""
    );
}

#[test]
fn sessions_are_listed_from_the_manifest() {
    let root = repository();
    let first = sessions::create(&root).unwrap();
    let second = sessions::create(&root).unwrap();

    let listed = sessions::list(&root).unwrap();

    assert_ne!(first.name, second.name);
    let names: Vec<_> = listed
        .iter()
        .map(|status| status.session.name.clone())
        .collect();
    assert_eq!(names, [first.name.clone(), second.name.clone()]);
    for status in &listed {
        assert_eq!(status.health, Health::Ready);
        assert!(!status.dirty);
        assert_eq!(status.ahead, 0);
    }
    // Listing from inside a session finds the same sessions.
    assert_eq!(sessions::list(&first.path).unwrap().len(), 2);
}

#[test]
fn the_list_shows_commits_and_uncommitted_work() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    fs::write(session.path.join("a.txt"), "two\n").unwrap();
    git(&session.path, &["commit", "-qam", "agent work"]);
    fs::write(session.path.join("new.txt"), "draft\n").unwrap();

    let status = sessions::list(&root).unwrap().remove(0);

    assert_eq!(status.ahead, 1);
    assert!(status.dirty);
}

#[test]
fn a_session_needs_a_branch_to_return_to() {
    let detached = repository();
    let head = git(&detached, &["rev-parse", "HEAD"]);
    git(&detached, &["checkout", "-q", "--detach", &head]);
    let error = sessions::create(&detached).unwrap_err();
    assert!(error.contains("on a branch"), "{error}");

    let empty = std::env::temp_dir().join(format!("gitview-sessions-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&empty);
    fs::create_dir(&empty).unwrap();
    git(&empty, &["init", "-q"]);
    let error = sessions::create(&empty).unwrap_err();
    assert!(error.contains("first commit"), "{error}");
}

#[test]
fn a_session_left_half_gone_by_a_crash_is_reported() {
    let root = repository();
    let lost_folder = sessions::create(&root).unwrap();
    let lost_branch = sessions::create(&root).unwrap();
    fs::remove_dir_all(&lost_folder.path).unwrap();
    git(
        &root,
        &[
            "worktree",
            "remove",
            "--force",
            lost_branch.path.to_str().unwrap(),
        ],
    );
    git(&root, &["branch", "-D", &lost_branch.branch]);

    let listed = sessions::list(&root).unwrap();

    assert_eq!(listed[0].health, Health::MissingWorktree);
    assert_eq!(listed[1].health, Health::MissingBranch);
}

#[test]
fn sessions_created_at_the_same_time_are_all_recorded() {
    let root = repository();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let root = root.clone();
            std::thread::spawn(move || sessions::create(&root).unwrap())
        })
        .collect();
    let created: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();

    let listed = sessions::list(&root).unwrap();

    assert_eq!(listed.len(), 8);
    for session in &created {
        let status = listed
            .iter()
            .find(|s| s.session.name == session.name)
            .unwrap();
        assert_eq!(status.health, Health::Ready, "{}", session.name);
    }
}
