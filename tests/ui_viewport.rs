//! The layout slot: Gitview can render into a sub-area of the frame, and with
//! no side pane requested the layout is the whole frame, unchanged.

use std::{ffi::OsString, path::Path};

use gitview::{
    app::View,
    branches::{Branches, HeadState},
    ui::{Cockpit, Screen, Viewport, render, render_in, split_side_pane},
    worktree::{Entry, Section},
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, widgets::ListState};

fn draw(width: u16, height: u16, draw_screen: impl FnOnce(&mut ratatui::Frame<'_>)) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(draw_screen).unwrap();
    terminal.backend().buffer().clone()
}

fn with_cockpit(draw_it: impl FnOnce(Screen<'_>)) {
    let branches = Ok(Branches {
        head: HeadState::Attached(OsString::from("main")),
        branches: vec![OsString::from("main")],
        head_commit: Some("abcdef0123456789".to_owned()),
    });
    let history = Ok(vec!["* abcdef0 Ship the cockpit".to_owned()]);
    let status = Ok(vec![Entry {
        section: Section::Unstaged,
        path: OsString::from("src/app.rs"),
        original: None,
    }]);
    let mut history_selection = ListState::default();
    history_selection.select(Some(0));
    let mut working_tree_selection = ListState::default();
    working_tree_selection.select(Some(0));
    draw_it(Screen::Cockpit(Cockpit {
        repository: Path::new("/work/gitview"),
        profile: "office",
        branches: Some(&branches),
        history: Some(&history),
        status: Some(&status),
        feedback: "",
        focused: View::History,
        history_selection: &mut history_selection,
        working_tree_selection: &mut working_tree_selection,
    }));
}

fn text_in(buffer: &Buffer, area: Rect) -> String {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const BORDER: ratatui::symbols::border::Set<'static> = ratatui::symbols::border::PLAIN;

#[test]
fn the_full_viewport_renders_exactly_like_the_plain_frame() {
    let mut plain = None;
    with_cockpit(|screen| {
        plain = Some(draw(120, 24, |frame| render(frame, screen, BORDER)));
    });
    let mut through_viewport = None;
    with_cockpit(|screen| {
        through_viewport = Some(draw(120, 24, |frame| {
            let area = frame.area();
            render_in(&mut Viewport::new(frame, area), screen, BORDER);
        }));
    });

    assert_eq!(plain, through_viewport);
}

#[test]
fn a_sub_area_viewport_draws_only_inside_that_area() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = None;
    with_cockpit(|screen| {
        buffer = Some(draw(120, 24, |frame| {
            render_in(&mut Viewport::new(frame, area), screen, BORDER);
        }));
    });
    let buffer = buffer.unwrap();

    for y in 0..24 {
        for x in 80..120 {
            assert_eq!(
                buffer[(x, y)].symbol(),
                " ",
                "cell ({x}, {y}) is outside the area"
            );
        }
    }
    let inside = text_in(&buffer, Rect::new(0, 0, 80, 24));
    assert!(inside.contains("GITVIEW"), "{inside}");
    assert!(inside.contains("Ship the cockpit"), "{inside}");
    let last_column: String = (0..24)
        .map(|y| buffer[(79, y)].symbol().to_owned())
        .collect();
    assert!(
        last_column.contains('│') || last_column.contains('┐'),
        "{last_column}"
    );
}

#[test]
fn a_viewport_offset_from_the_origin_keeps_the_screen_inside_it() {
    let area = Rect::new(40, 0, 80, 24);
    let mut buffer = None;
    with_cockpit(|screen| {
        buffer = Some(draw(120, 24, |frame| {
            render_in(&mut Viewport::new(frame, area), screen, BORDER);
        }));
    });
    let buffer = buffer.unwrap();

    for y in 0..24 {
        for x in 0..40 {
            assert_eq!(
                buffer[(x, y)].symbol(),
                " ",
                "cell ({x}, {y}) is outside the area"
            );
        }
    }
    let inside = text_in(&buffer, area);
    assert!(inside.contains("GITVIEW"), "{inside}");
    assert!(inside.contains("Ship the cockpit"), "{inside}");
}

#[test]
fn no_side_pane_requested_leaves_the_whole_area_to_gitview() {
    let area = Rect::new(0, 0, 120, 30);

    assert_eq!(split_side_pane(area, None), (area, None));
}

#[test]
fn a_side_pane_takes_columns_from_the_right() {
    let area = Rect::new(0, 0, 120, 30);

    let (main, side) = split_side_pane(area, Some(50));

    assert_eq!(main, Rect::new(0, 0, 70, 30));
    assert_eq!(side, Some(Rect::new(70, 0, 50, 30)));
}

#[test]
fn a_side_pane_is_refused_when_it_would_squeeze_gitview_too_narrow() {
    let area = Rect::new(0, 0, 70, 30);

    assert_eq!(split_side_pane(area, Some(50)), (area, None));
}

#[test]
fn a_side_pane_is_refused_when_it_would_be_too_narrow_to_use() {
    let area = Rect::new(0, 0, 120, 30);

    assert_eq!(split_side_pane(area, Some(5)), (area, None));
}

#[test]
fn widgets_are_clipped_to_the_viewport() {
    let buffer = draw(120, 10, |frame| {
        let mut viewport = Viewport::new(frame, Rect::new(0, 0, 40, 10));
        // Asks for the whole frame, but only 40 columns belong to the viewport.
        viewport.render_widget(
            ratatui::widgets::Block::bordered(),
            Rect::new(0, 0, 120, 10),
        );
    });

    assert_eq!(buffer[(39, 0)].symbol(), "┐");
    assert_eq!(buffer[(119, 0)].symbol(), " ");
}

#[test]
fn a_viewport_is_cut_down_to_the_frame() {
    let buffer = draw(50, 10, |frame| {
        let mut viewport = Viewport::new(frame, Rect::new(0, 0, 200, 50));
        assert_eq!(viewport.area(), Rect::new(0, 0, 50, 10));
        viewport.render_widget(ratatui::widgets::Block::bordered(), viewport.area());
    });

    assert_eq!(buffer[(49, 9)].symbol(), "┘");
}

#[test]
fn stateful_widgets_are_clipped_to_the_viewport() {
    let buffer = draw(120, 10, |frame| {
        let mut viewport = Viewport::new(frame, Rect::new(0, 0, 40, 10));
        let list =
            ratatui::widgets::List::new(["one", "two"]).block(ratatui::widgets::Block::bordered());
        viewport.render_stateful_widget(list, Rect::new(0, 0, 120, 10), &mut ListState::default());
    });

    assert_eq!(buffer[(39, 0)].symbol(), "┐");
    assert_eq!(buffer[(119, 0)].symbol(), " ");
}
