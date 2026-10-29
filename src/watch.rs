//! Tells the views when the repository changed under them.
//!
//! An agent edits files, stages, and commits while Gitview is open. The watcher
//! reports what kind of thing changed, once a burst of events has settled, so
//! the views can reload what is affected.
//!
//! Only events that can change what Gitview shows are kept: working files, the
//! index, and the refs (`HEAD`, `refs/**`, `packed-refs`). Everything else under
//! `.git` (objects, logs, lock files) is dropped in the file-system callback, so
//! it never wakes a thread. An idle watcher costs nothing.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// How long the repository must be quiet before a burst is reported.
const QUIET: Duration = Duration::from_millis(120);
/// The longest a change is held back, however long the burst goes on, so
/// constant edits can't postpone a refresh indefinitely.
const MAX_WAIT: Duration = Duration::from_millis(400);

/// The kind of thing one file-system event touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A working file or folder.
    Worktree,
    /// The staging area (`.git/index`).
    Index,
    /// `HEAD` or a branch, tag, or other ref.
    Refs,
}

/// What changed since the last report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    pub worktree: bool,
    pub index: bool,
    pub refs: bool,
}

impl Change {
    /// Everything may have changed, for when events were lost.
    pub fn everything() -> Self {
        Self {
            worktree: true,
            index: true,
            refs: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn merge(&mut self, other: Change) {
        self.worktree |= other.worktree;
        self.index |= other.index;
        self.refs |= other.refs;
    }

    fn add(&mut self, kind: Kind) {
        match kind {
            Kind::Worktree => self.worktree = true,
            Kind::Index => self.index = true,
            Kind::Refs => self.refs = true,
        }
    }
}

/// Where a repository's files live. For an ordinary checkout `git_dir` and
/// `common_dir` are the same `.git` folder; for a linked worktree `git_dir` is
/// its private folder and `common_dir` holds the refs shared with the others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoLayout {
    root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
}

impl RepoLayout {
    pub fn new(root: PathBuf, git_dir: PathBuf, common_dir: PathBuf) -> Self {
        Self {
            root,
            git_dir,
            common_dir,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// Asks Git where the repository containing `path` keeps its files. The
    /// paths are canonical, because the file-system events are: macOS temp
    /// folders, for one, sit behind a symbolic link.
    pub fn discover(path: &Path) -> Result<Self, String> {
        let output = crate::process::git_command()
            .current_dir(path)
            .args([
                "rev-parse",
                "--show-toplevel",
                "--absolute-git-dir",
                "--git-common-dir",
            ])
            .output()
            .map_err(|error| format!("Could not run Git: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Not a Git repository: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        let (Some(root), Some(git_dir), Some(common_dir)) =
            (lines.next(), lines.next(), lines.next())
        else {
            return Err("Git did not report the repository folders".into());
        };
        let resolve = |reported: &str| {
            let reported = Path::new(reported);
            let full = if reported.is_absolute() {
                reported.to_path_buf()
            } else {
                path.join(reported)
            };
            full.canonicalize()
                .map_err(|error| format!("Could not resolve {}: {error}", full.display()))
        };
        Ok(Self::new(
            resolve(root)?,
            resolve(git_dir)?,
            resolve(common_dir)?,
        ))
    }
}

fn is_lock_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "lock")
}

/// What `path` (relative to a git folder) changes, if anything Gitview shows.
fn git_dir_kind(relative: &Path, own_files: bool) -> Option<Kind> {
    if is_lock_file(relative) {
        return None;
    }
    if relative == Path::new("packed-refs") || relative.starts_with("refs") {
        return Some(Kind::Refs);
    }
    if own_files && relative == Path::new("HEAD") {
        return Some(Kind::Refs);
    }
    if own_files && relative == Path::new("index") {
        return Some(Kind::Index);
    }
    None
}

/// Decides whether an event at `path` matters to the views, and how.
pub fn classify(path: &Path, layout: &RepoLayout) -> Option<Kind> {
    if let Ok(relative) = path.strip_prefix(&layout.git_dir) {
        return git_dir_kind(relative, true);
    }
    if layout.common_dir != layout.git_dir
        && let Ok(relative) = path.strip_prefix(&layout.common_dir)
    {
        // Only the refs are shared; the other checkout's HEAD and index aren't ours.
        return git_dir_kind(relative, false);
    }
    path.starts_with(&layout.root).then_some(Kind::Worktree)
}

/// What one file-system event means for the views. Reads (`Access`) change
/// nothing, and Gitview's own Git commands cause plenty of them on Linux, so they
/// must not count. Lost events and errors mean anything may have changed.
pub fn change_for(result: notify::Result<Event>, layout: &RepoLayout) -> Change {
    let event = match result {
        Ok(event) => event,
        Err(_) => return Change::everything(),
    };
    if matches!(event.kind, EventKind::Access(_)) {
        return Change::default();
    }
    let mut change = if event.need_rescan() {
        Change::everything()
    } else {
        Change::default()
    };
    for path in &event.paths {
        if let Some(kind) = classify(path, layout) {
            change.add(kind);
        }
    }
    change
}

/// Watches a repository and reports debounced [`Change`]s.
pub struct RepoWatcher {
    changes: Receiver<Change>,
    // Dropping the watcher stops the callback, which ends the debounce thread.
    _watcher: RecommendedWatcher,
}

impl RepoWatcher {
    /// Starts watching the repository that contains `path`.
    pub fn start(path: &Path) -> Result<Self, String> {
        Self::start_with(path, QUIET, MAX_WAIT)
    }

    /// Like [`start`](Self::start), with the quiet gap and the longest hold-back
    /// chosen by the caller.
    pub fn start_with(path: &Path, quiet: Duration, max_wait: Duration) -> Result<Self, String> {
        let layout = RepoLayout::discover(path)?;
        let (raw_sender, raw_events) = mpsc::channel::<Change>();
        let callback_layout = layout.clone();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let change = change_for(result, &callback_layout);
            if !change.is_empty() {
                let _ = raw_sender.send(change);
            }
        })
        .map_err(|error| format!("Could not watch the repository: {error}"))?;

        let watch = |watcher: &mut RecommendedWatcher, folder: &Path, mode| {
            watcher
                .watch(folder, mode)
                .map_err(|error| format!("Could not watch {}: {error}", folder.display()))
        };
        watch(&mut watcher, layout.root(), RecursiveMode::Recursive)?;
        if !layout.git_dir().starts_with(layout.root()) {
            watch(&mut watcher, layout.git_dir(), RecursiveMode::NonRecursive)?;
        }
        if layout.common_dir() != layout.git_dir()
            && !layout.common_dir().starts_with(layout.root())
        {
            watch(
                &mut watcher,
                layout.common_dir(),
                RecursiveMode::NonRecursive,
            )?;
            let refs = layout.common_dir().join("refs");
            if refs.is_dir() {
                watch(&mut watcher, &refs, RecursiveMode::Recursive)?;
            }
        }

        let (sender, changes) = mpsc::channel();
        thread::spawn(move || debounce(raw_events, sender, quiet, max_wait));
        Ok(Self {
            changes,
            _watcher: watcher,
        })
    }

    /// A change that has settled, if one is waiting.
    pub fn try_recv(&self) -> Option<Change> {
        self.changes.try_recv().ok()
    }

    /// Waits up to `timeout` for a change that has settled.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Change> {
        self.changes.recv_timeout(timeout).ok()
    }
}

/// Merges events that arrive close together. It blocks while nothing happens, so
/// an idle repository costs no CPU. After the first event it waits for `quiet`
/// without another, but never longer than `max_wait` in total.
fn debounce(
    events: Receiver<Change>,
    reports: mpsc::Sender<Change>,
    quiet: Duration,
    max_wait: Duration,
) {
    while let Ok(first) = events.recv() {
        let mut change = first;
        let started = Instant::now();
        loop {
            let left = max_wait.saturating_sub(started.elapsed());
            if left.is_zero() {
                break;
            }
            match events.recv_timeout(quiet.min(left)) {
                Ok(more) => change.merge(more),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => {
                    let _ = reports.send(change);
                    return;
                }
            }
        }
        if reports.send(change).is_err() {
            return;
        }
    }
}
