use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use gitview::pane::encode_mouse;
use ratatui::layout::Rect;
use vt100::{MouseProtocolEncoding as Encoding, MouseProtocolMode as Mode};

fn event(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        column: 73,
        row: 3,
        modifiers: KeyModifiers::NONE,
    }
}
const AREA: Rect = Rect::new(71, 1, 68, 28);

#[test]
fn sgr_mouse_uses_coordinates_relative_to_the_agent() {
    let mut mouse = event(MouseEventKind::Down(MouseButton::Left));
    assert_eq!(
        encode_mouse(mouse, AREA, Mode::PressRelease, Encoding::Sgr),
        Some(b"\x1b[<0;3;3M".to_vec())
    );
    mouse.kind = MouseEventKind::Up(MouseButton::Left);
    assert_eq!(
        encode_mouse(mouse, AREA, Mode::PressRelease, Encoding::Sgr),
        Some(b"\x1b[<0;3;3m".to_vec())
    );
    mouse.kind = MouseEventKind::ScrollDown;
    mouse.modifiers = KeyModifiers::CONTROL;
    assert_eq!(
        encode_mouse(mouse, AREA, Mode::PressRelease, Encoding::Sgr),
        Some(b"\x1b[<81;3;3M".to_vec())
    );
}

#[test]
fn mouse_modes_and_pane_bounds_filter_events() {
    assert!(
        encode_mouse(
            event(MouseEventKind::ScrollUp),
            AREA,
            Mode::None,
            Encoding::Sgr
        )
        .is_none()
    );
    let mut mouse = event(MouseEventKind::Down(MouseButton::Left));
    mouse.column = 70;
    assert!(encode_mouse(mouse, AREA, Mode::AnyMotion, Encoding::Sgr).is_none());
    assert!(
        encode_mouse(
            event(MouseEventKind::Moved),
            AREA,
            Mode::PressRelease,
            Encoding::Sgr
        )
        .is_none()
    );
    assert!(
        encode_mouse(
            event(MouseEventKind::Up(MouseButton::Left)),
            AREA,
            Mode::Press,
            Encoding::Sgr
        )
        .is_none()
    );
    assert_eq!(
        encode_mouse(
            event(MouseEventKind::Drag(MouseButton::Right)),
            AREA,
            Mode::ButtonMotion,
            Encoding::Sgr
        ),
        Some(b"\x1b[<34;3;3M".to_vec())
    );
}

#[test]
fn legacy_and_utf8_mouse_encodings_are_bounded() {
    assert_eq!(
        encode_mouse(
            event(MouseEventKind::Down(MouseButton::Left)),
            AREA,
            Mode::PressRelease,
            Encoding::Default
        ),
        Some(b"\x1b[M ##".to_vec())
    );
    let mouse = MouseEvent {
        column: 300,
        row: 1,
        ..event(MouseEventKind::Down(MouseButton::Left))
    };
    let area = Rect::new(0, 0, 500, 10);
    assert!(encode_mouse(mouse, area, Mode::PressRelease, Encoding::Default).is_none());
    assert!(encode_mouse(mouse, area, Mode::PressRelease, Encoding::Utf8).is_some());
}
