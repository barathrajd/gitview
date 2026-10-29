//! Key-replay harness: drives the real application loop on ratatui's in-memory
//! backend from a script of keys and screen expectations.
#![allow(dead_code)]

use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm_keybind::KeyBindTrait;
use gitview::app::run::{Host, Navigation, run_app};
use ratatui::{Terminal, backend::TestBackend};

pub enum Step {
    Key(KeyCode),
    Char(char),
    Ctrl(char),
    /// One key event per character.
    Text(&'static str),
    /// Wait until the screen contains the text.
    WaitFor(&'static str),
    /// Wait until the screen no longer contains the text.
    WaitForGone(&'static str),
    /// Print the current screen to stderr (visible with `--nocapture`).
    Dump,
}

struct ScriptedHost {
    steps: VecDeque<Step>,
    deadline: Instant,
    last_screen: String,
}

pub fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    buffer
        .content()
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

impl Host for ScriptedHost {
    type Backend = TestBackend;

    fn next_event(
        &mut self,
        terminal: &Terminal<TestBackend>,
        timeout: Duration,
    ) -> Result<Event, RecvTimeoutError> {
        self.last_screen = screen_text(terminal);
        let screen = &self.last_screen;
        let waiting = match self.steps.front() {
            None => return Err(RecvTimeoutError::Disconnected),
            Some(Step::Key(code)) => {
                let event = key(*code, KeyModifiers::NONE);
                self.steps.pop_front();
                return Ok(event);
            }
            Some(Step::Char(c)) => {
                let event = key(KeyCode::Char(*c), KeyModifiers::NONE);
                self.steps.pop_front();
                return Ok(event);
            }
            Some(Step::Ctrl(c)) => {
                let event = key(KeyCode::Char(*c), KeyModifiers::CONTROL);
                self.steps.pop_front();
                return Ok(event);
            }
            Some(Step::Text(_)) => unreachable!("text steps are expanded into chars"),
            Some(Step::Dump) => {
                eprintln!("{screen}");
                self.steps.pop_front();
                return Err(RecvTimeoutError::Timeout);
            }
            Some(Step::WaitFor(text)) => !screen.contains(text),
            Some(Step::WaitForGone(text)) => screen.contains(text),
        };
        if !waiting {
            self.steps.pop_front();
            return Err(RecvTimeoutError::Timeout);
        }
        assert!(
            Instant::now() < self.deadline,
            "timed out waiting for {:?}; screen was:\n{screen}",
            self.steps.front().map(|step| match step {
                Step::WaitFor(text) => format!("`{text}` to appear"),
                Step::WaitForGone(text) => format!("`{text}` to disappear"),
                _ => String::new(),
            })
        );
        std::thread::sleep(timeout.min(Duration::from_millis(5)));
        Err(RecvTimeoutError::Timeout)
    }

    fn run_external<T>(
        &mut self,
        _terminal: &mut Terminal<TestBackend>,
        program: impl FnOnce() -> T,
    ) -> io::Result<T> {
        Ok(program())
    }
}

/// Replays `steps` on a 140x40 terminal and returns the last screen.
pub fn replay(root: &Path, steps: impl IntoIterator<Item = Step>) -> String {
    replay_sized(root, 140, 40, steps)
}

pub fn replay_sized(
    root: &Path,
    width: u16,
    height: u16,
    steps: impl IntoIterator<Item = Step>,
) -> String {
    // Defaults only, never the user's keybinding file. A second call reports
    // "already initialized", which is fine.
    let _ = Navigation::init_and_load(None::<()>);
    let mut queue = VecDeque::new();
    for step in steps {
        match step {
            Step::Text(text) => queue.extend(text.chars().map(Step::Char)),
            other => queue.push_back(other),
        }
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut host = ScriptedHost {
        steps: queue,
        deadline: Instant::now() + Duration::from_secs(10),
        last_screen: String::new(),
    };
    let mut profiles = Vec::new();
    run_app(
        &mut terminal,
        &mut host,
        root,
        false,
        "Primary".to_owned(),
        root,
        &mut profiles,
    )
    .unwrap();
    assert!(host.steps.is_empty(), "script did not finish");
    host.last_screen
}

/// A temporary Git repository with helpers for building history.
pub struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    pub fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        // The app runs plain `git commit`, so the identity must be in the repo.
        repo.git(&["config", "user.name", "Test"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo
    }

    pub fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(self.dir.path())
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    /// Runs Git and returns its stdout.
    pub fn output(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.dir.path())
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.dir.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    pub fn commit_all(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    /// Three commits: a readme, a 30-line library file, then edits to its
    /// first and last lines (two separate hunks).
    pub fn with_history() -> Self {
        let repo = Self::new();
        repo.write("README.md", "# Demo\n");
        repo.commit_all("Add readme");
        let lines = |first: &str, last: &str| {
            let mut text = format!("{first}\n");
            for number in 2..30 {
                text.push_str(&format!("line {number}\n"));
            }
            text.push_str(&format!("{last}\n"));
            text
        };
        repo.write("src/lib.rs", &lines("first original", "last original"));
        repo.commit_all("Add library");
        repo.write("src/lib.rs", &lines("first changed", "last changed"));
        repo.commit_all("Change library");
        repo
    }

    /// `with_history` plus a `v1.0` tag at HEAD and one file in each worktree
    /// section: `staged.txt` (staged), `README.md` (edited), `notes.txt`
    /// (untracked).
    pub fn with_changes() -> Self {
        let repo = Self::with_history();
        repo.git(&["tag", "v1.0"]);
        repo.write("README.md", "# Demo\nmore\n");
        repo.write("notes.txt", "new\n");
        repo.write("staged.txt", "s\n");
        repo.git(&["add", "staged.txt"]);
        repo
    }
}
