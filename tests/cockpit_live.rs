#![cfg(all(unix, feature = "test-support"))]

use std::{
    fs,
    io::{Read, Write},
    os::fd::FromRawFd,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
#[allow(clippy::unnecessary_mut_passed)]
fn startup_data_refreshes_the_cockpit_without_keyboard_input() {
    let repository = std::env::temp_dir().join(format!(
        "gitview-cockpit-live-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&repository).unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(&repository)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q", "-b", "main"]);
    git(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "Initial cockpit commit",
    ]);
    fs::write(repository.join("untracked-file"), "new content").unwrap();

    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
        ws_row: 24,
        ws_col: 140,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    assert_eq!(opened, 0, "could not create a pseudo-terminal");

    let slave = unsafe { fs::File::from_raw_fd(slave) };
    let mut command = Command::new(env!("CARGO_BIN_EXE_gitview"));
    command
        .current_dir(&repository)
        .arg("--ascii")
        .stdin(Stdio::from(slave.try_clone().unwrap()))
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave.try_clone().unwrap()));
    let mut child = command.spawn().unwrap();
    drop(command);
    drop(slave);
    let mut master = unsafe { fs::File::from_raw_fd(master) };
    let output = Arc::new(Mutex::new(Vec::new()));
    let output_reader = Arc::clone(&output);
    let mut reader = master.try_clone().unwrap();
    let reader_thread = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => output_reader
                    .lock()
                    .unwrap()
                    .extend_from_slice(&buffer[..count]),
                Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                Err(error) => panic!("could not read pseudo-terminal output: {error}"),
            }
        }
    });

    let deadline = Instant::now() + Duration::from_secs(3);
    let loaded = loop {
        let output = String::from_utf8_lossy(&output.lock().unwrap()).into_owned();
        if output.contains("Initial cockpit commit") && output.contains("untracked-file") {
            break Some(output);
        }
        if Instant::now() >= deadline {
            break None;
        }
        thread::sleep(Duration::from_millis(20));
    };

    master.write_all(b"q").unwrap();
    let exit_deadline = Instant::now() + Duration::from_secs(2);
    while child.try_wait().unwrap().is_none() && Instant::now() < exit_deadline {
        thread::sleep(Duration::from_millis(20));
    }
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
    }
    let _ = child.wait().unwrap();
    drop(master);
    reader_thread.join().unwrap();
    fs::remove_dir_all(repository).unwrap();

    let output =
        loaded.unwrap_or_else(|| String::from_utf8_lossy(&output.lock().unwrap()).into_owned());
    assert!(output.contains("Initial cockpit commit"));
    assert!(output.contains("untracked-file"));
    assert!(output.contains("main"), "{output}");
}
