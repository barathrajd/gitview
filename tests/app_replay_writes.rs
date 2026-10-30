//! A user's Git write in the real application loop waits out a lock that
//! another process, such as an agent's `git add`, holds for a moment.

#[path = "support/replay.rs"]
mod replay;

use std::{fs, time::Duration};

use replay::{Repo, Step::*, replay};

#[test]
fn stage_all_waits_for_another_processs_lock() {
    let repository = Repo::with_changes();
    let lock = repository.path().join(".git/index.lock");

    let screen = replay(
        &repository.path(),
        [
            Char('W'),
            WaitFor("UNTRACKED 1"),
            Do(Box::new({
                let lock = lock.clone();
                move || {
                    fs::write(&lock, "").unwrap();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(300));
                        fs::remove_file(lock).unwrap();
                    });
                }
            })),
            Char('S'),
            WaitFor("STAGED 3"),
        ],
    );

    assert!(!screen.contains("index.lock"), "{screen}");
    let staged = repository.output(&["diff", "--cached", "--name-only"]);
    assert_eq!(staged.lines().count(), 3, "{staged}");
}
