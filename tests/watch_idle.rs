//! An idle watcher must cost nothing. This is the only test in its binary so no
//! other test's work shows up in the process's CPU time.

use std::{fs, process::Command, time::Duration};

use gitview::watch::RepoWatcher;

fn cpu_time() -> Duration {
    let mut usage = unsafe { std::mem::zeroed::<libc::rusage>() };
    assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) }, 0);
    let to_duration = |t: libc::timeval| Duration::new(t.tv_sec as u64, (t.tv_usec as u32) * 1000);
    to_duration(usage.ru_utime) + to_duration(usage.ru_stime)
}

#[test]
fn a_watcher_with_nothing_to_report_uses_no_cpu() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .current_dir(dir.path())
            .args(["init", "-q", "-b", "main"])
            .status()
            .unwrap()
            .success()
    );
    for index in 0..500 {
        fs::write(dir.path().join(format!("file{index}.txt")), "x").unwrap();
    }
    let watcher = RepoWatcher::start(dir.path()).unwrap();
    // Let start-up work and the events from creating the files finish.
    while watcher.recv_timeout(Duration::from_millis(800)).is_some() {}

    let before = cpu_time();
    let report = watcher.recv_timeout(Duration::from_secs(4));
    let used = cpu_time() - before;

    assert!(report.is_none(), "an idle repository reported {report:?}");
    // Four seconds of polling at even 1% would be 40 ms.
    assert!(
        used < Duration::from_millis(25),
        "an idle watcher used {used:?} of CPU in 4 s"
    );
}
