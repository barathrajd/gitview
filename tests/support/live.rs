//! Runs the real `gitview` binary in a real pseudo-terminal and reads its
//! screen back through a terminal emulator.
#![allow(dead_code)]

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

pub const LEAVE_ALTERNATE_SCREEN: &str = "\x1b[?1049l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";
pub const DISABLE_BRACKETED_PASTE: &str = "\x1b[?2004l";

/// The terminal as an emulator sees it, fed incrementally so a resize keeps
/// the picture right.
pub struct Emulator {
    parser: vt100::Parser,
    consumed: usize,
}

pub struct Session {
    _only_session: std::sync::MutexGuard<'static, ()>,
    emulator: Mutex<Emulator>,
    child: Child,
    pub master: Option<fs::File>,
    close_reader: Arc<AtomicBool>,
    output: Arc<Mutex<Vec<u8>>>,
    reader: Option<thread::JoinHandle<()>>,
    pub repository: PathBuf,
}

impl Session {
    /// Starts `gitview --ascii --run <agent...>`.
    pub fn start(agent: &[&str]) -> Self {
        let mut args = vec!["--run"];
        args.extend_from_slice(agent);
        Self::start_with(&args, None)
    }

    /// Starts `gitview --ascii <args...>`; `path` replaces the search path.
    pub fn start_with(args: &[&str], path: Option<&std::ffi::OsStr>) -> Self {
        Self::start_full(args, path, &[])
    }

    /// Like `start_with`, with extra environment variables for Gitview.
    pub fn start_full(
        args: &[&str],
        path: Option<&std::ffi::OsStr>,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Self {
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
        for (name, value) in environment {
            command.env(name, value);
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
        let emulator = Mutex::new(Emulator {
            parser: vt100::Parser::new(
                std::num::NonZeroU16::new(40).unwrap(),
                std::num::NonZeroU16::new(140).unwrap(),
                0,
            ),
            consumed: 0,
        });
        Self {
            _only_session: only_session,
            emulator,
            child,
            master: Some(master),
            close_reader,
            output,
            reader: Some(reader),
            repository,
        }
    }

    /// Everything Gitview has written, escape sequences included.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    /// What a terminal would be showing: Gitview only repaints changed cells,
    /// so text is read from an emulated screen, not from the byte stream.
    pub fn screen(&self) -> String {
        let output = self.output.lock().unwrap();
        let mut emulator = self.emulator.lock().unwrap();
        let consumed = emulator.consumed;
        emulator.parser.process(&output[consumed..]);
        emulator.consumed = output.len();
        emulator.parser.screen().contents()
    }

    /// Resizes the terminal; Gitview hears about it through SIGWINCH.
    pub fn resize(&mut self, columns: u16, rows: u16) {
        // Apply what was drawn at the old size before the picture changes size.
        let _ = self.screen();
        let size = libc::winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let master = std::os::fd::AsRawFd::as_raw_fd(self.master.as_ref().unwrap());
        assert_eq!(unsafe { libc::ioctl(master, libc::TIOCSWINSZ, &size) }, 0);
        self.emulator.lock().unwrap().parser.screen_mut().set_size(
            std::num::NonZeroU16::new(rows).unwrap(),
            std::num::NonZeroU16::new(columns).unwrap(),
        );
    }

    pub fn wait_for(&self, expected: &str) {
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

    pub fn send(&mut self, bytes: &[u8]) {
        self.master.as_mut().unwrap().write_all(bytes).unwrap();
    }

    /// Waits for Gitview to exit and returns whether it succeeded.
    pub fn wait_for_exit(&mut self) -> bool {
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
    pub fn assert_terminal_restored(&self) {
        let text = self.text();
        for sequence in [LEAVE_ALTERNATE_SCREEN, SHOW_CURSOR, DISABLE_BRACKETED_PASTE] {
            assert!(
                text.contains(sequence),
                "terminal not restored, missing {sequence:?} in:\n{text:?}"
            );
        }
    }

    /// Closes every handle on the terminal, as closing its window does.
    pub fn close_terminal(&mut self) {
        self.close_reader.store(true, Ordering::Release);
        drop(self.master.take());
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }

    pub fn finish(mut self) {
        // Gitview may still be running; its reader must be told to let go.
        self.close_reader.store(true, Ordering::Release);
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

pub fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn wait_until_dead(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while process_is_alive(pid) {
        assert!(Instant::now() < deadline, "process {pid} is still running");
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn read_pid(path: &Path) -> u32 {
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
