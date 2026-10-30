mod keys;
mod mouse;
pub use mouse::encode_mouse;
mod launch;
mod queries;

pub use keys::{FOCUS_KEY_LABEL, encode_key, encode_paste, is_focus_key};
pub use launch::{
    AgentRequest, check_installed, find_executable, register_gitview_mcp, split_agent_args,
};

use queries::QueryScanner;

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::num::NonZeroU16;
use std::path::Path;
use std::sync::{
    Mutex,
    mpsc::{self, Receiver, TryRecvError},
};
use std::thread;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

/// Lines of history kept above the visible screen.
pub const SCROLLBACK_LIMIT: usize = 2000;

/// The embedded terminal's default background, also reported through OSC 11.
pub const DEFAULT_BACKGROUND: (u8, u8, u8) = (24, 24, 32);

/// Pane setup runs one at a time: macOS's `openpty` fails intermittently
/// ("Unknown error: -6") when several threads call it at once.
static SETUP: Mutex<()> = Mutex::new(());

/// How many distinct unhandled sequences are remembered.
const UNHANDLED_LIMIT: usize = 64;

/// Notes what the emulator could not interpret, so gaps in terminal support
/// show up as data instead of garbled output. Also remembers the window title.
#[derive(Default)]
struct Recorder {
    unhandled: BTreeMap<String, u32>,
    title: Option<String>,
}

impl Recorder {
    fn note(&mut self, sequence: String) {
        if self.unhandled.len() < UNHANDLED_LIMIT || self.unhandled.contains_key(&sequence) {
            *self.unhandled.entry(sequence).or_default() += 1;
        }
    }
}

fn printable(byte: u8) -> char {
    if byte.is_ascii_graphic() {
        char::from(byte)
    } else {
        '?'
    }
}

impl vt100::Callbacks for Recorder {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = Some(String::from_utf8_lossy(title).into_owned());
    }

    fn unhandled_char(&mut self, _: &mut vt100::Screen, c: char) {
        self.note(format!("char {c:?}"));
    }

    fn unhandled_control(&mut self, _: &mut vt100::Screen, byte: u8) {
        self.note(format!("control 0x{byte:02x}"));
    }

    fn unhandled_escape(
        &mut self,
        _: &mut vt100::Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        byte: u8,
    ) {
        let intermediates: String = [i1, i2].into_iter().flatten().map(printable).collect();
        self.note(format!("ESC {intermediates}{}", printable(byte)));
    }

    fn unhandled_csi(
        &mut self,
        _: &mut vt100::Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let params = params
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(":")
            })
            .collect::<Vec<_>>()
            .join(";");
        let lead: String = i1.into_iter().map(printable).collect();
        let tail: String = i2.into_iter().map(printable).collect();
        self.note(format!("CSI {lead}{params}{tail}{c}"));
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        let number = params
            .first()
            .map(|first| String::from_utf8_lossy(first).into_owned())
            .unwrap_or_default();
        self.note(format!("OSC {number}"));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneState {
    Running,
    /// The command ended; the code is `None` when it was killed by a signal.
    Exited(Option<u32>),
}

#[derive(Debug)]
pub struct PaneError {
    command: String,
    message: String,
}

impl fmt::Display for PaneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot start `{}`: {}", self.command, self.message)
    }
}

impl std::error::Error for PaneError {}

/// A command running in a pseudo-terminal whose output feeds a terminal
/// emulator. Reading happens on a worker thread; `poll` applies it.
pub struct AgentPane {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    output: Receiver<Vec<u8>>,
    parser: vt100::Parser<Recorder>,
    state: PaneState,
    command: String,
    generation: u64,
    queries: QueryScanner,
    note: Option<String>,
}

impl AgentPane {
    pub fn spawn(argv: &[String], cwd: &Path, rows: u16, cols: u16) -> Result<Self, PaneError> {
        Self::spawn_with_env(argv, cwd, rows, cols, &[])
    }

    /// Like `spawn`, with extra environment variables for the program.
    pub fn spawn_with_env(
        argv: &[String],
        cwd: &Path,
        rows: u16,
        cols: u16,
        env: &[(&str, &std::ffi::OsStr)],
    ) -> Result<Self, PaneError> {
        let command = argv.first().cloned().unwrap_or_default();
        let fail = |message: String| PaneError {
            command: command.clone(),
            message,
        };
        if argv.is_empty() {
            return Err(fail("no command given".to_string()));
        }

        let _setup = SETUP
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| fail(error.to_string()))?;
        let mut builder = CommandBuilder::new(&argv[0]);
        builder.args(&argv[1..]);
        builder.cwd(cwd);
        for (key, value) in env {
            builder.env(key, value);
        }
        let child = pair
            .slave
            .spawn_command(builder)
            .map_err(|error| fail(error.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| fail(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| fail(error.to_string()))?;
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            while let Ok(read) = reader.read(&mut buffer) {
                if read == 0 || sender.send(buffer[..read].to_vec()).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            child,
            output,
            parser: vt100::Parser::new_with_callbacks(
                non_zero(rows),
                non_zero(cols),
                SCROLLBACK_LIMIT,
                Recorder::default(),
            ),
            state: PaneState::Running,
            command,
            generation: 0,
            queries: QueryScanner::default(),
            note: None,
        })
    }

    /// Applies pending output and refreshes the exit state. Never blocks.
    pub fn poll(&mut self) -> PaneState {
        let mut closed = false;
        loop {
            match self.output.try_recv() {
                Ok(bytes) => {
                    // Answer each query with the screen as it was when the
                    // query arrived, not after the rest of the chunk.
                    let mut done = 0;
                    for (end, query) in self.queries.scan(&bytes) {
                        self.parser.process(&bytes[done..end]);
                        done = end;
                        let reply = match query {
                            queries::Answerable::BackgroundColor => {
                                let (r, g, b) = DEFAULT_BACKGROUND;
                                format!(
                                    "\x1b]11;rgb:{:04x}/{:04x}/{:04x}\x07",
                                    u16::from(r) * 257,
                                    u16::from(g) * 257,
                                    u16::from(b) * 257
                                )
                            }
                            queries::Answerable::CursorPosition => {
                                let (row, column) = self.parser.screen().cursor_position();
                                format!("\x1b[{};{}R", row + 1, column + 1)
                            }
                            // A VT100 with advanced video, as xterm reports at
                            // its plainest.
                            queries::Answerable::DeviceAttributes => "\x1b[?1;2c".to_owned(),
                        };
                        // A failed write means the program just ended.
                        let _ = self.send(reply.as_bytes());
                    }
                    self.parser.process(&bytes[done..]);
                    self.generation += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }
        if self.state == PaneState::Running
            && closed
            && let Ok(status) = self.child.wait()
        {
            let signaled = status.signal().is_some();
            self.state = PaneState::Exited((!signaled).then(|| status.exit_code()));
        }
        self.state
    }

    pub fn state(&self) -> PaneState {
        self.state
    }

    /// The operating-system process id, while the program is running.
    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// The pane's size in rows and columns.
    pub fn size(&self) -> (u16, u16) {
        let (rows, columns) = self.parser.screen().size();
        (rows.get(), columns.get())
    }

    /// The program that was started.
    /// A short remark shown in the pane's title, such as that Gitview's tools
    /// are off.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn set_note(&mut self, note: impl Into<String>) {
        self.note = Some(note.into());
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    /// The emulated screen, including the cursor and the input modes the
    /// program asked for.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Increases whenever output has been applied, so callers know to redraw.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The window title the program set, if any.
    pub fn title(&self) -> Option<&str> {
        self.parser.callbacks().title.as_deref()
    }

    /// Questions the program asked its terminal, with how many times each.
    /// Nothing answers them yet, so a program waiting on a reply may stall.
    pub fn unanswered_queries(&self) -> Vec<(String, u32)> {
        self.queries.counts()
    }

    /// Escape sequences the emulator did not understand, with how many times
    /// each was seen.
    pub fn unhandled_sequences(&self) -> Vec<(String, u32)> {
        self.parser
            .callbacks()
            .unhandled
            .iter()
            .map(|(sequence, count)| (sequence.clone(), *count))
            .collect()
    }

    pub fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    /// Browse ordinary terminal output without sending keys to the child.
    pub fn scroll_by(&mut self, lines: i16) {
        let screen = self.parser.screen_mut();
        let old = screen.scrollback();
        let next = if lines >= 0 {
            old.saturating_add(lines as usize)
        } else {
            old.saturating_sub(usize::from(lines.unsigned_abs()))
        };
        screen.set_scrollback(next);
        if screen.scrollback() != old {
            self.generation += 1;
        }
    }

    pub fn scroll_to_live(&mut self) {
        if self.parser.screen().scrollback() != 0 {
            self.parser.screen_mut().set_scrollback(0);
            self.generation += 1;
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser
            .screen_mut()
            .set_size(non_zero(rows), non_zero(cols));
    }

    pub fn screen_text(&self) -> String {
        self.parser.screen().contents()
    }

    /// Number of history lines currently kept above the visible screen.
    pub fn scrollback_len(&mut self) -> usize {
        let screen = self.parser.screen_mut();
        let previous = screen.scrollback();
        screen.set_scrollback(usize::MAX);
        let length = screen.scrollback();
        screen.set_scrollback(previous);
        length
    }
}

fn non_zero(value: u16) -> NonZeroU16 {
    NonZeroU16::new(value).unwrap_or(NonZeroU16::MIN)
}

impl Drop for AgentPane {
    /// Closing the pane ends the program; otherwise quitting Gitview would leave
    /// an agent running with no terminal.
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
