use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    widgets::{Block, Paragraph},
};
use std::{
    env, io,
    process::{Command, ExitCode},
};

fn run() -> Result<(), String> {
    if env::args()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        println!("Usage: gitview\n\nOpen the current Git repository.\n\nKeys: ? help, q quit");
        return Ok(());
    }
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Could not run Git; install Git and ensure it is in PATH: {e}"))?;
    if !output.status.success() {
        return Err("Not in a Git repository. Run gitview from a repository.".into());
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let mut terminal =
        ratatui::try_init().map_err(|e| format!("Could not initialize terminal: {e}"))?;
    let result = app(&mut terminal, &root);
    ratatui::restore();
    result.map_err(|e| format!("Terminal error: {e}"))
}

fn app(terminal: &mut DefaultTerminal, root: &str) -> io::Result<()> {
    let mut help = false;
    loop {
        terminal.draw(|frame| {
            let text = if help { "? Close help   q Quit\n\nGit history and review are coming in subsequent milestones.".to_string() } else { format!("Repository: {root}\n\n? Help   q Quit") };
            frame.render_widget(Paragraph::new(text).block(Block::bordered().title("Gitview")), frame.area());
        })?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => help = !help,
                    _ => {}
                }
            }
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
