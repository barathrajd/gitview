//! xterm mouse reports translated into the child terminal's coordinates.
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use vt100::{MouseProtocolEncoding as Encoding, MouseProtocolMode as Mode};

pub fn encode_mouse(
    event: MouseEvent,
    area: Rect,
    mode: Mode,
    encoding: Encoding,
) -> Option<Vec<u8>> {
    if mode == Mode::None || !area.contains(Position::new(event.column, event.row)) {
        return None;
    }
    let button = |button| match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    };
    let (mut code, release) = match event.kind {
        MouseEventKind::Down(b) => (button(b), false),
        MouseEventKind::Up(b) if mode != Mode::Press => (button(b), true),
        MouseEventKind::Drag(b) if matches!(mode, Mode::ButtonMotion | Mode::AnyMotion) => {
            (button(b) + 32, false)
        }
        MouseEventKind::Moved if mode == Mode::AnyMotion => (35, false),
        MouseEventKind::ScrollUp => (64, false),
        MouseEventKind::ScrollDown => (65, false),
        MouseEventKind::ScrollLeft => (66, false),
        MouseEventKind::ScrollRight => (67, false),
        _ => return None,
    };
    if mode != Mode::Press {
        code += u16::from(event.modifiers.contains(KeyModifiers::SHIFT)) * 4
            + u16::from(event.modifiers.contains(KeyModifiers::ALT)) * 8
            + u16::from(event.modifiers.contains(KeyModifiers::CONTROL)) * 16;
    }
    let x = event.column - area.x + 1;
    let y = event.row - area.y + 1;
    if encoding == Encoding::Sgr {
        return Some(
            format!("\x1b[<{code};{x};{y}{}", if release { 'm' } else { 'M' }).into_bytes(),
        );
    }
    if release {
        code = 3 + (code & 28);
    }
    let limit = if encoding == Encoding::Utf8 {
        2015
    } else {
        223
    };
    if x > limit || y > limit {
        return None;
    }
    let mut bytes = b"\x1b[M".to_vec();
    for value in [code + 32, x + 32, y + 32] {
        if encoding == Encoding::Utf8 {
            let mut buffer = [0; 4];
            bytes.extend_from_slice(
                char::from_u32(u32::from(value))?
                    .encode_utf8(&mut buffer)
                    .as_bytes(),
            );
        } else {
            bytes.push(value as u8);
        }
    }
    Some(bytes)
}
