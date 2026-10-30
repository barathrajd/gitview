use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    time::Duration,
};

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event,
};
use ratatui::{DefaultTerminal, Terminal};

use super::run::Host;

pub struct BracketedPasteMode {
    enabled: bool,
    mouse_enabled: bool,
}

impl BracketedPasteMode {
    pub fn enable() -> io::Result<Self> {
        crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
        Ok(Self {
            enabled: true,
            mouse_enabled: false,
        })
    }

    pub fn set_mouse_capture(&mut self, enabled: bool) -> io::Result<()> {
        if enabled != self.mouse_enabled {
            if enabled {
                crossterm::execute!(io::stdout(), EnableMouseCapture)?;
            } else {
                crossterm::execute!(io::stdout(), DisableMouseCapture)?;
            }
            self.mouse_enabled = enabled;
        }
        Ok(())
    }

    pub fn disable(&mut self) -> io::Result<()> {
        self.set_mouse_capture(false)?;
        if self.enabled {
            crossterm::execute!(io::stdout(), DisableBracketedPaste)?;
            self.enabled = false;
        }
        Ok(())
    }

    pub fn resume(&mut self) -> io::Result<()> {
        if !self.enabled {
            crossterm::execute!(io::stdout(), EnableBracketedPaste)?;
            self.enabled = true;
        }
        Ok(())
    }
}

impl Drop for BracketedPasteMode {
    fn drop(&mut self) {
        let _ = self.disable();
    }
}

/// The real terminal: a reader thread feeds crossterm events to the loop and
/// is paused while an external program owns the screen.
pub struct TerminalHost {
    paste_mode: BracketedPasteMode,
    events: Receiver<Event>,
    paused: Arc<AtomicBool>,
    gate: Arc<Mutex<()>>,
}

impl TerminalHost {
    pub fn new(paste_mode: BracketedPasteMode) -> Self {
        let (sender, events) = mpsc::channel();
        let paused = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(Mutex::new(()));
        let reader_paused = paused.clone();
        let reader_gate = gate.clone();
        std::thread::spawn(move || {
            loop {
                if reader_paused.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                let Ok(_guard) = reader_gate.lock() else {
                    break;
                };
                if reader_paused.load(Ordering::Acquire) {
                    continue;
                }
                match event::poll(Duration::from_millis(50)) {
                    Ok(true) => match event::read() {
                        Ok(event) => {
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        });
        Self {
            paste_mode,
            events,
            paused,
            gate,
        }
    }

    pub fn disable_paste(&mut self) -> io::Result<()> {
        self.paste_mode.disable()
    }
}

impl Host for TerminalHost {
    type Backend = ratatui::backend::CrosstermBackend<io::Stdout>;

    fn next_event(
        &mut self,
        _terminal: &DefaultTerminal,
        timeout: Duration,
    ) -> Result<Event, RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }

    fn set_mouse_capture(&mut self, enabled: bool) -> io::Result<()> {
        self.paste_mode.set_mouse_capture(enabled)
    }

    fn run_external<T>(
        &mut self,
        terminal: &mut Terminal<Self::Backend>,
        program: impl FnOnce() -> T,
    ) -> io::Result<T> {
        self.paste_mode.disable()?;
        self.paused.store(true, Ordering::Release);
        let guard = self
            .gate
            .lock()
            .map_err(|_| io::Error::other("Input reader unavailable"))?;
        ratatui::restore();
        let result = program();
        let restored = ratatui::try_init();
        while self.events.try_recv().is_ok() {}
        drop(guard);
        self.paused.store(false, Ordering::Release);
        *terminal = restored?;
        self.paste_mode.resume()?;
        terminal.clear()?;
        Ok(result)
    }
}
