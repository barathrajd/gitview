use std::{
    collections::VecDeque,
    io,
    path::Path,
    process::Command,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use gitview::app::run::{Host, run_app};
use ratatui::{Terminal, backend::TestBackend};

enum Step {
    Key(KeyCode),
    WaitFor(&'static str),
    WaitForGone(&'static str),
}

/// Replays a script of keys and screen expectations against `run_app`.
/// When the script ends the event stream disconnects and the app exits.
struct ScriptedHost {
    steps: VecDeque<Step>,
    deadline: Instant,
}

impl ScriptedHost {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            deadline: Instant::now() + Duration::from_secs(10),
        }
    }
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    buffer
        .content()
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

impl Host for ScriptedHost {
    type Backend = TestBackend;

    fn next_event(
        &mut self,
        terminal: &Terminal<TestBackend>,
        timeout: Duration,
    ) -> Result<Event, RecvTimeoutError> {
        let screen = screen_text(terminal);
        match self.steps.front() {
            None => Err(RecvTimeoutError::Disconnected),
            Some(Step::Key(code)) => {
                let event = Event::Key(KeyEvent::new(*code, KeyModifiers::NONE));
                self.steps.pop_front();
                Ok(event)
            }
            Some(Step::WaitFor(text)) if screen.contains(text) => {
                self.steps.pop_front();
                Err(RecvTimeoutError::Timeout)
            }
            Some(Step::WaitForGone(text)) if !screen.contains(text) => {
                self.steps.pop_front();
                Err(RecvTimeoutError::Timeout)
            }
            Some(_) => {
                assert!(
                    Instant::now() < self.deadline,
                    "timed out waiting for a screen change; screen was:\n{screen}"
                );
                std::thread::sleep(timeout.min(Duration::from_millis(5)));
                Err(RecvTimeoutError::Timeout)
            }
        }
    }

    fn run_external<T>(
        &mut self,
        _terminal: &mut Terminal<TestBackend>,
        program: impl FnOnce() -> T,
    ) -> io::Result<T> {
        Ok(program())
    }
}

fn repository_with_one_commit() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(dir.path())
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(dir.path().join("a.txt"), "one\n").unwrap();
    git(&["add", "a.txt"]);
    git(&["commit", "-q", "-m", "Initial commit"]);
    dir
}

fn replay(root: &Path, steps: impl IntoIterator<Item = Step>) {
    let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
    let mut host = ScriptedHost::new(steps);
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
}

#[test]
fn help_opens_and_closes_from_scripted_keys() {
    let repository = repository_with_one_commit();

    replay(
        repository.path(),
        [
            Step::WaitFor("Initial commit"),
            Step::Key(KeyCode::Char('?')),
            Step::WaitFor("HELP · Working tree"),
            Step::Key(KeyCode::Esc),
            Step::WaitForGone("HELP · Working tree"),
        ],
    );
}
