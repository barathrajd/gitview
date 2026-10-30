//! Gitview's Git writes run one at a time, and wait out a lock that another
//! Git process, such as an agent's `git add`, holds on the repository.

use gitview::{
    worktree::{self, Entry, Section},
    writes::{self, RETRY_FOR},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn repository() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "gitview-writes-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    fs::write(root.join("base.txt"), "base\n").unwrap();
    git(&root, &["add", "base.txt"]);
    git(&root, &["commit", "-qm", "base"]);
    root
}

fn git(root: &Path, args: &[&str]) -> std::process::Output {
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
    output
}

fn untracked(path: &str) -> Entry {
    Entry {
        section: Section::Untracked,
        path: path.into(),
        original: None,
    }
}

#[test]
fn writes_never_overlap() {
    let running = Arc::new(AtomicUsize::new(0));
    let most = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let (running, most) = (Arc::clone(&running), Arc::clone(&most));
            thread::spawn(move || {
                writes::serialized(|| {
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    most.fetch_max(now, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(10));
                    running.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, String>(())
                })
                .unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(most.load(Ordering::SeqCst), 1);
}

#[test]
fn a_write_waits_for_another_processs_index_lock() {
    let root = repository();
    fs::write(root.join("new.txt"), "x\n").unwrap();
    let lock = root.join(".git/index.lock");
    fs::write(&lock, "").unwrap();
    let release = {
        let lock = lock.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            fs::remove_file(lock).unwrap();
        })
    };

    let started = Instant::now();
    let result = writes::serialized(|| worktree::stage(&root, &untracked("new.txt")));

    release.join().unwrap();
    assert_eq!(result, Ok(()));
    assert!(started.elapsed() >= Duration::from_millis(250));
    let staged =
        String::from_utf8(git(&root, &["diff", "--cached", "--name-only"]).stdout).unwrap();
    assert_eq!(staged, "new.txt\n");
}

#[test]
fn a_lock_that_stays_is_reported_after_the_wait() {
    let root = repository();
    fs::write(root.join("new.txt"), "x\n").unwrap();
    fs::write(root.join(".git/index.lock"), "").unwrap();

    let started = Instant::now();
    let error = writes::serialized(|| worktree::stage(&root, &untracked("new.txt"))).unwrap_err();

    let waited = started.elapsed();
    assert!(waited >= RETRY_FOR, "{waited:?}");
    // Generous: the write also queues behind other tests' writes, since the
    // queue is shared by the whole process. It must still give up.
    assert!(waited < RETRY_FOR + Duration::from_secs(10), "{waited:?}");
    assert!(error.contains("Another Git command"), "{error}");
    assert!(error.contains("index.lock"), "{error}");
}

#[test]
fn other_errors_are_not_retried() {
    let calls = AtomicUsize::new(0);
    let started = Instant::now();

    let error = writes::serialized(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        Err::<(), _>("File status changed; refresh and try again".to_owned())
    })
    .unwrap_err();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(error, "File status changed; refresh and try again");
}

/// Starts an "agent" that runs `script` in a loop, as raw Git in its own
/// terminal would, until the returned flag is set. Returns how many rounds ran.
fn agent_loop(
    root: &Path,
    script: &'static [&'static [&'static str]],
) -> (
    Arc<std::sync::atomic::AtomicBool>,
    thread::JoinHandle<usize>,
) {
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let handle = {
        let (root, stop) = (root.to_owned(), Arc::clone(&stop));
        thread::spawn(move || {
            let mut rounds = 0;
            while !stop.load(Ordering::SeqCst) {
                for args in script {
                    // A failure on Gitview's lock is the agent's to retry.
                    let _ = Command::new("git").current_dir(&root).args(*args).output();
                }
                rounds += 1;
            }
            rounds
        })
    };
    (stop, handle)
}

fn staged_entry(path: &str) -> Entry {
    Entry {
        section: Section::Staged,
        path: path.into(),
        original: None,
    }
}

fn staged_names(root: &Path) -> Vec<String> {
    String::from_utf8(git(root, &["diff", "--cached", "--name-only"]).stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// While an agent keeps staging its own file, every Gitview stage and unstage
/// succeeds and the index ends exactly as Gitview's last write left it.
#[test]
fn stage_and_unstage_all_succeed_beside_an_agents_git_add() {
    let root = repository();
    fs::write(root.join("mine.txt"), "gitview\n").unwrap();
    fs::write(root.join("agent.txt"), "agent\n").unwrap();
    let (stop, agent) = agent_loop(&root, &[&["add", "agent.txt"], &["add", "-N", "agent.txt"]]);

    for _ in 0..20 {
        writes::serialized(|| worktree::stage(&root, &untracked("mine.txt"))).unwrap();
        writes::serialized(|| worktree::unstage(&root, &staged_entry("mine.txt"))).unwrap();
    }
    writes::serialized(|| worktree::stage(&root, &untracked("mine.txt"))).unwrap();
    stop.store(true, Ordering::SeqCst);
    let rounds = agent.join().unwrap();

    assert!(rounds > 5, "the agent barely ran: {rounds} rounds");
    assert!(staged_names(&root).contains(&"mine.txt".to_owned()));
}

/// An agent's `git reset <path>` can overwrite a change another process just
/// made to the index; Git itself loses that update, so Gitview can't prevent
/// it. What Gitview guarantees is that none of its writes fail on the agent's
/// lock, and that it notices the lost change instead of acting on stale state.
#[test]
fn no_write_fails_on_the_lock_of_an_agent_that_resets() {
    let root = repository();
    fs::write(root.join("mine.txt"), "gitview\n").unwrap();
    fs::write(root.join("agent.txt"), "agent\n").unwrap();
    let (stop, agent) = agent_loop(
        &root,
        &[&["add", "agent.txt"], &["reset", "-q", "agent.txt"]],
    );

    let mut succeeded = 0;
    for _ in 0..20 {
        for result in [
            writes::serialized(|| worktree::stage(&root, &untracked("mine.txt"))),
            writes::serialized(|| worktree::unstage(&root, &staged_entry("mine.txt"))),
        ] {
            match result {
                Ok(()) => succeeded += 1,
                Err(error) => assert_eq!(error, "File status changed; refresh and try again"),
            }
        }
    }
    stop.store(true, Ordering::SeqCst);
    let rounds = agent.join().unwrap();

    assert!(rounds > 5, "the agent barely ran: {rounds} rounds");
    assert!(succeeded >= 20, "only {succeeded} of 40 writes succeeded");
}
