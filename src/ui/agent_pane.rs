use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Widget},
};

use unicode_width::UnicodeWidthStr;

use crate::pane::{AgentPane, FOCUS_KEY_LABEL, PaneState};

use super::{
    theme::{ACCENT, MUTED},
    viewport::Viewport,
};

/// The emulated screen of an agent pane, drawn cell by cell.
struct PaneScreen<'a>(&'a vt100::Screen);

fn map_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(index) => Color::Indexed(index),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn cell_style(cell: &vt100::Cell) -> Style {
    let mut modifier = Modifier::empty();
    for (set, flag) in [
        (cell.bold(), Modifier::BOLD),
        (cell.dim(), Modifier::DIM),
        (cell.italic(), Modifier::ITALIC),
        (cell.underline(), Modifier::UNDERLINED),
        (cell.inverse(), Modifier::REVERSED),
    ] {
        if set {
            modifier |= flag;
        }
    }
    Style::default()
        .fg(map_color(cell.fgcolor()))
        .bg(map_color(cell.bgcolor()))
        .add_modifier(modifier)
}

impl Widget for PaneScreen<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let (rows, columns) = self.0.size();
        let (rows, columns) = (rows.get(), columns.get());
        for row in 0..rows.min(area.height) {
            for column in 0..columns.min(area.width) {
                let Some(cell) = self.0.cell(row, column) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let target = &mut buffer[(area.x + column, area.y + row)];
                target.set_style(cell_style(cell));
                if cell.has_contents() {
                    target.set_symbol(cell.contents());
                }
            }
        }
    }
}

fn title(pane: &AgentPane) -> String {
    let status = match pane.state() {
        PaneState::Running => "running".to_owned(),
        PaneState::Exited(Some(code)) => format!("exited {code}"),
        PaneState::Exited(None) => "ended by signal".to_owned(),
    };
    format!(" AGENT · {} · {status} ", pane.command())
}

/// The reminder of the focus key, as long as fits beside the title: the status
/// matters more than the hint, so the hint shrinks, then disappears.
fn focus_hint(title: &str, width: u16, focused: bool) -> String {
    let room = usize::from(width.saturating_sub(2));
    let action = if focused {
        "returns to Gitview"
    } else {
        "focuses the agent"
    };
    [
        format!(" {FOCUS_KEY_LABEL} {action} "),
        format!(" {FOCUS_KEY_LABEL} "),
    ]
    .into_iter()
    .find(|hint| title.width() + hint.width() <= room)
    .unwrap_or_default()
}

fn frame_block(title: String, hint: String, color: Color) -> Block<'static> {
    let style = Style::default().fg(color);
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(Line::from(title).style(style))
        .title(Line::from(hint).right_aligned().style(style))
}

/// The part of `area` where the program's screen is drawn, inside the border.
/// The program should be told this size.
pub fn agent_pane_inner(area: Rect) -> Rect {
    frame_block(String::new(), String::new(), Color::Reset).inner(area)
}

/// Draws `pane` in `area` with a border, and puts the terminal cursor where the
/// program's cursor is when `focused`.
pub fn render_agent_pane(
    viewport: &mut Viewport<'_, '_>,
    area: Rect,
    pane: &AgentPane,
    focused: bool,
) {
    let border_color = if focused { ACCENT } else { MUTED };
    let title = title(pane);
    let hint = focus_hint(&title, area.width, focused);
    let block = frame_block(title, hint, border_color);
    let inner = block.inner(area);
    viewport.render_widget(block, area);
    viewport.render_widget(PaneScreen(pane.screen()), inner);

    let screen = pane.screen();
    if focused && !screen.hide_cursor() {
        let (row, column) = screen.cursor_position();
        if row < inner.height && column < inner.width {
            viewport.set_cursor_position(Position::new(inner.x + column, inner.y + row));
        }
    }
}
