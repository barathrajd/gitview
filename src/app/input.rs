//! Decides which part of the screen an input event is for.
//!
//! Gitview is the only target today, so routing never changes anything. The
//! agent pane adds a second target here, with its own keys forwarded to it.

use crossterm::event::Event;

/// The part of the screen that receives input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Focus {
    #[default]
    Gitview,
}

/// The target for `event` given the current `focus`.
pub(super) fn route(focus: Focus, _event: &Event) -> Focus {
    match focus {
        Focus::Gitview => Focus::Gitview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    fn every_kind_of_event() -> Vec<Event> {
        vec![
            Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL)),
            Event::Paste("pasted text".to_owned()),
            Event::Resize(80, 24),
            Event::FocusGained,
            Event::FocusLost,
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 1,
                row: 1,
                modifiers: KeyModifiers::NONE,
            }),
        ]
    }

    #[test]
    fn gitview_is_the_default_focus() {
        assert_eq!(Focus::default(), Focus::Gitview);
    }

    #[test]
    fn every_event_goes_to_gitview_while_it_is_the_only_target() {
        for event in every_kind_of_event() {
            assert_eq!(route(Focus::Gitview, &event), Focus::Gitview, "{event:?}");
        }
    }
}
