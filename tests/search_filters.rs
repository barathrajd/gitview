use gitview::history::commit_id;
use gitview::search_query::{Filtered, parse_query, run_filtered};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Repo(PathBuf);

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(root: &Path, envs: &[(&str, &str)], args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .envs(envs.iter().copied())
        .args(["-c", "user.name=Test", "-c", "user.email=t@example.com"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// Four commits on `main` and one on `side`:
/// 2026-01-01 Alice Smith "fix login" (src/login.rs)
/// 2026-02-01 Bob         "fix logout" (src/logout.rs)
/// 2026-03-01 Bob         "feat profile" (docs/my notes.md)
/// 2026-04-01 Carol       "chore tidy" (README)
/// side: 2026-05-01 Dave  "side work" (side.txt)
fn sample() -> Repo {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("gitview-search-filters-{nanos}"));
    std::fs::create_dir(&root).unwrap();
    git(&root, &[], &["init", "-q", "-b", "main"]);
    let commit = |author: &str, date: &str, path: &str, message: &str| {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, message).unwrap();
        git(&root, &[], &["add", "--", path]);
        git(
            &root,
            &[
                ("GIT_AUTHOR_NAME", author),
                ("GIT_AUTHOR_EMAIL", "a@example.com"),
                ("GIT_AUTHOR_DATE", date),
                ("GIT_COMMITTER_DATE", date),
            ],
            &["commit", "-q", "-m", message],
        );
    };
    commit(
        "Alice Smith",
        "2026-01-01T12:00:00Z",
        "src/login.rs",
        "fix login",
    );
    commit("Bob", "2026-02-01T12:00:00Z", "src/logout.rs", "fix logout");
    commit(
        "Bob",
        "2026-03-01T12:00:00Z",
        "docs/my notes.md",
        "feat profile",
    );
    commit("Carol", "2026-04-01T12:00:00Z", "README", "chore tidy");
    git(&root, &[], &["switch", "-q", "-c", "side"]);
    commit("Dave", "2026-05-01T12:00:00Z", "side.txt", "side work");
    git(&root, &[], &["switch", "-q", "main"]);
    Repo(root)
}

fn subjects(repo: &Repo, query: &str) -> Vec<String> {
    let found = run_filtered(&repo.0, &parse_query(query), 1000).unwrap();
    assert!(!found.truncated);
    found
        .rows
        .iter()
        .map(|row| {
            assert!(commit_id(row).is_some(), "{row}");
            let shown = row.split('\x1f').next().unwrap();
            let subject = shown.rsplit_once(" - ").unwrap().0;
            let words: Vec<&str> = subject.split_whitespace().collect();
            words[words.len() - 2..].join(" ")
        })
        .collect()
}

#[test]
fn author_filter_matches_part_of_a_name_ignoring_case() {
    assert_eq!(subjects(&sample(), "author:alice"), ["fix login"]);
}

#[test]
fn quoted_author_keeps_the_space() {
    assert_eq!(subjects(&sample(), "author:\"alice smith\""), ["fix login"]);
}

#[test]
fn repeated_authors_are_either() {
    assert_eq!(
        subjects(&sample(), "author:alice author:carol"),
        ["chore tidy", "fix login"]
    );
}

#[test]
fn msg_filter_searches_messages() {
    assert_eq!(subjects(&sample(), "msg:FIX"), ["fix logout", "fix login"]);
}

#[test]
fn different_filters_are_both() {
    assert_eq!(subjects(&sample(), "author:bob msg:fix"), ["fix logout"]);
}

#[test]
fn path_filter_keeps_commits_touching_the_path() {
    assert_eq!(subjects(&sample(), "path:src"), ["fix logout", "fix login"]);
    assert_eq!(
        subjects(&sample(), "path:\"docs/my notes.md\""),
        ["feat profile"]
    );
}

#[test]
fn path_filter_is_literal() {
    assert!(subjects(&sample(), "path:src/*.rs").is_empty());
}

#[test]
fn since_and_until_bound_the_dates() {
    assert_eq!(
        subjects(&sample(), "since:2026-01-15 until:2026-03-15"),
        ["feat profile", "fix logout"]
    );
}

#[test]
fn filters_search_every_branch_by_default() {
    assert_eq!(subjects(&sample(), "author:dave"), ["side work"]);
}

#[test]
fn ref_filter_narrows_to_one_branch() {
    assert!(subjects(&sample(), "author:dave ref:main").is_empty());
    assert_eq!(subjects(&sample(), "author:dave ref:side"), ["side work"]);
}

#[test]
fn unknown_ref_reports_git_error() {
    let error = run_filtered(&sample().0, &parse_query("ref:nosuch"), 1000).unwrap_err();
    assert!(error.contains("nosuch"), "{error}");
}

#[test]
fn hitting_the_limit_is_reported() {
    let repo = sample();
    let Filtered { rows, truncated } = run_filtered(&repo.0, &parse_query("ref:main"), 3).unwrap();
    assert_eq!(rows.len(), 3);
    assert!(truncated);
    let Filtered { rows, truncated } = run_filtered(&repo.0, &parse_query("ref:main"), 4).unwrap();
    assert_eq!(rows.len(), 4);
    assert!(!truncated);
}
