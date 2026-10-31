use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::Block,
};

pub const ASCII_BORDER: ratatui::symbols::border::Set<'static> = ratatui::symbols::border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

pub(crate) const ACCENT: Color = Color::Rgb(94, 234, 212);

pub(super) const BORDER: Color = Color::Rgb(71, 85, 105);

pub(crate) const MUTED: Color = Color::Rgb(148, 163, 184);

pub(super) const BAR_BACKGROUND: Color = Color::Rgb(30, 41, 59);

pub(crate) fn page_block(
    title: impl Into<Line<'static>>,
    border: ratatui::symbols::border::Set<'static>,
) -> Block<'static> {
    Block::bordered()
        .border_set(border)
        .border_style(Style::default().fg(BORDER))
        .title(title.into().style(Style::default().fg(ACCENT).bold()))
}

/// Rows `text` takes when wrapped to `width` columns. Counts by display
/// width, so word wrapping can take a row more on long lines.
pub(crate) fn wrapped_rows(text: &ratatui::text::Text<'_>, width: u16) -> usize {
    let width = usize::from(width).max(1);
    text.lines
        .iter()
        .map(|line| line.width().div_ceil(width).max(1))
        .sum()
}

pub(crate) fn selection_style() -> Style {
    Style::default()
        .bg(Color::Rgb(25, 43, 52))
        .fg(Color::Rgb(241, 245, 249))
        .bold()
}

/// A history row as drawn, with the characters the finder matches in the accent
/// colour and underlined. The underline survives the selection colours.
pub(crate) fn highlighted_history_row(
    row: &str,
    finder: &mut crate::history::MatchFinder,
) -> Line<'static> {
    let visible = crate::history::display_row(row);
    let positions = finder.positions(row);
    if positions.is_empty() {
        return Line::raw(visible.to_owned());
    }
    let matched = Style::default()
        .fg(ACCENT)
        .add_modifier(ratatui::style::Modifier::UNDERLINED);
    let mut spans: Vec<ratatui::text::Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_matched = false;
    for (index, character) in visible.char_indices() {
        let is_match = positions.binary_search(&index).is_ok();
        if is_match != run_matched && !run.is_empty() {
            let text = std::mem::take(&mut run);
            spans.push(if run_matched {
                ratatui::text::Span::styled(text, matched)
            } else {
                ratatui::text::Span::raw(text)
            });
        }
        run_matched = is_match;
        run.push(character);
    }
    if !run.is_empty() {
        spans.push(if run_matched {
            ratatui::text::Span::styled(run, matched)
        } else {
            ratatui::text::Span::raw(run)
        });
    }
    Line::from(spans)
}
