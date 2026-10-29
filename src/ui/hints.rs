use crate::ui::Viewport;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

use super::theme::{ACCENT, BAR_BACKGROUND, BORDER};

pub(super) fn hint_rows<'a>(
    hints: &'a [(&'a str, &'a str)],
    width: u16,
    compact: bool,
) -> Vec<Vec<(&'a str, &'a str)>> {
    let width = usize::from(width.max(1));
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut row_width = 0usize;
    for hint in hints {
        let hint_width = hint.0.len() + hint.1.len() + if compact { 1 } else { 4 };
        let separator = usize::from(!row.is_empty()) * 2;
        if !row.is_empty() && row_width + separator + hint_width > width {
            rows.push(std::mem::take(&mut row));
            row_width = 0;
        }
        row_width += usize::from(!row.is_empty()) * 2 + hint_width;
        row.push(*hint);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

pub(crate) fn render_key_hints(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    hints: &[(&str, &str)],
) {
    render_key_hints_with_spacing(frame, area, hints, area.width <= 80);
}

pub(super) fn render_key_hints_padded(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    hints: &[(&str, &str)],
) {
    render_key_hints_with_spacing(frame, area, hints, false);
}

pub(super) fn render_key_hints_with_spacing(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    hints: &[(&str, &str)],
    compact: bool,
) {
    let rows = hint_rows(hints, area.width, compact);
    let lines = rows
        .iter()
        .map(|row| {
            let mut spans = Vec::with_capacity(row.len() * 3);
            for (index, (key, details)) in row.iter().enumerate() {
                if index > 0 {
                    spans.push(Span::styled(
                        "  ",
                        Style::default().fg(BORDER).bg(BAR_BACKGROUND),
                    ));
                }
                spans.push(Span::styled(
                    if compact {
                        (*key).to_owned()
                    } else {
                        format!(" {key} ")
                    },
                    Style::default()
                        .fg(Color::Rgb(12, 24, 30))
                        .bg(ACCENT)
                        .bold(),
                ));
                spans.push(Span::styled(
                    if compact {
                        format!(" {details}")
                    } else {
                        format!(" {details} ")
                    },
                    Style::default()
                        .fg(Color::Rgb(226, 232, 240))
                        .bg(BAR_BACKGROUND),
                ));
            }
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Text::from(lines)).style(Style::default().bg(BAR_BACKGROUND)),
        area,
    );
}

pub(crate) fn hint_row_count(hints: &[(&str, &str)], width: u16) -> u16 {
    u16::try_from(hint_rows(hints, width, width <= 80).len()).unwrap_or(u16::MAX)
}
