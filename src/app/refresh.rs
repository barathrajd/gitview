//! Keeps the views current while the repository changes underneath them.
//!
//! The watcher says what changed (working files, the index, or refs). Each view
//! that depends on it has a [`Slot`]: a request to reload it, and the id of the
//! reload in flight. A burst of changes while a reload runs becomes one more
//! reload, never a queue, and a slot waits as long as its last reload took
//! before starting another, so constant edits can't keep Git busy. Requests carry
//! the loop's usual ids, so a late result is dropped instead of overwriting a
//! newer one.

use std::{
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use super::{
    model::App,
    run::{Env, Host},
};
use crate::{
    diff_loader::{Request, Source},
    watch::{Change, RepoWatcher},
    worktree::Entry,
};

/// The least time a view waits between two reloads.
const MIN_GAP: Duration = Duration::from_millis(150);

/// One view's reload bookkeeping.
#[derive(Default)]
pub(super) struct Slot {
    wanted: bool,
    in_flight: Option<(u64, Instant)>,
    not_before: Option<Instant>,
}

impl Slot {
    pub(super) fn want(&mut self) {
        self.wanted = true;
    }

    fn ready(&self, now: Instant) -> bool {
        self.wanted
            && self.in_flight.is_none()
            && self.not_before.is_none_or(|earliest| now >= earliest)
    }

    fn start(&mut self, id: u64, now: Instant) {
        self.wanted = false;
        self.in_flight = Some((id, now));
    }

    /// Records that a result with request `id` arrived. A result for a newer
    /// request than ours also ends ours: it was superseded and won't be answered.
    pub(super) fn finish(&mut self, id: u64, now: Instant) {
        if let Some((started_id, started)) = self.in_flight
            && id >= started_id
        {
            self.in_flight = None;
            self.not_before = Some(now + MIN_GAP.max(now.duration_since(started)));
        }
    }

    fn busy_or_wanted(&self) -> bool {
        self.wanted || self.in_flight.is_some()
    }
}

/// Every view the watcher keeps current.
#[derive(Default)]
pub(super) struct Refresh {
    pub(super) status: Slot,
    pub(super) diff: Slot,
    pub(super) history: Slot,
    pub(super) branches: Slot,
}

/// How the changes are noticed: not yet known, by watching, or by polling.
pub(super) struct Watching {
    /// The watcher once it has started; it starts in the background because
    /// that takes a third of a second or more.
    pub(super) watcher: Option<RepoWatcher>,
    pub(super) starting: Option<Receiver<Result<RepoWatcher, String>>>,
    /// Where the polling fallback sends its results, kept until it is needed.
    pub(super) poll_results: Option<SyncSender<Vec<Entry>>>,
}

impl Watching {
    /// Starts watching `root` in the background. With `GITVIEW_NO_WATCH` set,
    /// or if the watcher can't start, the working tree is polled instead.
    pub(super) fn begin(root: &std::path::Path, poll_results: SyncSender<Vec<Entry>>) -> Self {
        if std::env::var_os("GITVIEW_NO_WATCH").is_some() {
            spawn_status_poller(poll_results, root.to_path_buf());
            return Self {
                watcher: None,
                starting: None,
                poll_results: None,
            };
        }
        let (sender, starting) = mpsc::channel();
        let path = root.to_path_buf();
        thread::spawn(move || {
            let _ = sender.send(RepoWatcher::start(&path));
        });
        Self {
            watcher: None,
            starting: Some(starting),
            poll_results: Some(poll_results),
        }
    }
}

/// The fallback: reload the working-tree status regularly and report changes.
fn spawn_status_poller(sender: SyncSender<Vec<Entry>>, path: std::path::PathBuf) {
    thread::spawn(move || {
        let mut previous = None;
        loop {
            thread::sleep(Duration::from_millis(750));
            let Ok(entries) = crate::worktree::load_status(&path) else {
                continue;
            };
            if previous.as_ref() != Some(&entries) {
                match sender.try_send(entries.clone()) {
                    Ok(()) => previous = Some(entries),
                    Err(mpsc::TrySendError::Full(_)) => {}
                    Err(mpsc::TrySendError::Disconnected(_)) => return,
                }
            }
        }
    });
}

/// Which views a change makes stale.
fn want(refresh: &mut Refresh, change: Change) {
    if change.worktree || change.index || change.refs {
        refresh.status.want();
        refresh.diff.want();
    }
    if change.refs {
        refresh.history.want();
        refresh.branches.want();
    }
}

/// One turn of the loop: pick up watcher news and start the reloads it calls for.
pub(super) fn step<H: Host>(app: &mut App, env: &mut Env<'_, H>) {
    let root = env.root;
    bring_up(app, root);
    if let Some(watcher) = &app.watching.watcher {
        while let Some(change) = watcher.try_recv() {
            want(&mut app.refresh, change);
        }
    }
    start_wanted(app, root);
}

/// Takes the watcher once it has started, or falls back to polling.
fn bring_up(app: &mut App, root: &std::path::Path) {
    let Some(starting) = &app.watching.starting else {
        return;
    };
    match starting.try_recv() {
        Ok(Ok(watcher)) => {
            app.watching.watcher = Some(watcher);
            app.watching.starting = None;
            // Changes made while it was starting were not seen.
            want(&mut app.refresh, Change::everything());
        }
        Ok(Err(_)) | Err(TryRecvError::Disconnected) => {
            app.watching.starting = None;
            if let Some(sender) = app.watching.poll_results.take() {
                spawn_status_poller(sender, root.to_path_buf());
            }
        }
        Err(TryRecvError::Empty) => {}
    }
}

/// Which reloads should start now.
#[derive(Debug, Default, PartialEq, Eq)]
struct Due {
    status: bool,
    history: bool,
    branches: bool,
    diff: bool,
}

/// Decides which reloads start. Nothing starts while the user's own Git
/// operation runs: it reloads everything when it finishes, and a reload that
/// started meanwhile would make its result look stale and lose where the
/// selection should go next. An open diff waits for the status to be current,
/// because the file may have moved between the unstaged and staged lists.
fn due(refresh: &Refresh, now: Instant, user_operation_running: bool) -> Due {
    if user_operation_running {
        return Due::default();
    }
    Due {
        status: refresh.status.ready(now),
        history: refresh.history.ready(now),
        branches: refresh.branches.ready(now),
        diff: refresh.diff.ready(now) && !refresh.status.busy_or_wanted(),
    }
}

fn start_wanted(app: &mut App, root: &std::path::Path) {
    let now = Instant::now();
    let due = due(&app.refresh, now, app.mutation_pending);

    if due.status {
        app.status_request += 1;
        let id = app.status_request;
        app.refresh.status.start(id, now);
        let sender = app.status_sender.clone();
        let path = root.to_path_buf();
        thread::spawn(move || {
            let _ = sender.send((id, crate::worktree::load_status(&path)));
        });
    }

    if due.history {
        app.history_request += 1;
        let id = app.history_request;
        app.refresh.history.start(id, now);
        let sender = app.history_sender.clone();
        let path = root.to_path_buf();
        let filter = app.history_filter.clone();
        thread::spawn(move || {
            let _ = sender.send((
                id,
                crate::history::load_history_for_path(&path, filter.as_deref()),
            ));
        });
    }

    if due.branches {
        app.branch_request += 1;
        let id = app.branch_request;
        app.refresh.branches.start(id, now);
        let sender = app.branch_sender.clone();
        let path = root.to_path_buf();
        thread::spawn(move || {
            let _ = sender.send((id, crate::branches::load_branches(&path)));
        });
    }

    if due.diff {
        reload_open_diff(app, now);
    }
}

/// Asks for the open working-tree diff again, keeping what is on screen, and
/// where the reader was, until the new text arrives.
fn reload_open_diff(app: &mut App, now: Instant) {
    let (Some(entry), Some((_, shown))) = (app.diff_entry.clone(), app.diff.clone()) else {
        // No working-tree diff is open, so there is nothing to reload.
        app.refresh.diff.wanted = false;
        return;
    };
    app.request_id += 1;
    let id = app.request_id;
    app.refresh.diff.start(id, now);
    app.diff_refresh = Some((id, app.diff_scroll, app.selected_hunk));
    app.diff = Some((id, shown));
    app.ordinary_diff_worker.request(Request {
        id,
        source: Source::Worktree(entry),
    });
}

/// After a status reload, points an open diff at the file's entry as it is now:
/// staging moves a file between the unstaged and staged lists.
pub(super) fn retarget_open_diff(app: &mut App) {
    let (Some(current), Some(Ok(entries))) = (&app.diff_entry, &app.status) else {
        return;
    };
    let same_file =
        |entry: &&Entry| entry.path == current.path && entry.original == current.original;
    let moved = entries
        .iter()
        .filter(same_file)
        .find(|entry| entry.section == current.section)
        .or_else(|| entries.iter().find(same_file))
        .cloned();
    if let Some(moved) = moved {
        app.diff_entry = Some(moved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later(base: Instant, milliseconds: u64) -> Instant {
        base + Duration::from_millis(milliseconds)
    }

    #[test]
    fn a_slot_starts_only_when_a_reload_is_wanted() {
        let now = Instant::now();
        let mut slot = Slot::default();
        assert!(!slot.ready(now), "nothing was asked for");

        slot.want();

        assert!(slot.ready(now));
    }

    #[test]
    fn changes_during_a_reload_become_exactly_one_more_reload() {
        let t0 = Instant::now();
        let mut slot = Slot::default();
        slot.want();
        slot.start(10, t0);

        // Fifty more changes arrive while the reload runs: still one request.
        for _ in 0..50 {
            slot.want();
        }
        assert!(!slot.ready(later(t0, 5)), "a reload is already running");

        slot.finish(10, later(t0, 20));
        let after_gap = later(t0, 20 + 200);
        assert!(slot.ready(after_gap), "the changes were not forgotten");
        slot.start(11, after_gap);
        assert!(!slot.ready(later(after_gap, 1)));
        slot.finish(11, later(after_gap, 20));
        assert!(
            !slot.ready(later(after_gap, 1000)),
            "one follow-up covers all fifty changes"
        );
    }

    #[test]
    fn a_slot_waits_as_long_as_its_last_reload_took() {
        let t0 = Instant::now();
        let mut slot = Slot::default();
        slot.want();
        slot.start(1, t0);
        slot.finish(1, later(t0, 1000));
        slot.want();

        assert!(
            !slot.ready(later(t0, 1000 + 500)),
            "half the run time is too soon"
        );
        assert!(!slot.ready(later(t0, 1000 + 999)));
        assert!(slot.ready(later(t0, 1000 + 1000)));
    }

    #[test]
    fn a_fast_reload_still_waits_the_minimum_gap() {
        let t0 = Instant::now();
        let mut slot = Slot::default();
        slot.want();
        slot.start(1, t0);
        slot.finish(1, later(t0, 5));
        slot.want();

        assert!(!slot.ready(later(t0, 5 + 100)));
        assert!(slot.ready(later(t0, 5 + 150)));
    }

    #[test]
    fn a_late_result_for_an_older_request_does_not_end_a_newer_reload() {
        let t0 = Instant::now();
        let mut slot = Slot::default();
        slot.want();
        slot.start(7, t0);

        slot.finish(6, later(t0, 10));

        assert!(slot.in_flight.is_some(), "request 7 is still running");
    }

    #[test]
    fn a_result_for_a_newer_request_ends_a_superseded_reload() {
        // The user did something that reloaded the view themselves (request 9
        // replaced ours, 8); that answer arriving means ours will never come.
        let t0 = Instant::now();
        let mut slot = Slot::default();
        slot.want();
        slot.start(8, t0);

        slot.finish(9, later(t0, 10));

        assert!(slot.in_flight.is_none());
    }

    #[test]
    fn finishing_with_nothing_in_flight_changes_nothing() {
        let t0 = Instant::now();
        let mut slot = Slot::default();

        slot.finish(3, t0);

        assert!(slot.in_flight.is_none());
        assert!(slot.not_before.is_none());
    }

    fn everything_wanted() -> Refresh {
        let mut refresh = Refresh::default();
        want(&mut refresh, Change::everything());
        refresh
    }

    #[test]
    fn everything_wanted_is_everything_due() {
        let due = due(&everything_wanted(), Instant::now(), false);

        // The diff waits for the status, which is also wanted and starts first.
        assert_eq!(
            due,
            Due {
                status: true,
                history: true,
                branches: true,
                diff: false,
            }
        );
    }

    #[test]
    fn nothing_starts_while_the_user_runs_their_own_git_operation() {
        let now = Instant::now();

        assert_eq!(due(&everything_wanted(), now, true), Due::default());
        // And it is not forgotten: the same request is due once it ends.
        assert!(due(&everything_wanted(), now, false).status);
    }

    #[test]
    fn an_open_diff_reloads_only_after_the_status_is_current() {
        let t0 = Instant::now();
        let mut refresh = everything_wanted();
        refresh.status.start(1, t0);
        assert!(
            !due(&refresh, t0, false).diff,
            "the status is still loading"
        );

        refresh.status.finish(1, later(t0, 10));
        let ready = later(t0, 10 + 200);
        assert!(due(&refresh, ready, false).diff);
    }

    #[test]
    fn a_change_marks_the_views_it_makes_stale() {
        let mut refresh = Refresh::default();
        want(
            &mut refresh,
            Change {
                worktree: true,
                ..Change::default()
            },
        );
        assert!(refresh.status.wanted && refresh.diff.wanted);
        assert!(!refresh.history.wanted && !refresh.branches.wanted);

        let mut refresh = Refresh::default();
        want(
            &mut refresh,
            Change {
                refs: true,
                ..Change::default()
            },
        );
        assert!(refresh.status.wanted && refresh.diff.wanted);
        assert!(refresh.history.wanted && refresh.branches.wanted);

        let mut refresh = Refresh::default();
        want(&mut refresh, Change::default());
        assert!(!refresh.status.wanted);
    }
}
