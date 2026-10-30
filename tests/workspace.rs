use gitview::workspace::{Panel, Workspace};
use ratatui::layout::Rect;

#[test]
fn hiding_focus_selects_a_visible_panel_and_keeps_one_panel() {
    let mut workspace = Workspace::new(true);
    assert!(workspace.focus(Panel::Agent));
    assert!(workspace.hide(Panel::Agent));
    assert_eq!(workspace.focused(), Panel::Center);
    assert!(!workspace.focus(Panel::Agent));
    assert!(workspace.hide(Panel::Sidebar));
    assert!(!workspace.hide(Panel::Center));
    workspace.show(Panel::Agent);
    assert!(workspace.focus(Panel::Agent));
}

#[test]
fn absent_agent_cannot_be_shown_or_focused() {
    let mut workspace = Workspace::new(false);
    workspace.show(Panel::Agent);
    assert!(!workspace.focus(Panel::Agent));
    assert!(workspace.layout(Rect::new(0, 0, 160, 40)).agent.is_none());
}

#[test]
fn narrow_layout_shows_only_the_focused_panel_above_the_bottom_bar() {
    let mut workspace = Workspace::new(true);
    workspace.focus(Panel::Sidebar);
    let layout = workspace.layout(Rect::new(2, 3, 118, 30));
    assert_eq!(layout.sidebar, Some(Rect::new(2, 3, 118, 29)));
    assert_eq!(layout.center, None);
    assert_eq!(layout.agent, None);
    assert_eq!(layout.bar, Rect::new(2, 32, 118, 1));
}

#[test]
fn resizing_preserves_bounds_and_center_space() {
    let mut workspace = Workspace::new(true);
    workspace.resize(Panel::Sidebar, i16::MAX);
    workspace.resize(Panel::Agent, i16::MAX);
    let area = Rect::new(5, 7, 119, 24);
    let layout = workspace.layout(area);
    let sidebar = layout.sidebar.unwrap();
    let center = layout.center.unwrap();
    let agent = layout.agent.unwrap();
    assert!(center.width >= 40);
    assert_eq!(sidebar.x, area.x);
    assert_eq!(sidebar.right(), center.x);
    assert_eq!(center.right(), agent.x);
    assert_eq!(agent.right(), area.right());
    assert_eq!(center.bottom(), layout.bar.y);
}

#[test]
fn tiny_layout_never_leaves_its_area() {
    let workspace = Workspace::new(false);
    for width in 0..3 {
        for height in 0..3 {
            let area = Rect::new(4, 5, width, height);
            let layout = workspace.layout(area);
            for rect in [
                layout.sidebar,
                layout.center,
                layout.agent,
                Some(layout.bar),
            ]
            .into_iter()
            .flatten()
            {
                assert_eq!(area.intersection(rect), rect);
            }
        }
    }
}

#[test]
fn center_can_hide_when_another_panel_remains() {
    let mut workspace = Workspace::new(false);
    assert!(workspace.hide(Panel::Center));
    assert_eq!(workspace.focused(), Panel::Sidebar);
    assert!(!workspace.hide(Panel::Sidebar));
    let layout = workspace.layout(Rect::new(0, 0, 140, 30));
    assert!(layout.center.is_none());
    assert_eq!(layout.sidebar.unwrap().width, 140);
}

#[test]
fn fullscreen_restores_sizes_and_hiding_exits_fullscreen() {
    let mut workspace = Workspace::new(true);
    let area = Rect::new(0, 0, 160, 30);
    let original = workspace.layout(area).sidebar.unwrap();
    workspace.focus(Panel::Sidebar);
    workspace.toggle_fullscreen();
    assert_eq!(workspace.layout(area).sidebar.unwrap().width, 160);
    assert!(workspace.layout(area).center.is_none());
    assert!(workspace.layout(area).agent.is_none());
    workspace.toggle_fullscreen();
    assert_eq!(workspace.layout(area).sidebar.unwrap(), original);
    workspace.focus(Panel::Agent);
    workspace.toggle_fullscreen();
    assert_eq!(workspace.layout(area).agent.unwrap().width, 160);
    workspace.hide(Panel::Agent);
    assert!(!workspace.is_fullscreen());
}
