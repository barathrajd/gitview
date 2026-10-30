//! The one path for Gitview's Git writes. Writes run one at a time, so two of
//! Gitview's own can't race on the index, and a write that finds another Git
//! process's lock, such as an agent's `git add`, waits for it instead of
//! failing at once.

use std::{
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

/// How long a write keeps retrying while another process holds a lock.
pub const RETRY_FOR: Duration = Duration::from_secs(2);

static WRITES: Mutex<()> = Mutex::new(());

/// Run `write` alone, retrying it while Git reports a held lock file.
///
/// Git takes its lock before changing anything, so a write refused for a lock
/// changed nothing and is safe to run again. A write made of several Git
/// commands rechecks what it previewed, so a retry after a partial change
/// fails with that check's message instead of repeating work.
pub fn serialized<T>(mut write: impl FnMut() -> Result<T, String>) -> Result<T, String> {
    let _turn = WRITES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let deadline = Instant::now() + RETRY_FOR;
    let mut pause = Duration::from_millis(25);
    loop {
        match write() {
            Err(error) if is_lock_error(&error) => {
                if Instant::now() >= deadline {
                    return Err(format!(
                        "Another Git command is still using this repository; try again \
                         when it finishes. {error}"
                    ));
                }
                thread::sleep(pause);
                pause = (pause * 2).min(Duration::from_millis(200));
            }
            result => return result,
        }
    }
}

/// Git's message when a lock file such as `index.lock` already exists.
fn is_lock_error(error: &str) -> bool {
    error.contains(".lock': File exists")
}
