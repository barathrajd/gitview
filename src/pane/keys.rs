//! Encodes input the way a terminal sends it to the program inside.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

const ESCAPE: u8 = 0x1b;
/// Codes for F5 to F12 in `CSI code ~` form; F1 to F4 use `SS3 P` to `SS3 S`.
const FUNCTION_CODES: [u8; 8] = [15, 17, 18, 19, 20, 21, 23, 24];

/// The bytes for `key`, or `None` for keys a terminal cannot express (releases,
/// lock keys, function keys past F12).
///
/// `application_cursor` is the program's DECCKM mode: arrows, Home, and End use
/// `SS3` instead of `CSI` while it is set, unless a modifier is held.
pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    // xterm's modifier parameter: 1 plus shift (1), alt (2), and control (4).
    let modifier = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    let with_escape = |mut bytes: Vec<u8>| {
        if alt {
            bytes.insert(0, ESCAPE);
        }
        bytes
    };

    match key.code {
        KeyCode::Char(c) => encode_char(c, ctrl).map(with_escape),
        KeyCode::Enter => Some(if alt || shift {
            vec![ESCAPE, b'\r']
        } else {
            vec![b'\r']
        }),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::BackTab => Some(vec![ESCAPE, b'[', b'Z']),
        KeyCode::Backspace => Some(with_escape(vec![if ctrl { 0x08 } else { 0x7f }])),
        KeyCode::Esc => Some(with_escape(vec![ESCAPE])),
        KeyCode::Up => Some(cursor_key(b'A', modifier, application_cursor)),
        KeyCode::Down => Some(cursor_key(b'B', modifier, application_cursor)),
        KeyCode::Right => Some(cursor_key(b'C', modifier, application_cursor)),
        KeyCode::Left => Some(cursor_key(b'D', modifier, application_cursor)),
        KeyCode::Home => Some(cursor_key(b'H', modifier, application_cursor)),
        KeyCode::End => Some(cursor_key(b'F', modifier, application_cursor)),
        KeyCode::Insert => Some(tilde_key(2, modifier)),
        KeyCode::Delete => Some(tilde_key(3, modifier)),
        KeyCode::PageUp => Some(tilde_key(5, modifier)),
        KeyCode::PageDown => Some(tilde_key(6, modifier)),
        KeyCode::F(n @ 1..=4) => {
            let letter = b'P' + (n - 1);
            Some(if modifier == 1 {
                vec![ESCAPE, b'O', letter]
            } else {
                format!("\x1b[1;{modifier}{}", char::from(letter)).into_bytes()
            })
        }
        KeyCode::F(n @ 5..=12) => Some(tilde_key(FUNCTION_CODES[usize::from(n - 5)], modifier)),
        _ => None,
    }
}

fn encode_char(c: char, ctrl: bool) -> Option<Vec<u8>> {
    if !ctrl {
        return Some(c.to_string().into_bytes());
    }
    let code = match c {
        'a'..='z' => c as u8 - b'a' + 1,
        'A'..='Z' => c as u8 - b'A' + 1,
        ' ' | '@' => 0,
        '[' => 0x1b,
        '\\' => 0x1c,
        ']' => 0x1d,
        '^' => 0x1e,
        '_' | '/' => 0x1f,
        '?' => 0x7f,
        _ => return None,
    };
    Some(vec![code])
}

fn cursor_key(letter: u8, modifier: u8, application_cursor: bool) -> Vec<u8> {
    if modifier > 1 {
        format!("\x1b[1;{modifier}{}", char::from(letter)).into_bytes()
    } else if application_cursor {
        vec![ESCAPE, b'O', letter]
    } else {
        vec![ESCAPE, b'[', letter]
    }
}

fn tilde_key(code: u8, modifier: u8) -> Vec<u8> {
    if modifier > 1 {
        format!("\x1b[{code};{modifier}~").into_bytes()
    } else {
        format!("\x1b[{code}~").into_bytes()
    }
}

/// The bytes for pasted `text`. With bracketed paste on, the text is wrapped so
/// the program can tell it from typing; escape bytes are dropped so pasted text
/// can't close the bracket early and run as commands. Otherwise newlines become
/// the carriage returns Enter would send.
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let safe: String = text.chars().filter(|&c| c != '\u{1b}').collect();
        format!("\x1b[200~{safe}\x1b[201~").into_bytes()
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}
