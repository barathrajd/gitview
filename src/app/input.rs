//! Decides which part of the screen an input event is for.
//!
//! Gitview and, when one is open, the agent pane can have focus. The focus key
//! moves it. While the agent has focus every key and paste goes to the agent
//! through the agent handler, which reserves workspace Ctrl-w chords and
//! forwards other sequences. The focus key always stays with Gitview.

use crossterm::event::Event;

use crate::pane::is_focus_key;

/// The part of the screen that receives input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Focus {
    #[default]
    Gitview,
    Agent,
}

/// Where one input event goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Route {
    Gitview,
    Agent,
    /// Move focus to the other side.
    ToggleFocus,
}

/// The route for `event`. `has_pane` is whether an agent was started; without
/// one, everything stays with Gitview. With one, the focus key always reaches
/// it: side by side, or by swapping full screens on a narrow terminal.
pub(super) fn route(focus: Focus, has_pane: bool, event: &Event) -> Route {
    if !has_pane {
        return Route::Gitview;
    }
    if let Event::Key(key) = event
        && is_focus_key(key)
    {
        return Route::ToggleFocus;
    }
    match (focus, event) {
        (Focus::Agent, Event::Key(_) | Event::Paste(_) | Event::Mouse(_)) => Route::Agent,
        _ => Route::Gitview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
    };

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn ordinary_events() -> Vec<Event> {
        vec![
            key(KeyCode::Char('q'), KeyModifiers::NONE),
            key(KeyCode::Esc, KeyModifiers::NONE),
            key(KeyCode::Tab, KeyModifiers::NONE),
            key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            key(KeyCode::Char(']'), KeyModifiers::CONTROL),
            key(KeyCode::Char('5'), KeyModifiers::CONTROL),
            Event::Paste("pasted text".to_owned()),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 1,
                row: 1,
                modifiers: KeyModifiers::NONE,
            }),
        ]
    }

    fn passive_events() -> Vec<Event> {
        vec![Event::Resize(80, 24), Event::FocusGained, Event::FocusLost]
    }

    fn focus_keys() -> Vec<Event> {
        vec![
            key(KeyCode::Char('\\'), KeyModifiers::CONTROL),
            // Legacy terminals report byte 0x1c as Ctrl+4.
            key(KeyCode::Char('4'), KeyModifiers::CONTROL),
        ]
    }

    #[test]
    fn gitview_is_the_default_focus() {
        assert_eq!(Focus::default(), Focus::Gitview);
    }

    #[test]
    fn without_a_pane_everything_goes_to_gitview() {
        for focus in [Focus::Gitview, Focus::Agent] {
            for event in ordinary_events()
                .into_iter()
                .chain(passive_events())
                .chain(focus_keys())
            {
                assert_eq!(route(focus, false, &event), Route::Gitview, "{event:?}");
            }
        }
    }

    #[test]
    fn while_gitview_has_focus_nothing_reaches_the_agent() {
        for event in ordinary_events().into_iter().chain(passive_events()) {
            assert_eq!(
                route(Focus::Gitview, true, &event),
                Route::Gitview,
                "{event:?}"
            );
        }
    }

    #[test]
    fn the_focus_key_toggles_focus_from_either_side() {
        for focus in [Focus::Gitview, Focus::Agent] {
            for event in focus_keys() {
                assert_eq!(route(focus, true, &event), Route::ToggleFocus, "{event:?}");
            }
        }
    }

    #[test]
    fn while_the_agent_has_focus_every_other_key_and_paste_reaches_it() {
        for event in ordinary_events() {
            assert_eq!(route(Focus::Agent, true, &event), Route::Agent, "{event:?}");
        }
    }

    #[test]
    fn terminal_events_that_are_not_input_stay_with_gitview() {
        for event in passive_events() {
            assert_eq!(
                route(Focus::Agent, true, &event),
                Route::Gitview,
                "{event:?}"
            );
        }
    }

    #[test]
    fn releasing_the_focus_key_does_not_toggle_focus() {
        let release = Event::Key(KeyEvent {
            code: KeyCode::Char('\\'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });

        assert_eq!(route(Focus::Gitview, true, &release), Route::Gitview);
        assert_eq!(route(Focus::Agent, true, &release), Route::Agent);
    }
}
