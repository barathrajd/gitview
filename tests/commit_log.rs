//! Parsed history pages: commits with parents, refs, author, and time from
//! NUL-delimited `git log`, loaded with a skip and a count.

#[path = "support/replay.rs"]
mod replay;

use gitview::commit_log::{Page, load_page};
use replay::Repo;

fn page(repository: &Repo, skip: usize, count: usize) -> Page {
    load_page(&repository.path(), None, skip, count).unwrap()
}

#[test]
fn parses_hashes_parents_refs_author_time_and_subject() {
    let repository = Repo::new();
    repository.write("a.txt", "a\n");
    repository.commit_all("First");
    repository.write("a.txt", "b\n");
    repository.commit_all("Second");
    repository.git(&["tag", "v1"]);

    let loaded = page(&repository, 0, 10);
    assert!(!loaded.more);
    let [second, first] = loaded.commits.as_slice() else {
        panic!("{:?}", loaded.commits);
    };
    assert_eq!(second.subject, "Second");
    assert_eq!(second.parents, std::slice::from_ref(&first.hash));
    assert!(first.parents.is_empty());
    assert_eq!(second.hash.len(), 40);
    assert_eq!(second.short_hash(), &second.hash[..7]);
    assert!(
        second.refs.iter().any(|r| r == "HEAD -> main"),
        "{:?}",
        second.refs
    );
    assert!(
        second.refs.iter().any(|r| r == "tag: v1"),
        "{:?}",
        second.refs
    );
    assert!(first.refs.is_empty());
    assert_eq!(second.author, "Test");
    assert!(second.time > 1_600_000_000, "{}", second.time);
}

#[test]
fn keeps_unusual_subjects_and_authors_intact() {
    let repository = Repo::new();
    let subject = "Tabs\tand | pipes, \"quotes\" and ünïcode — (HEAD -> fake)";
    repository.write("a.txt", "a\n");
    repository.git(&["add", "a.txt"]);
    repository.git(&[
        "-c",
        "user.name=Zoë O'Brien, Jr.",
        "-c",
        "user.email=z@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        subject,
    ]);
    let loaded = page(&repository, 0, 10);
    assert_eq!(loaded.commits[0].subject, subject);
    assert_eq!(loaded.commits[0].author, "Zoë O'Brien, Jr.");
}

#[test]
fn merges_list_every_parent_in_order() {
    let repository = Repo::new();
    repository.write("a.txt", "a\n");
    repository.commit_all("Base");
    for branch in ["one", "two"] {
        repository.git(&["switch", "-qc", branch, "main"]);
        repository.write(&format!("{branch}.txt"), "x\n");
        repository.commit_all(branch);
    }
    repository.git(&["switch", "-q", "main"]);
    repository.git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=t@e",
        "merge",
        "-q",
        "--no-ff",
        "--no-edit",
        "one",
        "two",
    ]);
    let commits = page(&repository, 0, 10).commits;
    let merge = &commits[0];
    let parent = |subject: &str| {
        commits
            .iter()
            .find(|c| c.subject == subject)
            .unwrap()
            .hash
            .clone()
    };
    assert_eq!(merge.parents.len(), 3, "{merge:?}");
    assert_eq!(merge.parents[1], parent("one"));
    assert_eq!(merge.parents[2], parent("two"));
}

#[test]
fn pages_skip_and_report_more() {
    let repository = Repo::new();
    for number in 1..=5 {
        repository.write("a.txt", &format!("{number}\n"));
        repository.commit_all(&format!("Commit {number}"));
    }
    let first = page(&repository, 0, 2);
    assert!(first.more);
    let subjects = |page: &Page| {
        page.commits
            .iter()
            .map(|c| c.subject.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(subjects(&first), ["Commit 5", "Commit 4"]);
    let last = page(&repository, 4, 2);
    assert!(!last.more);
    assert_eq!(subjects(&last), ["Commit 1"]);
}

#[test]
fn an_empty_repository_has_no_commits() {
    let repository = Repo::new();
    let loaded = page(&repository, 0, 10);
    assert!(loaded.commits.is_empty());
    assert!(!loaded.more);
}

#[test]
fn a_path_limits_the_page_and_rejects_parent_components() {
    let repository = Repo::new();
    repository.write("a.txt", "a\n");
    repository.commit_all("Touch a");
    repository.write("b.txt", "b\n");
    repository.commit_all("Touch b");
    let loaded = load_page(
        &repository.path(),
        Some(std::path::Path::new("a.txt")),
        0,
        10,
    )
    .unwrap();
    assert_eq!(loaded.commits.len(), 1);
    assert_eq!(loaded.commits[0].subject, "Touch a");

    // Parents skip commits the path filter hides.
    repository.write("a.txt", "a2\n");
    repository.commit_all("Touch a again");
    let loaded = load_page(
        &repository.path(),
        Some(std::path::Path::new("a.txt")),
        0,
        10,
    )
    .unwrap();
    let [again, first] = loaded.commits.as_slice() else {
        panic!("{:?}", loaded.commits);
    };
    assert_eq!(again.parents, std::slice::from_ref(&first.hash));
    assert!(first.parents.is_empty());
    assert!(
        load_page(
            &repository.path(),
            Some(std::path::Path::new("../x")),
            0,
            10
        )
        .is_err()
    );
}

#[test]
fn a_whole_history_with_merges_closes_every_lane() {
    let repository = Repo::new();
    repository.write("a.txt", "a\n");
    repository.commit_all("Base");
    for branch in ["one", "two", "three"] {
        repository.git(&["switch", "-qc", branch, "main"]);
        repository.write(&format!("{branch}.txt"), "x\n");
        repository.commit_all(branch);
    }
    repository.git(&["switch", "-q", "main"]);
    repository.write("main.txt", "m\n");
    repository.commit_all("Main moves on");
    repository.git(&["merge", "-q", "--no-ff", "--no-edit", "one"]);
    repository.git(&["merge", "-q", "--no-ff", "--no-edit", "two", "three"]);
    repository.git(&["tag", "v1"]);

    let commits = page(&repository, 0, 100).commits;
    let mut layout = gitview::graph::Layout::new();
    let rows: Vec<String> = commits
        .iter()
        .map(|commit| layout.push(commit).text(false))
        .collect();
    assert_eq!(layout.open_lanes(), 0, "{rows:#?}");
    assert_eq!(rows.len(), 7, "{rows:#?}");
    assert!(
        rows.iter().all(|row| row.matches('●').count() == 1),
        "{rows:#?}"
    );
    // The octopus merge opens two lanes.
    assert!(rows[0].starts_with("●─╮─╮"), "{rows:#?}");
}
