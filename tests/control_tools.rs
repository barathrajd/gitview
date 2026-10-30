use gitview::control::tools::{MAX_LOG, MAX_OUTPUT, call};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gitview-control-tools-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();
    git(&path, &["init", "-q"]);
    path
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}

fn commit_file(root: &Path, name: &str, text: &str, message: &str) {
    fs::write(root.join(name), text).unwrap();
    git(root, &["add", "--", name]);
    git(root, &["commit", "-qm", message]);
}

fn json_of(root: &Path, tool: &str, args: Value) -> Value {
    let output = call(root, tool, &args).unwrap();
    assert!(!output.truncated);
    serde_json::from_str(&output.text).unwrap()
}

#[test]
fn get_status_lists_each_section_with_unusual_names() {
    let root = fixture();
    commit_file(&root, "tracked file.txt", "one\n", "base");
    fs::write(root.join("tracked file.txt"), "two\n").unwrap();
    fs::write(root.join("staged é.txt"), "new\n").unwrap();
    git(&root, &["add", "--", "staged é.txt"]);
    fs::write(root.join("untracked\tname"), "x\n").unwrap();

    let status = json_of(&root, "get_status", json!({}));

    assert_eq!(
        status["files"],
        json!([
            {"section": "staged", "path": "staged é.txt"},
            {"section": "unstaged", "path": "tracked file.txt"},
            {"section": "untracked", "path": "untracked\tname"},
        ])
    );
}

#[test]
fn get_log_defaults_to_fifty_and_caps_the_limit() {
    let root = fixture();
    for index in 0..(MAX_LOG + 5) {
        git(
            &root,
            &["commit", "-q", "--allow-empty", "-m", &format!("c{index}")],
        );
    }

    let default = json_of(&root, "get_log", json!({}));
    let capped = json_of(&root, "get_log", json!({"limit": 10_000}));
    let small = json_of(&root, "get_log", json!({"limit": 2}));

    assert_eq!(default["commits"].as_array().unwrap().len(), 50);
    assert_eq!(capped["commits"].as_array().unwrap().len(), MAX_LOG);
    let commits = small["commits"].as_array().unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0]["subject"], format!("c{}", MAX_LOG + 4));
    assert_eq!(commits[0]["author"], "Test");
    assert_eq!(commits[0]["id"].as_str().unwrap().len(), 40);
}

#[test]
fn get_diff_shows_staged_and_unstaged_changes() {
    let root = fixture();
    commit_file(&root, "a.txt", "one\n", "base");
    fs::write(root.join("a.txt"), "two\n").unwrap();
    git(&root, &["add", "a.txt"]);
    fs::write(root.join("a.txt"), "three\n").unwrap();

    let unstaged = call(&root, "get_diff", &json!({"path": "a.txt"})).unwrap();
    let staged = call(&root, "get_diff", &json!({"path": "a.txt", "staged": true})).unwrap();

    assert!(unstaged.text.contains("-two\n+three"), "{}", unstaged.text);
    assert!(staged.text.contains("-one\n+two"), "{}", staged.text);
}

#[test]
fn get_show_returns_a_commit_with_its_patch() {
    let root = fixture();
    commit_file(&root, "a.txt", "one\n", "first change");

    let show = call(&root, "get_show", &json!({"revision": "HEAD"})).unwrap();

    assert!(show.text.contains("first change"), "{}", show.text);
    assert!(show.text.contains("+one"), "{}", show.text);
}

#[test]
fn paths_outside_the_repository_are_refused() {
    let root = fixture();
    commit_file(&root, "a.txt", "one\n", "base");
    for path in ["../secret", "/etc/passwd", "", "a/../../b"] {
        let error = call(&root, "get_diff", &json!({"path": path})).unwrap_err();
        assert!(error.contains("repository-relative"), "{path}: {error}");
    }
}

#[test]
fn revisions_that_look_like_options_are_refused() {
    let root = fixture();
    commit_file(&root, "a.txt", "one\n", "base");
    let target = root.join("written");
    let option = format!("--output={}", target.display());

    let error = call(&root, "get_show", &json!({"revision": option})).unwrap_err();

    assert!(error.contains("revision"), "{error}");
    assert!(!target.exists());
    assert!(call(&root, "get_show", &json!({"revision": "no-such-ref"})).is_err());
}

#[test]
fn output_is_capped_and_marked_truncated() {
    let root = fixture();
    let line = "x".repeat(99) + "\n";
    commit_file(&root, "big.txt", &line.repeat(4000), "big");

    let show = call(&root, "get_show", &json!({"revision": "HEAD"})).unwrap();

    assert!(show.truncated);
    assert!(show.text.len() <= MAX_OUTPUT, "{}", show.text.len());
}

#[test]
fn bad_arguments_and_unknown_tools_are_errors() {
    let root = fixture();
    assert!(
        call(&root, "stage_all", &json!({}))
            .unwrap_err()
            .contains("Unknown tool")
    );
    assert!(call(&root, "get_diff", &json!({})).is_err());
    assert!(call(&root, "get_diff", &json!({"path": 3})).is_err());
    assert!(call(&root, "get_log", &json!({"limit": "ten"})).is_err());
    assert!(call(&root, "get_log", &json!([])).is_err());
}

#[test]
fn get_log_in_an_empty_repository_is_empty() {
    let root = fixture();
    assert_eq!(json_of(&root, "get_log", json!({}))["commits"], json!([]));
}
