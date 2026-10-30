//! Discarding a session removes its worktree, branch, and manifest entry after
//! showing what will be lost, and also cleans up after a crash.

use gitview::sessions::{self, Health, Session};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn repository() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gitview-sessions-discard-{}-{}",
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

fn branch_exists(root: &Path, branch: &str) -> bool {
    Command::new("git")
        .current_dir(root)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .output()
        .unwrap()
        .status
        .success()
}

fn assert_gone(root: &Path, session: &Session) {
    assert!(!session.path.exists());
    assert!(!branch_exists(root, &session.branch));
    assert!(
        sessions::list(root)
            .unwrap()
            .iter()
            .all(|s| s.session.name != session.name)
    );
    assert!(!git(root, &["worktree", "list"]).contains(&session.name));
}

#[test]
fn discard_shows_what_will_be_lost_then_removes_everything() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    git(
        &session.path,
        &["commit", "-q", "--allow-empty", "-m", "unmerged work"],
    );
    fs::write(session.path.join("draft.txt"), "x\n").unwrap();

    let preview = sessions::preview_discard(&root, &session).unwrap();
    assert_eq!(preview.unmerged.len(), 1);
    assert!(preview.unmerged[0].ends_with("unmerged work"));
    assert_eq!(preview.uncommitted, ["?? draft.txt"]);

    sessions::discard(&root, &preview).unwrap();
    assert_gone(&root, &session);
}

#[test]
fn a_merged_session_loses_no_commits() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    git(
        &session.path,
        &["commit", "-q", "--allow-empty", "-m", "work"],
    );
    sessions::merge(&root, &sessions::review(&root, &session).unwrap()).unwrap();

    let preview = sessions::preview_discard(&root, &session).unwrap();

    assert!(preview.unmerged.is_empty(), "{:?}", preview.unmerged);
}

#[test]
fn discard_refuses_when_the_session_changed_since_the_preview() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    let preview = sessions::preview_discard(&root, &session).unwrap();
    fs::write(session.path.join("late.txt"), "x\n").unwrap();
    let error = sessions::discard(&root, &preview).unwrap_err();
    assert!(error.contains("changed since"), "{error}");

    let preview = sessions::preview_discard(&root, &session).unwrap();
    git(&session.path, &["add", "late.txt"]);
    git(&session.path, &["commit", "-qm", "late"]);
    let error = sessions::discard(&root, &preview).unwrap_err();
    assert!(error.contains("changed since"), "{error}");
    assert!(session.path.exists());
}

#[test]
fn a_session_whose_folder_was_deleted_can_be_cleaned_up() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    fs::remove_dir_all(&session.path).unwrap();

    let preview = sessions::preview_discard(&root, &session).unwrap();
    sessions::discard(&root, &preview).unwrap();

    assert_gone(&root, &session);
}

#[test]
fn a_session_whose_branch_was_deleted_can_be_cleaned_up() {
    let root = repository();
    let session = sessions::create(&root).unwrap();
    git(&session.path, &["switch", "-q", "--detach"]);
    git(&root, &["branch", "-D", &session.branch]);

    let preview = sessions::preview_discard(&root, &session).unwrap();
    sessions::discard(&root, &preview).unwrap();

    assert_gone(&root, &session);
}

#[test]
fn a_worktree_a_crash_left_out_of_the_manifest_is_listed_and_discardable() {
    let root = repository();
    let known = sessions::create(&root).unwrap();
    // As if Gitview died between adding the worktree and saving the manifest.
    let orphan_path = known.path.parent().unwrap().join("session-orphan");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "gitview/session-orphan",
            orphan_path.to_str().unwrap(),
        ],
    );

    let listed = sessions::list(&root).unwrap();
    let orphan = listed
        .iter()
        .find(|s| s.session.name == "session-orphan")
        .unwrap();
    assert_eq!(orphan.health, Health::Unrecorded);
    assert_eq!(orphan.session.branch, "gitview/session-orphan");
    assert!(sessions::review(&root, &orphan.session).is_err());

    let preview = sessions::preview_discard(&root, &orphan.session).unwrap();
    sessions::discard(&root, &preview).unwrap();
    assert_gone(&root, &orphan.session);
    assert_eq!(sessions::list(&root).unwrap().len(), 1);
}

#[test]
fn the_session_gitview_is_running_in_cant_be_discarded() {
    let root = repository();
    let session = sessions::create(&root).unwrap();

    let preview = sessions::preview_discard(&session.path, &session).unwrap();
    let error = sessions::discard(&session.path, &preview).unwrap_err();

    assert!(error.contains("running in"), "{error}");
    assert!(session.path.exists());
}
