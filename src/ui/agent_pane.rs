use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Widget},
};

use crate::pane::{AgentPane, PaneState};

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

/// Draws `pane` in `area` with a border, and puts the terminal cursor where the
/// program's cursor is when `focused`.
pub fn render_agent_pane(
    viewport: &mut Viewport<'_, '_>,
    area: Rect,
    pane: &AgentPane,
    focused: bool,
) {
    let border_color = if focused { ACCENT } else { MUTED };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title(pane)).style(Style::default().fg(border_color)));
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
