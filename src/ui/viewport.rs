use ratatui::{
    Frame,
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};

/// The narrowest Gitview is given when a side pane shares the screen.
const MIN_MAIN_WIDTH: u16 = 60;
/// The narrowest side pane worth showing.
const MIN_SIDE_WIDTH: u16 = 20;

/// A frame limited to a sub-area. Rendering code asks it for `area()` instead
/// of the whole frame, so the same screens can fill the full terminal or share
/// it with a side pane. Widgets are clipped to the area.
pub struct Viewport<'a, 'f> {
    frame: &'a mut Frame<'f>,
    area: Rect,
}

impl<'a, 'f> Viewport<'a, 'f> {
    /// Limits `frame` to `area`, which is cut down to the frame's own bounds.
    pub fn new(frame: &'a mut Frame<'f>, area: Rect) -> Self {
        let area = area.intersection(frame.area());
        Self { frame, area }
    }

    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        self.frame
            .render_widget(widget, area.intersection(self.area));
    }

    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) {
        self.frame
            .render_stateful_widget(widget, area.intersection(self.area), state);
    }
}

/// Splits `area` into Gitview's part and, when `side_width` is given, a side
/// pane on the right. The split is refused (Gitview keeps the whole area)
/// when either part would be too narrow to use.
pub fn split_side_pane(area: Rect, side_width: Option<u16>) -> (Rect, Option<Rect>) {
    let Some(side_width) = side_width else {
        return (area, None);
    };
    let main_width = area.width.saturating_sub(side_width);
    if side_width < MIN_SIDE_WIDTH || main_width < MIN_MAIN_WIDTH {
        return (area, None);
    }
    let main = Rect {
        width: main_width,
        ..area
    };
    let side = Rect {
        x: area.x + main_width,
        width: side_width,
        ..area
    };
    (main, Some(side))
}
