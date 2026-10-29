//! Where Gitview and the agent pane go for a given terminal size.

use gitview::ui::{AgentLayout, plan_agent_layout};
use ratatui::layout::Rect;

fn total(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}

#[test]
fn without_an_agent_gitview_has_the_whole_screen() {
    for focused in [false, true] {
        assert_eq!(
            plan_agent_layout(total(100, 30), false, focused),
            AgentLayout::GitviewOnly(total(100, 30))
        );
    }
}

#[test]
fn a_wide_terminal_puts_the_agent_beside_gitview_whoever_has_focus() {
    for focused in [false, true] {
        assert_eq!(
            plan_agent_layout(total(140, 40), true, focused),
            AgentLayout::SideBySide {
                gitview: Rect::new(0, 0, 70, 40),
                agent: Rect::new(70, 0, 70, 40),
            }
        );
    }
}

#[test]
fn sharing_starts_at_119_columns() {
    // The agent gets half, rounded down, so Gitview keeps the larger half and
    // its 60 columns: 60 + 59 fits, 59 + 59 does not.
    assert_eq!(
        plan_agent_layout(total(119, 30), true, false),
        AgentLayout::SideBySide {
            gitview: Rect::new(0, 0, 60, 30),
            agent: Rect::new(60, 0, 59, 30),
        }
    );
    assert!(matches!(
        plan_agent_layout(total(118, 30), true, false),
        AgentLayout::GitviewWithBar { .. }
    ));
}

#[test]
fn a_narrow_terminal_shows_gitview_with_a_bar_for_the_agent() {
    assert_eq!(
        plan_agent_layout(total(80, 24), true, false),
        AgentLayout::GitviewWithBar {
            gitview: Rect::new(0, 0, 80, 23),
            bar: Rect::new(0, 23, 80, 1),
        }
    );
}

#[test]
fn a_narrow_terminal_gives_the_agent_the_whole_screen_when_it_has_focus() {
    assert_eq!(
        plan_agent_layout(total(80, 24), true, true),
        AgentLayout::AgentFull(total(80, 24))
    );
}

#[test]
fn a_terminal_too_short_for_a_bar_shows_gitview_alone() {
    assert_eq!(
        plan_agent_layout(total(80, 2), true, false),
        AgentLayout::GitviewOnly(total(80, 2))
    );
}

#[test]
fn an_offset_area_keeps_its_origin() {
    let area = Rect::new(5, 3, 80, 24);

    assert_eq!(
        plan_agent_layout(area, true, false),
        AgentLayout::GitviewWithBar {
            gitview: Rect::new(5, 3, 80, 23),
            bar: Rect::new(5, 26, 80, 1),
        }
    );
}
