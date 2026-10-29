//! The repository watcher: which file-system events matter to Gitview's views,
//! and how a burst of them is reported.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use gitview::watch::{Change, Kind, RepoLayout, RepoWatcher, change_for, classify};
use notify::{
    Event, EventKind,
    event::{AccessKind, AccessMode, CreateKind, Flag, ModifyKind},
};

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "init"]);
    dir
}

/// Everything reported until the watcher has been quiet for `quiet`.
fn collect(watcher: &RepoWatcher, quiet: Duration) -> Vec<Change> {
    let mut changes = Vec::new();
    while let Some(change) = watcher.recv_timeout(quiet) {
        changes.push(change);
    }
    changes
}

fn merged(changes: &[Change]) -> Change {
    changes.iter().fold(Change::default(), |mut all, change| {
        all.merge(*change);
        all
    })
}

const QUIET: Duration = Duration::from_millis(700);

// ---- classification -------------------------------------------------------

fn layout(root: &str, git_dir: &str, common_dir: &str) -> RepoLayout {
    RepoLayout::new(root.into(), git_dir.into(), common_dir.into())
}

#[test]
fn git_internals_matter_only_when_they_change_what_the_views_show() {
    let layout = layout("/repo", "/repo/.git", "/repo/.git");
    let kind = |path: &str| classify(Path::new(path), &layout);

    assert_eq!(kind("/repo/.git/HEAD"), Some(Kind::Refs));
    assert_eq!(kind("/repo/.git/refs/heads/main"), Some(Kind::Refs));
    assert_eq!(kind("/repo/.git/refs/tags/v1"), Some(Kind::Refs));
    assert_eq!(kind("/repo/.git/packed-refs"), Some(Kind::Refs));
    assert_eq!(kind("/repo/.git/index"), Some(Kind::Index));

    assert_eq!(kind("/repo/.git/index.lock"), None);
    assert_eq!(kind("/repo/.git/HEAD.lock"), None);
    assert_eq!(kind("/repo/.git/refs/heads/main.lock"), None);
    assert_eq!(kind("/repo/.git/objects/ab/cdef"), None);
    assert_eq!(kind("/repo/.git/logs/HEAD"), None);
    assert_eq!(kind("/repo/.git/FETCH_HEAD"), None);
    assert_eq!(kind("/repo/.git/config"), None);
    assert_eq!(kind("/repo/.git"), None);
}

#[test]
fn files_that_only_look_like_git_internals_are_worktree_changes() {
    let layout = layout("/repo", "/repo/.git", "/repo/.git");
    let kind = |path: &str| classify(Path::new(path), &layout);

    assert_eq!(kind("/repo/src/main.rs"), Some(Kind::Worktree));
    assert_eq!(kind("/repo/.gitignore"), Some(Kind::Worktree));
    assert_eq!(kind("/repo/.github/workflows/ci.yml"), Some(Kind::Worktree));
    assert_eq!(kind("/repo/Cargo.lock"), Some(Kind::Worktree));
    // A nested repository's files are ordinary files of this one.
    assert_eq!(kind("/repo/vendor/lib/.git/HEAD"), Some(Kind::Worktree));
    assert_eq!(kind("/elsewhere/file"), None);
}

#[test]
fn a_linked_worktree_watches_its_own_files_and_the_shared_refs() {
    let layout = layout("/linked", "/main/.git/worktrees/linked", "/main/.git");
    let kind = |path: &str| classify(Path::new(path), &layout);

    assert_eq!(kind("/linked/src/lib.rs"), Some(Kind::Worktree));
    assert_eq!(kind("/main/.git/worktrees/linked/HEAD"), Some(Kind::Refs));
    assert_eq!(kind("/main/.git/worktrees/linked/index"), Some(Kind::Index));
    assert_eq!(kind("/main/.git/refs/heads/feature"), Some(Kind::Refs));
    assert_eq!(kind("/main/.git/packed-refs"), Some(Kind::Refs));
    // The main checkout's own HEAD and index are not this worktree's.
    assert_eq!(kind("/main/.git/HEAD"), None);
    assert_eq!(kind("/main/.git/index"), None);
    assert_eq!(kind("/main/.git/objects/ab/cd"), None);
}

#[test]
fn the_layout_of_a_real_repository_is_found() {
    let dir = repository();
    let root = dir.path().canonicalize().unwrap();

    let layout = RepoLayout::discover(dir.path()).unwrap();

    assert_eq!(layout.root(), root);
    assert_eq!(layout.git_dir(), root.join(".git"));
    assert_eq!(layout.common_dir(), root.join(".git"));
}

#[test]
fn the_layout_of_a_linked_worktree_points_at_the_shared_directory() {
    let dir = repository();
    let holder = tempfile::tempdir().unwrap();
    let linked = holder.path().join("linked-tree");
    git(
        dir.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            linked.to_str().unwrap(),
        ],
    );
    let linked = linked.canonicalize().unwrap();
    let main = dir.path().canonicalize().unwrap();

    let layout = RepoLayout::discover(&linked).unwrap();

    assert_eq!(layout.root(), linked);
    assert_eq!(layout.git_dir(), main.join(".git/worktrees/linked-tree"));
    assert_eq!(layout.common_dir(), main.join(".git"));
}

#[test]
fn a_folder_that_is_not_a_repository_has_no_layout() {
    let dir = tempfile::tempdir().unwrap();

    assert!(RepoLayout::discover(dir.path()).is_err());
}

// ---- reporting ------------------------------------------------------------

#[test]
fn editing_a_file_is_reported_as_a_worktree_change() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();

    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();

    let change = watcher
        .recv_timeout(Duration::from_secs(2))
        .expect("an edit must be reported");
    assert!(change.worktree);
    assert!(!change.refs);
    assert!(collect(&watcher, QUIET).iter().all(|c| !c.refs));
}

#[test]
fn creating_and_deleting_files_and_folders_is_reported() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();

    fs::create_dir_all(dir.path().join("a/b")).unwrap();
    fs::write(dir.path().join("a/b/new.txt"), "x").unwrap();
    assert!(
        watcher
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .worktree
    );
    collect(&watcher, QUIET);

    fs::remove_file(dir.path().join("a/b/new.txt")).unwrap();
    assert!(
        watcher
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .worktree
    );
    collect(&watcher, QUIET);

    fs::remove_dir_all(dir.path().join("a")).unwrap();
    assert!(
        watcher
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .worktree
    );
}

#[test]
fn a_folder_created_after_the_watcher_started_is_watched_too() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();
    fs::create_dir_all(dir.path().join("later/deeper")).unwrap();
    collect(&watcher, QUIET);

    fs::write(dir.path().join("later/deeper/file.txt"), "x").unwrap();

    assert!(
        watcher
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .worktree
    );
}

#[test]
fn a_burst_of_edits_is_reported_as_a_few_changes_not_one_per_write() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();

    for index in 0..300 {
        fs::write(dir.path().join("tracked.txt"), format!("{index}\n")).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }

    let changes = collect(&watcher, QUIET);
    assert!(!changes.is_empty());
    assert!(
        changes.len() <= 4,
        "{} changes for one burst",
        changes.len()
    );
    assert!(merged(&changes).worktree);
}

#[test]
fn edits_that_never_pause_are_still_reported_promptly() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();
    let path = dir.path().join("tracked.txt");
    let writer = std::thread::spawn({
        let path = path.clone();
        move || {
            let started = Instant::now();
            let mut index = 0;
            while started.elapsed() < Duration::from_millis(2500) {
                fs::write(&path, format!("{index}\n")).unwrap();
                index += 1;
                std::thread::sleep(Duration::from_millis(15));
            }
        }
    });

    // The edits never leave a quiet gap, yet the first report must not wait
    // for one: the longest a change can be held back is bounded.
    let started = Instant::now();
    let first = watcher.recv_timeout(Duration::from_millis(1500));
    let waited = started.elapsed();
    let second = watcher.recv_timeout(Duration::from_millis(1500));
    writer.join().unwrap();

    assert!(first.is_some(), "nothing reported during constant edits");
    assert!(
        waited < Duration::from_millis(900),
        "first report took {waited:?}"
    );
    assert!(second.is_some(), "reporting stopped during constant edits");
}

#[test]
fn git_internals_that_change_no_view_are_not_reported() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();

    // Writes loose objects, a log entry, and lock files: all under .git.
    fs::write(dir.path().join("../outside-input"), "data").ok();
    git(dir.path(), &["hash-object", "-w", "tracked.txt"]);
    fs::write(dir.path().join(".git/FETCH_HEAD"), "x").unwrap();
    fs::write(dir.path().join(".git/index.lock"), "").unwrap();
    fs::remove_file(dir.path().join(".git/index.lock")).unwrap();
    fs::create_dir_all(dir.path().join(".git/logs")).unwrap();
    fs::write(dir.path().join(".git/logs/HEAD"), "x").unwrap();

    assert!(
        collect(&watcher, QUIET).is_empty(),
        "internals produced a change"
    );
}

#[test]
fn staging_is_an_index_change_and_committing_is_a_refs_change() {
    let dir = repository();
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    let watcher = RepoWatcher::start(dir.path()).unwrap();

    git(dir.path(), &["add", "tracked.txt"]);
    let staged = merged(&collect(&watcher, QUIET));
    assert!(staged.index, "{staged:?}");
    assert!(
        !staged.worktree,
        "staging touches no working file: {staged:?}"
    );
    assert!(!staged.refs);

    git(dir.path(), &["commit", "-q", "-m", "second"]);
    let committed = merged(&collect(&watcher, QUIET));
    assert!(committed.refs, "{committed:?}");
}

#[test]
fn branches_tags_checkouts_and_packed_refs_are_refs_changes() {
    let dir = repository();
    let watcher = RepoWatcher::start(dir.path()).unwrap();

    for args in [
        &["branch", "feature"][..],
        &["tag", "v1"][..],
        &["checkout", "-q", "feature"][..],
        &["pack-refs", "--all"][..],
        &["branch", "-q", "-D", "main"][..],
    ] {
        git(dir.path(), args);
        let change = merged(&collect(&watcher, QUIET));
        assert!(change.refs, "git {args:?} was not reported: {change:?}");
    }
}

#[test]
fn a_linked_worktree_hears_about_commits_made_in_the_main_checkout() {
    let dir = repository();
    let holder = tempfile::tempdir().unwrap();
    let linked = holder.path().join("linked-for-watch");
    git(
        dir.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            linked.to_str().unwrap(),
        ],
    );
    let linked = linked.canonicalize().unwrap();
    let watcher = RepoWatcher::start(&linked).unwrap();

    // A new branch in the main checkout moves a shared ref.
    git(dir.path(), &["branch", "created-elsewhere"]);
    let shared = merged(&collect(&watcher, QUIET));
    assert!(shared.refs, "{shared:?}");

    // Files of the main checkout are not this worktree's files.
    fs::write(dir.path().join("only-in-main.txt"), "x").unwrap();
    assert!(collect(&watcher, QUIET).is_empty());

    // Its own files and commits are.
    fs::write(linked.join("mine.txt"), "x").unwrap();
    assert!(merged(&collect(&watcher, QUIET)).worktree);
    git(&linked, &["add", "-A"]);
    git(&linked, &["commit", "-q", "-m", "in the linked tree"]);
    assert!(merged(&collect(&watcher, QUIET)).refs);
}

#[test]
fn a_repository_reached_through_a_symbolic_link_is_watched() {
    let dir = repository();
    let holder = tempfile::tempdir().unwrap();
    let link: PathBuf = holder.path().join("via-link");
    std::os::unix::fs::symlink(dir.path(), &link).unwrap();
    let watcher = RepoWatcher::start(&link).unwrap();

    fs::write(dir.path().join("tracked.txt"), "changed\n").unwrap();

    assert!(
        watcher
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .worktree
    );
}

#[test]
fn a_large_tree_starts_as_fast_as_a_small_one_and_a_flood_of_new_files_is_coalesced() {
    // Starting costs a fixed amount (the first in a process more), so the size
    // of the tree must not matter. Compared with a small repository started a
    // moment before, so a busy machine slows both alike.
    let small_repository = repository();
    let small_started = Instant::now();
    drop(RepoWatcher::start(small_repository.path()).unwrap());
    let small = small_started.elapsed();

    let dir = repository();
    for folder in 0..150 {
        let path = dir.path().join(format!("dir{folder}/nested"));
        fs::create_dir_all(&path).unwrap();
        for file in 0..20 {
            fs::write(path.join(format!("file{file}.txt")), "x").unwrap();
        }
    }

    let started = Instant::now();
    let watcher = RepoWatcher::start(dir.path()).unwrap();
    let large = started.elapsed();
    assert!(
        large < small + Duration::from_millis(1500),
        "3,000 files took {large:?} to start; a small repository took {small:?}"
    );

    fs::write(dir.path().join("dir7/nested/file3.txt"), "changed").unwrap();
    assert!(
        watcher
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .worktree
    );
    collect(&watcher, QUIET);

    for folder in 0..150 {
        for file in 20..40 {
            fs::write(
                dir.path()
                    .join(format!("dir{folder}/nested/file{file}.txt")),
                "new",
            )
            .unwrap();
        }
    }
    let changes = collect(&watcher, QUIET);
    assert!(!changes.is_empty());
    assert!(
        changes.len() <= 12,
        "{} changes for 3000 new files",
        changes.len()
    );
}

#[test]
fn watching_a_folder_that_is_not_a_repository_fails_with_a_message() {
    let dir = tempfile::tempdir().unwrap();

    let error = RepoWatcher::start(dir.path()).err().expect("must fail");

    assert!(!error.is_empty());
}

#[test]
fn dropping_a_watcher_returns_promptly_and_stops_it() {
    let dir = repository();

    for _ in 0..10 {
        let watcher = RepoWatcher::start(dir.path()).unwrap();
        fs::write(dir.path().join("tracked.txt"), "x\n").unwrap();
        let dropping = Instant::now();
        drop(watcher);
        assert!(
            // Dropping takes under a millisecond alone; this only has to tell a
            // hang, which never ends, from a busy machine.
            dropping.elapsed() < Duration::from_secs(5),
            "dropping took {:?}",
            dropping.elapsed()
        );
    }
}

// ---- turning events into changes -----------------------------------------

fn event(kind: EventKind, path: &str) -> notify::Result<Event> {
    Ok(Event::new(kind).add_path(path.into()))
}

fn plain_layout() -> RepoLayout {
    layout("/repo", "/repo/.git", "/repo/.git")
}

#[test]
fn reading_a_file_is_not_a_change() {
    // Linux reports every read, including those of Gitview's own `git status`.
    let layout = plain_layout();
    for kind in [
        EventKind::Access(AccessKind::Open(AccessMode::Any)),
        EventKind::Access(AccessKind::Close(AccessMode::Read)),
        EventKind::Access(AccessKind::Any),
    ] {
        assert!(change_for(event(kind, "/repo/src/main.rs"), &layout).is_empty());
        assert!(change_for(event(kind, "/repo/.git/index"), &layout).is_empty());
    }
}

#[test]
fn writes_creations_and_removals_are_changes() {
    let layout = plain_layout();

    let modified = change_for(
        event(EventKind::Modify(ModifyKind::Any), "/repo/a.rs"),
        &layout,
    );
    assert_eq!(
        modified,
        Change {
            worktree: true,
            ..Change::default()
        }
    );

    let created = change_for(
        event(EventKind::Create(CreateKind::File), "/repo/b.rs"),
        &layout,
    );
    assert!(created.worktree);

    let removed = change_for(
        event(
            EventKind::Remove(notify::event::RemoveKind::Any),
            "/repo/b.rs",
        ),
        &layout,
    );
    assert!(removed.worktree);

    let staged = change_for(
        event(EventKind::Modify(ModifyKind::Any), "/repo/.git/index"),
        &layout,
    );
    assert_eq!(
        staged,
        Change {
            index: true,
            ..Change::default()
        }
    );
}

#[test]
fn an_event_for_a_path_nobody_shows_is_nothing() {
    let layout = plain_layout();

    assert!(
        change_for(
            event(
                EventKind::Modify(ModifyKind::Any),
                "/repo/.git/objects/ab/cd"
            ),
            &layout
        )
        .is_empty()
    );
    assert!(
        change_for(
            event(EventKind::Create(CreateKind::File), "/repo/.git/index.lock"),
            &layout
        )
        .is_empty()
    );
}

#[test]
fn lost_events_and_errors_mean_anything_may_have_changed() {
    let layout = plain_layout();

    let rescan = Ok(Event::new(EventKind::Any).set_flag(Flag::Rescan));
    assert_eq!(change_for(rescan, &layout), Change::everything());

    let error = Err(notify::Error::generic("watch limit reached"));
    assert_eq!(change_for(error, &layout), Change::everything());
}

#[test]
fn one_event_naming_several_paths_reports_all_of_them() {
    // A rename names both ends.
    let layout = plain_layout();
    let rename = Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        notify::event::RenameMode::Both,
    )))
    .add_path("/repo/.git/index.lock".into())
    .add_path("/repo/.git/index".into()));

    assert_eq!(
        change_for(rename, &layout),
        Change {
            index: true,
            ..Change::default()
        }
    );
}
