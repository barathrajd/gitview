//! Turning crossterm input into the bytes a program in a terminal expects.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use gitview::pane::{AgentPane, encode_key, encode_paste};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn bytes(code: KeyCode, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    encode_key(key(code, modifiers), false)
}

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

#[test]
fn printable_characters_are_sent_as_utf8() {
    assert_eq!(bytes(KeyCode::Char('a'), NONE), Some(b"a".to_vec()));
    assert_eq!(bytes(KeyCode::Char('A'), SHIFT), Some(b"A".to_vec()));
    assert_eq!(
        bytes(KeyCode::Char('é'), NONE),
        Some("é".as_bytes().to_vec())
    );
    assert_eq!(
        bytes(KeyCode::Char('日'), NONE),
        Some("日".as_bytes().to_vec())
    );
    assert_eq!(bytes(KeyCode::Char(' '), NONE), Some(b" ".to_vec()));
}

#[test]
fn control_letters_become_control_codes() {
    assert_eq!(bytes(KeyCode::Char('a'), CTRL), Some(vec![0x01]));
    assert_eq!(bytes(KeyCode::Char('c'), CTRL), Some(vec![0x03]));
    assert_eq!(bytes(KeyCode::Char('z'), CTRL), Some(vec![0x1a]));
    assert_eq!(bytes(KeyCode::Char('R'), CTRL | SHIFT), Some(vec![0x12]));
    assert_eq!(bytes(KeyCode::Char(' '), CTRL), Some(vec![0x00]));
    assert_eq!(bytes(KeyCode::Char('['), CTRL), Some(vec![0x1b]));
    assert_eq!(bytes(KeyCode::Char(']'), CTRL), Some(vec![0x1d]));
    assert_eq!(bytes(KeyCode::Char('_'), CTRL), Some(vec![0x1f]));
    assert_eq!(bytes(KeyCode::Char('/'), CTRL), Some(vec![0x1f]));
}

#[test]
fn alt_prefixes_an_escape() {
    assert_eq!(bytes(KeyCode::Char('b'), ALT), Some(b"\x1bb".to_vec()));
    assert_eq!(bytes(KeyCode::Char('f'), ALT), Some(b"\x1bf".to_vec()));
    assert_eq!(
        bytes(KeyCode::Char('d'), ALT | CTRL),
        Some(b"\x1b\x04".to_vec())
    );
    assert_eq!(bytes(KeyCode::Backspace, ALT), Some(b"\x1b\x7f".to_vec()));
    assert_eq!(bytes(KeyCode::Enter, ALT), Some(b"\x1b\r".to_vec()));
}

#[test]
fn editing_keys() {
    assert_eq!(bytes(KeyCode::Enter, NONE), Some(b"\r".to_vec()));
    assert_eq!(bytes(KeyCode::Tab, NONE), Some(b"\t".to_vec()));
    assert_eq!(bytes(KeyCode::BackTab, SHIFT), Some(b"\x1b[Z".to_vec()));
    assert_eq!(bytes(KeyCode::Backspace, NONE), Some(vec![0x7f]));
    assert_eq!(bytes(KeyCode::Backspace, CTRL), Some(vec![0x08]));
    assert_eq!(bytes(KeyCode::Esc, NONE), Some(vec![0x1b]));
    assert_eq!(bytes(KeyCode::Delete, NONE), Some(b"\x1b[3~".to_vec()));
    assert_eq!(bytes(KeyCode::Insert, NONE), Some(b"\x1b[2~".to_vec()));
    // Without the kitty protocol Shift+Enter has no code of its own; it is sent
    // like Alt+Enter, which agents read as "insert a newline".
    assert_eq!(bytes(KeyCode::Enter, SHIFT), Some(b"\x1b\r".to_vec()));
}

#[test]
fn navigation_keys() {
    assert_eq!(bytes(KeyCode::Up, NONE), Some(b"\x1b[A".to_vec()));
    assert_eq!(bytes(KeyCode::Down, NONE), Some(b"\x1b[B".to_vec()));
    assert_eq!(bytes(KeyCode::Right, NONE), Some(b"\x1b[C".to_vec()));
    assert_eq!(bytes(KeyCode::Left, NONE), Some(b"\x1b[D".to_vec()));
    assert_eq!(bytes(KeyCode::Home, NONE), Some(b"\x1b[H".to_vec()));
    assert_eq!(bytes(KeyCode::End, NONE), Some(b"\x1b[F".to_vec()));
    assert_eq!(bytes(KeyCode::PageUp, NONE), Some(b"\x1b[5~".to_vec()));
    assert_eq!(bytes(KeyCode::PageDown, NONE), Some(b"\x1b[6~".to_vec()));
}

#[test]
fn application_cursor_mode_changes_the_arrow_and_home_end_prefix() {
    let encode = |code| encode_key(key(code, NONE), true);

    assert_eq!(encode(KeyCode::Up), Some(b"\x1bOA".to_vec()));
    assert_eq!(encode(KeyCode::Left), Some(b"\x1bOD".to_vec()));
    assert_eq!(encode(KeyCode::Home), Some(b"\x1bOH".to_vec()));
    assert_eq!(encode(KeyCode::End), Some(b"\x1bOF".to_vec()));
    // Keys that are not cursor keys are unchanged.
    assert_eq!(encode(KeyCode::PageUp), Some(b"\x1b[5~".to_vec()));
    assert_eq!(encode(KeyCode::Delete), Some(b"\x1b[3~".to_vec()));
}

#[test]
fn modified_navigation_keys_carry_the_modifier() {
    assert_eq!(bytes(KeyCode::Left, CTRL), Some(b"\x1b[1;5D".to_vec()));
    assert_eq!(bytes(KeyCode::Right, ALT), Some(b"\x1b[1;3C".to_vec()));
    assert_eq!(bytes(KeyCode::Up, SHIFT), Some(b"\x1b[1;2A".to_vec()));
    assert_eq!(
        bytes(KeyCode::Down, CTRL | SHIFT),
        Some(b"\x1b[1;6B".to_vec())
    );
    assert_eq!(bytes(KeyCode::Home, CTRL), Some(b"\x1b[1;5H".to_vec()));
    assert_eq!(bytes(KeyCode::Delete, CTRL), Some(b"\x1b[3;5~".to_vec()));
    assert_eq!(bytes(KeyCode::PageUp, SHIFT), Some(b"\x1b[5;2~".to_vec()));
    // Modified cursor keys use the same form in application cursor mode.
    assert_eq!(
        encode_key(key(KeyCode::Left, CTRL), true),
        Some(b"\x1b[1;5D".to_vec())
    );
}

#[test]
fn function_keys() {
    let f = |n| bytes(KeyCode::F(n), NONE);
    assert_eq!(f(1), Some(b"\x1bOP".to_vec()));
    assert_eq!(f(4), Some(b"\x1bOS".to_vec()));
    assert_eq!(f(5), Some(b"\x1b[15~".to_vec()));
    assert_eq!(f(6), Some(b"\x1b[17~".to_vec()));
    assert_eq!(f(10), Some(b"\x1b[21~".to_vec()));
    assert_eq!(f(12), Some(b"\x1b[24~".to_vec()));
    assert_eq!(bytes(KeyCode::F(1), SHIFT), Some(b"\x1b[1;2P".to_vec()));
    assert_eq!(bytes(KeyCode::F(5), CTRL), Some(b"\x1b[15;5~".to_vec()));
    assert_eq!(f(13), None);
}

#[test]
fn keys_that_have_no_terminal_encoding_are_dropped() {
    assert_eq!(bytes(KeyCode::CapsLock, NONE), None);
    assert_eq!(bytes(KeyCode::Null, NONE), None);
    assert_eq!(bytes(KeyCode::Char('1'), CTRL), None);
}

#[test]
fn key_releases_are_not_sent() {
    let release = KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: NONE,
        kind: KeyEventKind::Release,
        state: KeyEventState::NONE,
    };
    let repeat = KeyEvent {
        kind: KeyEventKind::Repeat,
        ..release
    };

    assert_eq!(encode_key(release, false), None);
    assert_eq!(encode_key(repeat, false), Some(b"a".to_vec()));
}

#[test]
fn bracketed_paste_wraps_the_text() {
    assert_eq!(
        encode_paste("line one\nline two", true),
        b"\x1b[200~line one\nline two\x1b[201~".to_vec()
    );
}

#[test]
fn bracketed_paste_cannot_end_itself_early() {
    let hostile = "safe\x1b[201~rm -rf /";

    let sent = String::from_utf8(encode_paste(hostile, true)).unwrap();

    assert_eq!(sent.matches("\x1b[201~").count(), 1, "{sent:?}");
    assert!(sent.ends_with("\x1b[201~"));
    assert!(sent.contains("safe"));
}

#[test]
fn bracketed_paste_cannot_rebuild_the_end_marker_by_splicing() {
    // Removing only the marker would leave "ESC [ 2 0 1 ~" behind.
    let hostile = "\x1b[2\x1b[201~01~tail";

    let sent = String::from_utf8(encode_paste(hostile, true)).unwrap();

    assert_eq!(sent.matches("\x1b[201~").count(), 1, "{sent:?}");
}

#[test]
fn plain_paste_turns_newlines_into_carriage_returns() {
    assert_eq!(encode_paste("a\nb\r\nc", false), b"a\rb\rc".to_vec());
}

fn wait_for(pane: &mut AgentPane, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pane.screen_text().contains(expected) {
        pane.poll();
        assert!(
            Instant::now() < deadline,
            "never showed {expected:?}; screen was:\n{}",
            pane.screen_text()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_program_in_a_terminal_receives_exactly_these_bytes() {
    // `cat -v` prints control bytes as ^X, so the screen shows what arrived.
    let mut pane = AgentPane::spawn(
        &["sh", "-c", "stty raw -echo; echo ready; cat -v"].map(String::from),
        Path::new("."),
        10,
        60,
    )
    .unwrap();
    wait_for(&mut pane, "ready");

    for event in [
        key(KeyCode::Up, NONE),
        key(KeyCode::Char('c'), CTRL),
        key(KeyCode::Char('b'), ALT),
        key(KeyCode::Delete, NONE),
    ] {
        pane.send(&encode_key(event, false).unwrap()).unwrap();
    }

    wait_for(&mut pane, "^[[A^C^[b^[[3~");
}
