//! How well the pane's terminal emulation copes with real programs.
//!
//! The interactive probes start the actual `claude` and `codex` in an empty
//! folder, let them draw their first screen, and quit. They send no prompt and
//! accept no dialog. They are ignored by default; run them with
//! `cargo test --test pane_fidelity -- --ignored --nocapture --test-threads=1`.

use std::{
    process::Command,
    time::{Duration, Instant},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gitview::pane::{AgentPane, PaneState, encode_key};

const ROWS: u16 = 40;
const COLUMNS: u16 = 120;

fn executable_on_path(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {name} >/dev/null")])
        .status()
        .is_ok_and(|status| status.success())
}

/// Waits for the first non-blank text on the screen and returns how long it took.
fn time_to_first_content(pane: &mut AgentPane, limit: Duration) -> Option<Duration> {
    let started = Instant::now();
    while started.elapsed() < limit {
        pane.poll();
        if !pane.screen_text().trim().is_empty() {
            return Some(started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

/// Applies output until nothing new has arrived for `quiet`, or `limit` passes.
fn settle(pane: &mut AgentPane, quiet: Duration, limit: Duration) -> Duration {
    let started = Instant::now();
    let mut last_change = Instant::now();
    let mut seen = pane.generation();
    while started.elapsed() < limit {
        pane.poll();
        if pane.generation() != seen {
            seen = pane.generation();
            last_change = Instant::now();
        } else if last_change.elapsed() >= quiet && seen > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    started.elapsed()
}

fn send_key(pane: &mut AgentPane, key: KeyEvent) {
    let bytes = encode_key(key, pane.screen().application_cursor()).unwrap();
    pane.send(&bytes).unwrap();
}

fn report(name: &str, pane: &AgentPane, settled_after: Duration) {
    let screen = pane.screen();
    println!("=== {name}");
    println!(
        "state: {:?}, first screen settled after {settled_after:?}",
        pane.state()
    );
    println!(
        "modes: alternate_screen={} application_cursor={} bracketed_paste={} cursor_hidden={} mouse={:?}/{:?}",
        screen.alternate_screen(),
        screen.application_cursor(),
        screen.bracketed_paste(),
        screen.hide_cursor(),
        screen.mouse_protocol_mode(),
        screen.mouse_protocol_encoding(),
    );
    println!("title: {:?}", pane.title());
    println!(
        "queries the program sent (unanswered): {:?}",
        pane.unanswered_queries()
    );
    println!(
        "sequences the emulator did not understand: {:?}",
        pane.unhandled_sequences()
    );
    let mut styled = 0;
    let mut truecolor = 0;
    let mut indexed = 0;
    let (rows, columns) = pane.size();
    for row in 0..rows {
        for column in 0..columns {
            let Some(cell) = screen.cell(row, column) else {
                continue;
            };
            let colors = [cell.fgcolor(), cell.bgcolor()];
            if colors.iter().any(|c| *c != vt100::Color::Default) || cell.bold() || cell.inverse() {
                styled += 1;
            }
            truecolor += colors
                .iter()
                .filter(|c| matches!(c, vt100::Color::Rgb(..)))
                .count();
            indexed += colors
                .iter()
                .filter(|c| matches!(c, vt100::Color::Idx(_)))
                .count();
        }
    }
    println!("styled cells: {styled} (truecolor uses: {truecolor}, indexed uses: {indexed})");
    println!("--- screen");
    for line in screen.contents().lines() {
        println!("| {line}");
    }
    println!("--- end");
}

/// Starts `argv` in an empty folder, records its first screen, then quits it
/// with Ctrl+C twice, which both agents treat as "exit".
fn probe_interactive(name: &str, argv: &[&str]) {
    if !executable_on_path(argv[0]) {
        println!("=== {name}: `{}` is not installed; skipped", argv[0]);
        return;
    }
    let folder = tempfile::tempdir().unwrap();
    let command: Vec<String> = argv.iter().map(|part| part.to_string()).collect();
    let mut pane = AgentPane::spawn(&command, folder.path(), ROWS, COLUMNS).unwrap();
    let first_content = time_to_first_content(&mut pane, Duration::from_secs(25));
    println!("first visible text after: {first_content:?}");
    let settled_after = settle(&mut pane, Duration::from_secs(3), Duration::from_secs(25));
    report(name, &pane, settled_after);

    // Down only moves the highlight in both dialogs; nothing is confirmed.
    let marker = |pane: &AgentPane| {
        pane.screen_text()
            .lines()
            .find(|line| line.contains('❯') || line.contains('›'))
            .map(|line| line.trim().to_owned())
    };
    let selected_before = marker(&pane);
    send_key(&mut pane, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    settle(
        &mut pane,
        Duration::from_millis(700),
        Duration::from_secs(5),
    );
    let selected_after = marker(&pane);
    println!("selection before Down: {selected_before:?}");
    println!("selection after Down:  {selected_after:?}");
    assert_ne!(
        selected_before, selected_after,
        "the Down key did not reach the program or it did not redraw"
    );

    // A resize reaches the program as SIGWINCH; it should redraw for the new size.
    let before = pane.generation();
    pane.resize(30, 90);
    settle(&mut pane, Duration::from_secs(1), Duration::from_secs(8));
    println!(
        "resize to 30x90: program redrew = {} ({} new chunks), size now {:?}",
        pane.generation() > before,
        pane.generation() - before,
        pane.size()
    );

    // Ctrl+C twice is "exit" for both agents; give them a few seconds to leave.
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    for _ in 0..2 {
        send_key(&mut pane, ctrl_c);
        std::thread::sleep(Duration::from_millis(300));
    }
    let deadline = Instant::now() + Duration::from_secs(6);
    while pane.poll() == PaneState::Running && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("after Ctrl+C twice: {:?}", pane.state());
    println!("queries after exit: {:?}", pane.unanswered_queries());
    println!("unhandled after exit: {:?}", pane.unhandled_sequences());
}

#[test]
fn a_shell_works_in_the_pane() {
    let folder = tempfile::tempdir().unwrap();
    let command: Vec<String> = ["sh", "-i"].map(String::from).to_vec();
    let mut pane = AgentPane::spawn(&command, folder.path(), 24, 80).unwrap();
    settle(
        &mut pane,
        Duration::from_millis(300),
        Duration::from_secs(5),
    );

    pane.send(b"printf 'colour \\033[32mgreen\\033[0m %s\\n' \"$((6*7))\"\r")
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pane.screen_text().contains("colour green 42") {
        pane.poll();
        assert!(
            Instant::now() < deadline,
            "shell output never appeared:\n{}",
            pane.screen_text()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    pane.send(b"exit\r").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while pane.poll() == PaneState::Running {
        assert!(Instant::now() < deadline, "shell did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(pane.state(), PaneState::Exited(Some(0)));
    // Readline asks for "8-bit meta" mode (DECSET 1034). The emulator ignores
    // it, which is harmless for a UTF-8 terminal; anything else is a gap.
    let unexpected: Vec<_> = pane
        .unhandled_sequences()
        .into_iter()
        .filter(|(sequence, _)| sequence != "CSI ?1034h")
        .collect();
    assert!(unexpected.is_empty(), "{unexpected:?}");
}

#[test]
#[ignore = "starts the real claude; run with --ignored --nocapture"]
fn probe_claude() {
    probe_interactive("claude", &["claude"]);
}

#[test]
#[ignore = "starts the real codex; run with --ignored --nocapture"]
fn probe_codex() {
    probe_interactive("codex", &["codex"]);
}

#[test]
#[ignore = "starts installed agy without sending a prompt"]
fn probe_agy_terminal_modes() {
    if !executable_on_path("agy") {
        return;
    }
    let folder = std::env::current_dir().unwrap();
    let mut pane = AgentPane::spawn(&["agy".to_owned()], folder.as_path(), ROWS, COLUMNS).unwrap();
    time_to_first_content(&mut pane, Duration::from_secs(25));
    settle(&mut pane, Duration::from_secs(3), Duration::from_secs(15));
    println!(
        "AGY state={:?} mouse={:?}/{:?} alternate={} scrollback={} queries={:?} unhandled={:?}",
        pane.state(),
        pane.screen().mouse_protocol_mode(),
        pane.screen().mouse_protocol_encoding(),
        pane.screen().alternate_screen(),
        pane.screen().scrollback_len(),
        pane.unanswered_queries(),
        pane.unhandled_sequences()
    );
    println!(
        "AGY bracketed_paste={} application_cursor={} hidden_cursor={} startup_prompts={:?}",
        pane.screen().bracketed_paste(),
        pane.screen().application_cursor(),
        pane.screen().hide_cursor(),
        pane.screen_text()
            .lines()
            .filter(|line| {
                let lower = line.to_lowercase();
                ["trust", "permission", "welcome", "continue", "error"]
                    .iter()
                    .any(|word| lower.contains(word))
                    && !line.contains("http")
                    && !line.contains('@')
            })
            .collect::<Vec<_>>()
    );
    send_key(
        &mut pane,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    send_key(
        &mut pane,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
}
