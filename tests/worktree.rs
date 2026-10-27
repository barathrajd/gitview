use gitview::worktree::{Section, load_status};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn separates_staged_unstaged_and_untracked_changes() {
    let root = std::env::temp_dir().join(format!(
        "gitview-status-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("both file"), "base\n").unwrap();
    git(&["add", "--", "both file"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::write(root.join("both file"), "staged\n").unwrap();
    git(&["add", "--", "both file"]);
    fs::write(root.join("both file"), "unstaged\n").unwrap();
    fs::write(root.join("new file"), "new\n").unwrap();
    let entries = load_status(&root).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Staged && entry.path == "both file")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Unstaged && entry.path == "both file")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Untracked && entry.path == "new file")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stages_all_tracked_and_untracked_worktree_changes() {
    let root = std::env::temp_dir().join(format!(
        "gitview-stage-all-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("modified"), "before\n").unwrap();
    fs::write(root.join("deleted"), "remove me\n").unwrap();
    git(&["add", "--", "modified", "deleted"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::write(root.join("modified"), "after\n").unwrap();
    fs::remove_file(root.join("deleted")).unwrap();
    fs::write(root.join("new file"), "new\n").unwrap();

    gitview::worktree::stage_all(&root).unwrap();

    let status = load_status(&root).unwrap();
    assert_eq!(status.len(), 3);
    assert!(status.iter().all(|entry| entry.section == Section::Staged));
    let cached = Command::new("git")
        .current_dir(&root)
        .args(["diff", "--cached", "--name-status"])
        .output()
        .unwrap();
    let cached = String::from_utf8_lossy(&cached.stdout);
    assert!(cached.contains("modified"));
    assert!(cached.contains("deleted"));
    assert!(cached.contains("new file"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stages_and_unstages_only_the_selected_file_on_an_unborn_branch() {
    use gitview::worktree::{stage, unstage};
    let root = std::env::temp_dir().join(format!(
        "gitview-stage-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    fs::write(root.join("-one"), "one\n").unwrap();
    fs::write(root.join("other"), "other\n").unwrap();
    let entry = load_status(&root)
        .unwrap()
        .into_iter()
        .find(|entry| entry.path == "-one")
        .unwrap();
    stage(&root, &entry).unwrap();
    let entries = load_status(&root).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Staged && entry.path == "-one")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Untracked && entry.path == "other")
    );
    let staged = entries
        .iter()
        .find(|entry| entry.section == Section::Staged && entry.path == "-one")
        .unwrap();
    unstage(&root, staged).unwrap();
    assert!(
        load_status(&root)
            .unwrap()
            .iter()
            .any(|entry| entry.section == Section::Untracked && entry.path == "-one")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stages_only_the_selected_hunk_of_a_tracked_file() {
    use gitview::worktree::stage_hunk;
    let root = std::env::temp_dir().join(format!(
        "gitview-stage-hunk-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    let base = (1..=20)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    fs::write(root.join("tracked"), &base).unwrap();
    git(&["add", "--", "tracked"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    let changed = base
        .replace("line 2\n", "first hunk\n")
        .replace("line 18\n", "second hunk\n");
    fs::write(root.join("tracked"), changed).unwrap();
    let entry = load_status(&root)
        .unwrap()
        .into_iter()
        .find(|entry| entry.section == Section::Unstaged)
        .unwrap();

    stage_hunk(&root, &entry, 0).unwrap();

    let staged = Command::new("git")
        .current_dir(&root)
        .args(["diff", "--cached", "--unified=0", "--", "tracked"])
        .output()
        .unwrap();
    let staged = String::from_utf8_lossy(&staged.stdout);
    assert!(staged.contains("+first hunk"));
    assert!(!staged.contains("+second hunk"));
    let unstaged = Command::new("git")
        .current_dir(&root)
        .args(["diff", "--unified=0", "--", "tracked"])
        .output()
        .unwrap();
    let unstaged = String::from_utf8_lossy(&unstaged.stdout);
    assert!(!unstaged.contains("+first hunk"));
    assert!(unstaged.contains("+second hunk"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unstages_only_the_selected_hunk_of_a_tracked_file() {
    use gitview::worktree::unstage_hunk;
    let root = std::env::temp_dir().join(format!(
        "gitview-unstage-hunk-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    let base = (1..=20)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    fs::write(root.join("tracked"), &base).unwrap();
    git(&["add", "--", "tracked"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    let changed = base
        .replace("line 2\n", "first hunk\n")
        .replace("line 18\n", "second hunk\n");
    fs::write(root.join("tracked"), changed).unwrap();
    git(&["add", "--", "tracked"]);
    let entry = load_status(&root)
        .unwrap()
        .into_iter()
        .find(|entry| entry.section == Section::Staged)
        .unwrap();

    unstage_hunk(&root, &entry, 1).unwrap();

    let staged = Command::new("git")
        .current_dir(&root)
        .args(["diff", "--cached", "--unified=0", "--", "tracked"])
        .output()
        .unwrap();
    let staged = String::from_utf8_lossy(&staged.stdout);
    assert!(staged.contains("+first hunk"));
    assert!(!staged.contains("+second hunk"));
    let unstaged = Command::new("git")
        .current_dir(&root)
        .args(["diff", "--unified=0", "--", "tracked"])
        .output()
        .unwrap();
    let unstaged = String::from_utf8_lossy(&unstaged.stdout);
    assert!(!unstaged.contains("+first hunk"));
    assert!(unstaged.contains("+second hunk"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn previews_staged_unstaged_and_untracked_files() {
    use gitview::worktree::load_diff;
    let root = std::env::temp_dir().join(format!(
        "gitview-tree-diff-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("tracked"), "base\n").unwrap();
    git(&["add", "--", "tracked"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::write(root.join("tracked"), "staged\n").unwrap();
    git(&["add", "--", "tracked"]);
    fs::write(root.join("tracked"), "unstaged\n").unwrap();
    fs::write(root.join("-new file"), "untracked\n").unwrap();
    let entries = load_status(&root).unwrap();
    let find = |section| {
        entries
            .iter()
            .find(|entry| entry.section == section)
            .unwrap()
    };
    assert!(
        load_diff(&root, find(Section::Staged))
            .unwrap()
            .contains("+staged")
    );
    assert!(
        load_diff(&root, find(Section::Unstaged))
            .unwrap()
            .contains("+unstaged")
    );
    assert!(
        load_diff(&root, find(Section::Untracked))
            .unwrap()
            .contains("+untracked")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unstages_both_sides_of_a_selected_rename() {
    use gitview::worktree::unstage;
    let root = std::env::temp_dir().join(format!(
        "gitview-rename-stage-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("old name"), "content\n").unwrap();
    git(&["add", "--", "old name"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::rename(root.join("old name"), root.join("new name")).unwrap();
    git(&["add", "-A"]);
    let entries = load_status(&root).unwrap();
    let renamed = entries
        .iter()
        .find(|entry| entry.section == Section::Staged && entry.path == "new name")
        .unwrap();
    let preview = gitview::worktree::load_diff(&root, renamed).unwrap();
    assert!(preview.contains("rename from old name") && preview.contains("rename to new name"));
    unstage(&root, renamed).unwrap();
    let entries = load_status(&root).unwrap();
    assert!(!entries.iter().any(|entry| entry.section == Section::Staged));
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Unstaged && entry.path == "old name")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.section == Section::Untracked && entry.path == "new name")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stages_and_unstages_a_tracked_deletion() {
    use gitview::worktree::{stage, unstage};
    let root = std::env::temp_dir().join(format!(
        "gitview-delete-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("gone"), "content\n").unwrap();
    git(&["add", "--", "gone"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::remove_file(root.join("gone")).unwrap();
    let entries = load_status(&root).unwrap();
    stage(
        &root,
        entries
            .iter()
            .find(|e| e.section == Section::Unstaged)
            .unwrap(),
    )
    .unwrap();
    let entries = load_status(&root).unwrap();
    unstage(
        &root,
        entries
            .iter()
            .find(|e| e.section == Section::Staged)
            .unwrap(),
    )
    .unwrap();
    assert!(
        load_status(&root)
            .unwrap()
            .iter()
            .any(|e| e.section == Section::Unstaged && e.path == "gone")
    );
    assert!(!root.join("gone").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_a_stale_selection_without_changing_the_index() {
    use gitview::worktree::stage;
    let root = std::env::temp_dir().join(format!(
        "gitview-stale-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    fs::write(root.join("new"), "content").unwrap();
    let entry = load_status(&root).unwrap().remove(0);
    fs::remove_file(root.join("new")).unwrap();
    assert!(stage(&root, &entry).unwrap_err().contains("status changed"));
    assert!(load_status(&root).unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_to_unstage_a_file_that_is_no_longer_staged() {
    use gitview::worktree::unstage;
    let root = std::env::temp_dir().join(format!(
        "gitview-stale-unstage-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(args)
                .status()
                .unwrap()
                .success()
        )
    };
    git(&["init", "-q"]);
    fs::write(root.join("tracked"), "base\n").unwrap();
    git(&["add", "--", "tracked"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "base",
    ]);
    fs::write(root.join("tracked"), "changed\n").unwrap();
    git(&["add", "--", "tracked"]);
    let selected = load_status(&root)
        .unwrap()
        .into_iter()
        .find(|entry| entry.section == Section::Staged)
        .unwrap();

    git(&["restore", "--staged", "--", "tracked"]);
    assert!(
        unstage(&root, &selected)
            .unwrap_err()
            .contains("status changed")
    );
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["diff", "--cached", "--quiet"])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(fs::read(root.join("tracked")).unwrap(), b"changed\n");
    fs::remove_dir_all(root).unwrap();
}
