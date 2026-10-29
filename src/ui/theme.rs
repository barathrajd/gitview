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

pub(crate) fn selection_style() -> Style {
    Style::default()
        .bg(Color::Rgb(25, 43, 52))
        .fg(Color::Rgb(241, 245, 249))
        .bold()
}
