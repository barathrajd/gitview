use crate::ui::Viewport;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};

use super::theme::{ACCENT, BAR_BACKGROUND, BORDER};

/// The hints that fit on one row of `width` columns, in order. When some
/// don't fit, the row ends with `? more`, since help lists every key.
pub(super) fn hint_rows<'a>(
    hints: &'a [(&'a str, &'a str)],
    width: u16,
    compact: bool,
) -> Vec<Vec<(&'a str, &'a str)>> {
    if hints.is_empty() {
        return Vec::new();
    }
    let width = usize::from(width.max(1));
    let cost = |hint: &(&str, &str), first: bool| {
        hint.0.len() + hint.1.len() + if compact { 1 } else { 4 } + usize::from(!first) * 2
    };
    let total: usize = hints
        .iter()
        .enumerate()
        .map(|(index, hint)| cost(hint, index == 0))
        .sum();
    if total <= width {
        return vec![hints.to_vec()];
    }
    const MORE: (&str, &str) = ("?", "more");
    let mut row = Vec::new();
    let mut used = 0;
    for hint in hints.iter().filter(|hint| hint.0 != "?") {
        let next = cost(hint, row.is_empty());
        if used + next + cost(&MORE, false) > width {
            break;
        }
        used += next;
        row.push(*hint);
    }
    row.push(MORE);
    vec![row]
}

/// Rows a page's hint bar takes: one, or none without hints or while the
/// page's hints go to the workspace's bottom bar.
pub(crate) fn hint_height(frame: &Viewport<'_, '_>, hints: &[(&str, &str)]) -> u16 {
    u16::from(!hints.is_empty() && frame.hint_sink().is_none())
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
    if let Some(sink) = frame.hint_sink() {
        sink.borrow_mut().extend(
            hints
                .iter()
                .map(|(key, details)| ((*key).to_owned(), (*details).to_owned())),
        );
        return;
    }
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

/// Draw `hints` as the workspace's one-row bottom bar in `area`.
pub fn render_bar(
    frame: &mut Viewport<'_, '_>,
    area: ratatui::layout::Rect,
    hints: &[(String, String)],
) {
    let hints: Vec<(&str, &str)> = hints
        .iter()
        .map(|(key, details)| (key.as_str(), details.as_str()))
        .collect();
    render_key_hints_with_spacing(frame, area, &hints, false);
}
