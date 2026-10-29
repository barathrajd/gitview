use std::fmt;
use std::io::{self, Read, Write};
use std::num::NonZeroU16;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

/// Lines of history kept above the visible screen.
pub const SCROLLBACK_LIMIT: usize = 2000;

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
    parser: vt100::Parser,
    state: PaneState,
}

impl AgentPane {
    pub fn spawn(argv: &[String], cwd: &Path, rows: u16, cols: u16) -> Result<Self, PaneError> {
        let command = argv.first().cloned().unwrap_or_default();
        let fail = |message: String| PaneError {
            command: command.clone(),
            message,
        };
        if argv.is_empty() {
            return Err(fail("no command given".to_string()));
        }

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
            parser: vt100::Parser::new(non_zero(rows), non_zero(cols), SCROLLBACK_LIMIT),
            state: PaneState::Running,
        })
    }

    /// Applies pending output and refreshes the exit state. Never blocks.
    pub fn poll(&mut self) -> PaneState {
        let mut closed = false;
        loop {
            match self.output.try_recv() {
                Ok(bytes) => self.parser.process(&bytes),
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

    pub fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
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
