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
