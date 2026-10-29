#![cfg(all(unix, feature = "test-support"))]
//! The real binary with an agent pane, in a real pseudo-terminal: what reaches
//! the program, and that the terminal is restored however the program ends.

use std::{
    fs,
    io::{Read, Write},
    os::{fd::FromRawFd, unix::process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// One live session at a time. `openpty` does not set close-on-exec, so a child
/// spawned by a concurrent session would inherit this one's terminal and keep it
/// open, and closing the terminal would never hang anything up.
static ONE_SESSION: Mutex<()> = Mutex::new(());

const LEAVE_ALTERNATE_SCREEN: &str = "\x1b[?1049l";
const SHOW_CURSOR: &str = "\x1b[?25h";
const DISABLE_BRACKETED_PASTE: &str = "\x1b[?2004l";

struct Session {
    _only_session: std::sync::MutexGuard<'static, ()>,
    child: Child,
    master: Option<fs::File>,
    close_reader: Arc<AtomicBool>,
    output: Arc<Mutex<Vec<u8>>>,
    reader: Option<thread::JoinHandle<()>>,
    repository: PathBuf,
}

impl Session {
    /// Starts `gitview --ascii --run <agent...>`.
    fn start(agent: &[&str]) -> Self {
        let mut args = vec!["--run"];
        args.extend_from_slice(agent);
        Self::start_with(&args, None)
    }

    /// Starts `gitview --ascii <args...>`; `path` replaces the search path.
    fn start_with(args: &[&str], path: Option<&std::ffi::OsStr>) -> Self {
        let only_session = ONE_SESSION
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let repository = std::env::temp_dir().join(format!(
            "gitview-agent-live-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        let _ = fs::remove_dir_all(&repository);
        fs::create_dir(&repository).unwrap();
        let git = |args: &[&str]| {
            assert!(
                Command::new("git")
                    .current_dir(&repository)
                    .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                    .args(["-c", "commit.gpgsign=false"])
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["commit", "--allow-empty", "-qm", "Initial commit"]);

        let (master, slave) = {
            let (mut master, mut slave) = (-1, -1);
            let mut size = libc::winsize {
                ws_row: 40,
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
            // Gitview must not inherit the master, or closing it never hangs up.
            assert_eq!(
                unsafe { libc::fcntl(master, libc::F_SETFD, libc::FD_CLOEXEC) },
                0
            );
            unsafe { (fs::File::from_raw_fd(master), fs::File::from_raw_fd(slave)) }
        };

        let mut command = Command::new(env!("CARGO_BIN_EXE_gitview"));
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command
            .current_dir(&repository)
            .arg("--ascii")
            .args(args)
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        // Like a terminal window: Gitview leads a session whose controlling
        // terminal is the pseudo-terminal, so closing it hangs Gitview up.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 || libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().unwrap();
        drop(command);
        drop(slave);

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_reader = Arc::clone(&output);
        let mut reader = master.try_clone().unwrap();
        let close_reader = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&close_reader);
        let reader = thread::spawn(move || {
            let mut buffer = [0; 4096];
            // Poll so the thread can notice it was asked to let go of its
            // copy of the master; closing a terminal closes every copy.
            while !stop.load(Ordering::Acquire) {
                let mut ready = libc::pollfd {
                    fd: std::os::fd::AsRawFd::as_raw_fd(&reader),
                    events: libc::POLLIN,
                    revents: 0,
                };
                if unsafe { libc::poll(&mut ready, 1, 50) } <= 0 {
                    continue;
                }
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => output_reader
                        .lock()
                        .unwrap()
                        .extend_from_slice(&buffer[..count]),
                }
            }
        });
        Self {
            _only_session: only_session,
            child,
            master: Some(master),
            close_reader,
            output,
            reader: Some(reader),
            repository,
        }
    }

    /// Everything Gitview has written, escape sequences included.
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    /// What a terminal would be showing: Gitview only repaints changed cells,
    /// so text is read from an emulated screen, not from the byte stream.
    fn screen(&self) -> String {
        let mut parser = vt100::Parser::new(
            std::num::NonZeroU16::new(40).unwrap(),
            std::num::NonZeroU16::new(140).unwrap(),
            0,
        );
        parser.process(&self.output.lock().unwrap());
        parser.screen().contents()
    }

    fn wait_for(&self, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.screen().contains(expected) {
            assert!(
                Instant::now() < deadline,
                "never saw {expected:?}; screen was:\n{}",
                self.screen()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.master.as_mut().unwrap().write_all(bytes).unwrap();
    }

    /// Waits for Gitview to exit and returns whether it succeeded.
    fn wait_for_exit(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.success();
            }
            if Instant::now() >= deadline {
                self.child.kill().unwrap();
                panic!("gitview did not exit; output was:\n{:?}", self.text());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Everything after the last time the screen was left, which must hold the
    /// terminal-restoring sequences.
    fn assert_terminal_restored(&self) {
        let text = self.text();
        for sequence in [LEAVE_ALTERNATE_SCREEN, SHOW_CURSOR, DISABLE_BRACKETED_PASTE] {
            assert!(
                text.contains(sequence),
                "terminal not restored, missing {sequence:?} in:\n{text:?}"
            );
        }
    }

    /// Closes every handle on the terminal, as closing its window does.
    fn close_terminal(&mut self) {
        self.close_reader.store(true, Ordering::Release);
        drop(self.master.take());
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }

    fn finish(mut self) {
        drop(self.master.take());
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        let _ = fs::remove_dir_all(&self.repository);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn wait_until_dead(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while process_is_alive(pid) {
        assert!(Instant::now() < deadline, "process {pid} is still running");
        thread::sleep(Duration::from_millis(20));
    }
}

fn read_pid(path: &Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = fs::read_to_string(path)
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        assert!(Instant::now() < deadline, "no pid in {}", path.display());
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn quitting_restores_the_terminal_and_ends_the_agent() {
    let pid_file = std::env::temp_dir().join(format!("gitview-live-quit-{}", std::process::id()));
    let script = format!(
        "echo $$ > {}; echo agent up; exec sleep 60",
        pid_file.display()
    );
    let mut session = Session::start(&["sh", "-c", &script]);
    session.wait_for("agent up");
    let pid = read_pid(&pid_file);

    session.send(b"q");

    assert!(session.wait_for_exit(), "gitview should exit successfully");
    session.assert_terminal_restored();
    wait_until_dead(pid);
    session.finish();
    let _ = fs::remove_file(pid_file);
}

#[test]
fn an_agent_that_exits_with_an_error_leaves_gitview_running() {
    let mut session = Session::start(&["sh", "-c", "echo boom; exit 3"]);
    session.wait_for("exited 3");

    // Gitview still answers: `q` quits it and the terminal is restored.
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}

#[test]
fn an_agent_killed_from_outside_is_reported_and_the_terminal_stays_intact() {
    let pid_file = std::env::temp_dir().join(format!("gitview-live-kill-{}", std::process::id()));
    let script = format!(
        "echo $$ > {}; echo agent up; exec sleep 60",
        pid_file.display()
    );
    let mut session = Session::start(&["sh", "-c", &script]);
    session.wait_for("agent up");
    let pid = read_pid(&pid_file);

    Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .unwrap();
    session.wait_for("ended by signal");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
    let _ = fs::remove_file(pid_file);
}

#[test]
fn the_focus_key_and_arrow_keys_cross_a_real_terminal_unchanged() {
    // `cat -v` prints control bytes as ^X, so the screen shows what arrived.
    let mut session = Session::start(&["sh", "-c", "stty raw -echo; echo cat ready; cat -v"]);
    session.wait_for("cat ready");

    // Byte 0x1c is Ctrl+\ on a terminal without the kitty keyboard protocol.
    session.send(&[0x1c]);
    session.wait_for("returns to Gitview");
    session.send(b"\x1b[A\x03\x1bb");
    session.wait_for("^[[A^C^[b");
    session.send(&[0x1c]);
    session.wait_for("focuses the agent");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}

#[test]
fn closing_the_terminal_ends_gitview_and_the_agent() {
    let pid_file = std::env::temp_dir().join(format!("gitview-live-hup-{}", std::process::id()));
    let script = format!(
        "echo $$ > {}; echo agent up; exec sleep 60",
        pid_file.display()
    );
    let mut session = Session::start(&["sh", "-c", &script]);
    session.wait_for("agent up");
    let pid = read_pid(&pid_file);

    session.close_terminal();

    // The hang-up (SIGHUP) ends Gitview, so it does not exit cleanly.
    assert!(!session.wait_for_exit());
    wait_until_dead(pid);
    session.finish();
    let _ = fs::remove_file(pid_file);
}

/// A folder holding an executable `name` that prints its arguments and waits,
/// followed by the real search path so `git` is still found.
fn path_with_fake(name: &str) -> (tempfile::TempDir, std::ffi::OsString) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join(name);
    fs::write(
        &program,
        format!("#!/bin/sh\necho \"fake {name} args: $*\"\nexec sleep 60\n"),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let mut folders = vec![dir.path().to_path_buf()];
    folders.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    (dir, std::env::join_paths(folders).unwrap())
}

#[test]
fn claude_runs_the_claude_program_with_the_rest_of_the_line() {
    let (_dir, path) = path_with_fake("claude");
    let mut session =
        Session::start_with(&["--claude", "--resume", "--model", "opus"], Some(&path));

    session.wait_for("fake claude args: --resume --model opus");
    session.wait_for("AGENT · claude · running");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.assert_terminal_restored();
    session.finish();
}

#[test]
fn codex_runs_the_codex_program() {
    let (_dir, path) = path_with_fake("codex");
    let mut session = Session::start_with(&["--codex"], Some(&path));

    session.wait_for("fake codex args:");
    session.wait_for("AGENT · codex · running");
    session.send(b"q");

    assert!(session.wait_for_exit());
    session.finish();
}

#[test]
fn a_missing_program_stops_before_the_terminal_is_touched() {
    let empty = tempfile::tempdir().unwrap();
    let mut session = Session::start_with(&["--claude"], Some(empty.path().as_os_str()));

    let succeeded = session.wait_for_exit();

    assert!(!succeeded);
    let text = session.text();
    assert!(
        text.contains("Claude Code (`claude`) was not found on your PATH"),
        "{text:?}"
    );
    assert!(text.contains("gitview --run <command>"), "{text:?}");
    assert!(
        !text.contains("\x1b[?1049h"),
        "entered the alternate screen: {text:?}"
    );
    assert!(
        !text.contains("\x1b[?2004h"),
        "enabled bracketed paste: {text:?}"
    );
    session.finish();
}
